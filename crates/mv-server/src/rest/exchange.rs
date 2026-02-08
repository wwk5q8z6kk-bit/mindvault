use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mv_core::{
    ChronicleEntry, KnowledgeNode, NodeKind, Proposal, ProposalAction, ProposalSender,
    ProposalState,
};

use crate::auth::{
    authorize_namespace, authorize_read, authorize_write, namespace_for_create, AuthContext,
};
use crate::limits::{enforce_namespace_quota, NamespaceQuotaError};
use crate::state::AppState;
use crate::validation::validate_node_payload;

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

#[derive(Deserialize)]
struct ProposalNodePayload {
    kind: Option<String>,
    content: Option<String>,
    title: Option<String>,
    source: Option<String>,
    namespace: Option<String>,
    tags: Option<Vec<String>>,
    importance: Option<f64>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
pub struct ProposalCountResponse {
    pub count: usize,
}

fn map_mv_error(err: mv_core::MvError) -> (StatusCode, String) {
    match err {
        mv_core::MvError::NodeNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        mv_core::MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn map_namespace_quota_error(err: NamespaceQuotaError) -> (StatusCode, String) {
    match err {
        NamespaceQuotaError::Exceeded {
            namespace,
            quota,
            count,
        } => (
            StatusCode::TOO_MANY_REQUESTS,
            format!("namespace '{namespace}' quota exceeded ({count}/{quota} nodes)"),
        ),
        NamespaceQuotaError::Backend(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("backend error: {msg}"),
        ),
    }
}

// --- Handlers ---

/// GET /api/v1/exchange/proposals
pub async fn list_proposals(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListProposalsQuery>,
) -> Result<Json<Vec<Proposal>>, (StatusCode, String)> {
    authorize_read(&auth)?;

    let filter_state = if let Some(ref s) = params.state {
        Some(
            s.parse::<ProposalState>()
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid state: {e}")))?,
        )
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
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_read(&auth)?;

    let uuid =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

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
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitProposalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let sender: ProposalSender = req
        .sender
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, format!("invalid sender: {e}")))?;
    let action: ProposalAction = req
        .action
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, format!("invalid action: {e}")))?;

    let mut proposal = Proposal::new(sender, action);

    if let Some(ref target_id_str) = req.target_node_id {
        let target_id = Uuid::parse_str(target_id_str).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "invalid target_node_id".to_string(),
            )
        })?;
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
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let uuid =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

    let proposal = state
        .engine
        .get_proposal(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "proposal not found".to_string()))?;

    let mut created_node_id: Option<String> = None;
    let mut updated_node_id: Option<String> = None;
    let mut deleted_node_id: Option<String> = None;

    match &proposal.action {
        ProposalAction::CreateNode => {
            let payload_value = serde_json::to_value(&proposal.payload)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid payload: {e}")))?;
            let payload: ProposalNodePayload = serde_json::from_value(payload_value)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid payload: {e}")))?;

            let content = payload.content.ok_or((
                StatusCode::BAD_REQUEST,
                "proposal payload missing content".to_string(),
            ))?;
            let kind_raw = payload.kind.unwrap_or_else(|| "fact".to_string());
            let kind: NodeKind = kind_raw
                .parse()
                .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;
            let tags = payload.tags.unwrap_or_default();

            validate_node_payload(
                kind,
                payload.title.as_deref(),
                &content,
                payload.source.as_deref(),
                payload.namespace.as_deref(),
                &tags,
                payload.importance,
                payload.metadata.as_ref(),
            )
            .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

            let namespace = namespace_for_create(&auth, payload.namespace, "default")?;
            enforce_namespace_quota(&state.engine, &namespace)
                .await
                .map_err(map_namespace_quota_error)?;

            let mut node = KnowledgeNode::new(kind, content).with_namespace(namespace);
            if let Some(title) = payload.title {
                node = node.with_title(title);
            }
            if let Some(source) = payload.source {
                node = node.with_source(source);
            }
            if !tags.is_empty() {
                node = node.with_tags(tags);
            }
            if let Some(importance) = payload.importance {
                node = node.with_importance(importance);
            }
            if let Some(metadata) = payload.metadata {
                node.metadata = metadata;
            }

            let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;
            state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
            created_node_id = Some(stored.id.to_string());
        }
        ProposalAction::UpdateNode | ProposalAction::SuggestTag => {
            let target_id = proposal.target_node_id.ok_or((
                StatusCode::BAD_REQUEST,
                "proposal missing target_node_id".to_string(),
            ))?;
            let existing = state
                .engine
                .get_node(target_id)
                .await
                .map_err(map_mv_error)?
                .ok_or((StatusCode::NOT_FOUND, "target node not found".to_string()))?;

            authorize_namespace(&auth, &existing.namespace)?;

            let payload_value = serde_json::to_value(&proposal.payload)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid payload: {e}")))?;
            let payload: ProposalNodePayload = serde_json::from_value(payload_value)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid payload: {e}")))?;

            let mut updated = existing.clone();
            if let Some(kind) = payload.kind {
                updated.kind = kind
                    .parse()
                    .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;
            }
            if let Some(content) = payload.content {
                updated.content = content;
            }
            if let Some(title) = payload.title {
                updated.title = Some(title);
            }
            if let Some(source) = payload.source {
                updated.source = Some(source);
            }

            let mut tags = updated.tags.clone();
            if matches!(&proposal.action, ProposalAction::SuggestTag) {
                if let Some(tag_val) = proposal.payload.get("tag").and_then(|v| v.as_str()) {
                    let tag = tag_val.trim();
                    if !tag.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                        tags.push(tag.to_string());
                    }
                } else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "proposal payload missing tag".to_string(),
                    ));
                }
            } else if let Some(new_tags) = payload.tags {
                tags = new_tags;
            }

            if let Some(importance) = payload.importance {
                updated.importance = importance;
            }
            if let Some(metadata) = payload.metadata {
                updated.metadata = metadata;
            }

            if let Some(namespace) = payload.namespace {
                let ns = namespace_for_create(&auth, Some(namespace), &existing.namespace)?;
                updated.namespace = ns;
            }

            validate_node_payload(
                updated.kind,
                updated.title.as_deref(),
                &updated.content,
                updated.source.as_deref(),
                Some(&updated.namespace),
                &tags,
                Some(updated.importance),
                Some(&updated.metadata),
            )
            .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

            updated.tags = tags;
            let saved = state
                .engine
                .update_node(updated)
                .await
                .map_err(map_mv_error)?;
            state.notify_change(&saved.id.to_string(), "update", Some(&saved.namespace));
            updated_node_id = Some(saved.id.to_string());
        }
        ProposalAction::DeleteNode => {
            let target_id = proposal.target_node_id.ok_or((
                StatusCode::BAD_REQUEST,
                "proposal missing target_node_id".to_string(),
            ))?;
            let existing = state
                .engine
                .get_node(target_id)
                .await
                .map_err(map_mv_error)?
                .ok_or((StatusCode::NOT_FOUND, "target node not found".to_string()))?;
            authorize_namespace(&auth, &existing.namespace)?;

            let deleted = state
                .engine
                .delete_node(target_id)
                .await
                .map_err(map_mv_error)?;
            if deleted {
                state.notify_change(&target_id.to_string(), "delete", Some(&existing.namespace));
                deleted_node_id = Some(target_id.to_string());
            } else {
                return Err((StatusCode::NOT_FOUND, "target node not found".to_string()));
            }
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "proposal action not supported for approval".to_string(),
            ))
        }
    }

    let resolved = state
        .engine
        .resolve_proposal(uuid, ProposalState::Approved)
        .await
        .map_err(map_mv_error)?;

    if !resolved {
        return Err((StatusCode::NOT_FOUND, "proposal not found".to_string()));
    }

    // Log chronicle entry for transparency
    let chronicle =
        ChronicleEntry::new("exchange.approve", format!("User approved proposal {uuid}"));
    let _ = state.engine.log_chronicle(&chronicle).await;

    Ok(Json(serde_json::json!({
        "id": uuid.to_string(),
        "state": "approved",
        "created_node_id": created_node_id,
        "updated_node_id": updated_node_id,
        "deleted_node_id": deleted_node_id
    })))
}

/// POST /api/v1/exchange/proposals/:id/reject
pub async fn reject_proposal(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let uuid =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

    let resolved = state
        .engine
        .resolve_proposal(uuid, ProposalState::Rejected)
        .await
        .map_err(map_mv_error)?;

    if !resolved {
        return Err((StatusCode::NOT_FOUND, "proposal not found".to_string()));
    }

    // Log chronicle entry for transparency
    let chronicle =
        ChronicleEntry::new("exchange.reject", format!("User rejected proposal {uuid}"));
    let _ = state.engine.log_chronicle(&chronicle).await;

    Ok(Json(
        serde_json::json!({ "id": uuid.to_string(), "state": "rejected" }),
    ))
}

/// GET /api/v1/exchange/inbox/count
pub async fn inbox_count(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProposalCountResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;

    let count = state
        .engine
        .count_proposals(Some(ProposalState::Pending))
        .await
        .map_err(map_mv_error)?;

    Ok(Json(ProposalCountResponse { count }))
}
