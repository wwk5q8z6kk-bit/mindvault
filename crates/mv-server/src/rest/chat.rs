use std::sync::Arc;

use axum::{extract::State, Extension, Json};
use axum::http::StatusCode;
use mv_core::{MemoryQuery, QueryFilters, SearchResult, SearchStrategy};
use mv_engine::llm;
use serde::{Deserialize, Serialize};

use crate::auth::{authorize_read, scoped_namespace, AuthContext};
use crate::limits::enforce_ai_rate_limit;
use crate::state::AppState;
use crate::validation::{validate_query_text, validate_recall_limit};

fn map_mv_error(err: mv_core::MvError) -> (StatusCode, String) {
    match err {
        mv_core::MvError::NodeNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        mv_core::MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        mv_core::MvError::DuplicateNode(_) => (StatusCode::CONFLICT, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatHistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub history: Option<Vec<ChatHistoryMessage>>,
    pub limit: Option<usize>,
    pub strategy: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChatSourceDto {
    pub node_id: String,
    pub title: String,
    pub kind: String,
    pub score: f64,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<ChatSourceDto>,
    pub provider: String,
    pub mode: String,
    pub grounded: bool,
}

fn preview_content(content: &str, max_chars: usize) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }
    collapsed.chars().take(max_chars).collect::<String>() + "..."
}

fn sources_from_results(results: &[SearchResult], limit: usize) -> Vec<ChatSourceDto> {
    results
        .iter()
        .take(limit)
        .map(|result| ChatSourceDto {
            node_id: result.node.id.to_string(),
            title: result
                .node
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string()),
            kind: result.node.kind.as_str().to_string(),
            score: result.score,
            preview: preview_content(&result.node.content, 200),
        })
        .collect()
}

fn numbered_context(results: &[SearchResult], limit: usize) -> String {
    results
        .iter()
        .take(limit)
        .enumerate()
        .map(|(idx, result)| {
            let title = result.node.title.as_deref().unwrap_or("Untitled");
            let body = preview_content(&result.node.content, 500);
            format!("[{}] {title}\n{body}", idx + 1)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn heuristic_chat_answer(question: &str, results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "I could not find relevant notes or tasks in your vault for that question. Try different keywords, or capture related material first.".to_string();
    }

    let mut out = String::from(
        "I found related material in your vault. Here is a grounded digest with citations:\n",
    );
    for (idx, result) in results.iter().take(5).enumerate() {
        let title = result.node.title.as_deref().unwrap_or("Untitled");
        let preview = preview_content(&result.node.content, 180);
        out.push_str(&format!("\n[{}] **{title}** — {preview}", idx + 1));
    }
    out.push_str(&format!(
        "\n\nFor a synthesized answer to “{question}”, configure an LLM provider (local or remote)."
    ));
    out
}

fn empty_vault_answer() -> String {
    "I could not find relevant notes or tasks in your vault for that question. Try different keywords, or capture related material first.".to_string()
}

/// POST /api/v1/chat — grounded RAG chat over the owner's vault.
pub async fn chat(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    enforce_ai_rate_limit(&auth).map_err(|err| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "AI rate limit exceeded; retry after {} seconds (limit {} per {}s)",
                err.retry_after_secs, err.max_requests, err.window_secs
            ),
        )
    })?;
    validate_query_text("message", &req.message)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    let requested_limit = req.limit.unwrap_or(8);
    validate_recall_limit(requested_limit)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    let limit = requested_limit.clamp(1, 16);

    let strategy = req
        .strategy
        .as_deref()
        .unwrap_or("hybrid")
        .parse::<SearchStrategy>()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    let query = MemoryQuery {
        text: req.message.clone(),
        strategy,
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, req.namespace.take())?,
            ..Default::default()
        },
        limit,
        min_score: 0.0,
        rewrite_strategy: None,
        session_id: None,
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;
    let sources = sources_from_results(&results, limit);

    if sources.is_empty() {
        return Ok(Json(ChatResponse {
            answer: empty_vault_answer(),
            sources: vec![],
            provider: "retrieval-empty".to_string(),
            mode: "native".to_string(),
            grounded: false,
        }));
    }

    let history: Vec<(String, String)> = req
        .history
        .unwrap_or_default()
        .into_iter()
        .filter(|item| !item.content.trim().is_empty())
        .filter(|item| {
            let role = item.role.trim().to_ascii_lowercase();
            role == "user" || role == "assistant"
        })
        .map(|item| (item.role, item.content))
        .collect();

    let context = numbered_context(&results, limit);

    if let Some(ref llm_provider) = state.engine.llm {
        match llm::llm_chat_answer(
            llm_provider.as_ref(),
            &req.message,
            &history,
            &context,
        )
        .await
        {
            Ok(answer) => {
                return Ok(Json(ChatResponse {
                    answer,
                    sources,
                    provider: "native-rag-llm".to_string(),
                    mode: "native".to_string(),
                    grounded: true,
                }));
            }
            Err(err) => {
                tracing::warn!(error = %err, "native chat LLM failed; using heuristic digest");
            }
        }
    }

    Ok(Json(ChatResponse {
        answer: heuristic_chat_answer(&req.message, &results),
        sources,
        provider: "native-rag-heuristic".to_string(),
        mode: "native".to_string(),
        grounded: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_core::{KnowledgeNode, MatchSource, NodeKind};

    #[test]
    fn heuristic_answer_includes_citations() {
        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Ship hybrid search defaults across the product.".to_string(),
        )
        .with_title("Search defaults");
        let results = vec![SearchResult {
            node,
            score: 0.9,
            match_source: MatchSource::Hybrid,
        }];
        let answer = heuristic_chat_answer("What should we ship?", &results);
        assert!(answer.contains("[1]"));
        assert!(answer.contains("Search defaults"));
        assert!(answer.contains("What should we ship?"));
    }

    #[test]
    fn empty_results_use_ungrounded_copy() {
        let answer = heuristic_chat_answer("obscure", &[]);
        assert!(answer.to_ascii_lowercase().contains("could not find"));
    }
}
