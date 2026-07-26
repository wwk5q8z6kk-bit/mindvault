use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mv_core::traits::ConversationStore;

use crate::state::AppState;

const MAX_SOURCES_PER_MESSAGE: usize = 32;
const MAX_SOURCE_PREVIEW_CHARS: usize = 500;
const MAX_SOURCE_TITLE_CHARS: usize = 200;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

#[derive(Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationSourceDto {
    pub node_id: String,
    pub title: String,
    pub kind: String,
    pub score: f64,
    pub preview: String,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub role: String,
    pub content: String,
    pub sources: Option<Vec<ConversationSourceDto>>,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<ConversationSourceDto>>,
}

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Serialize)]
pub struct ConversationListItem {
    pub id: String,
    pub title: Option<String>,
    pub updated_at: String,
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>() + "..."
}

pub(crate) fn normalize_sources(
    sources: Option<Vec<ConversationSourceDto>>,
) -> Result<Option<String>, String> {
    let Some(sources) = sources else {
        return Ok(None);
    };
    if sources.is_empty() {
        return Ok(None);
    }
    if sources.len() > MAX_SOURCES_PER_MESSAGE {
        return Err(format!(
            "sources cannot exceed {MAX_SOURCES_PER_MESSAGE} items"
        ));
    }

    let mut normalized = Vec::with_capacity(sources.len());
    for source in sources {
        let node_id = source.node_id.trim().to_string();
        if node_id.is_empty() {
            return Err("source.node_id is required".into());
        }
        if Uuid::parse_str(&node_id).is_err() {
            // Allow non-uuid ids used by tests/clients, but reject empty/control junk.
            if node_id.len() > 128 {
                return Err("source.node_id is too long".into());
            }
        }
        let title = truncate_chars(source.title.trim(), MAX_SOURCE_TITLE_CHARS);
        let kind = source.kind.trim().to_string();
        let preview = truncate_chars(source.preview.trim(), MAX_SOURCE_PREVIEW_CHARS);
        if kind.is_empty() {
            return Err("source.kind is required".into());
        }
        normalized.push(ConversationSourceDto {
            node_id,
            title: if title.is_empty() {
                "Untitled".to_string()
            } else {
                title
            },
            kind,
            score: source.score.clamp(0.0, 1.0e6),
            preview,
        });
    }

    serde_json::to_string(&normalized)
        .map(Some)
        .map_err(|err| err.to_string())
}

pub(crate) fn parse_sources_json(raw: Option<&str>) -> Option<Vec<ConversationSourceDto>> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed = serde_json::from_str::<Vec<ConversationSourceDto>>(raw).ok()?;
    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/conversations
pub async fn create_conversation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConversationRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    match state
        .engine
        .store
        .nodes
        .create_conversation(id, req.title.as_deref())
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(ConversationResponse {
                id: id.to_string(),
                title: req.title,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/conversations
pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match state
        .engine
        .store
        .nodes
        .list_conversations(params.limit, params.offset)
        .await
    {
        Ok(convs) => {
            let items: Vec<ConversationListItem> = convs
                .into_iter()
                .map(|(id, title, ts)| ConversationListItem {
                    id: id.to_string(),
                    title,
                    updated_at: ts.to_rfc3339(),
                })
                .collect();
            Json(items).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /api/v1/conversations/:id/message
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid conversation id"})),
            )
                .into_response()
        }
    };

    let sources_json = match normalize_sources(req.sources) {
        Ok(value) => value,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": message})),
            )
                .into_response()
        }
    };

    match state
        .engine
        .store
        .nodes
        .add_message(conv_id, &req.role, &req.content, sources_json.as_deref())
        .await
    {
        Ok(msg_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "id": msg_id.to_string(),
                "conversation_id": id,
                "role": req.role,
                "sources": parse_sources_json(sources_json.as_deref()),
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/conversations/:id/messages
pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid conversation id"})),
            )
                .into_response()
        }
    };

    match state
        .engine
        .store
        .nodes
        .get_messages(conv_id, params.limit)
        .await
    {
        Ok(messages) => {
            let items: Vec<MessageResponse> = messages
                .into_iter()
                .map(|(id, role, content, sources_json, ts)| MessageResponse {
                    id: id.to_string(),
                    role,
                    content,
                    created_at: ts.to_rfc3339(),
                    sources: parse_sources_json(sources_json.as_deref()),
                })
                .collect();
            Json(items).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/conversations/:id
pub async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid conversation id"})),
            )
                .into_response()
        }
    };

    match state.engine.store.nodes.delete_conversation(conv_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "conversation not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_sources_serializes_trimmed_payload() {
        let json = normalize_sources(Some(vec![ConversationSourceDto {
            node_id: "  n1  ".into(),
            title: "  Launch  ".into(),
            kind: " fact ".into(),
            score: 0.91,
            preview: "Ship hybrid search".into(),
        }]))
        .expect("normalize")
        .expect("some json");

        let parsed: Vec<ConversationSourceDto> =
            serde_json::from_str(&json).expect("parse sources json");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].node_id, "n1");
        assert_eq!(parsed[0].title, "Launch");
        assert_eq!(parsed[0].kind, "fact");
    }

    #[test]
    fn normalize_sources_rejects_over_limit() {
        let sources = (0..MAX_SOURCES_PER_MESSAGE + 1)
            .map(|i| ConversationSourceDto {
                node_id: format!("n{i}"),
                title: "T".into(),
                kind: "fact".into(),
                score: 0.1,
                preview: "p".into(),
            })
            .collect();
        let err = normalize_sources(Some(sources)).expect_err("should reject");
        assert!(err.contains("cannot exceed"));
    }

    #[test]
    fn parse_sources_json_returns_none_for_invalid() {
        assert!(parse_sources_json(None).is_none());
        assert!(parse_sources_json(Some("")).is_none());
        assert!(parse_sources_json(Some("{not-json")).is_none());
        assert!(parse_sources_json(Some("[]")).is_none());
    }
}
