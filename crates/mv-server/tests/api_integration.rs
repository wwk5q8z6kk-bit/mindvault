//! Full-stack REST API integration tests.
//!
//! Each test spins up a real MindVaultEngine backed by a tempdir, constructs
//! the axum Router, and sends actual HTTP requests via `tower::ServiceExt`.
//! This validates routing, serialisation, handler logic, and storage in one pass.
//!
//! NOTE: Tests must run sequentially (`--test-threads=1`) because the fastembed
//! ort runtime uses a global mutex that poisons if any test panics.

use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
use tempfile::TempDir;
use tower::ServiceExt; // for `.oneshot()`

use mv_engine::config::EngineConfig;
use mv_engine::engine::MindVaultEngine;
use mv_server::rest::create_router;
use mv_server::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_config(data_dir: &str) -> EngineConfig {
    let mut cfg = EngineConfig {
        data_dir: data_dir.to_string(),
        ..Default::default()
    };
    cfg.embedding.provider = "noop".to_string();
    cfg
}

async fn setup() -> (axum::Router, TempDir) {
    for key in [
        "MINDVAULT_AUTH_TOKEN",
        "MINDVAULT_AUTH_ROLE",
        "MINDVAULT_AUTH_NAMESPACE",
        "MINDVAULT_JWT_SECRET",
        "MINDVAULT_JWT_ISSUER",
        "MINDVAULT_JWT_AUDIENCE",
        "MINDVAULT_NAMESPACE_NODE_QUOTA",
        "MINDVAULT_WORKSPACE_ALLOWED_ROOTS",
        "MINDVAULT_COMMAND_ADMISSION_MODE",
    ] {
        std::env::remove_var(key);
    }
    let tmp = TempDir::new().expect("tempdir");
    let config = test_config(&tmp.path().to_string_lossy());
    setup_with_config(config, tmp).await
}

async fn setup_with_config(config: EngineConfig, tmp: TempDir) -> (axum::Router, TempDir) {
    // The request rate limiter is a process-global `OnceLock` (`limits.rs`)
    // holding one bucket for the entire suite, defaulting to 120 requests per
    // 60 seconds. This suite runs serially in a single process and finishes
    // well inside that window, so without a raised ceiling the whole run sits
    // near the limit and *adding a test* makes some unrelated later test fail
    // with 429. Raise it here, before the first request initializes the lock.
    //
    // This does not weaken any assertion: the two tests in this workspace that
    // expect 429 are exercising the namespace node quota, not the rate limiter.
    std::env::set_var("MINDVAULT_RATE_LIMIT_REQUESTS", "1000000");

    let engine = MindVaultEngine::init(config).await.expect("engine init");
    let state = Arc::new(AppState::new(Arc::new(engine)));
    let router = create_router(state);
    (router, tmp)
}

async fn setup_with_workspace_root() -> (axum::Router, TempDir, std::path::PathBuf) {
    for key in [
        "MINDVAULT_AUTH_TOKEN",
        "MINDVAULT_AUTH_ROLE",
        "MINDVAULT_AUTH_NAMESPACE",
        "MINDVAULT_JWT_SECRET",
        "MINDVAULT_JWT_ISSUER",
        "MINDVAULT_JWT_AUDIENCE",
        "MINDVAULT_NAMESPACE_NODE_QUOTA",
        "MINDVAULT_WORKSPACE_ALLOWED_ROOTS",
        "MINDVAULT_COMMAND_ADMISSION_MODE",
    ] {
        std::env::remove_var(key);
    }
    let tmp = TempDir::new().expect("tempdir");
    let workspace_root = tmp.path().join("knowledge");
    std::fs::create_dir_all(workspace_root.join("Projects/Empty")).unwrap();
    std::fs::write(
        workspace_root.join("Projects/MindVault.md"),
        "# MindVault\n\nFile-first knowledge.",
    )
    .unwrap();
    let data_dir = tmp.path().join("data");
    let config = test_config(&data_dir.to_string_lossy());
    let engine = MindVaultEngine::init(config).await.expect("engine init");
    let state = Arc::new(
        AppState::new(Arc::new(engine))
            .with_workspace_allowed_roots(vec![tmp.path().to_path_buf()]),
    );
    (create_router(state), tmp, workspace_root)
}

fn json_request(method: Method, uri: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    match body {
        Some(val) => builder.body(Body::from(val.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()))
}

// ---------------------------------------------------------------------------
// Mounted knowledge workspaces
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workspace_api_mounts_lists_trees_and_reads_without_exposing_root_locator() {
    let (router, _tmp, workspace_root) = setup_with_workspace_root().await;
    let mount = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/workspaces",
            Some(json!({
                "root_path": workspace_root,
                "namespace": "default",
                "display_name": "Knowledge"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(mount.status(), StatusCode::CREATED);
    let mounted = body_json(mount).await;
    let workspace_id = mounted["workspace"]["id"].as_str().expect("workspace id");
    assert_eq!(mounted["workspace"]["display_name"], "Knowledge");
    assert!(mounted["workspace"].get("root_path").is_none());
    assert_eq!(mounted["reconciliation"]["inserted_documents"], 1);
    assert_eq!(
        mounted["reconciliation"]["projection"]["projected_documents"],
        1
    );
    assert_eq!(
        mounted["reconciliation"]["projection"]["failed_documents"],
        0
    );

    let listed = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/workspaces", None))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = body_json(listed).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert!(listed[0].get("root_path").is_none());

    let tree = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace_id}/tree"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tree.status(), StatusCode::OK);
    let tree = body_json(tree).await;
    assert!(tree["entries"].as_array().unwrap().iter().any(|entry| {
        entry["kind"] == "directory" && entry["relative_path"] == "Projects/Empty"
    }));
    let document = tree["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["relative_path"] == "Projects/MindVault.md")
        .expect("document entry");
    assert_eq!(document["manifest_match"], "current");
    assert_eq!(document["projection_state"], "ready");
    let document_id = document["document_id"].as_str().expect("document id");
    assert_eq!(document["projected_node_id"], document_id);

    let read = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace_id}/documents/{document_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::OK);
    let read = body_json(read).await;
    assert_eq!(read["relative_path"], "Projects/MindVault.md");
    assert_eq!(read["manifest_match"], "current");
    assert!(read["content"]
        .as_str()
        .unwrap()
        .contains("File-first knowledge"));

    let second_root = _tmp.path().join("second-workspace");
    std::fs::create_dir(&second_root).unwrap();
    std::fs::write(second_root.join("Other.md"), "# Other workspace").unwrap();
    let second_mount = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/workspaces",
            Some(json!({
                "root_path": second_root,
                "namespace": "default",
                "display_name": "Second"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(second_mount.status(), StatusCode::CREATED);
    let second_mount = body_json(second_mount).await;
    let second_workspace_id = second_mount["workspace"]["id"]
        .as_str()
        .expect("second workspace id");

    let cross_workspace_read = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/workspaces/{second_workspace_id}/documents/{document_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(cross_workspace_read.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn workspace_projections_are_searchable_linked_rebuildable_and_generic_mutation_safe() {
    let (router, _tmp, workspace_root) = setup_with_workspace_root().await;
    std::fs::write(
        workspace_root.join("Projects/References.md"),
        "---\ntitle: Reference Map\ntags: [workspace, graph]\n---\n\nSee [[MindVault]].",
    )
    .unwrap();

    let mount = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/workspaces",
            Some(json!({
                "root_path": workspace_root,
                "namespace": "default",
                "display_name": "Knowledge"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(mount.status(), StatusCode::CREATED);
    let mounted = body_json(mount).await;
    let workspace_id = mounted["workspace"]["id"]
        .as_str()
        .expect("workspace id")
        .to_string();
    assert_eq!(
        mounted["reconciliation"]["projection"]["projected_documents"],
        2
    );
    assert_eq!(
        mounted["reconciliation"]["projection"]["failed_documents"],
        0
    );

    let tree = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace_id}/tree"),
            None,
        ))
        .await
        .unwrap();
    let tree = body_json(tree).await;
    let entries = tree["entries"].as_array().unwrap();
    let mindvault_id = entries
        .iter()
        .find(|entry| entry["relative_path"] == "Projects/MindVault.md")
        .and_then(|entry| entry["projected_node_id"].as_str())
        .expect("MindVault projection id")
        .to_string();
    let references_id = entries
        .iter()
        .find(|entry| entry["relative_path"] == "Projects/References.md")
        .and_then(|entry| entry["projected_node_id"].as_str())
        .expect("References projection id")
        .to_string();

    let mut search_results = Value::Array(Vec::new());
    for _ in 0..20 {
        let search = router
            .clone()
            .oneshot(json_request(
                Method::GET,
                "/api/v1/search?q=File-first%20knowledge&type=fulltext",
                None,
            ))
            .await
            .unwrap();
        assert_eq!(search.status(), StatusCode::OK);
        search_results = body_json(search).await;
        if search_results
            .as_array()
            .is_some_and(|results| !results.is_empty())
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let projected_node = search_results
        .as_array()
        .and_then(|results| {
            results
                .iter()
                .find(|result| result["node"]["id"] == mindvault_id)
        })
        .expect("canonical Markdown should be full-text searchable");
    assert_eq!(
        projected_node["node"]["metadata"]["mindvault.workspace_projection"]["workspace_id"],
        workspace_id
    );

    let relationships = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/graph/relationships/{references_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(relationships.status(), StatusCode::OK);
    let relationships = body_json(relationships).await;
    assert!(relationships["outgoing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| {
            edge["related_node_id"] == mindvault_id
                && edge["relation_kind"] == "references"
                && edge["auto_managed"] == true
        }));

    let projected_get = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{mindvault_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(projected_get.status(), StatusCode::OK);
    let projected_payload = body_json(projected_get).await;

    let generic_update = router
        .clone()
        .oneshot(json_request(
            Method::PUT,
            &format!("/api/v1/nodes/{mindvault_id}"),
            Some(projected_payload),
        ))
        .await
        .unwrap();
    assert_eq!(generic_update.status(), StatusCode::CONFLICT);

    let generic_delete = router
        .clone()
        .oneshot(json_request(
            Method::DELETE,
            &format!("/api/v1/nodes/{mindvault_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(generic_delete.status(), StatusCode::CONFLICT);

    let rebuild = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/workspaces/{workspace_id}/projections/rebuild"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(rebuild.status(), StatusCode::OK);
    let rebuild = body_json(rebuild).await;
    assert_eq!(rebuild["projection"]["projected_documents"], 2);
    assert_eq!(rebuild["projection"]["failed_documents"], 0);

    std::fs::remove_file(workspace_root.join("Projects/MindVault.md")).unwrap();
    let reconcile = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/workspaces/{workspace_id}/reconcile"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(reconcile.status(), StatusCode::OK);
    let reconcile = body_json(reconcile).await;
    assert_eq!(
        reconcile["reconciliation"]["projection"]["removed_documents"],
        1
    );

    let removed_projection = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{mindvault_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(removed_projection.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn workspace_mount_fails_closed_without_an_allowlisted_root() {
    let (router, tmp) = setup().await;
    let root = tmp.path().join("untrusted");
    std::fs::create_dir(&root).unwrap();

    let response = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/workspaces",
            Some(json!({
                "root_path": root,
                "namespace": "default"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct ScopedEnvVar {
    key: &'static str,
    original: Option<String>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl ScopedEnvVar {
    fn set(key: &'static str, value: impl Into<String>) -> Self {
        let guard = test_env_lock().lock().expect("env lock");
        let original = std::env::var(key).ok();
        std::env::set_var(key, value.into());
        Self {
            key,
            original,
            _guard: guard,
        }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        if let Some(original) = self.original.as_deref() {
            std::env::set_var(self.key, original);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let (router, _tmp) = setup().await;
    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn sealed_mode_blocks_routes_until_unsealed() {
    let tmp = TempDir::new().expect("tempdir");
    let mut config = test_config(&tmp.path().to_string_lossy());
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // Non-allowlisted routes are blocked while sealed.
    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = body_json(resp).await;
    assert_eq!(body["error"], "Vault sealed - please unseal");

    // Keychain status remains reachable while sealed.
    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/keychain/status", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Shamir endpoints required for share-based unseal remain reachable while sealed.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v1/keychain/shamir/status",
            None,
        ))
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/shamir/submit",
            Some(json!({
                "share": "test-share",
                "passphrase": null
            })),
        ))
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Initializing the vault unseals runtime key state.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({
                "password": "integration-test-password",
                "macos_bridge": false
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Route access is restored while unsealed.
    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Seal again and verify gating returns.
    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Explicit unseal should restore access.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({
                "password": "integration-test-password",
                "from_macos_keychain": false,
                "from_secure_enclave": false
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Node CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_and_get_node() {
    let (router, _tmp) = setup().await;

    // Create a node
    let create_body = json!({
        "kind": "fact",
        "content": "Integration test node",
        "title": "Test Fact",
        "tags": ["integration", "test"]
    });
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(create_body),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let created: Value = body_json(resp).await;
    let id = created["id"].as_str().expect("node should have an id");
    assert_eq!(created["content"], "Integration test node");
    assert_eq!(created["title"], "Test Fact");

    // Get it back
    let get_uri = format!("/api/v1/nodes/{id}");
    let resp = router
        .oneshot(json_request(Method::GET, &get_uri, None))
        .await
        .unwrap();
    let status = resp.status();
    let fetched: Value = body_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "GET {get_uri} returned {status}; body: {fetched}"
    );
    assert_eq!(fetched["id"], id);
    assert_eq!(fetched["content"], "Integration test node");
}

#[tokio::test]
async fn update_node() {
    let (router, _tmp) = setup().await;

    let create_body = json!({
        "kind": "fact",
        "content": "Original",
        "tags": []
    });
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(create_body),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let mut created: Value = body_json(resp).await;
    let id = created["id"].as_str().unwrap().to_string();

    // PUT /nodes/:id expects a full KnowledgeNode, so modify the created node
    created["content"] = json!("Updated content");
    created["title"] = json!("New Title");

    let resp = router
        .oneshot(json_request(
            Method::PUT,
            &format!("/api/v1/nodes/{id}"),
            Some(created),
        ))
        .await
        .unwrap();
    let status = resp.status();
    let updated: Value = body_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "PUT /nodes/:id returned {status}; body: {updated}"
    );
    assert_eq!(updated["content"], "Updated content");
    assert_eq!(updated["title"], "New Title");
}

#[tokio::test]
async fn delete_node() {
    let (router, _tmp) = setup().await;

    let create_body = json!({
        "kind": "fact",
        "content": "To be deleted",
        "tags": []
    });
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(create_body),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: Value = body_json(resp).await;
    let id = created["id"].as_str().unwrap();

    // Delete returns 200 with { "deleted": true }
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::DELETE,
            &format!("/api/v1/nodes/{id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // Verify it's gone — get_node returns 404 for missing nodes
    let resp = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Public Shares
// ---------------------------------------------------------------------------

#[tokio::test]
async fn public_share_lifecycle() {
    let (router, _tmp) = setup().await;

    let create_body = json!({
        "kind": "fact",
        "content": "Share me",
        "title": "Shareable"
    });
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(create_body),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: Value = body_json(resp).await;
    let node_id = created["id"].as_str().unwrap();

    let share_resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/shares",
            Some(json!({ "node_id": node_id })),
        ))
        .await
        .unwrap();
    assert_eq!(share_resp.status(), StatusCode::OK);
    let share_body: Value = body_json(share_resp).await;
    let share_id = share_body["id"].as_str().unwrap();
    let token = share_body["token"].as_str().unwrap();

    let list_resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/shares?node_id={node_id}&include_revoked=true"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body: Value = body_json(list_resp).await;
    assert_eq!(list_body.as_array().unwrap().len(), 1);

    let public_resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/public/shares/{token}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(public_resp.status(), StatusCode::OK);
    let public_body: Value = body_json(public_resp).await;
    assert_eq!(public_body["node"]["id"], node_id);
    assert_eq!(public_body["node"]["content"], "Share me");

    let revoke_resp = router
        .clone()
        .oneshot(json_request(
            Method::DELETE,
            &format!("/api/v1/shares/{share_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);

    let missing_resp = router
        .oneshot(json_request(
            Method::GET,
            &format!("/public/shares/{token}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(missing_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_nodes_with_kind_filter() {
    let (router, _tmp) = setup().await;

    // Create a fact and a task
    for (kind, content) in [("fact", "A fact"), ("task", "A task")] {
        let body = json!({ "kind": kind, "content": content, "tags": [] });
        let resp = router
            .clone()
            .oneshot(json_request(Method::POST, "/api/v1/nodes", Some(body)))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "creating {kind} should succeed"
        );
    }

    // List only facts
    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/nodes?kind=fact", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list: Value = body_json(resp).await;
    let items = list.as_array().expect("should be array");
    assert!(items.iter().all(|n| n["kind"] == "fact"));
}

// ---------------------------------------------------------------------------
// Search / Recall
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recall_returns_results() {
    let (router, _tmp) = setup().await;

    // Store a node first
    let body = json!({
        "kind": "fact",
        "content": "Quantum computing uses qubits for parallel computation",
        "title": "Quantum Computing",
        "tags": ["science"]
    });
    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/nodes", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Recall using "text" field (not "query")
    let recall_body = json!({
        "text": "quantum",
        "limit": 10,
        "strategy": "fulltext"
    });
    let resp = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/recall",
            Some(recall_body),
        ))
        .await
        .unwrap();
    let recall_status = resp.status();
    let results: Value = body_json(resp).await;
    assert_eq!(
        recall_status,
        StatusCode::OK,
        "recall returned {recall_status}; body: {results}"
    );
    let items = results.as_array().expect("should be array");
    assert!(
        !items.is_empty(),
        "recall should find the quantum computing node"
    );
}

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

#[tokio::test]
async fn profile_get_and_update() {
    let (router, _tmp) = setup().await;

    // GET default profile
    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/profile", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let profile: Value = body_json(resp).await;
    assert_eq!(profile["timezone"], "UTC");

    // PUT update
    let update = json!({
        "display_name": "Test User",
        "timezone": "America/New_York",
        "bio": "Integration tester"
    });
    let resp = router
        .clone()
        .oneshot(json_request(Method::PUT, "/api/v1/profile", Some(update)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify the update persisted
    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/profile", None))
        .await
        .unwrap();
    let updated_profile: Value = body_json(resp).await;
    assert_eq!(updated_profile["display_name"], "Test User");
    assert_eq!(updated_profile["timezone"], "America/New_York");
    assert_eq!(updated_profile["bio"], "Integration tester");
}

// ---------------------------------------------------------------------------
// Exchange (Proposals)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn proposal_lifecycle_submit_and_list() {
    let (router, _tmp) = setup().await;

    // Submit a proposal using the correct DTO format:
    // sender must be a valid ProposalSender (agent/mcp/webhook/watcher/relay/self)
    // action is a snake_case string matching ProposalAction variants
    let proposal = json!({
        "sender": "agent",
        "action": "create_node",
        "confidence": 0.85,
        "payload": {
            "kind": "fact",
            "content": "Proposed node from agent"
        }
    });
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/exchange/proposals",
            Some(proposal),
        ))
        .await
        .unwrap();
    // Accept 200 or 201
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED,
        "submit proposal should succeed, got {}",
        resp.status()
    );

    // List proposals
    let resp = router
        .oneshot(json_request(
            Method::GET,
            "/api/v1/exchange/proposals",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let proposals: Value = body_json(resp).await;
    let items = proposals.as_array().expect("should be array");
    assert!(
        !items.is_empty(),
        "should have at least one proposal after submitting"
    );
}

// ---------------------------------------------------------------------------
// Node not found
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_nonexistent_node_returns_404() {
    let (router, _tmp) = setup().await;
    let fake_id = "00000000-0000-0000-0000-000000000000";
    let resp = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{fake_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Parameterized route matching
// ---------------------------------------------------------------------------

#[tokio::test]
async fn parameterized_routes_resolve() {
    let (router, _tmp) = setup().await;

    // Create a real node, then hit several parameterized endpoints
    let body = json!({ "kind": "fact", "content": "route test", "tags": [] });
    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/nodes", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: Value = body_json(resp).await;
    let id = created["id"].as_str().unwrap();

    // GET /api/v1/nodes/:id  — should resolve (not router-level 404)
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{id}"),
            None,
        ))
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "GET /nodes/:id should resolve to handler"
    );

    // GET /api/v1/graph/neighbors/:id — should resolve even if node has no edges
    let resp = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/graph/neighbors/{id}"),
            None,
        ))
        .await
        .unwrap();
    // A 200 with empty array is expected; the key thing is it's NOT a router-level 404
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "GET /graph/neighbors/:id should resolve to handler"
    );
}

// ---------------------------------------------------------------------------
// Invalid payload
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_node_with_empty_content_returns_400() {
    let (router, _tmp) = setup().await;

    let body = json!({
        "kind": "fact",
        "content": "",
        "tags": []
    });
    let resp = router
        .oneshot(json_request(Method::POST, "/api/v1/nodes", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// Sealed-mode: comprehensive route gating
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_mode_blocks_data_routes_comprehensively() {
    let tmp = TempDir::new().expect("tempdir");
    let mut config = test_config(&tmp.path().to_string_lossy());
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // All data-access routes must return 503 while sealed.
    let blocked_routes: Vec<(Method, &str, Option<Value>)> = vec![
        (Method::GET, "/api/v1/health", None),
        (
            Method::POST,
            "/api/v1/nodes",
            Some(json!({"kind":"fact","content":"x","tags":[]})),
        ),
        (Method::GET, "/api/v1/nodes", None),
        (
            Method::POST,
            "/api/v1/recall",
            Some(json!({"text":"q","limit":5,"strategy":"fulltext"})),
        ),
        (Method::GET, "/api/v1/profile", None),
        (Method::GET, "/api/v1/files", None),
        (Method::GET, "/api/v1/exchange/proposals", None),
    ];

    for (method, uri, body) in &blocked_routes {
        let resp = router
            .clone()
            .oneshot(json_request(method.clone(), uri, body.clone()))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "{method} {uri} should be blocked while sealed, got {}",
            resp.status()
        );
    }

    // Allowlisted keychain routes must NOT be blocked.
    let allowed_routes = ["/api/v1/keychain/status", "/api/v1/keychain/shamir/status"];
    for uri in &allowed_routes {
        let resp = router
            .clone()
            .oneshot(json_request(Method::GET, uri, None))
            .await
            .unwrap();
        assert_ne!(
            resp.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "GET {uri} should not be blocked while sealed"
        );
    }
}

// ---------------------------------------------------------------------------
// Sealed-mode: unseal/seal transitions produce audit chronicle entries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_unseal_transitions_produce_audit_entries() {
    let tmp = TempDir::new().expect("tempdir");
    let mut config = test_config(&tmp.path().to_string_lossy());
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // Init vault (implicitly unseals).
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"audit-test-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "vault init should succeed");

    // Seal.
    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "seal should succeed");

    // Unseal.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({"password":"audit-test-pw","from_macos_keychain":false,"from_secure_enclave":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "unseal should succeed");

    // Query chronicle for unseal_attempt entries.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v1/agent/chronicle?limit=50",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let entries: Value = body_json(resp).await;
    let entries_arr = entries.as_array().expect("chronicle should be array");

    let unseal_entries: Vec<&Value> = entries_arr
        .iter()
        .filter(|e| e["step_name"].as_str() == Some("unseal_attempt"))
        .collect();
    assert!(
        !unseal_entries.is_empty(),
        "chronicle should contain unseal_attempt entries; got: {entries:?}"
    );

    // At least one entry should record outcome=success.
    let has_success = unseal_entries.iter().any(|e| {
        e["logic"]
            .as_str()
            .map(|l| l.contains("outcome=success"))
            .unwrap_or(false)
    });
    assert!(
        has_success,
        "chronicle should contain a successful unseal; entries: {unseal_entries:?}"
    );
}

// ---------------------------------------------------------------------------
// Sealed-mode: full CRUD + recall lifecycle after unseal
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_mode_crud_and_recall_after_unseal() {
    let tmp = TempDir::new().expect("tempdir");
    let mut config = test_config(&tmp.path().to_string_lossy());
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // Everything blocked while sealed.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({"kind":"fact","content":"blocked","tags":[]})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Init vault.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"lifecycle-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // CREATE node.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({"kind":"fact","content":"Sealed lifecycle test node","title":"Lifecycle","tags":["sealed"]})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: Value = body_json(resp).await;
    let node_id = created["id"].as_str().expect("created node must have id");

    // GET node.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{node_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let fetched: Value = body_json(resp).await;
    assert_eq!(fetched["content"], "Sealed lifecycle test node");

    // UPDATE node.
    let mut updated_node = created.clone();
    updated_node["content"] = json!("Updated sealed content");
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::PUT,
            &format!("/api/v1/nodes/{node_id}"),
            Some(updated_node),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let updated: Value = body_json(resp).await;
    assert_eq!(updated["content"], "Updated sealed content");

    // RECALL (fulltext search).
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/recall",
            Some(json!({"text":"sealed","limit":10,"strategy":"fulltext"})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let results: Value = body_json(resp).await;
    let items = results.as_array().expect("recall results should be array");
    assert!(
        !items.is_empty(),
        "recall should find the sealed lifecycle node"
    );

    // DELETE node.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::DELETE,
            &format!("/api/v1/nodes/{node_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Confirm deletion.
    let resp = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{node_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Sealed-mode: seal → unseal → recall verifies index rebuild
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_mode_recall_survives_seal_unseal_cycle() {
    let tmp = TempDir::new().expect("tempdir");
    let mut config = test_config(&tmp.path().to_string_lossy());
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // Init + store data.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"rebuild-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({"kind":"fact","content":"Photosynthesis converts sunlight to energy","title":"Biology","tags":["science"]})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Verify recall works before seal.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/recall",
            Some(json!({"text":"photosynthesis","limit":10,"strategy":"fulltext"})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let pre_seal: Value = body_json(resp).await;
    assert!(
        !pre_seal.as_array().unwrap().is_empty(),
        "recall before seal should find the node"
    );

    // Seal.
    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Recall should be blocked while sealed.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/recall",
            Some(json!({"text":"photosynthesis","limit":10,"strategy":"fulltext"})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Unseal — triggers rebuild_runtime_indexes.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({"password":"rebuild-pw","from_macos_keychain":false,"from_secure_enclave":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Recall should work again with rebuilt indexes.
    let resp = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/recall",
            Some(json!({"text":"photosynthesis","limit":10,"strategy":"fulltext"})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let post_unseal: Value = body_json(resp).await;
    assert!(
        !post_unseal.as_array().unwrap().is_empty(),
        "recall after unseal should find the node (indexes rebuilt)"
    );
}

// ---------------------------------------------------------------------------
// Sealed-mode: wrong password fails unseal and is logged
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_mode_wrong_password_fails_and_logged() {
    let tmp = TempDir::new().expect("tempdir");
    let mut config = test_config(&tmp.path().to_string_lossy());
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // Init vault.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"correct-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Seal.
    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Attempt unseal with wrong password — should fail.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({"password":"wrong-pw","from_macos_keychain":false,"from_secure_enclave":false})),
        ))
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::OK,
        "wrong password should not unseal"
    );

    // Still sealed — routes blocked.
    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Unseal with correct password to read chronicle.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({"password":"correct-pw","from_macos_keychain":false,"from_secure_enclave":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Chronicle should have a failed unseal entry.
    let resp = router
        .oneshot(json_request(
            Method::GET,
            "/api/v1/agent/chronicle?limit=50",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let entries: Value = body_json(resp).await;
    let has_failure = entries.as_array().unwrap().iter().any(|e| {
        e["step_name"].as_str() == Some("unseal_attempt")
            && e["logic"]
                .as_str()
                .map(|l| l.contains("outcome=fail"))
                .unwrap_or(false)
    });
    assert!(
        has_failure,
        "chronicle should contain a failed unseal entry"
    );
}

#[tokio::test]
async fn sealed_mode_unseal_failure_injected_at_migrate_reseals_vault() {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().to_string_lossy().to_string();
    let mut config = test_config(&data_dir);
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"failpoint-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let _failpoint = ScopedEnvVar::set("MINDVAULT_TEST_FAIL_POST_UNSEAL_MIGRATE", &data_dir);

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({"password":"failpoint-pw","from_macos_keychain":false,"from_secure_enclave":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/keychain/status", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let status: Value = body_json(resp).await;
    assert_eq!(status["state"], "sealed");
}

#[tokio::test]
async fn sealed_mode_unseal_failure_injected_at_rebuild_reseals_vault() {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().to_string_lossy().to_string();
    let mut config = test_config(&data_dir);
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"failpoint-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = router
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let _failpoint = ScopedEnvVar::set("MINDVAULT_TEST_FAIL_POST_UNSEAL_REBUILD", &data_dir);

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/unseal",
            Some(json!({"password":"failpoint-pw","from_macos_keychain":false,"from_secure_enclave":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let resp = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/keychain/status", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let status: Value = body_json(resp).await;
    assert_eq!(status["state"], "sealed");
}

// ---------------------------------------------------------------------------
// Sealed-mode: uploaded blob is encrypted on disk
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_blob_encrypted_on_disk() {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().to_string_lossy().to_string();
    let mut config = test_config(&data_dir);
    config.sealed_mode = true;
    let (router, _tmp) = setup_with_config(config, tmp).await;

    // Init vault.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/keychain/init",
            Some(json!({"password":"blob-test-pw","macos_bridge":false})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Create a node to attach to.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({"kind":"fact","content":"Blob test node","tags":[]})),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: Value = body_json(resp).await;
    let node_id = created["id"].as_str().unwrap();

    // Upload a file via multipart.
    let plaintext_content = b"TOP SECRET PLAINTEXT PAYLOAD FOR AT-REST AUDIT";
    let boundary = "----TestBoundary12345";
    let mut body_bytes = Vec::new();
    // node_id field
    body_bytes.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body_bytes.extend_from_slice(b"Content-Disposition: form-data; name=\"node_id\"\r\n\r\n");
    body_bytes.extend_from_slice(node_id.as_bytes());
    body_bytes.extend_from_slice(b"\r\n");
    // file field
    body_bytes.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body_bytes.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"secret.txt\"\r\n",
    );
    body_bytes.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
    body_bytes.extend_from_slice(plaintext_content);
    body_bytes.extend_from_slice(b"\r\n");
    body_bytes.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/files/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let resp = router.clone().oneshot(req).await.unwrap();
    let upload_status = resp.status();
    let upload_body: Value = body_json(resp).await;
    assert!(
        upload_status == StatusCode::OK || upload_status == StatusCode::CREATED,
        "upload should succeed, got {upload_status}; body: {upload_body}"
    );

    // Scan blobs/ directory on disk — every file must start with MVB1 magic.
    let blobs_dir = std::path::PathBuf::from(&data_dir).join("blobs");
    assert!(
        blobs_dir.exists(),
        "blobs directory should exist after upload"
    );

    let mut found_blob = false;
    let mut stack = vec![blobs_dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read blobs dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("read blob file");
            assert!(
                bytes.starts_with(b"MVB1"),
                "blob at {} must start with MVB1 magic prefix; first 4 bytes: {:?}",
                path.display(),
                &bytes[..bytes.len().min(4)]
            );
            // Ensure the plaintext is NOT present in the raw encrypted bytes.
            let blob_str = String::from_utf8_lossy(&bytes);
            assert!(
                !blob_str.contains("TOP SECRET PLAINTEXT"),
                "blob at {} must not contain plaintext content",
                path.display()
            );
            found_blob = true;
        }
    }
    assert!(found_blob, "at least one blob file should exist on disk");
}

// ---------------------------------------------------------------------------
// Sealed-mode: lifecycle survives engine restart (simulated)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sealed_mode_lifecycle_survives_restart() {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().to_string_lossy().to_string();

    // --- Session 1: init vault, store data, seal ---
    {
        let mut config = test_config(&data_dir);
        config.sealed_mode = true;
        let engine = MindVaultEngine::init(config).await.expect("engine init");
        let state = Arc::new(AppState::new(Arc::new(engine)));
        let router = create_router(state.clone());

        // Init vault.
        let resp = router
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v1/keychain/init",
                Some(json!({"password":"restart-pw","macos_bridge":false})),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Store a node.
        let resp = router
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v1/nodes",
                Some(json!({"kind":"fact","content":"Mitochondria is the powerhouse of the cell","title":"Biology 101","tags":["bio"]})),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // Verify recall works.
        let resp = router
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v1/recall",
                Some(json!({"text":"mitochondria","limit":10,"strategy":"fulltext"})),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let results: Value = body_json(resp).await;
        assert!(!results.as_array().unwrap().is_empty());

        // Seal.
        let resp = router
            .oneshot(json_request(Method::POST, "/api/v1/keychain/seal", None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Engine dropped here — simulates process shutdown.
    }

    // --- Session 2: new engine on same data dir, unseal, verify data ---
    {
        let mut config = test_config(&data_dir);
        config.sealed_mode = true;
        let engine = MindVaultEngine::init(config).await.expect("engine init");
        let state = Arc::new(AppState::new(Arc::new(engine)));
        let router = create_router(state.clone());

        // Routes blocked while sealed.
        let resp = router
            .clone()
            .oneshot(json_request(Method::GET, "/api/v1/health", None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Unseal with same password.
        let resp = router
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v1/keychain/unseal",
                Some(json!({"password":"restart-pw","from_macos_keychain":false,"from_secure_enclave":false})),
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "unseal after restart should succeed"
        );

        // Health restored.
        let resp = router
            .clone()
            .oneshot(json_request(Method::GET, "/api/v1/health", None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Node data survives restart.
        let resp = router
            .clone()
            .oneshot(json_request(Method::GET, "/api/v1/nodes?kind=fact", None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let nodes: Value = body_json(resp).await;
        let items = nodes.as_array().expect("should be array");
        assert!(
            items.iter().any(|n| n["content"]
                .as_str()
                .map(|c| c.contains("Mitochondria"))
                .unwrap_or(false)),
            "node content should survive restart"
        );

        // FTS recall works after post-unseal rebuild.
        let resp = router
            .oneshot(json_request(
                Method::POST,
                "/api/v1/recall",
                Some(json!({"text":"mitochondria","limit":10,"strategy":"fulltext"})),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let results: Value = body_json(resp).await;
        assert!(
            !results.as_array().unwrap().is_empty(),
            "recall should work after restart + unseal (indexes rebuilt)"
        );
    }
}

// ---------------------------------------------------------------------------
// Namespace node quota
// ---------------------------------------------------------------------------

#[tokio::test]
async fn namespace_node_quota_blocks_extra_creates_and_isolates_namespaces() {
    let (router, _tmp) = setup().await;
    let _quota = ScopedEnvVar::set("MINDVAULT_NAMESPACE_NODE_QUOTA", "1");

    let first = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "quota-a-1",
                "namespace": "quota-a",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(
        first.status(),
        StatusCode::CREATED,
        "first node in quota-a should succeed"
    );

    let blocked = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "quota-a-2",
                "namespace": "quota-a",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    let blocked_status = blocked.status();
    let blocked_body = body_json(blocked).await;
    assert_eq!(
        blocked_status,
        StatusCode::TOO_MANY_REQUESTS,
        "second node in same namespace should hit quota; body={blocked_body}"
    );
    let err = blocked_body["error"]
        .as_str()
        .or_else(|| blocked_body.as_str())
        .unwrap_or_default();
    assert!(
        err.contains("quota exceeded") && err.contains("quota-a"),
        "unexpected quota error: {err} (body={blocked_body})"
    );

    let other = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "quota-b-1",
                "namespace": "quota-b",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(
        other.status(),
        StatusCode::CREATED,
        "different namespace should have its own quota"
    );
}

#[tokio::test]
async fn namespace_node_quota_allows_update_when_at_limit() {
    let (router, _tmp) = setup().await;
    let _quota = ScopedEnvVar::set("MINDVAULT_NAMESPACE_NODE_QUOTA", "1");

    let create = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "original quota node",
                "namespace": "quota-update",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let mut node = body_json(create).await;
    let id = node["id"].as_str().unwrap().to_string();
    node["content"] = json!("updated while at quota");

    let update = router
        .oneshot(json_request(
            Method::PUT,
            &format!("/api/v1/nodes/{id}"),
            Some(node),
        ))
        .await
        .unwrap();
    let status = update.status();
    let body = body_json(update).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "updates must not consume quota slots; body={body}"
    );
    assert_eq!(body["content"], "updated while at quota");
}

// ---------------------------------------------------------------------------
// Governed agent execution graph
//
// Contract: docs/architecture/WORK_ORDER_MODEL.md
// ---------------------------------------------------------------------------

fn work_order_body(write_scope: Vec<&str>, key: &str) -> Value {
    json!({
        "goal": "extract candidate decisions from the meeting",
        "non_goals": ["do not contact external services"],
        "success_criteria": ["candidate decisions carry provenance"],
        "budget": {
            "wall_clock_secs": 3600,
            "run_attempts": 5,
            "model_tokens": 100000,
            "effect_actions": 10
        },
        "idempotency_key": key,
        "nodes": [{
            "purpose": "extract decisions",
            "executor_kind": "engine",
            "risk_tier": "standard",
            "write_scope": write_scope,
            "timeout_secs": 600,
            "max_attempts": 3
        }]
    })
}

#[tokio::test]
async fn work_order_admission_refuses_scope_without_a_tool_grant() {
    let (router, _tmp) = setup().await;

    // No Tool Grant has been issued, so the declared write scope cannot be
    // authorized. This is gate G0, and it must refuse before any run exists.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/work-orders",
            Some(work_order_body(
                vec!["mindvault://schemas/alpha"],
                "wo-ungranted",
            )),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "an unauthorized write scope is a governed refusal, not a server error"
    );
    let body = body_json(resp).await;
    let message = body.as_str().unwrap_or_default();
    assert!(
        message.contains("Tool Grant") && message.contains("mindvault://schemas/alpha"),
        "the refusal must name the offending target: {message}"
    );

    // Nothing became schedulable.
    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/work-orders", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await, json!([]));
}

#[tokio::test]
async fn work_order_admission_rejects_an_authored_conflict_edge() {
    let (router, _tmp) = setup().await;

    // An empty write scope needs no Tool Grant, so gate G0 passes trivially
    // and edge validation is actually reached. (Scope resolution deliberately
    // precedes edge checks: authorization before syntax.)
    let mut body = work_order_body(vec![], "wo-authored-conflict");
    let node = body["nodes"][0].clone();
    body["nodes"].as_array_mut().unwrap().push(node);
    body["edges"] = json!([{ "from": 0, "to": 1, "kind": "conflict" }]);

    let resp = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/work-orders",
            Some(body),
        ))
        .await
        .unwrap();
    // Conflict edges are derived from write scope, never authored.
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn work_order_queries_are_reachable_without_a_user_interface() {
    // Constitutional law 2: a first-party capability needs a governed query
    // API, not only a rendered view.
    let (router, _tmp) = setup().await;

    for uri in [
        "/api/v1/work-orders",
        "/api/v1/work-orders?status=admitted&limit=10",
    ] {
        let resp = router
            .clone()
            .oneshot(json_request(Method::GET, uri, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "GET {uri}");
    }

    // An unknown work order is a 404, not a 500.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v1/work-orders/00000000-0000-0000-0000-000000000000",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // An invalid status filter is a client error.
    let resp = router
        .oneshot(json_request(
            Method::GET,
            "/api/v1/work-orders?status=not-a-status",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn failing_a_run_requires_an_explicit_failure_class() {
    let (router, _tmp) = setup().await;
    let run = "/api/v1/work-orders/00000000-0000-0000-0000-000000000000/runs/00000000-0000-0000-0000-000000000000";

    // Blind retry is not recovery: an unclassified failure is refused before
    // the run is even looked up.
    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("{run}/fail"),
            Some(json!({ "failure_class": "not-a-class" })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // A valid class reaches the engine, which reports the missing run.
    let resp = router
        .oneshot(json_request(
            Method::POST,
            &format!("{run}/fail"),
            Some(json!({ "failure_class": "deterministic" })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// The run lifecycle is drivable over HTTP, end to end.
///
/// Before the start endpoint existed, every downstream route — readiness,
/// gates, approval, completion — had no subject it could ever act on: an
/// `AgentRun` could only be created from Rust. A governed layer that can be
/// admitted and audited but never run is not reachable, which is the condition
/// constitutional law 2 exists to prevent.
#[tokio::test]
async fn a_run_can_be_started_and_approved_entirely_over_http() {
    let (router, _tmp) = setup().await;

    // An empty write scope passes G0 trivially, so this exercises the run
    // lifecycle rather than re-testing grant resolution.
    let mut body = work_order_body(vec![], "wo-http-lifecycle");
    body["nodes"][0]["risk_tier"] = json!("low");
    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/work-orders",
            Some(body),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    // Admission returns the contracts it created, so a client never has to
    // guess a node identifier in order to act on one.
    let created: Value = body_json(response).await;
    let work_order_id = created["work_order"]["work_order_id"]
        .as_str()
        .unwrap()
        .to_string();
    let node_id = created["nodes"][0]["node_id"].as_str().unwrap().to_string();

    // Start the attempt. No autonomy rule is configured, so the gate defers and
    // the run parks for the owner — the human-led default, not a failure.
    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/nodes/{node_id}/runs"),
            Some(json!({
                "actor": "mindvault://schemas/http-agent",
                "confidence": 0.99
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let started: Value = body_json(response).await;
    assert_eq!(started["awaiting_approval"], json!(true));
    assert_eq!(started["run"]["status"], json!("awaiting_approval"));
    assert_eq!(started["run"]["attempt_no"], json!(1));
    // A parked run holds no leases: an unbounded approval wait must not block
    // every other run touching those targets.
    assert_eq!(started["lease_target_digests"], json!([]));
    let run_id = started["run"]["run_id"].as_str().unwrap().to_string();

    // The run is now visible to the query surface that previously had no
    // possible subject.
    let response = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/work-orders/{work_order_id}/runs"),
            None,
        ))
        .await
        .unwrap();
    let runs: Value = body_json(response).await;
    assert_eq!(runs.as_array().unwrap().len(), 1);

    // Approving returns it to the ready set rather than straight to execution:
    // approval authorizes the action, not a stale write set.
    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/approve"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let approved: Value = body_json(response).await;
    assert_eq!(approved["status"], json!("ready"));

    // Completion is refused while required gates are outstanding, and reports
    // which — that is normal progress, not an error.
    let response = router
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/complete"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let completion: Value = body_json(response).await;
    assert_eq!(completion["completed"], json!(false));
    assert!(!completion["outstanding_gates"]
        .as_array()
        .unwrap()
        .is_empty());
}

/// A run reaches completion over HTTP: artifact, verified G2, then complete.
///
/// This is the loop the Constitution's acceptance gate describes — work order
/// execution producing a verified artifact. It was previously impossible over
/// the API twice over: no route created a run, and no route recorded an
/// artifact, so gate G2 could never have content to verify.
#[tokio::test]
async fn a_run_produces_a_verified_artifact_and_completes_over_http() {
    let (router, _tmp) = setup().await;

    let mut body = work_order_body(vec![], "wo-http-artifact");
    body["nodes"][0]["risk_tier"] = json!("low");
    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/work-orders",
            Some(body),
        ))
        .await
        .unwrap();
    let created: Value = body_json(response).await;
    let work_order_id = created["work_order"]["work_order_id"]
        .as_str()
        .unwrap()
        .to_string();
    let node_id = created["nodes"][0]["node_id"].as_str().unwrap().to_string();

    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/nodes/{node_id}/runs"),
            Some(json!({ "actor": "mindvault://schemas/http-agent" })),
        ))
        .await
        .unwrap();
    let started: Value = body_json(response).await;
    let run_id = started["run"]["run_id"].as_str().unwrap().to_string();

    // Approve so the run leaves the parked state.
    router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/approve"),
            None,
        ))
        .await
        .unwrap();

    // Record an output. The digest is derived from these bytes server-side.
    let payload = b"candidate decisions extracted from the meeting";
    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/artifacts"),
            Some(json!({
                "artifact_kind": "decision-summary",
                "content_base64": BASE64_STANDARD.encode(payload),
                "provenance": [{
                    "relation": "WasDerivedFrom",
                    "resource": "mindvault://schemas/meeting-source"
                }]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let artifact: Value = body_json(response).await;
    let artifact_id = artifact["artifact_id"].as_str().unwrap().to_string();

    let mut hasher = Sha256::new();
    hasher.update(payload);
    assert_eq!(
        artifact["content_digest"].as_str().unwrap(),
        format!("{:x}", hasher.finalize()),
        "the server must derive the digest from the bytes it stored"
    );

    // The content is retrievable and byte-identical.
    let response = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/work-orders/{work_order_id}/artifacts/{artifact_id}/content"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), payload);

    // G2 now has content to verify. The outcome is computed by the server; the
    // claimed "fail" below is discarded.
    let response = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/gates"),
            Some(json!({
                "gate": "g2",
                "outcome": "fail",
                "evaluator_actor": "mindvault://schemas/reviewer",
                "evidence_digest": "0".repeat(64)
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let gate: Value = body_json(response).await;
    assert_eq!(
        gate["outcome"],
        json!("pass"),
        "G2 is evaluated against stored content, not the caller's assertion"
    );
    assert_ne!(
        gate["evidence_digest"].as_str().unwrap(),
        "0".repeat(64),
        "the server records its own evidence digest"
    );
}

/// The default path is untouched: no env var, no admission, ordinary create.
///
/// Paired with the enforced case below so the two together show the flag is
/// what changes behaviour, rather than something incidental to the fixture.
#[tokio::test]
async fn node_create_is_unaffected_when_command_admission_is_unset() {
    let (router, _tmp) = setup().await;

    let resp = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "admission unset",
                "title": "Unset",
            })),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
}

/// Over real HTTP, an enforced vault with no issued grant refuses the create
/// and writes nothing.
///
/// `ScopedEnvVar` holds the shared env lock and restores the previous value on
/// drop; without it the mode would leak into unrelated tests in this
/// single-process suite and surface as unexplained 403s.
#[tokio::test]
async fn node_create_fails_closed_under_enforced_command_admission() {
    let _mode = ScopedEnvVar::set("MINDVAULT_COMMAND_ADMISSION_MODE", "enforce");
    let tmp = TempDir::new().expect("tempdir");
    let config = test_config(&tmp.path().to_string_lossy());
    let (router, _tmp) = setup_with_config(config, tmp).await;

    let resp = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "should never be stored",
                "title": "Refused",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // The refusal must not have written a node.
    let resp = router
        .oneshot(json_request(Method::GET, "/api/v1/nodes", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let listed: Value = body_json(resp).await;
    let nodes = listed
        .get("nodes")
        .and_then(Value::as_array)
        .or_else(|| listed.as_array())
        .expect("a node list");
    assert!(
        nodes.is_empty(),
        "a refused create must not persist a node: {listed}"
    );
}

/// IK-001a — a fresh vault can register its local Context Node over HTTP.
///
/// This is the bootstrap prerequisite for issuing Tool Grants, which is itself
/// the prerequisite for running command admission in `enforce` on a real vault.
#[tokio::test]
async fn local_context_node_registers_idempotently_over_http() {
    let (router, _tmp) = setup().await;

    let missing = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/context-nodes/local", None))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    let created = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/context-nodes/local",
            Some(json!({ "display_name": "Personal Vault" })),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = body_json(created).await;
    assert_eq!(body["newly_registered"], true);
    assert_eq!(body["status"], "active");
    assert_eq!(body["display_name"], "Personal Vault");
    assert_eq!(body["trust_class"], "local");
    assert!(
        body["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "command"),
        "capabilities: {body}"
    );
    let node_id = body["node_id"].as_str().unwrap().to_string();

    let again = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/context-nodes/local",
            Some(json!({ "display_name": "Must Be Ignored" })),
        ))
        .await
        .unwrap();
    assert_eq!(again.status(), StatusCode::OK);
    let body = body_json(again).await;
    assert_eq!(body["newly_registered"], false);
    assert_eq!(body["node_id"], node_id);
    assert_eq!(body["display_name"], "Personal Vault");

    let fetched = router
        .oneshot(json_request(Method::GET, "/api/v1/context-nodes/local", None))
        .await
        .unwrap();
    assert_eq!(fetched.status(), StatusCode::OK);
    let body = body_json(fetched).await;
    assert_eq!(body["node_id"], node_id);
    assert_eq!(body["status"], "active");
}

/// IK-001b — issue a Tool Grant over HTTP, then enforce admits a node create.
///
/// Registers the local Context Node, grants `local-system` (the default admin
/// principal) a node-scoped Tool/`command` grant, creates under `enforce`, then
/// suspends and revokes the grant so later creates fail closed again.
#[tokio::test]
async fn authority_grant_lifecycle_enables_enforced_node_create() {
    let _mode = ScopedEnvVar::set("MINDVAULT_COMMAND_ADMISSION_MODE", "enforce");
    let tmp = TempDir::new().expect("tempdir");
    let config = test_config(&tmp.path().to_string_lossy());
    let (router, _tmp) = setup_with_config(config, tmp).await;

    let registered = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/context-nodes/local",
            Some(json!({ "display_name": "Grant Vault" })),
        ))
        .await
        .unwrap();
    assert_eq!(registered.status(), StatusCode::CREATED);

    let refused = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "no grant yet",
                "title": "Denied",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);

    let issued = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/authority-grants",
            Some(json!({
                "grantee_subject": "local-system",
                "purpose": "admit node creates for the vault admin",
                "idempotency_key": "issue-admin-tool-grant",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(issued.status(), StatusCode::CREATED);
    let grant = body_json(issued).await;
    assert_eq!(grant["newly_issued"], true);
    assert_eq!(grant["status"], "active");
    assert_eq!(grant["kind"], "tool");
    assert!(
        grant["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "command"),
        "capabilities: {grant}"
    );
    let grant_id = grant["grant_id"].as_str().unwrap().to_string();

    let replay = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/authority-grants",
            Some(json!({
                "grantee_subject": "local-system",
                "purpose": "admit node creates for the vault admin",
                "idempotency_key": "issue-admin-tool-grant",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    let again = body_json(replay).await;
    assert_eq!(again["newly_issued"], false);
    assert_eq!(again["grant_id"], grant_id);

    let created = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "granted create",
                "title": "Admitted",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);

    let suspended = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/authority-grants/{grant_id}/suspend"),
            Some(json!({
                "reason": "operator review",
                "idempotency_key": "suspend-admin-tool-grant",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(suspended.status(), StatusCode::OK);
    assert_eq!(body_json(suspended).await["status"], "suspended");

    let blocked = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "suspended grant",
                "title": "Blocked",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);

    let revoked = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/authority-grants/{grant_id}/revoke"),
            Some(json!({
                "reason": "no longer needed",
                "idempotency_key": "revoke-admin-tool-grant",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::OK);
    assert_eq!(body_json(revoked).await["status"], "revoked");

    let listed = router
        .oneshot(json_request(Method::GET, "/api/v1/authority-grants", None))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let grants = body_json(listed).await;
    assert!(
        grants
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["grant_id"] == grant_id && g["status"] == "revoked"),
        "grants: {grants}"
    );
}

