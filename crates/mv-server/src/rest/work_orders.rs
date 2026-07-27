//! Governed agent execution graph transport.
//!
//! Contract: `docs/architecture/WORK_ORDER_MODEL.md`.
//! Decision: `docs/adr/012-governed-agent-execution-graph.md`.
//!
//! Command and query surfaces ship together: constitutional law 2 forbids a
//! first-party capability that only a user interface can reach.
//!
//! Admission refusal is not a server error. A contract whose declared write
//! scope exceeds its Tool Grant is a governed outcome, reported as `403` with
//! the offending targets named so the caller can correct the request.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use mv_core::{
    AgentRun, EdgeKind, ExecutorKind, GateId, GateOutcome, InteroperabilityStore,
    ProvenanceReference, ProvenanceRelation, RetentionClass, RiskTier, RunArtifact,
    RunFailureClass, Sensitivity, StableUri, WorkOrder, WorkOrderBudget, WorkOrderExport,
    WorkOrderStatus, MAX_ARTIFACT_PROVENANCE,
};
use mv_engine::engine::{
    AdmissionRefusal, ProposedEdge, ProposedNode, ProposedWorkOrder, RunReadiness,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{authorize_read, authorize_write, AuthContext};
use crate::state::{AgentNotification, AppState};

/// Decoded artifact payload ceiling. Matches the email attachment default so a
/// write-capable client cannot OOM the vault with one base64 body.
pub(crate) const MAX_RUN_ARTIFACT_BYTES: usize = 5 * 1024 * 1024;

/// HTTP body ceiling for `POST …/artifacts`: decoded limit plus base64 expansion
/// and a small JSON envelope budget.
pub(crate) const MAX_RUN_ARTIFACT_REQUEST_BYTES: usize =
    MAX_RUN_ARTIFACT_BYTES.saturating_mul(4).div_ceil(3).saturating_add(64 * 1024);

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct CreateWorkOrderRequest {
    goal: String,
    #[serde(default)]
    non_goals: Vec<String>,
    #[serde(default)]
    anchors: Vec<String>,
    success_criteria: Vec<String>,
    #[serde(default)]
    prohibited_outcomes: Vec<String>,
    budget: BudgetRequest,
    idempotency_key: String,
    nodes: Vec<NodeRequest>,
    #[serde(default)]
    edges: Vec<EdgeRequest>,
    #[serde(default)]
    sensitivity: Option<String>,
    #[serde(default)]
    retention: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BudgetRequest {
    wall_clock_secs: u64,
    run_attempts: u64,
    model_tokens: u64,
    effect_actions: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NodeRequest {
    purpose: String,
    executor_kind: String,
    risk_tier: String,
    #[serde(default)]
    read_scope: Vec<String>,
    #[serde(default)]
    write_scope: Vec<String>,
    #[serde(default)]
    inputs: Vec<String>,
    timeout_secs: u32,
    max_attempts: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EdgeRequest {
    from: usize,
    to: usize,
    kind: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RecordGateRequest {
    gate: String,
    outcome: String,
    evaluator_actor: String,
    evidence_digest: String,
    #[serde(default)]
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StartRunRequest {
    /// Acting principal. Attribution is required, never inferred: gate G5 turns
    /// on a run's actor differing from its reviewer.
    actor: String,
    /// Confidence the caller claims for this attempt, evaluated by the autonomy
    /// gate. Absent, it is treated as zero — the value least likely to clear an
    /// auto-apply threshold, so an omitted field cannot buy autonomy.
    #[serde(default)]
    confidence: f32,
}

#[derive(Debug, Serialize)]
pub(crate) struct StartRunResponse {
    run: RunView,
    /// Digests of the write targets this run now holds exclusively. Digests,
    /// not plaintext URIs: a sealed vault must not expose scope through a lease
    /// listing.
    lease_target_digests: Vec<String>,
    /// True when the owner must authorize before the run proceeds. Not an
    /// error, and it has no deadline.
    awaiting_approval: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RecordArtifactRequest {
    artifact_kind: String,
    /// Base64-encoded content. The server derives the digest from these bytes;
    /// a caller-supplied digest would let gate G2 verify a claim against itself.
    content_base64: String,
    /// Required. Derived knowledge never erases the authority of its evidence,
    /// so an artifact citing nothing is refused.
    provenance: Vec<ProvenanceRequest>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProvenanceRequest {
    relation: String,
    resource: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FailRunRequest {
    /// Required: blind retry is not recovery, so a failure must be classified.
    failure_class: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListWorkOrdersQuery {
    status: Option<String>,
    limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub(crate) struct WorkOrderSummary {
    work_order_id: Uuid,
    work_order_uri: String,
    revision: u64,
    status: String,
    goal: String,
    non_goals: Vec<String>,
    anchors: Vec<String>,
    success_criteria: Vec<String>,
    budget: BudgetView,
    remaining: BudgetView,
    sensitivity: String,
    retention: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct BudgetView {
    wall_clock_secs: u64,
    run_attempts: u64,
    model_tokens: u64,
    effect_actions: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct WorkOrderDetail {
    work_order: WorkOrderSummary,
    nodes: Vec<NodeView>,
    edges: Vec<EdgeView>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NodeView {
    node_id: Uuid,
    node_uri: String,
    purpose: String,
    executor_kind: String,
    risk_tier: String,
    status: String,
    read_scope: Vec<String>,
    write_scope: Vec<String>,
    required_gates: Vec<String>,
    authorizing_grant_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub(crate) struct EdgeView {
    edge_id: Uuid,
    from_node_id: Uuid,
    to_node_id: Uuid,
    kind: String,
    /// Conflict edges are derived from intersecting write scope rather than
    /// authored, so operators can tell a guard from a declared dependency.
    derived: bool,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunView {
    run_id: Uuid,
    run_uri: String,
    node_id: Uuid,
    attempt_no: u32,
    status: String,
    failure_class: Option<String>,
    actor: String,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GateResultView {
    gate: String,
    outcome: String,
    evaluator_actor: String,
    evidence_digest: String,
    detail: Option<String>,
    evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ArtifactView {
    artifact_id: Uuid,
    artifact_uri: String,
    run_id: Uuid,
    artifact_kind: String,
    content_digest: String,
    sensitivity: String,
    retention: String,
    provenance: Vec<ProvenanceView>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProvenanceView {
    relation: String,
    resource: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReadinessView {
    ready: bool,
    reason: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CompleteRunResponse {
    completed: bool,
    run: Option<RunView>,
    /// Gates still outstanding. Completion is an evidence state, not a claim.
    outstanding_gates: Vec<String>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// POST /api/v1/work-orders — admit a Work Order.
pub(crate) async fn create_work_order(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateWorkOrderRequest>,
) -> Result<(StatusCode, Json<WorkOrderDetail>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let proposal = build_proposal(&request)?;
    let local_node_id = state
        .engine
        .store
        .nodes
        .local_context_node_id()
        .await
        .map_err(crate::rest::map_mv_error)?;
    // A stable per-subject principal; anonymous local access maps to "owner".
    let subject = auth.subject.as_deref().unwrap_or("owner");
    let principal = StableUri::principal(
        local_node_id,
        Uuid::new_v5(&local_node_id, subject.as_bytes()),
    );

    let admitted = state
        .engine
        .admit_work_order(&principal, &principal, &proposal)
        .await
        .map_err(crate::rest::map_mv_error)?
        .map_err(map_admission_refusal)?;

    let detail = load_detail(&state, &admitted).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

/// POST /api/v1/work-orders/:id/nodes/:node_id/runs — start the next attempt.
///
/// The only path that creates an Agent Run. It executes nothing: ADR 012 governs
/// internal runs, and no external dispatcher, provider, or third-party agent is
/// invoked. It spends a budgeted attempt, records the run and its event
/// atomically, and takes write leases only if the owner's autonomy policy allows.
///
/// An unconfigured vault parks the run for approval rather than proceeding.
pub(crate) async fn start_run(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, node_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<StartRunRequest>,
) -> Result<(StatusCode, Json<StartRunResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;
    let actor = StableUri::parse(&request.actor).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let started = state
        .engine
        .start_run(work_order_id, node_id, &actor, request.confidence)
        .await
        .map_err(crate::rest::map_mv_error)?;

    state.notify_agent(AgentNotification::run_transitioned(&started.run));

    Ok((
        StatusCode::CREATED,
        Json(StartRunResponse {
            run: run_view(&started.run),
            lease_target_digests: started
                .leases
                .iter()
                .map(|lease| lease.target_digest.clone())
                .collect(),
            awaiting_approval: started.awaiting_approval,
        }),
    ))
}

/// POST /api/v1/work-orders/:id/runs/:run_id/artifacts — record an output.
///
/// The digest is derived from the submitted bytes, never taken from the
/// request. Without this route a run driven over HTTP could never satisfy gate
/// G2, because it had no way to produce the outputs G2 verifies.
pub(crate) async fn record_artifact(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RecordArtifactRequest>,
) -> Result<(StatusCode, Json<ArtifactView>), (StatusCode, String)> {
    authorize_write(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;

    // Reject oversized base64 before decode so peak memory stays near the limit
    // rather than 4/3 of it plus the decoded copy.
    let max_base64_chars = MAX_RUN_ARTIFACT_BYTES
        .saturating_mul(4)
        .div_ceil(3)
        .saturating_add(4);
    if request.content_base64.len() > max_base64_chars {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("artifact content exceeds {MAX_RUN_ARTIFACT_BYTES} bytes"),
        ));
    }

    if request.provenance.len() > MAX_ARTIFACT_PROVENANCE {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "an artifact may cite at most {MAX_ARTIFACT_PROVENANCE} provenance references"
            ),
        ));
    }

    let payload = BASE64_STANDARD
        .decode(&request.content_base64)
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("content is not valid base64: {err}"),
            )
        })?;
    if payload.len() > MAX_RUN_ARTIFACT_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("artifact content exceeds {MAX_RUN_ARTIFACT_BYTES} bytes"),
        ));
    }

    let mut provenance = Vec::with_capacity(request.provenance.len());
    for reference in &request.provenance {
        provenance.push(ProvenanceReference {
            resource: StableUri::parse(&reference.resource)
                .map_err(|err| (StatusCode::BAD_REQUEST, err))?,
            // Accepts exactly the spelling `artifact_view` emits, so a client
            // can round-trip an artifact it just read back into a new one.
            relation: match reference.relation.as_str() {
                "WasDerivedFrom" => ProvenanceRelation::WasDerivedFrom,
                "WasAttributedTo" => ProvenanceRelation::WasAttributedTo,
                "WasGeneratedBy" => ProvenanceRelation::WasGeneratedBy,
                "PrimarySource" => ProvenanceRelation::PrimarySource,
                other => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("unknown provenance relation: {other}"),
                    ))
                }
            },
        });
    }

    let artifact = state
        .engine
        .record_artifact(run_id, request.artifact_kind, &payload, provenance)
        .await
        .map_err(crate::rest::map_mv_error)?;

    Ok((StatusCode::CREATED, Json(artifact_view(&artifact))))
}

/// POST /api/v1/work-orders/:id/runs/:run_id/gates — record gate evidence.
///
/// Gate G2 is *computed here, not accepted from the caller.* The request
/// triggers evaluation; the server reads every artifact's stored bytes back,
/// rehashes them, and records its own verdict and evidence digest. A caller's
/// asserted `outcome` and `evidence_digest` for G2 are discarded.
///
/// Otherwise the strongest verification in the system would be the one nobody
/// had to pass: any client could POST `{"gate":"g2","outcome":"pass"}` for a run
/// that produced nothing, and `missing_gates` — which only asks whether a pass
/// exists — would let it complete. Authoring your own verdict is not evidence.
pub(crate) async fn record_gate(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RecordGateRequest>,
) -> Result<(StatusCode, Json<GateResultView>), (StatusCode, String)> {
    authorize_write(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;

    let gate: GateId = request
        .gate
        .parse()
        .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?;
    let outcome: GateOutcome = request
        .outcome
        .parse()
        .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?;
    let evaluator =
        StableUri::parse(&request.evaluator_actor).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let result = if gate == GateId::G2 {
        state
            .engine
            .verify_run_artifacts(run_id, &evaluator)
            .await
            .map_err(crate::rest::map_mv_error)?
    } else {
        state
            .engine
            .record_run_gate(
                run_id,
                gate,
                outcome,
                &evaluator,
                request.evidence_digest,
                request.detail,
            )
            .await
            .map_err(crate::rest::map_mv_error)?
    };

    state.notify_agent(AgentNotification::gate_recorded(&result));

    // 201, matching this route's OpenAPI declaration and the artifact route:
    // recording evidence creates an immutable resource.
    Ok((
        StatusCode::CREATED,
        Json(GateResultView {
            gate: result.gate.as_str().into(),
            outcome: result.outcome.as_str().into(),
            evaluator_actor: result.evaluator_actor.as_str().into(),
            evidence_digest: result.evidence_digest,
            detail: result.detail,
            evaluated_at: result.evaluated_at,
        }),
    ))
}

/// POST /api/v1/work-orders/:id/runs/:run_id/approve — resume an approved run.
///
/// The run returns to the ready set rather than straight to execution: approval
/// authorizes the action, not a stale write set, so conflicts are re-checked
/// and leases re-acquired.
pub(crate) async fn approve_run(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RunView>, (StatusCode, String)> {
    authorize_write(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;
    let run = state
        .engine
        .resume_approved_run(run_id)
        .await
        .map_err(crate::rest::map_mv_error)?;
    state.notify_agent(AgentNotification::run_transitioned(&run));
    Ok(Json(run_view(&run)))
}

/// POST /api/v1/work-orders/:id/runs/:run_id/complete — attempt completion.
pub(crate) async fn complete_run(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CompleteRunResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;
    match state
        .engine
        .complete_run(run_id)
        .await
        .map_err(crate::rest::map_mv_error)?
    {
        Ok(run) => {
            state.notify_agent(AgentNotification::run_transitioned(&run));
            Ok(Json(CompleteRunResponse {
                completed: true,
                run: Some(run_view(&run)),
                outstanding_gates: Vec::new(),
            }))
        }
        // Outstanding gates are reported, not raised: the caller's next action
        // is to supply evidence, which is normal progress.
        Err(outstanding) => Ok(Json(CompleteRunResponse {
            completed: false,
            run: None,
            outstanding_gates: outstanding
                .into_iter()
                .map(|gate| gate.as_str().to_string())
                .collect(),
        })),
    }
}

/// POST /api/v1/work-orders/:id/runs/:run_id/fail — fail with a classification.
pub(crate) async fn fail_run(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<FailRunRequest>,
) -> Result<Json<RunView>, (StatusCode, String)> {
    authorize_write(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;
    let class: RunFailureClass = request
        .failure_class
        .parse()
        .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?;
    let run = state
        .engine
        .fail_run(run_id, class)
        .await
        .map_err(crate::rest::map_mv_error)?;
    state.notify_agent(AgentNotification::run_transitioned(&run));
    Ok(Json(run_view(&run)))
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// GET /api/v1/work-orders
pub(crate) async fn list_work_orders(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrderSummary>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let status = query
        .status
        .as_deref()
        .map(|value| value.parse::<WorkOrderStatus>())
        .transpose()
        .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?;
    let limit = query.limit.unwrap_or(50).clamp(1, 1000);

    let work_orders = state
        .engine
        .store
        .nodes
        .list_work_orders(status, limit)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok(Json(work_orders.iter().map(work_order_summary).collect()))
}

/// GET /api/v1/work-orders/:id
pub(crate) async fn get_work_order(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(work_order_id): Path<Uuid>,
) -> Result<Json<WorkOrderDetail>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let work_order = state
        .engine
        .store
        .nodes
        .get_work_order(work_order_id)
        .await
        .map_err(crate::rest::map_mv_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "work order not found".to_string()))?;
    Ok(Json(load_detail(&state, &work_order).await?))
}

/// GET /api/v1/work-orders/:id/runs
pub(crate) async fn list_runs(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(work_order_id): Path<Uuid>,
) -> Result<Json<Vec<RunView>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let runs = state
        .engine
        .store
        .nodes
        .list_agent_runs(work_order_id, 1000)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok(Json(runs.iter().map(run_view).collect()))
}

/// GET /api/v1/work-orders/:id/runs/:run_id/gates
pub(crate) async fn list_gates(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<GateResultView>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;
    let results = state
        .engine
        .store
        .nodes
        .list_gate_results(run_id)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok(Json(
        results
            .into_iter()
            .map(|result| GateResultView {
                gate: result.gate.as_str().into(),
                outcome: result.outcome.as_str().into(),
                evaluator_actor: result.evaluator_actor.as_str().into(),
                evidence_digest: result.evidence_digest,
                detail: result.detail,
                evaluated_at: result.evaluated_at,
            })
            .collect(),
    ))
}

/// GET /api/v1/work-orders/:id/runs/:run_id/readiness
pub(crate) async fn get_run_readiness(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ReadinessView>, (StatusCode, String)> {
    authorize_read(&auth)?;
    require_run_for_work_order(&state, work_order_id, run_id).await?;
    let readiness = state
        .engine
        .run_readiness(run_id)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok(Json(readiness_view(&readiness)))
}

/// GET /api/v1/work-orders/:id/artifacts
pub(crate) async fn list_artifacts(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(work_order_id): Path<Uuid>,
) -> Result<Json<Vec<ArtifactView>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let artifacts = state
        .engine
        .store
        .nodes
        .list_run_artifacts(work_order_id)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok(Json(artifacts.iter().map(artifact_view).collect()))
}

/// GET /api/v1/work-orders/:id/artifacts/:artifact_id/content
///
/// Returns the artifact's bytes, re-verified against the recorded digest by the
/// store. Content is served as an opaque octet stream: an artifact may hold a
/// diff, an image, or a binary, and guessing a content type here would invite a
/// client to render untrusted agent output as markup.
///
/// The digest is returned in a header so a caller can pin what it received
/// without re-reading the artifact listing.
pub(crate) async fn read_artifact_content(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((work_order_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    authorize_read(&auth)?;
    let artifact = state
        .engine
        .store
        .nodes
        .get_run_artifact(artifact_id)
        .await
        .map_err(crate::rest::map_mv_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "artifact not found".to_string()))?;

    // The artifact must belong to the work order named in the path, so a
    // caller cannot read across orders by guessing an identifier.
    if artifact.work_order_id != work_order_id {
        return Err((StatusCode::NOT_FOUND, "artifact not found".to_string()));
    }

    let payload = state
        .engine
        .store
        .nodes
        .read_run_artifact_payload(artifact_id)
        .await
        .map_err(crate::rest::map_mv_error)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "artifact content not found".to_string(),
            )
        })?;

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .header("x-mindvault-content-digest", artifact.content_digest)
        .header(
            "content-disposition",
            format!("attachment; filename=\"{artifact_id}\""),
        )
        // Agent output is not trusted markup.
        .header("x-content-type-options", "nosniff")
        .body(axum::body::Body::from(payload))
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("build artifact response: {err}"),
            )
        })
}

/// GET /api/v1/work-orders/:id/export
///
/// The complete governed graph as a portable document: contracts, typed edges,
/// every run attempt, all gate evidence, and artifact content with provenance.
///
/// Constitutional law 11 — a user can leave with their data — is not satisfied
/// by a capability whose history is only reachable through this server's own
/// query API.
pub(crate) async fn export_work_order(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(work_order_id): Path<Uuid>,
) -> Result<Json<WorkOrderExport>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let export = state
        .engine
        .export_work_order(work_order_id)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok(Json(export))
}

/// POST /api/v1/work-orders/restore
///
/// Restore a previously exported Work Order. The document is untrusted input:
/// it is validated for internal consistency and every artifact digest is
/// re-derived from its bytes before anything is written.
///
/// Refuses with 409 when the Work Order already exists — divergent histories of
/// one identifier cannot be reconciled by overwriting, and picking a winner
/// would destroy gate evidence. Authority does not travel with the record: a
/// restored contract carries no grant and must be re-authorized locally.
pub(crate) async fn restore_work_order(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(export): Json<WorkOrderExport>,
) -> Result<(StatusCode, Json<RestoreSummaryView>), (StatusCode, String)> {
    authorize_write(&auth)?;
    let summary = state
        .engine
        .restore_work_order(&export)
        .await
        .map_err(crate::rest::map_mv_error)?;
    Ok((
        StatusCode::CREATED,
        Json(RestoreSummaryView {
            work_order_id: summary.work_order_id,
            nodes: summary.nodes,
            edges: summary.edges,
            runs: summary.runs,
            gate_results: summary.gate_results,
            artifacts: summary.artifacts,
            authority_restored: false,
        }),
    ))
}

/// What a restore actually wrote, so a caller can verify rather than assume.
#[derive(Debug, Serialize)]
pub(crate) struct RestoreSummaryView {
    pub work_order_id: Uuid,
    pub nodes: usize,
    pub edges: usize,
    pub runs: usize,
    pub gate_results: usize,
    pub artifacts: usize,
    /// Always false: an export is not a way to move authority between vaults.
    /// The restored contract must be re-authorized locally before any new run
    /// can pass gate G0.
    pub authority_restored: bool,
}

// ---------------------------------------------------------------------------
// Mapping
// ---------------------------------------------------------------------------

fn build_proposal(
    request: &CreateWorkOrderRequest,
) -> Result<ProposedWorkOrder, (StatusCode, String)> {
    let parse_uris = |values: &[String]| -> Result<Vec<StableUri>, (StatusCode, String)> {
        values
            .iter()
            .map(|value| StableUri::parse(value).map_err(|err| (StatusCode::BAD_REQUEST, err)))
            .collect()
    };

    let mut nodes = Vec::with_capacity(request.nodes.len());
    for node in &request.nodes {
        nodes.push(ProposedNode {
            purpose: node.purpose.clone(),
            executor_kind: node
                .executor_kind
                .parse::<ExecutorKind>()
                .map_err(|err| (StatusCode::BAD_REQUEST, err))?,
            risk_tier: node
                .risk_tier
                .parse::<RiskTier>()
                .map_err(|err| (StatusCode::BAD_REQUEST, err))?,
            read_scope: parse_uris(&node.read_scope)?,
            write_scope: parse_uris(&node.write_scope)?,
            inputs: parse_uris(&node.inputs)?,
            timeout_secs: node.timeout_secs,
            max_attempts: node.max_attempts,
        });
    }

    let mut edges = Vec::with_capacity(request.edges.len());
    for edge in &request.edges {
        edges.push(ProposedEdge {
            from: edge.from,
            to: edge.to,
            kind: edge
                .kind
                .parse::<EdgeKind>()
                .map_err(|err| (StatusCode::BAD_REQUEST, err))?,
        });
    }

    let sensitivity = match request.sensitivity.as_deref() {
        Some(value) => value
            .parse::<Sensitivity>()
            .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?,
        None => Sensitivity::Internal,
    };
    let retention = match request.retention.as_deref() {
        Some(value) => value
            .parse::<RetentionClass>()
            .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?,
        None => RetentionClass::Operational,
    };

    Ok(ProposedWorkOrder {
        goal: request.goal.clone(),
        non_goals: request.non_goals.clone(),
        anchors: parse_uris(&request.anchors)?,
        success_criteria: request.success_criteria.clone(),
        prohibited_outcomes: request.prohibited_outcomes.clone(),
        budget: WorkOrderBudget {
            wall_clock_secs: request.budget.wall_clock_secs,
            run_attempts: request.budget.run_attempts,
            model_tokens: request.budget.model_tokens,
            effect_actions: request.budget.effect_actions,
        },
        sensitivity,
        retention,
        idempotency_key: request.idempotency_key.clone(),
        nodes,
        edges,
    })
}

/// An over-scoped contract is a governed refusal, not a server fault. The
/// offending targets are named so the caller can correct the request.
fn map_admission_refusal(refusal: AdmissionRefusal) -> (StatusCode, String) {
    match refusal {
        AdmissionRefusal::WriteScopeOutsideGrant { .. }
        | AdmissionRefusal::ReadScopeOutsideGrant { .. } => {
            (StatusCode::FORBIDDEN, refusal.to_string())
        }
        AdmissionRefusal::DependencyCycle { .. } | AdmissionRefusal::Invalid { .. } => {
            (StatusCode::BAD_REQUEST, refusal.to_string())
        }
    }
}

async fn load_detail(
    state: &Arc<AppState>,
    work_order: &WorkOrder,
) -> Result<WorkOrderDetail, (StatusCode, String)> {
    let nodes = state
        .engine
        .store
        .nodes
        .list_work_order_nodes(work_order.work_order_id)
        .await
        .map_err(crate::rest::map_mv_error)?;
    let edges = state
        .engine
        .store
        .nodes
        .list_work_order_edges(work_order.work_order_id)
        .await
        .map_err(crate::rest::map_mv_error)?;

    Ok(WorkOrderDetail {
        work_order: work_order_summary(work_order),
        nodes: nodes
            .iter()
            .map(|node| NodeView {
                node_id: node.node_id,
                node_uri: node.node_uri.as_str().into(),
                purpose: node.purpose.clone(),
                executor_kind: node.executor_kind.as_str().into(),
                risk_tier: node.risk_tier.as_str().into(),
                status: node.status.as_str().into(),
                read_scope: node
                    .read_scope
                    .iter()
                    .map(|uri| uri.as_str().to_string())
                    .collect(),
                write_scope: node
                    .write_scope
                    .iter()
                    .map(|uri| uri.as_str().to_string())
                    .collect(),
                required_gates: node
                    .risk_tier
                    .required_gates()
                    .iter()
                    .map(|gate| gate.as_str().to_string())
                    .collect(),
                authorizing_grant_id: node.authorizing_grant_id,
            })
            .collect(),
        edges: edges
            .iter()
            .map(|edge| EdgeView {
                edge_id: edge.edge_id,
                from_node_id: edge.from_node_id,
                to_node_id: edge.to_node_id,
                kind: edge.kind.as_str().into(),
                derived: edge.derived,
                detail: edge.detail.clone(),
            })
            .collect(),
    })
}


/// Bind a run-scoped URL to the work order named in the path.
///
/// Without this check a caller who knows only `run_id` can mutate another
/// order's run while the path claims a different work order — the asymmetry
/// already hardened on artifact content GET.
async fn require_run_for_work_order(
    state: &AppState,
    work_order_id: Uuid,
    run_id: Uuid,
) -> Result<AgentRun, (StatusCode, String)> {
    let run = state
        .engine
        .store
        .nodes
        .get_agent_run(run_id)
        .await
        .map_err(crate::rest::map_mv_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "agent run not found".to_string()))?;
    if run.work_order_id != work_order_id {
        return Err((StatusCode::NOT_FOUND, "agent run not found".to_string()));
    }
    Ok(run)
}

fn work_order_summary(work_order: &WorkOrder) -> WorkOrderSummary {
    WorkOrderSummary {
        work_order_id: work_order.work_order_id,
        work_order_uri: work_order.work_order_uri.as_str().into(),
        revision: work_order.revision,
        status: work_order.status.as_str().into(),
        goal: work_order.goal.clone(),
        non_goals: work_order.non_goals.clone(),
        anchors: work_order
            .anchors
            .iter()
            .map(|uri| uri.as_str().to_string())
            .collect(),
        success_criteria: work_order.success_criteria.clone(),
        budget: budget_view(&work_order.budget),
        remaining: budget_view(&work_order.remaining),
        sensitivity: work_order.sensitivity.as_str().into(),
        retention: work_order.retention.as_str().into(),
        created_at: work_order.created_at,
        updated_at: work_order.updated_at,
    }
}

fn budget_view(budget: &WorkOrderBudget) -> BudgetView {
    BudgetView {
        wall_clock_secs: budget.wall_clock_secs,
        run_attempts: budget.run_attempts,
        model_tokens: budget.model_tokens,
        effect_actions: budget.effect_actions,
    }
}

fn run_view(run: &AgentRun) -> RunView {
    RunView {
        run_id: run.run_id,
        run_uri: run.run_uri.as_str().into(),
        node_id: run.node_id,
        attempt_no: run.attempt_no,
        status: run.status.as_str().into(),
        failure_class: run.failure_class.map(|class| class.as_str().to_string()),
        actor: run.actor.as_str().into(),
        started_at: run.started_at,
        ended_at: run.ended_at,
    }
}

fn artifact_view(artifact: &RunArtifact) -> ArtifactView {
    ArtifactView {
        artifact_id: artifact.artifact_id,
        artifact_uri: artifact.artifact_uri.as_str().into(),
        run_id: artifact.run_id,
        artifact_kind: artifact.artifact_kind.clone(),
        content_digest: artifact.content_digest.clone(),
        sensitivity: artifact.sensitivity.as_str().into(),
        retention: artifact.retention.as_str().into(),
        provenance: artifact
            .provenance
            .iter()
            .map(|reference| ProvenanceView {
                relation: format!("{:?}", reference.relation),
                resource: reference.resource.as_str().into(),
            })
            .collect(),
        created_at: artifact.created_at,
    }
}

fn readiness_view(readiness: &RunReadiness) -> ReadinessView {
    match readiness {
        RunReadiness::Ready => ReadinessView {
            ready: true,
            reason: "ready".into(),
            detail: None,
        },
        RunReadiness::BlockedByConflict { targets } => ReadinessView {
            ready: false,
            reason: "conflict".into(),
            detail: Some(format!(
                "{targets} declared write target(s) are leased by another run"
            )),
        },
        RunReadiness::BlockedByVerification { predecessor } => ReadinessView {
            ready: false,
            reason: "verification".into(),
            detail: Some(format!("run {predecessor} has outstanding gates")),
        },
        RunReadiness::BlockedByPredecessor { predecessor } => ReadinessView {
            ready: false,
            reason: "predecessor".into(),
            detail: Some(format!("contract {predecessor} has not completed")),
        },
        // Unbounded by design: a run parked on owner approval is the system
        // working correctly, not a stalled request.
        RunReadiness::BlockedByApproval => ReadinessView {
            ready: false,
            reason: "awaiting_approval".into(),
            detail: Some("owner approval is outstanding".into()),
        },
    }
}
