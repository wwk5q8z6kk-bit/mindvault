use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mv_engine::federation::{FederatedResult, FederationPeer};

use crate::auth::{authorize_read, authorize_write, AuthContext};
use crate::state::AppState;

// --- DTOs ---

#[derive(Deserialize)]
pub struct AddPeerDto {
    pub vault_id: String,
    pub display_name: String,
    pub endpoint: String,
    pub public_key: Option<String>,
    pub allowed_namespaces: Option<Vec<String>>,
    pub max_results: Option<usize>,
}

#[derive(Deserialize)]
pub struct FederatedQueryDto {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct FederatedQueryResponse {
    pub results: Vec<FederatedResult>,
    pub peer_count: usize,
}

#[derive(Serialize)]
pub struct PeerHealthResponse {
    pub peer_id: String,
    pub healthy: bool,
}

// --- Handlers ---

/// List all federation peers.
pub async fn list_peers(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_read(&auth)?;

    let peers = state.engine.federation.list_peers().await;
    Ok(Json(serde_json::json!({
        "peers": peers,
        "count": peers.len(),
    })))
}

/// Add a new federation peer.
pub async fn add_peer(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddPeerDto>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let mut peer = FederationPeer::new(body.vault_id, body.display_name, body.endpoint);

    if let Some(pk) = body.public_key {
        peer.public_key = Some(pk);
    }
    if let Some(ns) = body.allowed_namespaces {
        peer.allowed_namespaces = ns;
    }
    if let Some(max) = body.max_results {
        peer.max_results = max;
    }

    let id = peer.id;
    state.engine.federation.add_peer(peer).await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id.to_string(), "status": "added" })),
    ))
}

/// Remove a federation peer by ID.
pub async fn remove_peer(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_write(&auth)?;

    let uuid = Uuid::parse_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid peer id: {e}")))?;

    let removed = state.engine.federation.remove_peer(uuid).await;
    if removed {
        Ok(Json(serde_json::json!({ "status": "removed" })))
    } else {
        Err((StatusCode::NOT_FOUND, "peer not found".into()))
    }
}

/// Execute a federated query across all enabled peers.
pub async fn federated_query(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<FederatedQueryDto>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_read(&auth)?;

    let limit = body.limit.unwrap_or(50);
    let results = state
        .engine
        .federation
        .federated_query(&body.query, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let peer_count = state.engine.federation.list_peers().await.len();

    Ok(Json(FederatedQueryResponse {
        results,
        peer_count,
    }))
}

/// Health check a specific federation peer.
pub async fn peer_health(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_read(&auth)?;

    let uuid = Uuid::parse_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid peer id: {e}")))?;

    let healthy = state
        .engine
        .federation
        .health_check(uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PeerHealthResponse {
        peer_id: id,
        healthy,
    }))
}
