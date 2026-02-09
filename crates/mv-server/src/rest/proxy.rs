use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mv_core::{ExecProxyRequest, HttpProxyRequest, ProxyAuditEntry};
use mv_engine::proxy::ProxyEngine;

use crate::auth::{authorize_read, authorize_write, AuthContext};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct HttpProxyResponseDto {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub sanitized: bool,
}

#[derive(Serialize)]
pub struct ExecProxyResponseDto {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub sanitized: bool,
}

#[derive(Serialize)]
pub struct AuditEntryResponse {
    pub id: Uuid,
    pub consumer: String,
    pub secret_ref: String,
    pub action: String,
    pub target: String,
    pub intent: String,
    pub timestamp: String,
    pub success: Option<bool>,
    pub sanitized: bool,
    pub error: Option<String>,
    pub request_summary: String,
    pub response_status: Option<i32>,
}

impl From<ProxyAuditEntry> for AuditEntryResponse {
    fn from(e: ProxyAuditEntry) -> Self {
        Self {
            id: e.id,
            consumer: e.consumer,
            secret_ref: e.secret_ref,
            action: e.action,
            target: e.target,
            intent: e.intent,
            timestamp: e.timestamp.to_rfc3339(),
            success: e.success,
            sanitized: e.sanitized,
            error: e.error,
            request_summary: e.request_summary,
            response_status: e.response_status,
        }
    }
}

#[derive(Deserialize)]
pub struct AuditListQuery {
    pub consumer: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/proxy/http
///
/// Proxy an HTTP request with credential injection. Requires consumer authentication.
pub async fn proxy_http(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<HttpProxyRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_write(&auth) {
        return (
            err.0,
            Json(ErrorBody { error: err.1 }),
        )
            .into_response();
    }

    let consumer_name = match &auth.consumer_name {
        Some(name) => name.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "proxy requires consumer authentication (use a consumer bearer token)"
                        .into(),
                }),
            )
                .into_response();
        }
    };

    if req.url.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "url is required".into(),
            }),
        )
            .into_response();
    }

    if req.secret_ref.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "secret_ref is required".into(),
            }),
        )
            .into_response();
    }

    match ProxyEngine::execute_http(&req, &consumer_name, &state.engine).await {
        Ok(response) => Json(HttpProxyResponseDto {
            status: response.status,
            headers: response.headers,
            body: response.body,
            sanitized: response.sanitized,
        })
        .into_response(),
        Err(e) => {
            let status = if e.contains("access denied") || e.contains("no policy") {
                StatusCode::FORBIDDEN
            } else if e.contains("blocked") {
                StatusCode::FORBIDDEN
            } else if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(ErrorBody { error: e })).into_response()
        }
    }
}

/// POST /api/v1/proxy/exec
///
/// Run a command with credential env injection. Requires consumer authentication.
/// Uses tokio::process::Command with explicit arg list (no shell interpolation).
pub async fn proxy_exec(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecProxyRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_write(&auth) {
        return (
            err.0,
            Json(ErrorBody { error: err.1 }),
        )
            .into_response();
    }

    let consumer_name = match &auth.consumer_name {
        Some(name) => name.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "proxy requires consumer authentication (use a consumer bearer token)"
                        .into(),
                }),
            )
                .into_response();
        }
    };

    if req.command.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "command is required".into(),
            }),
        )
            .into_response();
    }

    match ProxyEngine::execute_exec(&req, &consumer_name, &state.engine).await {
        Ok(response) => Json(ExecProxyResponseDto {
            exit_code: response.exit_code,
            stdout: response.stdout,
            stderr: response.stderr,
            sanitized: response.sanitized,
        })
        .into_response(),
        Err(e) => {
            let status = if e.contains("access denied") || e.contains("no policy") {
                StatusCode::FORBIDDEN
            } else if e.contains("blocked") || e.contains("not in the allowed list") {
                StatusCode::FORBIDDEN
            } else if e.contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.contains("timed out") {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(ErrorBody { error: e })).into_response()
        }
    }
}

/// GET /api/v1/proxy/audit
///
/// List proxy audit log entries. Admins see all; consumers see their own.
pub async fn list_audit(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditListQuery>,
) -> impl IntoResponse {
    if let Err(err) = authorize_read(&auth) {
        return (
            err.0,
            Json(ErrorBody { error: err.1 }),
        )
            .into_response();
    }

    // If authenticated as consumer, scope to their own entries
    let consumer_filter = match &auth.consumer_name {
        Some(name) => Some(name.as_str()),
        None => query.consumer.as_deref(),
    };

    let limit = query.limit.min(500);

    match state
        .engine
        .list_proxy_audit(consumer_filter, limit, query.offset)
        .await
    {
        Ok(entries) => {
            let responses: Vec<AuditEntryResponse> =
                entries.into_iter().map(AuditEntryResponse::from).collect();
            Json(responses).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: format!("failed to list audit entries: {e}"),
            }),
        )
            .into_response(),
    }
}
