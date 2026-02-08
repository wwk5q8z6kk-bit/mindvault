use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Serialize;

use crate::auth::{authorize_read, AuthContext};
use crate::state::AppState;

// --- DTOs ---

#[derive(Serialize)]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub hooks: Vec<String>,
}

#[derive(Serialize)]
pub struct HookPointInfo {
    pub name: String,
    pub description: String,
}

// --- Handlers ---

/// List registered plugins.
pub async fn list_plugins(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_read(&auth)?;

    let registry = state.plugin_registry.read().await;
    let plugins: Vec<PluginSummary> = registry
        .list_plugins()
        .into_iter()
        .map(|m| PluginSummary {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            description: m.description.clone(),
            author: m.author.clone(),
            hooks: m.hooks.clone(),
        })
        .collect();
    let count = plugins.len();
    Ok(Json(serde_json::json!({
        "plugins": plugins,
        "count": count,
    })))
}

/// List available hook points that plugins can register for.
pub async fn list_hook_points(
    Extension(auth): Extension<AuthContext>,
    State(_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    authorize_read(&auth)?;

    let hooks = vec![
        HookPointInfo {
            name: "pre_ingest".into(),
            description: "Before a node is ingested into the vault".into(),
        },
        HookPointInfo {
            name: "post_ingest".into(),
            description: "After a node is ingested".into(),
        },
        HookPointInfo {
            name: "pre_search".into(),
            description: "Before a search query is executed".into(),
        },
        HookPointInfo {
            name: "post_search".into(),
            description: "After search results are returned".into(),
        },
        HookPointInfo {
            name: "on_change".into(),
            description: "When a node is changed (created/updated/deleted)".into(),
        },
        HookPointInfo {
            name: "scheduled".into(),
            description: "On a scheduled interval".into(),
        },
        HookPointInfo {
            name: "on_intent".into(),
            description: "When an intent is detected by the watcher".into(),
        },
    ];

    Ok(Json(serde_json::json!({ "hooks": hooks })))
}
