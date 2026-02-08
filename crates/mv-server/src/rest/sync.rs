use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use mv_engine::sync::{SyncSnapshot, SyncStats};

use crate::auth::{authorize_read, authorize_write, AuthContext};
use crate::state::AppState;

fn map_mv_error(err: mv_core::MvError) -> (StatusCode, String) {
    match err {
        mv_core::MvError::NodeNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        mv_core::MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        mv_core::MvError::DuplicateNode(_) => (StatusCode::CONFLICT, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

#[derive(Deserialize)]
pub struct SyncExportRequest {
    pub since: Option<String>,
    pub namespace: Option<String>,
}

/// POST /api/v1/sync/export -- Export snapshot
pub async fn sync_export(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<SyncExportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let since = body.since.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    });
    let snapshot = state
        .engine
        .sync
        .export_snapshot(since, body.namespace.as_deref())
        .await
        .map_err(map_mv_error)?;
    Ok(Json(serde_json::to_value(snapshot).unwrap_or_default()))
}

/// POST /api/v1/sync/import -- Import snapshot
pub async fn sync_import(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(snapshot): Json<SyncSnapshot>,
) -> Result<Json<SyncStats>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let stats = state
        .engine
        .sync
        .import_snapshot(snapshot)
        .await
        .map_err(map_mv_error)?;
    Ok(Json(stats))
}

/// GET /api/v1/sync/status
pub async fn sync_status(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_read(&auth)?;
    Ok(Json(serde_json::json!({
        "device_id": state.engine.sync.device_id(),
        "status": "ready"
    })))
}
