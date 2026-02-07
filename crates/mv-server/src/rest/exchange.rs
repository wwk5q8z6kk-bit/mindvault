use std::collections::HashMap;
use std::sync::Arc;

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mv_core::{
	ChronicleEntry, Proposal, ProposalAction, ProposalSender, ProposalState,
};

use crate::state::AppState;

// --- DTOs ---

#[derive(Deserialize)]
pub struct ListProposalsQuery {
	pub state: Option<String>,
	pub limit: Option<usize>,
	pub offset: Option<usize>,
}

#[derive(Deserialize)]
pub struct SubmitProposalRequest {
	pub sender: String,
	pub action: String,
	pub target_node_id: Option<String>,
	pub confidence: Option<f32>,
	pub diff_preview: Option<String>,
	pub payload: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
pub struct ProposalCountResponse {
	pub count: usize,
}

#[derive(Serialize)]
struct ErrorBody {
	error: String,
}

fn err_json(msg: impl ToString) -> Json<ErrorBody> {
	Json(ErrorBody {
		error: msg.to_string(),
	})
}

fn map_mv_error(err: mv_core::MvError) -> (StatusCode, String) {
	match err {
		mv_core::MvError::NodeNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
		mv_core::MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
		_ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
	}
}

// --- Handlers ---

/// GET /api/v1/exchange/proposals
pub async fn list_proposals(
	State(state): State<Arc<AppState>>,
	Query(params): Query<ListProposalsQuery>,
) -> Result<Json<Vec<Proposal>>, (StatusCode, String)> {
	let filter_state = if let Some(ref s) = params.state {
		Some(s.parse::<ProposalState>().map_err(|e| {
			(StatusCode::BAD_REQUEST, format!("invalid state: {e}"))
		})?)
	} else {
		None
	};

	let limit = params.limit.unwrap_or(50).min(200);
	let offset = params.offset.unwrap_or(0);

	let proposals = state
		.engine
		.list_proposals(filter_state, limit, offset)
		.await
		.map_err(map_mv_error)?;

	Ok(Json(proposals))
}

/// GET /api/v1/exchange/proposals/:id
pub async fn get_proposal(
	State(state): State<Arc<AppState>>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let uuid = Uuid::parse_str(&id)
		.map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

	let proposal = state
		.engine
		.get_proposal(uuid)
		.await
		.map_err(map_mv_error)?;

	match proposal {
		Some(p) => Ok(Json(p).into_response()),
		None => Err((StatusCode::NOT_FOUND, "proposal not found".to_string())),
	}
}

/// POST /api/v1/exchange/proposals
pub async fn submit_proposal(
	State(state): State<Arc<AppState>>,
	Json(req): Json<SubmitProposalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let sender: ProposalSender = req.sender.parse().map_err(|e: String| {
		(StatusCode::BAD_REQUEST, format!("invalid sender: {e}"))
	})?;
	let action: ProposalAction = req.action.parse().map_err(|e: String| {
		(StatusCode::BAD_REQUEST, format!("invalid action: {e}"))
	})?;

	let mut proposal = Proposal::new(sender, action);

	if let Some(ref target_id_str) = req.target_node_id {
		let target_id = Uuid::parse_str(target_id_str)
			.map_err(|_| (StatusCode::BAD_REQUEST, "invalid target_node_id".to_string()))?;
		proposal = proposal.with_target(target_id);
	}

	if let Some(confidence) = req.confidence {
		proposal = proposal.with_confidence(confidence);
	}

	if let Some(diff) = req.diff_preview {
		proposal = proposal.with_diff(diff);
	}

	if let Some(payload) = req.payload {
		proposal = proposal.with_payload(payload);
	}

	state
		.engine
		.submit_proposal(&proposal)
		.await
		.map_err(map_mv_error)?;

	Ok((StatusCode::CREATED, Json(proposal)).into_response())
}

/// POST /api/v1/exchange/proposals/:id/approve
pub async fn approve_proposal(
	State(state): State<Arc<AppState>>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let uuid = Uuid::parse_str(&id)
		.map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

	let resolved = state
		.engine
		.resolve_proposal(uuid, ProposalState::Approved)
		.await
		.map_err(map_mv_error)?;

	if !resolved {
		return Err((StatusCode::NOT_FOUND, "proposal not found".to_string()));
	}

	// Log chronicle entry for transparency
	let chronicle = ChronicleEntry::new(
		"exchange.approve",
		format!("User approved proposal {uuid}"),
	);
	let _ = state.engine.log_chronicle(&chronicle).await;

	Ok(Json(serde_json::json!({ "id": uuid.to_string(), "state": "approved" })))
}

/// POST /api/v1/exchange/proposals/:id/reject
pub async fn reject_proposal(
	State(state): State<Arc<AppState>>,
	Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
	let uuid = Uuid::parse_str(&id)
		.map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

	let resolved = state
		.engine
		.resolve_proposal(uuid, ProposalState::Rejected)
		.await
		.map_err(map_mv_error)?;

	if !resolved {
		return Err((StatusCode::NOT_FOUND, "proposal not found".to_string()));
	}

	// Log chronicle entry for transparency
	let chronicle = ChronicleEntry::new(
		"exchange.reject",
		format!("User rejected proposal {uuid}"),
	);
	let _ = state.engine.log_chronicle(&chronicle).await;

	Ok(Json(serde_json::json!({ "id": uuid.to_string(), "state": "rejected" })))
}

/// GET /api/v1/exchange/inbox/count
pub async fn inbox_count(
	State(state): State<Arc<AppState>>,
) -> Result<Json<ProposalCountResponse>, (StatusCode, String)> {
	let count = state
		.engine
		.count_proposals(Some(ProposalState::Pending))
		.await
		.map_err(map_mv_error)?;

	Ok(Json(ProposalCountResponse { count }))
}
