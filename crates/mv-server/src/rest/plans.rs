//! Superseded plan endpoints.
//!
//! Superseded by: `docs/adr/012-governed-agent-execution-graph.md`.
//! Replacement: `crates/mv-server/src/rest/work_orders.rs`.
//!
//! The previous implementation reported unconditional success: `approve_plan`
//! returned `{"status": "approved"}` for any identifier without performing a
//! lookup, and `get_plan` always reported not found. Nothing read or wrote the
//! `plans` tables, so an approval reported success for work that never
//! happened — the false canonical success `PROTOCOL_BOUNDARIES.md` prohibits.
//!
//! These endpoints now fail explicitly and name their replacement. An honest
//! `410 Gone` is a better contract than a fabricated `200`.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use mv_engine::planner::Planner;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreatePlanRequest {
    pub goal: String,
}

/// Response for every superseded plan operation.
fn superseded(operation: &str) -> impl IntoResponse {
    (
        StatusCode::GONE,
        Json(serde_json::json!({
            "error": format!("the plan API is superseded; {operation} is unavailable"),
            "superseded_by": "/api/v1/work-orders",
            "decision_record": "docs/adr/012-governed-agent-execution-graph.md",
            "reason": "plan approval reported success without executing or \
                       persisting anything; Work Orders record governed intent \
                       with declared scope, typed edges, budgets, and gate \
                       evidence"
        })),
    )
}

/// POST /api/v1/plans — decompose a goal into a candidate plan.
///
/// Retained as a read-only planning aid: it returns a suggested decomposition
/// and persists nothing. Turning a suggestion into governed work means
/// admitting it as a Work Order, where its scope is resolved against a Tool
/// Grant before anything becomes schedulable.
pub async fn create_plan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePlanRequest>,
) -> impl IntoResponse {
    let planner = Planner::new(
        state.engine.llm.clone(),
        state.engine.config.planning.clone(),
    );

    if !planner.is_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "planning is disabled in configuration"})),
        )
            .into_response();
    }

    let plan = planner.create_plan(&req.goal).await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "plan": plan,
            "persisted": false,
            "note": "a suggestion only; admit it at /api/v1/work-orders to \
                     obtain governed execution"
        })),
    )
        .into_response()
}

/// POST /api/v1/plans/:id/approve — superseded.
pub async fn approve_plan(Path(_id): Path<String>) -> impl IntoResponse {
    superseded("plan approval").into_response()
}

/// GET /api/v1/plans/:id — superseded.
pub async fn get_plan(Path(_id): Path<String>) -> impl IntoResponse {
    superseded("plan retrieval").into_response()
}
