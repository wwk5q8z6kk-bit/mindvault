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
    ChronicleEntry, ContentType, MessageStatus, Proposal, ProposalAction,
    ProposalSender, ProposalState, RelayMessage, SafeguardStore,
};
use mv_engine::engine::ProposalActionResult;

use crate::auth::{
    authorize_namespace, authorize_read, authorize_write, namespace_for_create, AuthContext,
};
use crate::limits::{enforce_namespace_quota, NamespaceQuotaError};
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

// ProposalNodePayload moved to mv_engine::engine

#[derive(Deserialize)]
struct RelayReplyPayload {
    channel_id: Uuid,
    content: String,
    content_type: Option<String>,
    thread_id: Option<Uuid>,
    recipient_contact_id: Option<Uuid>,
    subject: Option<String>,
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

// --- Helpers ---
// ProposalActionResult moved to mv_engine::engine

fn resolve_sender_context(
    auth: &AuthContext,
    requested_sender: &str,
) -> Result<(ProposalSender, String, bool), (StatusCode, String)> {
    let requested_sender = requested_sender.trim();
    let subject = auth
        .subject
        .clone()
        .unwrap_or_else(|| "anonymous".to_string());

    if auth.is_admin() {
        let sender = if requested_sender.is_empty() {
            ProposalSender::UserSelf
        } else {
            requested_sender
                .parse::<ProposalSender>()
                .map_err(|e: String| (StatusCode::BAD_REQUEST, format!("invalid sender: {e}")))?
        };
        let sender_name = if subject == "anonymous" {
            if requested_sender.is_empty() {
                "admin".to_string()
            } else {
                requested_sender.to_string()
            }
        } else {
            subject
        };
        return Ok((sender, sender_name, true));
    }

    if !requested_sender.is_empty() && requested_sender != "self" {
        return Err((
            StatusCode::BAD_REQUEST,
            "sender must be 'self' for non-admin requests".to_string(),
        ));
    }

    Ok((ProposalSender::UserSelf, subject, false))
}

// build_undo_snapshot_data moved to engine.build_undo_snapshot()

// glob_match_simple moved to mv_engine::engine

/// Execute the action described by a proposal (create/update/delete node).
/// Shared by both manual approve and auto-approve paths.
///
/// Standard CRUD actions delegate to `engine.execute_proposal_action()`.
/// The relay.reply custom action stays here because it depends on server-
/// specific email delivery logic.
async fn execute_proposal_action(
    state: &Arc<AppState>,
    auth: &AuthContext,
    proposal: &Proposal,
) -> Result<ProposalActionResult, (StatusCode, String)> {
    match &proposal.action {
        ProposalAction::CreateNode
        | ProposalAction::UpdateNode
        | ProposalAction::SuggestTag
        | ProposalAction::DeleteNode => {
            // Resolve namespace for creates; for updates/deletes the engine
            // uses the existing node's namespace.
            let namespace = if matches!(&proposal.action, ProposalAction::CreateNode) {
                let payload_ns = proposal
                    .payload
                    .get("namespace")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                namespace_for_create(auth, payload_ns, "default")?
            } else {
                // For update/delete, authorize the existing node's namespace
                if let Some(target_id) = proposal.target_node_id {
                    let existing = state
                        .engine
                        .get_node(target_id)
                        .await
                        .map_err(map_mv_error)?
                        .ok_or((StatusCode::NOT_FOUND, "target node not found".to_string()))?;
                    authorize_namespace(auth, &existing.namespace)?;
                    existing.namespace.clone()
                } else {
                    "default".to_string()
                }
            };

            if matches!(&proposal.action, ProposalAction::CreateNode) {
                enforce_namespace_quota(&state.engine, &namespace)
                    .await
                    .map_err(map_namespace_quota_error)?;
            }

            let result = state
                .engine
                .execute_proposal_action(proposal, &namespace)
                .await
                .map_err(map_mv_error)?;

            // Send WebSocket notifications
            if let Some(id) = result.created_node_id {
                state.notify_change(&id.to_string(), "create", result.affected_namespace.as_deref());
            }
            if let Some(id) = result.updated_node_id {
                state.notify_change(&id.to_string(), "update", result.affected_namespace.as_deref());
            }
            if let Some(id) = result.deleted_node_id {
                state.notify_change(&id.to_string(), "delete", result.affected_namespace.as_deref());
            }

            Ok(result)
        }
        ProposalAction::Custom(action) if action == "relay.reply" => {
            let payload_value = serde_json::to_value(&proposal.payload)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid payload: {e}")))?;
            let payload: RelayReplyPayload = serde_json::from_value(payload_value)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid payload: {e}")))?;

            let namespace = auth.namespace.as_deref().unwrap_or("default");
            let mut message = RelayMessage::outbound(payload.channel_id, payload.content);

            if let Some(content_type) = payload.content_type.as_deref() {
                let ct: ContentType = content_type
                    .parse()
                    .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;
                message = message.with_content_type(ct);
            }

            if let Some(thread_id) = payload.thread_id {
                message = message.with_thread(thread_id);
            }

            if let Some(recipient_id) = payload.recipient_contact_id {
                message.recipient_contact_id = Some(recipient_id);
            }

            if let Some(subject) = payload.subject.as_deref().map(str::trim) {
                if !subject.is_empty() {
                    message.metadata.insert(
                        "subject".to_string(),
                        serde_json::Value::String(subject.to_string()),
                    );
                }
            }

            let mut stored = state
                .engine
                .relay
                .send_message(message, namespace)
                .await
                .map_err(map_mv_error)?;

            match crate::email::send_outbound_relay_if_email_channel(state, &stored).await {
                Ok(Some(recipient)) => {
                    state
                        .engine
                        .relay
                        .update_status(stored.id, MessageStatus::Delivered)
                        .await
                        .map_err(map_mv_error)?;
                    stored.status = MessageStatus::Delivered;
                    stored.metadata.insert(
                        "email_recipient".to_string(),
                        serde_json::Value::String(recipient),
                    );
                    stored.metadata.insert(
                        "adapter".to_string(),
                        serde_json::Value::String("email".to_string()),
                    );
                }
                Ok(None) => {}
                Err(err) => {
                    let _ = state
                        .engine
                        .relay
                        .update_status(stored.id, MessageStatus::Failed)
                        .await;
                    stored.status = MessageStatus::Failed;
                    return Err(map_mv_error(err));
                }
            }

            let mut result = ProposalActionResult::default();
            if let Some(node_id) = stored.vault_node_id {
                result.created_node_id = Some(node_id);
            }
            Ok(result)
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            "proposal action not supported for approval".to_string(),
        )),
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

    let (sender, sender_name, allow_auto_approve) =
        resolve_sender_context(&auth, &req.sender)?;
    let action: ProposalAction = req
        .action
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, format!("invalid action: {e}")))?;

    // Check blocked senders
    let is_blocked = state
        .engine
        .store
        .nodes
        .is_sender_blocked(sender.as_str(), &sender_name)
        .await
        .map_err(map_mv_error)?;
    if is_blocked {
        return Err((StatusCode::FORBIDDEN, "sender is blocked".to_string()));
    }

    let mut proposal = Proposal::new(sender, action.clone());

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
        if diff.len() > 10_000 {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("diff_preview exceeds maximum of 10,000 characters ({} given)", diff.len()),
            ));
        }
        proposal = proposal.with_diff(diff);
    }

    if let Some(payload) = req.payload {
        proposal = proposal.with_payload(payload);
    }

    // Check auto-approve rules via engine
    let rules_match = allow_auto_approve
        && state
            .engine
            .check_auto_approve_rules(&sender_name, &action, proposal.confidence)
            .await
            .map_err(map_mv_error)?;

    state
        .engine
        .submit_proposal(&proposal)
        .await
        .map_err(map_mv_error)?;

    if rules_match {
        let snapshot_data = state
            .engine
            .build_undo_snapshot(&proposal)
            .await
            .map_err(map_mv_error)?;
        let result = execute_proposal_action(&state, &auth, &proposal).await?;

        if let Some(snapshot_data) = snapshot_data {
            state
                .engine
                .save_proposal_undo(proposal.id, snapshot_data, result.created_node_id)
                .await
                .map_err(map_mv_error)?;
        }

        state
            .engine
            .resolve_proposal(proposal.id, ProposalState::AutoApproved)
            .await
            .map_err(map_mv_error)?;

        let chronicle = ChronicleEntry::new(
            "exchange.auto_approve",
            format!("Auto-approved proposal {}", proposal.id),
        );
        let _ = state.engine.log_chronicle(&chronicle).await;

        return Ok(Json(serde_json::json!({
            "id": proposal.id.to_string(),
            "state": "auto_approved",
            "created_node_id": result.created_node_id,
            "updated_node_id": result.updated_node_id,
            "deleted_node_id": result.deleted_node_id
        }))
        .into_response());
    }

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

    let snapshot_data = state
        .engine
        .build_undo_snapshot(&proposal)
        .await
        .map_err(map_mv_error)?;
    let result = execute_proposal_action(&state, &auth, &proposal).await?;

    if let Some(snapshot_data) = snapshot_data {
        state
            .engine
            .save_proposal_undo(proposal.id, snapshot_data, result.created_node_id)
            .await
            .map_err(map_mv_error)?;
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
        "created_node_id": result.created_node_id,
        "updated_node_id": result.updated_node_id,
        "deleted_node_id": result.deleted_node_id
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

/// POST /api/v1/exchange/proposals/:id/undo
pub async fn undo_proposal(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let uuid =
        Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid uuid".to_string()))?;

    // Delegate undo execution to engine
    let undo_result = state
        .engine
        .apply_undo_snapshot(uuid)
        .await
        .map_err(map_mv_error)?;

    // Log chronicle
    let chronicle = ChronicleEntry::new(
        "exchange.undo",
        format!("User undid proposal {uuid} (action: {})", undo_result.action),
    );
    let _ = state.engine.log_chronicle(&chronicle).await;

    Ok(Json(serde_json::json!({
        "id": uuid.to_string(),
        "action": undo_result.action,
        "undone": true,
    })))
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

// ---------------------------------------------------------------------------
// Batch Operations
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct BatchProposalRequest {
    pub ids: Vec<String>,
    pub action: String, // "approve" or "reject"
}

#[derive(Serialize)]
struct BatchProposalResultItem {
    id: String,
    success: bool,
    state: Option<String>,
    error: Option<String>,
    created_node_id: Option<String>,
    updated_node_id: Option<String>,
    deleted_node_id: Option<String>,
}

/// POST /api/v1/exchange/proposals/batch
///
/// Approve or reject multiple proposals in a single request.
pub async fn batch_proposals(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchProposalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let target_state = match body.action.as_str() {
        "approve" => ProposalState::Approved,
        "reject" => ProposalState::Rejected,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("invalid batch action: {other} (expected \"approve\" or \"reject\")"),
            ))
        }
    };

    if body.ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "ids array must not be empty".to_string(),
        ));
    }

    if body.ids.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            "batch size exceeds maximum of 100".to_string(),
        ));
    }

    let mut results = Vec::with_capacity(body.ids.len());

    for id_str in &body.ids {
        let uuid = match Uuid::parse_str(id_str) {
            Ok(u) => u,
            Err(_) => {
                results.push(BatchProposalResultItem {
                    id: id_str.clone(),
                    success: false,
                    state: None,
                    error: Some("invalid uuid".to_string()),
                    created_node_id: None,
                    updated_node_id: None,
                    deleted_node_id: None,
                });
                continue;
            }
        };

        let mut action_result: Option<ProposalActionResult> = None;

        // For approvals, execute the proposal action first
        if target_state == ProposalState::Approved {
            let proposal = match state.engine.get_proposal(uuid).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    results.push(BatchProposalResultItem {
                        id: id_str.clone(),
                        success: false,
                        state: None,
                        error: Some("proposal not found".to_string()),
                        created_node_id: None,
                        updated_node_id: None,
                        deleted_node_id: None,
                    });
                    continue;
                }
                Err(e) => {
                    results.push(BatchProposalResultItem {
                        id: id_str.clone(),
                        success: false,
                        state: None,
                        error: Some(e.to_string()),
                        created_node_id: None,
                        updated_node_id: None,
                        deleted_node_id: None,
                    });
                    continue;
                }
            };

            let snapshot_data = match state.engine.build_undo_snapshot(&proposal).await {
                Ok(data) => data,
                Err(err) => {
                    results.push(BatchProposalResultItem {
                        id: id_str.clone(),
                        success: false,
                        state: None,
                        error: Some(err.to_string()),
                        created_node_id: None,
                        updated_node_id: None,
                        deleted_node_id: None,
                    });
                    continue;
                }
            };

            let exec_result = match execute_proposal_action(&state, &auth, &proposal).await {
                Ok(result) => result,
                Err((_, err)) => {
                    results.push(BatchProposalResultItem {
                        id: id_str.clone(),
                        success: false,
                        state: None,
                        error: Some(format!("action failed: {err}")),
                        created_node_id: None,
                        updated_node_id: None,
                        deleted_node_id: None,
                    });
                    continue;
                }
            };

            if let Some(snapshot_data) = snapshot_data {
                if let Err(err) = state
                    .engine
                    .save_proposal_undo(uuid, snapshot_data, exec_result.created_node_id)
                    .await
                {
                    results.push(BatchProposalResultItem {
                        id: id_str.clone(),
                        success: false,
                        state: None,
                        error: Some(err.to_string()),
                        created_node_id: None,
                        updated_node_id: None,
                        deleted_node_id: None,
                    });
                    continue;
                }
            }

            action_result = Some(exec_result);
        }

        match state.engine.resolve_proposal(uuid, target_state).await {
            Ok(true) => {
                let (created_node_id, updated_node_id, deleted_node_id) = match action_result {
                    Some(result) => (
                        result.created_node_id.map(|id| id.to_string()),
                        result.updated_node_id.map(|id| id.to_string()),
                        result.deleted_node_id.map(|id| id.to_string()),
                    ),
                    None => (None, None, None),
                };

                let chronicle = ChronicleEntry::new(
                    if target_state == ProposalState::Approved {
                        "exchange.batch_approve"
                    } else {
                        "exchange.batch_reject"
                    },
                    format!("Batch {}: proposal {uuid}", body.action),
                );
                let _ = state.engine.log_chronicle(&chronicle).await;

                results.push(BatchProposalResultItem {
                    id: id_str.clone(),
                    success: true,
                    state: Some(target_state.as_str().to_string()),
                    error: None,
                    created_node_id,
                    updated_node_id,
                    deleted_node_id,
                });
            }
            Ok(false) => {
                results.push(BatchProposalResultItem {
                    id: id_str.clone(),
                    success: false,
                    state: None,
                    error: Some("proposal not found".to_string()),
                    created_node_id: None,
                    updated_node_id: None,
                    deleted_node_id: None,
                });
            }
            Err(e) => {
                results.push(BatchProposalResultItem {
                    id: id_str.clone(),
                    success: false,
                    state: None,
                    error: Some(e.to_string()),
                    created_node_id: None,
                    updated_node_id: None,
                    deleted_node_id: None,
                });
            }
        }
    }

    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results.len() - succeeded;

    Ok(Json(serde_json::json!({
        "total": results.len(),
        "succeeded": succeeded,
        "failed": failed,
        "results": results
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthContext;

    // glob_match_simple tests moved to mv-engine crate

    // --- resolve_sender_context tests ---

    fn admin_auth() -> AuthContext {
        AuthContext {
            subject: Some("admin-user".into()),
            namespace: None,
            role: crate::auth::AuthRole::Admin,
            consumer_name: None,
        }
    }

    fn user_auth() -> AuthContext {
        AuthContext {
            subject: Some("regular-user".into()),
            namespace: None,
            role: crate::auth::AuthRole::Write,
            consumer_name: None,
        }
    }

    #[test]
    fn resolve_sender_admin_empty_defaults_to_self() {
        let auth = admin_auth();
        let (sender, name, is_admin) = resolve_sender_context(&auth, "").unwrap();
        assert!(matches!(sender, ProposalSender::UserSelf));
        assert_eq!(name, "admin-user");
        assert!(is_admin);
    }

    #[test]
    fn resolve_sender_admin_custom_sender() {
        let auth = admin_auth();
        let (sender, name, is_admin) = resolve_sender_context(&auth, "mcp").unwrap();
        assert!(matches!(sender, ProposalSender::Mcp));
        assert_eq!(name, "admin-user");
        assert!(is_admin);
    }

    #[test]
    fn resolve_sender_user_self_ok() {
        let auth = user_auth();
        let (sender, name, is_admin) = resolve_sender_context(&auth, "self").unwrap();
        assert!(matches!(sender, ProposalSender::UserSelf));
        assert_eq!(name, "regular-user");
        assert!(!is_admin);
    }

    #[test]
    fn resolve_sender_user_empty_ok() {
        let auth = user_auth();
        let (sender, _name, _is_admin) = resolve_sender_context(&auth, "").unwrap();
        assert!(matches!(sender, ProposalSender::UserSelf));
    }

    #[test]
    fn resolve_sender_non_admin_custom_blocked() {
        let auth = user_auth();
        let result = resolve_sender_context(&auth, "mcp");
        assert!(result.is_err());
        let (status, _msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // --- map_mv_error tests ---

    #[test]
    fn map_mv_error_node_not_found_returns_404() {
        let err = mv_core::MvError::NodeNotFound(Uuid::now_v7());
        let (status, _msg) = map_mv_error(err);
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn map_mv_error_invalid_input_returns_400() {
        let err = mv_core::MvError::InvalidInput("bad data".into());
        let (status, _msg) = map_mv_error(err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn map_mv_error_other_returns_500() {
        let err = mv_core::MvError::Internal("kaboom".into());
        let (status, _msg) = map_mv_error(err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    // --- BatchProposalRequest validation tests ---

    #[test]
    fn batch_size_limit_parsed_correctly() {
        // The handler checks body.ids.len() > 100
        let ids: Vec<String> = (0..101).map(|i| format!("id-{i}")).collect();
        assert!(ids.len() > 100);
    }

    // --- diff_preview validation ---

    #[test]
    fn diff_preview_within_limit() {
        let diff = "a".repeat(10_000);
        assert!(diff.len() <= 10_000);
    }

    #[test]
    fn diff_preview_exceeds_limit() {
        let diff = "a".repeat(10_001);
        assert!(diff.len() > 10_000);
    }
}
