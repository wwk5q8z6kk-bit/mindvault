//! Full-stack REST API integration tests.
//!
//! Each test spins up a real MindVaultEngine backed by a tempdir, constructs
//! the axum Router, and sends actual HTTP requests via `tower::ServiceExt`.
//! This validates routing, serialisation, handler logic, and storage in one pass.
//!
//! NOTE: Tests must run sequentially (`--test-threads=1`) because the fastembed
//! ort runtime uses a global mutex that poisons if any test panics.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
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
    ] {
        std::env::remove_var(key);
    }
    let tmp = TempDir::new().expect("tempdir");
    let config = test_config(&tmp.path().to_string_lossy());
    setup_with_config(config, tmp).await
}

async fn setup_with_config(config: EngineConfig, tmp: TempDir) -> (axum::Router, TempDir) {
    let engine = MindVaultEngine::init(config).await.expect("engine init");
    let state = Arc::new(AppState::new(Arc::new(engine)));
    let router = create_router(state);
    (router, tmp)
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
