//! Transport parity smoke tests (REST vs in-process gRPC).
//!
//! Covers health, cross-transport store/get, recall, list, relationships,
//! namespace quota deny codes, namespace-scoped shared-token deny/allow, and JWT
//! role/namespace claim parity on both transports.
//!
//! Run with: cargo test -p mv-server --test transport_parity -- --test-threads=1

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use tempfile::TempDir;
use tonic::Code;
use tower::ServiceExt;

use mv_engine::config::EngineConfig;
use mv_engine::engine::MindVaultEngine;
use mv_server::grpc::proto::mind_vault_service_server::MindVaultService;
use mv_server::grpc::proto::{
    AddRelationshipRequest, GetNeighborsRequest, GetNodeRequest, HealthRequest, ListNodesRequest,
    RecallRequest, StoreNodeRequest,
};
use mv_server::grpc::MindVaultGrpc;
use mv_server::rest::create_router;
use mv_server::state::AppState;

fn test_config(data_dir: &str) -> EngineConfig {
    let mut cfg = EngineConfig {
        data_dir: data_dir.to_string(),
        ..Default::default()
    };
    cfg.embedding.provider = "noop".to_string();
    cfg
}

fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct ScopedEnvVars {
    originals: Vec<(&'static str, Option<String>)>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl ScopedEnvVars {
    fn set(pairs: &[(&'static str, &str)]) -> Self {
        let guard = test_env_lock().lock().expect("env lock");
        let mut originals = Vec::with_capacity(pairs.len());
        for &(key, value) in pairs {
            originals.push((key, std::env::var(key).ok()));
            std::env::set_var(key, value);
        }
        Self {
            originals,
            _guard: guard,
        }
    }
}

impl Drop for ScopedEnvVars {
    fn drop(&mut self) {
        for (key, original) in self.originals.drain(..).rev() {
            match original {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

async fn setup() -> (Arc<AppState>, axum::Router, MindVaultGrpc, TempDir) {
    for key in [
        "MINDVAULT_AUTH_TOKEN",
        "MINDVAULT_AUTH_ROLE",
        "MINDVAULT_AUTH_NAMESPACE",
        "MINDVAULT_JWT_SECRET",
        "MINDVAULT_JWT_ISSUER",
        "MINDVAULT_JWT_AUDIENCE",
        "MINDVAULT_NAMESPACE_NODE_QUOTA",
        "MINDVAULT_COMMAND_ADMISSION_MODE",
    ] {
        std::env::remove_var(key);
    }
    let tmp = TempDir::new().expect("tempdir");
    let config = test_config(&tmp.path().to_string_lossy());
    let engine = MindVaultEngine::init(config).await.expect("engine init");
    let state = Arc::new(AppState::new(Arc::new(engine)));
    let router = create_router(Arc::clone(&state));
    let grpc = MindVaultGrpc::new(Arc::clone(&state));
    (state, router, grpc, tmp)
}

fn json_request(method: Method, uri: &str, body: Option<Value>) -> Request<Body> {
    json_request_with_auth(method, uri, body, None)
}

fn json_request_with_auth(
    method: Method,
    uri: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    match body {
        Some(val) => builder.body(Body::from(val.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

fn grpc_request<T>(message: T, bearer: Option<&str>) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    if let Some(token) = bearer {
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {token}")
                .parse()
                .expect("authorization metadata"),
        );
    }
    request
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()))
}

fn response_text(body: &Value) -> String {
    body["error"]
        .as_str()
        .or_else(|| body.as_str())
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug, Serialize)]
struct JwtTestClaims {
    sub: String,
    exp: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
}

fn jwt_exp_secs(offset_secs: i64) -> usize {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_secs() as i64;
    (now + offset_secs).max(0) as usize
}

fn mint_jwt(
    secret: &str,
    sub: &str,
    role: Option<&str>,
    namespace: Option<&str>,
    exp_offset_secs: i64,
) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        &JwtTestClaims {
            sub: sub.to_string(),
            exp: jwt_exp_secs(exp_offset_secs),
            role: role.map(str::to_string),
            namespace: namespace.map(str::to_string),
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("jwt encode")
}

#[tokio::test]
async fn health_status_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;

    let rest = router
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(rest.status(), StatusCode::OK);
    let rest_body = body_json(rest).await;

    let grpc_resp = grpc
        .health(tonic::Request::new(HealthRequest {}))
        .await
        .expect("grpc health");
    let grpc_body = grpc_resp.into_inner();

    assert_eq!(rest_body["status"], "ok");
    assert_eq!(grpc_body.status, "ok");
    assert_eq!(
        rest_body["node_count"].as_u64().unwrap_or(u64::MAX),
        grpc_body.node_count
    );
    assert_eq!(
        rest_body["version"].as_str().unwrap_or_default(),
        grpc_body.version
    );
}

#[tokio::test]
async fn store_rest_get_grpc_and_store_grpc_get_rest() {
    let (_state, router, grpc, _tmp) = setup().await;

    let create = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "parity rest-to-grpc marker",
                "title": "Parity A",
                "namespace": "parity",
                "tags": ["parity"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = body_json(create).await;
    let id = created["id"].as_str().expect("id").to_string();

    let got = grpc
        .get_node(tonic::Request::new(GetNodeRequest { id: id.clone() }))
        .await
        .expect("grpc get")
        .into_inner()
        .node
        .expect("node present");
    assert_eq!(got.id, id);
    assert_eq!(got.content, "parity rest-to-grpc marker");
    assert_eq!(got.namespace, "parity");

    let stored = grpc
        .store_node(tonic::Request::new(StoreNodeRequest {
            kind: "fact".into(),
            content: "parity grpc-to-rest marker".into(),
            title: Some("Parity B".into()),
            source: None,
            namespace: "parity".into(),
            tags: vec!["parity".into()],
            importance: None,
            metadata_json: None,
        }))
        .await
        .expect("grpc store")
        .into_inner()
        .node
        .expect("stored node");

    let rest_get = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes/{}", stored.id),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(rest_get.status(), StatusCode::OK);
    let fetched = body_json(rest_get).await;
    assert_eq!(fetched["id"], stored.id);
    assert_eq!(fetched["content"], "parity grpc-to-rest marker");
}

#[tokio::test]
async fn recall_finds_nodes_from_either_transport() {
    let (_state, router, grpc, _tmp) = setup().await;

    let marker = "unique-parity-recall-token-42";
    let create = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": format!("note about {marker}"),
                "namespace": "parity-recall",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let rest_recall = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/recall",
            Some(json!({
                "text": marker,
                "limit": 10,
                "strategy": "fulltext",
                "namespace": "parity-recall"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(rest_recall.status(), StatusCode::OK);
    let rest_results = body_json(rest_recall).await;
    assert!(
        rest_results
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        "REST recall should find the node"
    );

    let grpc_recall = grpc
        .recall(tonic::Request::new(RecallRequest {
            text: marker.into(),
            strategy: "fulltext".into(),
            limit: 10,
            min_score: 0.0,
            namespace: Some("parity-recall".into()),
            kinds: vec![],
            tags: vec![],
        }))
        .await
        .expect("grpc recall")
        .into_inner();
    assert!(
        !grpc_recall.results.is_empty(),
        "gRPC recall should find the node"
    );
}

#[tokio::test]
async fn namespace_quota_deny_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;
    let _quota = ScopedEnvVars::set(&[("MINDVAULT_NAMESPACE_NODE_QUOTA", "1")]);

    let first = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "quota rest first",
                "namespace": "parity-quota",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);

    let rest_blocked = router
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "quota rest second",
                "namespace": "parity-quota",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(rest_blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    let rest_err = body_json(rest_blocked).await;
    let rest_msg = response_text(&rest_err);
    assert!(
        rest_msg.contains("quota exceeded"),
        "unexpected rest quota body: {rest_err}"
    );

    let grpc_blocked = grpc
        .store_node(tonic::Request::new(StoreNodeRequest {
            kind: "fact".into(),
            content: "quota grpc second".into(),
            title: None,
            source: None,
            namespace: "parity-quota".into(),
            tags: vec![],
            importance: None,
            metadata_json: None,
        }))
        .await;
    let status = grpc_blocked.expect_err("grpc should deny at quota");
    assert_eq!(status.code(), Code::ResourceExhausted);
    assert!(
        status.message().contains("quota exceeded"),
        "unexpected grpc message: {}",
        status.message()
    );
}

#[tokio::test]
async fn namespace_scoped_token_deny_allow_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;

    // Seed a foreign-namespace node while auth is disabled (admin).
    let foreign = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "foreign namespace secret",
                "namespace": "team-b",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::CREATED);
    let foreign_id = body_json(foreign).await["id"]
        .as_str()
        .expect("foreign id")
        .to_string();

    let identity = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/identities",
            Some(json!({
                "subject_binding": "shared-token",
                "actor_kind": "service",
                "display_name": "Scoped Shared Token",
                "idempotency_key": "transport-parity-shared-token"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(identity.status(), StatusCode::CREATED);

    let token = "parity-scoped-write-token";
    let _auth = ScopedEnvVars::set(&[
        ("MINDVAULT_AUTH_TOKEN", token),
        ("MINDVAULT_AUTH_ROLE", "write"),
        ("MINDVAULT_AUTH_NAMESPACE", "team-a"),
    ]);

    // Missing bearer → unauthenticated on both transports.
    let rest_unauth = router
        .clone()
        .oneshot(json_request(Method::GET, "/api/v1/health", None))
        .await
        .unwrap();
    assert_eq!(rest_unauth.status(), StatusCode::UNAUTHORIZED);

    let grpc_unauth = grpc
        .health(tonic::Request::new(HealthRequest {}))
        .await
        .expect_err("grpc health without token");
    assert_eq!(grpc_unauth.code(), Code::Unauthenticated);

    // Allowed namespace create/get.
    let allowed = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "team-a allowed note",
                "namespace": "team-a",
                "tags": []
            })),
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(
        allowed.status(),
        StatusCode::CREATED,
        "scoped write token should create in team-a"
    );
    let allowed_id = body_json(allowed).await["id"]
        .as_str()
        .expect("allowed id")
        .to_string();

    let rest_get_ok = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::GET,
            &format!("/api/v1/nodes/{allowed_id}"),
            None,
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_get_ok.status(), StatusCode::OK);

    let grpc_get_ok = grpc
        .get_node(grpc_request(
            GetNodeRequest {
                id: allowed_id.clone(),
            },
            Some(token),
        ))
        .await
        .expect("grpc get allowed");
    assert_eq!(
        grpc_get_ok.into_inner().node.expect("node").namespace,
        "team-a"
    );

    // Cross-namespace create denied.
    let rest_deny_create = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "should not land in team-b",
                "namespace": "team-b",
                "tags": []
            })),
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_deny_create.status(), StatusCode::FORBIDDEN);
    let rest_deny_create_body = body_json(rest_deny_create).await;
    assert!(
        response_text(&rest_deny_create_body).contains("namespace"),
        "rest create deny body: {rest_deny_create_body}"
    );

    let grpc_deny_create = grpc
        .store_node(grpc_request(
            StoreNodeRequest {
                kind: "fact".into(),
                content: "should not land in team-b".into(),
                title: None,
                source: None,
                namespace: "team-b".into(),
                tags: vec![],
                importance: None,
                metadata_json: None,
            },
            Some(token),
        ))
        .await
        .expect_err("grpc create cross-namespace");
    assert_eq!(grpc_deny_create.code(), Code::PermissionDenied);
    assert!(
        grpc_deny_create.message().contains("namespace"),
        "grpc create deny: {}",
        grpc_deny_create.message()
    );

    // Cross-namespace get denied.
    let rest_deny_get = router
        .oneshot(json_request_with_auth(
            Method::GET,
            &format!("/api/v1/nodes/{foreign_id}"),
            None,
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_deny_get.status(), StatusCode::FORBIDDEN);

    let grpc_deny_get = grpc
        .get_node(grpc_request(
            GetNodeRequest {
                id: foreign_id.clone(),
            },
            Some(token),
        ))
        .await
        .expect_err("grpc get cross-namespace");
    assert_eq!(grpc_deny_get.code(), Code::PermissionDenied);
}

#[tokio::test]
async fn read_role_write_deny_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;

    let token = "parity-scoped-read-token";
    let _auth = ScopedEnvVars::set(&[
        ("MINDVAULT_AUTH_TOKEN", token),
        ("MINDVAULT_AUTH_ROLE", "read"),
        ("MINDVAULT_AUTH_NAMESPACE", "team-read"),
    ]);

    let rest_health = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::GET,
            "/api/v1/health",
            None,
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_health.status(), StatusCode::OK);

    let grpc_health = grpc
        .health(grpc_request(HealthRequest {}, Some(token)))
        .await
        .expect("read role can health over grpc");
    assert_eq!(grpc_health.into_inner().status, "ok");

    let rest_write = router
        .oneshot(json_request_with_auth(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "read role should not write",
                "namespace": "team-read",
                "tags": []
            })),
            Some(token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_write.status(), StatusCode::FORBIDDEN);
    let rest_body = body_json(rest_write).await;
    assert!(
        response_text(&rest_body).contains("write"),
        "rest write deny body: {rest_body}"
    );

    let grpc_write = grpc
        .store_node(grpc_request(
            StoreNodeRequest {
                kind: "fact".into(),
                content: "read role should not write".into(),
                title: None,
                source: None,
                namespace: "team-read".into(),
                tags: vec![],
                importance: None,
                metadata_json: None,
            },
            Some(token),
        ))
        .await
        .expect_err("grpc write with read role");
    assert_eq!(grpc_write.code(), Code::PermissionDenied);
    assert!(
        grpc_write.message().contains("write"),
        "grpc write deny: {}",
        grpc_write.message()
    );
}

#[tokio::test]
async fn jwt_namespace_claim_deny_allow_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;

    let foreign = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "jwt foreign namespace secret",
                "namespace": "jwt-b",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::CREATED);
    let foreign_id = body_json(foreign).await["id"]
        .as_str()
        .expect("foreign id")
        .to_string();

    // JWT subjects are intentionally fail-closed until an administrator binds
    // them in the governed identity registry. Register the writer before auth
    // is enabled so this parity test exercises the normal registry path rather
    // than the opt-in legacy derivation fallback.
    let identity = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/identities",
            Some(json!({
                "subject_binding": "writer",
                "actor_kind": "human",
                "display_name": "JWT Writer",
                "idempotency_key": "transport-parity-jwt-writer"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(identity.status(), StatusCode::CREATED);

    let secret = "parity-jwt-secret";
    let _auth = ScopedEnvVars::set(&[("MINDVAULT_JWT_SECRET", secret)]);

    let write_token = mint_jwt(secret, "writer", Some("write"), Some("jwt-a"), 3600);
    let expired = mint_jwt(secret, "writer", Some("write"), Some("jwt-a"), -120);
    let wrong_secret = mint_jwt("other-secret", "writer", Some("write"), Some("jwt-a"), 3600);

    // Bad/expired JWT → unauthenticated on both transports.
    for bad in [expired.as_str(), wrong_secret.as_str()] {
        let rest = router
            .clone()
            .oneshot(json_request_with_auth(
                Method::GET,
                "/api/v1/health",
                None,
                Some(bad),
            ))
            .await
            .unwrap();
        assert_eq!(rest.status(), StatusCode::UNAUTHORIZED);

        let grpc_err = grpc
            .health(grpc_request(HealthRequest {}, Some(bad)))
            .await
            .expect_err("bad jwt should fail grpc health");
        assert_eq!(grpc_err.code(), Code::Unauthenticated);
    }

    // Allowed namespace create/get.
    let allowed = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "jwt-a allowed note",
                "namespace": "jwt-a",
                "tags": []
            })),
            Some(&write_token),
        ))
        .await
        .unwrap();
    assert_eq!(
        allowed.status(),
        StatusCode::CREATED,
        "JWT write+namespace claim should create in jwt-a"
    );
    let allowed_id = body_json(allowed).await["id"]
        .as_str()
        .expect("allowed id")
        .to_string();

    let rest_get_ok = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::GET,
            &format!("/api/v1/nodes/{allowed_id}"),
            None,
            Some(&write_token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_get_ok.status(), StatusCode::OK);

    let grpc_get_ok = grpc
        .get_node(grpc_request(
            GetNodeRequest {
                id: allowed_id.clone(),
            },
            Some(&write_token),
        ))
        .await
        .expect("grpc get allowed with jwt");
    assert_eq!(
        grpc_get_ok.into_inner().node.expect("node").namespace,
        "jwt-a"
    );

    // Cross-namespace create denied.
    let rest_deny_create = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "should not land in jwt-b",
                "namespace": "jwt-b",
                "tags": []
            })),
            Some(&write_token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_deny_create.status(), StatusCode::FORBIDDEN);

    let grpc_deny_create = grpc
        .store_node(grpc_request(
            StoreNodeRequest {
                kind: "fact".into(),
                content: "should not land in jwt-b".into(),
                title: None,
                source: None,
                namespace: "jwt-b".into(),
                tags: vec![],
                importance: None,
                metadata_json: None,
            },
            Some(&write_token),
        ))
        .await
        .expect_err("jwt cross-namespace create");
    assert_eq!(grpc_deny_create.code(), Code::PermissionDenied);
    assert!(
        grpc_deny_create.message().contains("namespace"),
        "grpc create deny: {}",
        grpc_deny_create.message()
    );

    // Cross-namespace get denied.
    let rest_deny_get = router
        .oneshot(json_request_with_auth(
            Method::GET,
            &format!("/api/v1/nodes/{foreign_id}"),
            None,
            Some(&write_token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_deny_get.status(), StatusCode::FORBIDDEN);

    let grpc_deny_get = grpc
        .get_node(grpc_request(
            GetNodeRequest {
                id: foreign_id.clone(),
            },
            Some(&write_token),
        ))
        .await
        .expect_err("jwt cross-namespace get");
    assert_eq!(grpc_deny_get.code(), Code::PermissionDenied);
}

#[tokio::test]
async fn jwt_read_role_claim_write_deny_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;

    let secret = "parity-jwt-read-secret";
    let _auth = ScopedEnvVars::set(&[("MINDVAULT_JWT_SECRET", secret)]);
    let read_token = mint_jwt(secret, "reader", Some("read"), Some("jwt-read"), 3600);

    let rest_health = router
        .clone()
        .oneshot(json_request_with_auth(
            Method::GET,
            "/api/v1/health",
            None,
            Some(&read_token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_health.status(), StatusCode::OK);

    let grpc_health = grpc
        .health(grpc_request(HealthRequest {}, Some(&read_token)))
        .await
        .expect("jwt read role can health over grpc");
    assert_eq!(grpc_health.into_inner().status, "ok");

    let rest_write = router
        .oneshot(json_request_with_auth(
            Method::POST,
            "/api/v1/nodes",
            Some(json!({
                "kind": "fact",
                "content": "jwt read role should not write",
                "namespace": "jwt-read",
                "tags": []
            })),
            Some(&read_token),
        ))
        .await
        .unwrap();
    assert_eq!(rest_write.status(), StatusCode::FORBIDDEN);
    let rest_body = body_json(rest_write).await;
    assert!(
        response_text(&rest_body).contains("write"),
        "rest jwt write deny body: {rest_body}"
    );

    let grpc_write = grpc
        .store_node(grpc_request(
            StoreNodeRequest {
                kind: "fact".into(),
                content: "jwt read role should not write".into(),
                title: None,
                source: None,
                namespace: "jwt-read".into(),
                tags: vec![],
                importance: None,
                metadata_json: None,
            },
            Some(&read_token),
        ))
        .await
        .expect_err("jwt read role write over grpc");
    assert_eq!(grpc_write.code(), Code::PermissionDenied);
    assert!(
        grpc_write.message().contains("write"),
        "grpc jwt write deny: {}",
        grpc_write.message()
    );
}

#[tokio::test]
async fn list_and_relationship_parity_matches_across_rest_and_grpc() {
    let (_state, router, grpc, _tmp) = setup().await;

    let namespace = "parity-list-rel";
    let mut created_ids = Vec::new();
    for (title, content) in [
        ("List A", "parity list node alpha"),
        ("List B", "parity list node beta"),
        ("List C", "parity list node gamma"),
    ] {
        let create = router
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v1/nodes",
                Some(json!({
                    "kind": "fact",
                    "content": content,
                    "title": title,
                    "namespace": namespace,
                    "tags": ["parity-list"]
                })),
            ))
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::CREATED);
        let id = body_json(create).await["id"]
            .as_str()
            .expect("id")
            .to_string();
        created_ids.push(id);
    }
    created_ids.sort();

    let rest_list = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/nodes?namespace={namespace}&limit=50&offset=0"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(rest_list.status(), StatusCode::OK);
    let rest_nodes = body_json(rest_list).await;
    let rest_arr = rest_nodes.as_array().expect("rest list array");
    let mut rest_ids: Vec<String> = rest_arr
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_string))
        .collect();
    rest_ids.sort();
    assert_eq!(
        rest_ids, created_ids,
        "REST list should return all created nodes"
    );

    let grpc_list = grpc
        .list_nodes(tonic::Request::new(ListNodesRequest {
            namespace: Some(namespace.to_string()),
            kinds: vec![],
            limit: 50,
            offset: 0,
        }))
        .await
        .expect("grpc list")
        .into_inner();
    let mut grpc_ids: Vec<String> = grpc_list.nodes.iter().map(|n| n.id.clone()).collect();
    grpc_ids.sort();
    assert_eq!(
        grpc_ids, created_ids,
        "gRPC list should return the same node ids as REST"
    );
    assert!(
        grpc_list.total as usize >= created_ids.len(),
        "gRPC total should cover listed nodes"
    );

    // Relationship: REST create → visible via gRPC neighbors; gRPC create → REST overview.
    let from_id = &created_ids[0];
    let to_id = &created_ids[1];
    let rest_rel = router
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/graph/relationships",
            Some(json!({
                "from_node": from_id,
                "to_node": to_id,
                "kind": "references",
                "weight": 0.75
            })),
        ))
        .await
        .unwrap();
    assert_eq!(rest_rel.status(), StatusCode::CREATED);
    let rest_rel_id = body_json(rest_rel).await["id"]
        .as_str()
        .expect("relationship id")
        .to_string();

    let grpc_neighbors = grpc
        .get_neighbors(tonic::Request::new(GetNeighborsRequest {
            node_id: from_id.clone(),
            depth: 1,
        }))
        .await
        .expect("grpc neighbors after REST relationship")
        .into_inner();
    assert!(
        grpc_neighbors.neighbor_ids.contains(to_id),
        "gRPC neighbors should include REST-linked target: {:?}",
        grpc_neighbors.neighbor_ids
    );

    let rest_neighbors = router
        .clone()
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/graph/neighbors/{from_id}?depth=1"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(rest_neighbors.status(), StatusCode::OK);
    let rest_neighbor_ids = body_json(rest_neighbors).await;
    let rest_neighbor_arr = rest_neighbor_ids.as_array().expect("neighbors array");
    assert!(
        rest_neighbor_arr
            .iter()
            .any(|id| id.as_str() == Some(to_id.as_str())),
        "REST neighbors should include linked target: {rest_neighbor_ids}"
    );

    let grpc_rel = grpc
        .add_relationship(tonic::Request::new(AddRelationshipRequest {
            from_node: created_ids[1].clone(),
            to_node: created_ids[2].clone(),
            kind: "relates_to".into(),
            weight: 0.5,
        }))
        .await
        .expect("grpc add relationship")
        .into_inner();
    assert!(!grpc_rel.id.is_empty());

    let overview = router
        .oneshot(json_request(
            Method::GET,
            &format!("/api/v1/graph/relationships/{}", created_ids[1]),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(overview.status(), StatusCode::OK);
    let overview_body = body_json(overview).await;
    let outgoing = overview_body["outgoing"]
        .as_array()
        .expect("outgoing edges");
    assert!(
        outgoing.iter().any(|edge| {
            edge["related_node_id"].as_str() == Some(created_ids[2].as_str())
                && edge["relation_kind"].as_str() == Some("relates_to")
        }),
        "REST relationship overview should show gRPC-created edge: {overview_body}"
    );
    // Keep REST-created relationship id referenced so regressions surface clearly.
    assert!(!rest_rel_id.is_empty());
}
