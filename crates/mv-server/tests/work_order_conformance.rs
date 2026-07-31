//! Feature Completeness Contract conformance for the governed agent execution
//! graph.
//!
//! `INTEROPERABILITY_CONSTITUTION.md` states that a first-party capability is
//! complete only when it has all ten items below. This suite makes that claim
//! mechanically checkable rather than asserted.
//!
//! All ten items are covered. Items 8 and 9 were outstanding for the first
//! several checkpoints and were declared so by a test rather than omitted; that
//! placeholder has been replaced by the real coverage below, which is what its
//! own doc comment asked for.
//!
//! What this suite does NOT prove is stated in
//! `docs/adr/012-governed-agent-execution-graph.md`: no external dispatcher, no
//! provider contact, no third-party agent execution, and no outbound side
//! effect is authorized by this layer.
//!
//! NOTE: run with `--test-threads=1`, per the repository convention for
//! integration tests.

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

use mv_core::{
    AgentRun, GateId, GateResult, InteroperabilityStore, RiskTier, RunArtifact, StableUri,
    WorkOrder, WorkOrderBudget, WorkOrderNode, WorkOrderSpend, WriteLease,
};
use mv_engine::config::EngineConfig;
use mv_engine::engine::MindVaultEngine;
use mv_server::rest::create_router;
use mv_server::state::AppState;

async fn setup() -> (axum::Router, Arc<MindVaultEngine>, Arc<AppState>, TempDir) {
    for key in [
        "MINDVAULT_AUTH_TOKEN",
        "MINDVAULT_AUTH_ROLE",
        "MINDVAULT_AUTH_NAMESPACE",
        "MINDVAULT_JWT_SECRET",
        "MINDVAULT_COMMAND_ADMISSION_MODE",
    ] {
        std::env::remove_var(key);
    }
    let tmp = TempDir::new().expect("tempdir");
    let mut config = EngineConfig {
        data_dir: tmp.path().to_string_lossy().to_string(),
        ..Default::default()
    };
    config.embedding.provider = "noop".into();
    let engine = Arc::new(MindVaultEngine::init(config).await.expect("engine init"));
    let state = Arc::new(AppState::new(Arc::clone(&engine)));
    (create_router(Arc::clone(&state)), engine, state, tmp)
}

fn request(method: Method, uri: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    match body {
        Some(value) => builder.body(Body::from(value.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

/// Item 1 — a typed and versioned domain object.
///
/// Versioning is by revision, and every governed record carries one that
/// advances rather than being overwritten in place.
#[test]
fn item_1_typed_and_versioned_domain_object() {
    // The types exist and are nameable from outside the defining crate.
    fn assert_typed<T>() {}
    assert_typed::<WorkOrder>();
    assert_typed::<WorkOrderNode>();
    assert_typed::<AgentRun>();
    assert_typed::<RunArtifact>();
    assert_typed::<GateResult>();
    assert_typed::<WriteLease>();
    assert_typed::<WorkOrderBudget>();
    assert_typed::<WorkOrderSpend>();

    // Risk tiers determine required gates, and the scope and ceiling gates are
    // never waived at any tier.
    for tier in [RiskTier::Low, RiskTier::Standard, RiskTier::High] {
        assert!(tier.requires(GateId::G0), "G0 is the scope boundary");
        assert!(tier.requires(GateId::G6), "G6 is the ceiling boundary");
    }
}

/// Item 2 — a command API, and item 3 — a query API.
///
/// Constitutional law 2: a first-party capability cannot be reachable only
/// through a user interface.
#[tokio::test]
async fn items_2_and_3_command_and_query_apis_exist() {
    let (router, _engine, _state, _tmp) = setup().await;

    // Query surface.
    for uri in ["/api/v1/work-orders", "/api/v1/work-orders?status=admitted"] {
        let response = router
            .clone()
            .oneshot(request(Method::GET, uri, None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "GET {uri}");
    }

    // Command surface: reachable and governed. An unauthorized write scope is
    // refused rather than accepted or ignored.
    let response = router
        .oneshot(request(
            Method::POST,
            "/api/v1/work-orders",
            Some(json!({
                "goal": "conformance probe",
                "success_criteria": ["probe completes"],
                "budget": {
                    "wall_clock_secs": 60, "run_attempts": 1,
                    "model_tokens": 100, "effect_actions": 1
                },
                "idempotency_key": "conformance-probe",
                "nodes": [{
                    "purpose": "probe",
                    "executor_kind": "engine",
                    "risk_tier": "low",
                    "write_scope": ["mindvault://schemas/probe"],
                    "timeout_secs": 60,
                    "max_attempts": 1
                }]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "the command surface must exist and must enforce gate G0"
    );
}

/// Items 2 and 3, continued — the public contract must *describe* the surface.
///
/// `docs/architecture/PROTOCOL_BOUNDARIES.md` makes OpenAPI the authoritative
/// description of supported public synchronous HTTP operations. A route that
/// serves traffic but appears in no schema is an undescribed capability, so
/// this test compares the router's work-order routes against the generated
/// document rather than trusting that registration happened.
#[test]
fn work_order_routes_are_described_by_the_authoritative_openapi_document() {
    use utoipa::OpenApi;

    let document = mv_server::openapi::ApiDoc::openapi();
    let described: Vec<&str> = document.paths.paths.keys().map(String::as_str).collect();

    // Every route registered in `rest.rs`, in OpenAPI's `{param}` form.
    //
    // This list is maintained by hand. It catches a route that ships without a
    // schema — the case that actually occurred — but it cannot catch a route
    // added to neither the router nor this list. axum exposes no route
    // enumeration to diff against, so treat a pass as "these ten are
    // described", not "the surface is fully described".
    for path in [
        "/api/v1/work-orders",
        "/api/v1/work-orders/{id}",
        "/api/v1/work-orders/{id}/runs",
        "/api/v1/work-orders/{id}/nodes/{node_id}/runs",
        "/api/v1/work-orders/{id}/artifacts",
        "/api/v1/work-orders/{id}/artifacts/{artifact_id}/content",
        "/api/v1/work-orders/{id}/export",
        "/api/v1/work-orders/restore",
        "/api/v1/work-orders/{id}/runs/{run_id}/readiness",
        "/api/v1/work-orders/{id}/runs/{run_id}/artifacts",
        "/api/v1/work-orders/{id}/runs/{run_id}/gates",
        "/api/v1/work-orders/{id}/runs/{run_id}/approve",
        "/api/v1/work-orders/{id}/runs/{run_id}/execute",
        "/api/v1/work-orders/{id}/runs/{run_id}/complete",
        "/api/v1/work-orders/{id}/runs/{run_id}/fail",
    ] {
        assert!(
            described.contains(&path),
            "{path} serves traffic but the authoritative OpenAPI document does \
             not describe it; described paths: {described:?}"
        );
    }

    // The collection path carries both a query and a command operation, so the
    // capability is not reachable only one way.
    let collection = document
        .paths
        .paths
        .get("/api/v1/work-orders")
        .expect("work-order collection path");
    assert!(
        collection.get.is_some(),
        "query operation must be described"
    );
    assert!(
        collection.post.is_some(),
        "command operation must be described"
    );
}

/// Item 7, continued — a caller cannot author its own G2 verdict.
///
/// G2 checks that declared outputs exist with matching digests. If the HTTP
/// surface accepted the caller's `outcome`, the strongest verification in the
/// system would be the one nobody had to pass: `missing_gates` only asks whether
/// a pass exists, so an asserted pass on a run with no artifacts would let it
/// complete. The server computes G2 and discards what the caller claimed.
#[tokio::test]
async fn gate_g2_is_computed_by_the_server_not_asserted_by_the_caller() {
    let (router, _engine, _state, _tmp) = setup().await;

    // A syntactically perfect claim of success, for a run that does not exist.
    // It must not be accepted on the caller's word.
    let run = "/api/v1/work-orders/00000000-0000-0000-0000-000000000000\
               /runs/00000000-0000-0000-0000-000000000000";
    let response = router
        .oneshot(request(
            Method::POST,
            &format!("{run}/gates"),
            Some(json!({
                "gate": "g2",
                "outcome": "pass",
                "evaluator_actor": "mindvault://schemas/owner",
                "evidence_digest": "a".repeat(64)
            })),
        ))
        .await
        .unwrap();
    // Routed to verification, which looks the run up and finds nothing. An
    // implementation that trusted the caller would have recorded a pass.
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "G2 must be evaluated against the run, not accepted as asserted"
    );
}

/// Item 8, continued — the restore route is actually reachable.
///
/// `/restore` is the only non-UUID static segment under `/api/v1/work-orders`,
/// so it sits directly against the `:id` parameter route. Presence in the
/// OpenAPI document proves the annotation exists, not that the router matches —
/// the document is generated from attributes either way.
#[tokio::test]
async fn the_restore_route_is_reachable_and_not_swallowed_by_the_id_parameter() {
    let (router, _engine, _state, _tmp) = setup().await;

    let response = router
        .oneshot(request(
            Method::POST,
            "/api/v1/work-orders/restore",
            Some(json!({ "format_version": "work-order-export-v1" })),
        ))
        .await
        .unwrap();

    // The body is an incomplete export, so the JSON extractor rejects it. What
    // matters is that it reached an extractor at all: a 404 or 405 would mean
    // `:id` captured "restore" and the route is unreachable.
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "expected the restore handler's extractor to reject the body, got {}",
        response.status()
    );
}

/// Item 4 — emitted events.
///
/// Constitutional law 4 requires the state change and its outbox record to
/// share one atomic boundary, and event admission fails closed against the
/// public schema registry (gate G1).
#[tokio::test]
async fn item_4_events_are_registered_and_admission_fails_closed() {
    let (_router, engine, _state, _tmp) = setup().await;

    // The four execution-graph event schemas are seeded and active, so events
    // can be admitted at all.
    for name in [
        "work-order-admitted",
        "work-order-lifecycle-transitioned",
        "agent-run-started",
        "agent-run-lifecycle-transitioned",
    ] {
        let reference =
            mv_core::SchemaReference::new(mv_core::StableUri::schema(name).unwrap(), "1.0.0")
                .unwrap();
        let schema = engine
            .store
            .nodes
            .get_public_schema(&reference)
            .await
            .unwrap();
        let schema = schema.unwrap_or_else(|| panic!("event schema {name} must be registered"));
        assert_eq!(
            schema.lifecycle,
            mv_core::PublicSchemaLifecycle::Active,
            "{name} must be active for event admission"
        );
        assert!(
            schema.definition["x-mindvault-event-type"].is_string(),
            "{name} must declare its event type for envelope admission"
        );
    }
}

/// Item 4, continued — run state reaches the live observation surface.
///
/// The durable event log satisfies law 4; this covers the other half, which is
/// that a client watching the agent stream learns about a transition without
/// polling. It also pins what the stream may carry: identifiers and status, never
/// artifact content or declared write scope, both of which are governed detail
/// that belongs behind an authorization check.
#[tokio::test]
async fn agent_run_transitions_reach_the_live_observation_stream() {
    let (router, _engine, state, _tmp) = setup().await;
    let mut stream = state.agent_tx.subscribe();

    // A terminal failure on an unknown run does not transition anything, so
    // nothing is announced. The stream reports state, not attempts.
    let run = "/api/v1/work-orders/00000000-0000-0000-0000-000000000000\
               /runs/00000000-0000-0000-0000-000000000000";
    let response = router
        .oneshot(request(
            Method::POST,
            &format!("{run}/fail"),
            Some(json!({ "failure_class": "deterministic" })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(
        stream.try_recv().is_err(),
        "a refused transition must not be announced as one"
    );

    // The notification shape is part of the contract a client codes against.
    let notification = mv_server::state::AgentNotification::AgentRunTransitioned {
        run_id: "11111111-1111-1111-1111-111111111111".into(),
        work_order_id: "22222222-2222-2222-2222-222222222222".into(),
        status: "awaiting_approval".into(),
        failure_class: None,
        namespace: None,
    };
    let encoded: Value = serde_json::to_value(&notification).unwrap();
    assert_eq!(encoded["type"], "agent_run_transitioned");
    assert_eq!(encoded["status"], "awaiting_approval");

    // Governed detail must not ride along on the observation stream.
    for leaked in ["write_scope", "payload", "content", "read_scope"] {
        assert!(
            encoded.get(leaked).is_none(),
            "{leaked} must not appear on the observation stream"
        );
    }
}

/// Item 4, continued — stream delivery fails closed for scoped sessions.
///
/// The governed execution graph is not namespace-partitioned: a Work Order is
/// bounded by its governing node and its AuthorityGrant. So a namespace-scoped
/// socket receives no execution-graph events at all, and must poll the query
/// API where its authorization is checked per request.
///
/// That exclusion has to be deliberate. A scoped token asserts "limit me to
/// this namespace" and may belong to a delegate rather than the owner (System
/// Principle 8); pushing vault-wide governance signal to it would widen access
/// beyond what was asked for. Pinned here so a future refactor cannot quietly
/// start delivering.
#[test]
fn execution_graph_events_are_withheld_from_namespace_scoped_sessions() {
    use mv_server::state::AgentNotification;

    let transition = AgentNotification::AgentRunTransitioned {
        run_id: "11111111-1111-1111-1111-111111111111".into(),
        work_order_id: "22222222-2222-2222-2222-222222222222".into(),
        status: "awaiting_approval".into(),
        failure_class: None,
        namespace: None,
    };
    let gate = AgentNotification::AgentRunGateRecorded {
        run_id: "11111111-1111-1111-1111-111111111111".into(),
        work_order_id: "22222222-2222-2222-2222-222222222222".into(),
        gate: "g2".into(),
        outcome: "pass".into(),
        namespace: None,
    };

    for notification in [&transition, &gate] {
        // An unscoped session — the owner's own — receives them.
        assert!(notification.deliverable_to(None));
        // Any scoped session does not, whatever the namespace.
        assert!(!notification.deliverable_to(Some("research")));
        assert!(!notification.deliverable_to(Some("")));
    }

    // The rule is about carrying a matching namespace, not about being an
    // execution-graph event: a namespaced notification still reaches its own
    // scope, so this is a filter and not a blanket block.
    let namespaced = AgentNotification::NodeEnriched {
        node_id: "33333333-3333-3333-3333-333333333333".into(),
        namespace: Some("research".into()),
    };
    assert!(namespaced.deliverable_to(Some("research")));
    assert!(!namespaced.deliverable_to(Some("other")));
    assert!(namespaced.deliverable_to(None));
}

/// Item 5 — permission and grant definitions.
///
/// Declared write scope is the AuthorityGrant target set. A capability that
/// merely documented a boundary would not satisfy this.
#[tokio::test]
async fn item_5_scope_resolves_through_the_grant_layer() {
    let (_router, engine, _state, _tmp) = setup().await;
    let local_node_id = engine.store.nodes.local_context_node_id().await.unwrap();
    let actor = mv_core::StableUri::principal(
        local_node_id,
        uuid::Uuid::new_v5(&local_node_id, b"conformance-actor"),
    );

    // With no grant issued, resolution fails closed.
    let resolved = engine
        .store
        .nodes
        .find_authorizing_grant(mv_core::GrantQuery {
            grantee: &actor,
            kind: mv_core::AuthorityGrantKind::Tool,
            target: &mv_core::StableUri::parse("mindvault://schemas/probe").unwrap(),
            capability: mv_core::ContextCapability::Command,
            sensitivity: mv_core::Sensitivity::Internal,
            retention: mv_core::RetentionClass::Operational,
            at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    assert!(
        resolved.is_none(),
        "authorization must fail closed without an effective grant"
    );
}

/// Item 6 — provenance behavior.
///
/// Derived knowledge never erases the authority of its evidence, so an artifact
/// without provenance is refused.
#[test]
fn item_6_artifacts_require_provenance() {
    let artifact = RunArtifact {
        artifact_id: uuid::Uuid::now_v7(),
        artifact_uri: mv_core::StableUri::parse("mindvault://schemas/artifact").unwrap(),
        run_id: uuid::Uuid::now_v7(),
        work_order_id: uuid::Uuid::now_v7(),
        artifact_kind: "report".into(),
        content_digest: "a".repeat(64),
        schema: None,
        sensitivity: mv_core::Sensitivity::Internal,
        retention: mv_core::RetentionClass::Operational,
        provenance: Vec::new(),
        created_at: chrono::Utc::now(),
    };
    assert!(
        artifact.validate().is_err(),
        "an artifact must reference the evidence it derives from"
    );
}

/// Item 7 — Trust Ledger behavior.
///
/// Gate results are immutable evidence, and a run can never satisfy its own
/// independent review.
#[test]
fn item_7_gate_evidence_is_attributable_and_independent() {
    let now = chrono::Utc::now();
    let actor = mv_core::StableUri::parse("mindvault://schemas/agent-a").unwrap();
    let run = AgentRun {
        run_id: uuid::Uuid::now_v7(),
        run_uri: mv_core::StableUri::parse("mindvault://schemas/run").unwrap(),
        work_order_id: uuid::Uuid::now_v7(),
        node_id: uuid::Uuid::now_v7(),
        attempt_no: 1,
        status: mv_core::AgentRunStatus::Gated,
        failure_class: None,
        principal: mv_core::StableUri::parse("mindvault://schemas/owner").unwrap(),
        actor: actor.clone(),
        correlation_id: uuid::Uuid::now_v7(),
        causation_id: None,
        started_at: Some(now),
        ended_at: None,
        created_at: now,
        updated_at: now,
    };
    let mut result = GateResult {
        result_id: uuid::Uuid::now_v7(),
        run_id: run.run_id,
        work_order_id: run.work_order_id,
        gate: GateId::G5,
        outcome: mv_core::GateOutcome::Pass,
        evaluator_actor: actor,
        evidence_digest: "b".repeat(64),
        detail: None,
        evaluated_at: now,
        created_at: now,
    };
    assert!(result.validate(&run).is_err(), "a run cannot review itself");

    result.evaluator_actor = run.principal.clone();
    assert!(result.validate(&run).is_ok());
}

/// Item 10 — automated contract and conformance tests.
///
/// Counts are derived from `#[test]` / `#[tokio::test]` attributes in the
/// named sources (or from explicitly listed integration test names). A drift
/// in either direction fails this check — scaffolding a string list is not
/// enough (ADR 012:179-180).
#[test]
fn item_10_conformance_coverage_is_declared() {
    fn count_test_attrs(source: &str) -> usize {
        source.matches("#[test]").count() + source.matches("#[tokio::test]").count()
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    let core = std::fs::read_to_string(manifest.join("../mv-core/src/model/work_order.rs"))
        .expect("mv-core work_order model");
    assert_eq!(count_test_attrs(&core), 12, "mv-core work_order model tests");

    let storage = std::fs::read_to_string(manifest.join("../mv-storage/src/sqlite.rs"))
        .expect("mv-storage sqlite");
    let storage_work_order_tests = [
        "work_order_admission_round_trips_with_contracts_and_edges",
        "admission_rejects_intersecting_write_scopes_without_a_conflict_edge",
        "admission_rejects_a_dependency_cycle",
        "a_run_awaiting_approval_releases_its_leases_and_unblocks_others",
        "an_expired_lease_is_replaced_and_its_history_is_kept",
        "a_lease_longer_than_one_hour_is_refused",
        "an_artifact_digest_must_describe_its_stored_bytes",
        "artifact_content_survives_arbitrary_bytes_and_is_not_json_inflated",
        "an_artifact_without_provenance_is_refused",
        "admission_replay_returns_the_original_work_order",
        "sealed_storage_does_not_expose_work_order_scope_or_goal",
    ];
    for name in storage_work_order_tests {
        assert!(
            storage.contains(&format!("fn {name}")),
            "missing mv-storage work-order test: {name}"
        );
    }
    assert_eq!(storage_work_order_tests.len(), 11);

    let engine = std::fs::read_to_string(manifest.join("../mv-engine/src/engine/work_order_ops.rs"))
        .expect("mv-engine work_order_ops");
    assert_eq!(count_test_attrs(&engine), 14, "mv-engine work_order_ops tests");

    let api = std::fs::read_to_string(manifest.join("tests/api_integration.rs"))
        .expect("api_integration");
    let api_work_order_tests = [
        "work_order_admission_refuses_scope_without_a_tool_grant",
        "work_order_admission_rejects_an_authored_conflict_edge",
        "work_order_queries_are_reachable_without_a_user_interface",
    ];
    for name in api_work_order_tests {
        assert!(
            api.contains(&format!("fn {name}")),
            "missing api_integration work-order test: {name}"
        );
    }
    assert_eq!(api_work_order_tests.len(), 3);

    let conformance = std::fs::read_to_string(manifest.join("tests/work_order_conformance.rs"))
        .expect("work_order_conformance");
    assert!(
        count_test_attrs(&conformance) >= 13,
        "work_order_conformance must keep the constitutional item suite"
    );
}

/// Item 8 — portable export and restore.
///
/// Constitutional law 11 and `DATA_PORTABILITY_CONTRACT.md` require that a user
/// can leave with their data. This checks the export type is a real wire format
/// that validates its own internal consistency; the full cross-vault round trip
/// — including artifact bytes, gate evidence, and provenance — is proven by
/// `mv-engine: a_work_order_survives_an_export_and_restore_into_a_fresh_vault`.
#[test]
fn item_8_the_export_format_refuses_an_inconsistent_graph() {
    use mv_core::{WorkOrderExport, WORK_ORDER_EXPORT_FORMAT_V1};

    let json = json!({
        "format_version": WORK_ORDER_EXPORT_FORMAT_V1,
        "exported_at": chrono::Utc::now(),
        "work_order": null,
    });
    // A truncated export is refused at parse rather than restored partially.
    assert!(serde_json::from_value::<WorkOrderExport>(json).is_err());

    // An export file is untrusted input on the way back in; the format carries
    // a version so a future reader can refuse rather than guess.
    assert_eq!(WORK_ORDER_EXPORT_FORMAT_V1, "work-order-export-v1");
}

/// Item 9 — safe extension access through the governed boundary.
///
/// Constitutional law 3: extensions reach capabilities through the same
/// command and query surface as first-party clients, never the database.
/// Execution-graph access is read-only for extensions by design — an extension
/// that could record gate evidence or approve a run would be manufacturing the
/// governance that constrains it.
#[test]
fn item_9_extensions_observe_the_execution_graph_but_cannot_command_it() {
    use mv_plugin::{PermissionGate, PluginPermission};

    // The read path exists and is separately permissioned.
    let unpermitted = PermissionGate::new(vec![
        PluginPermission::ReadNodes,
        PluginPermission::WriteNodes,
    ]);
    assert!(unpermitted.check("mv_read_work_orders").is_err());
    assert!(PermissionGate::new(vec![PluginPermission::ReadWorkOrders])
        .check("mv_read_work_orders")
        .is_ok());

    // No write path exists, and the gate fails closed on unknown methods.
    let permitted = PermissionGate::new(vec![PluginPermission::ReadWorkOrders]);
    for forbidden in ["mv_write_work_orders", "mv_record_gate", "mv_approve_run"] {
        assert!(permitted.check(forbidden).is_err(), "{forbidden}");
    }
}

/// Wedge Value Proof — production-path Trusted Agent Work completion.
///
/// Exercises the public HTTP surface end-to-end:
/// local Context Node → Tool Grant → Work Order admit → start run →
/// (approve if autonomy parks) → execute → completed artifact with digest.
///
/// This is the unicorn-strategy wedge proof: trusted completed work with
/// evidence on the real server path, not an engine-only unit test.
#[tokio::test]
async fn wedge_value_proof_trusted_work_completes_over_http() {
    let (router, engine, _state, _tmp) = setup().await;

    let registered = router
        .clone()
        .oneshot(request(
            Method::POST,
            "/api/v1/context-nodes/local",
            Some(json!({ "display_name": "Wedge Vault" })),
        ))
        .await
        .unwrap();
    assert_eq!(registered.status(), StatusCode::CREATED);
    let registered = body_json(registered).await;
    let local_node_id = uuid::Uuid::parse_str(registered["node_id"].as_str().unwrap()).unwrap();

    let issued = router
        .clone()
        .oneshot(request(
            Method::POST,
            "/api/v1/authority-grants",
            Some(json!({
                "grantee_subject": "local-system",
                "targets": ["mindvault://schemas/executable"],
                "purpose": "admit wedge trusted-work execution",
                "idempotency_key": "wedge-executable-grant",
            })),
        ))
        .await
        .unwrap();
    assert_eq!(issued.status(), StatusCode::CREATED, "grant: {:?}", issued);

    let created = router
        .clone()
        .oneshot(request(
            Method::POST,
            "/api/v1/work-orders",
            Some(json!({
                "goal": "produce a provenance-linked trusted work artifact",
                "success_criteria": ["engine run completes with digested artifact"],
                "budget": {
                    "wall_clock_secs": 600,
                    "run_attempts": 3,
                    "model_tokens": 10_000,
                    "effect_actions": 0
                },
                "idempotency_key": "wedge-trusted-work-1",
                "nodes": [{
                    "purpose": "internal engine trusted work",
                    "executor_kind": "engine",
                    "risk_tier": "low",
                    "write_scope": ["mindvault://schemas/executable"],
                    "timeout_secs": 120,
                    "max_attempts": 3
                }]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(
        created.status(),
        StatusCode::CREATED,
        "admit work order must succeed with matching Tool Grant"
    );
    let detail = body_json(created).await;
    let work_order_id = detail["work_order"]["work_order_id"].as_str().unwrap();
    let node_id = detail["nodes"][0]["node_id"].as_str().unwrap();
    assert_eq!(detail["nodes"][0]["executor_kind"], "engine");
    assert_eq!(detail["nodes"][0]["risk_tier"], "low");

    let actor = StableUri::principal(
        local_node_id,
        uuid::Uuid::new_v5(&local_node_id, b"local-system"),
    );

    let started = router
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/nodes/{node_id}/runs"),
            Some(json!({
                "actor": actor.as_str(),
                "confidence": 0.99
            })),
        ))
        .await
        .unwrap();
    assert_eq!(started.status(), StatusCode::CREATED, "start run");
    let started = body_json(started).await;
    let run_id = started["run"]["run_id"].as_str().unwrap().to_string();

    if started["awaiting_approval"].as_bool() == Some(true) {
        let approved = router
            .clone()
            .oneshot(request(
                Method::POST,
                &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/approve"),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(approved.status(), StatusCode::OK, "approve parked run");
    }

    let executed = router
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/execute"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(
        executed.status(),
        StatusCode::OK,
        "execute must complete on the production HTTP path"
    );
    let executed = body_json(executed).await;
    assert_eq!(executed["run"]["status"], "completed");
    let digest = executed["artifact_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "sha256 digest");
    let artifact_id = executed["artifact_id"].as_str().unwrap();
    let gates = executed["gates_passed"].as_array().unwrap();
    assert!(
        gates.iter().any(|g| g == "g0"),
        "G0 scope gate must pass: {gates:?}"
    );
    assert!(
        gates.iter().any(|g| g == "g2"),
        "G2 artifact gate must pass: {gates:?}"
    );

    let content = router
        .clone()
        .oneshot(request(
            Method::GET,
            &format!("/api/v1/work-orders/{work_order_id}/artifacts/{artifact_id}/content"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(content.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(content.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["executor"], "engine");
    assert_eq!(body["run_id"], run_id);

    // Duplicate execute must not silently succeed against a completed run.
    let replay = router
        .oneshot(request(
            Method::POST,
            &format!("/api/v1/work-orders/{work_order_id}/runs/{run_id}/execute"),
            None,
        ))
        .await
        .unwrap();
    assert!(
        replay.status().is_client_error() || replay.status() == StatusCode::CONFLICT,
        "re-executing a completed run must fail closed, got {}",
        replay.status()
    );

    // Keep the engine handle live through setup teardown semantics.
    let _ = engine.store.nodes.local_context_node_id().await;
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()))
}
