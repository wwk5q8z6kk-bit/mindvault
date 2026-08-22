use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use mv_core::{KnowledgeWorkspace, MvError, WorkspaceConflict, WORKSPACE_CONFLICT_RESOLUTIONS};
use mv_engine::engine::WorkspaceProjectionOutcome;
use mv_engine::workspace::{
    decode_workspace_descriptor, WorkspaceDocumentRead, WorkspaceReconciliationOutcome,
    WorkspaceTree,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{
    authorize_namespace, authorize_read, authorize_write, scoped_namespace, AuthContext,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub(crate) struct ListWorkspacesQuery {
    namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MountWorkspaceRequest {
    root_path: String,
    namespace: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkspaceSummary {
    id: Uuid,
    namespace: String,
    display_name: String,
    root_name: String,
    mode: String,
    state: String,
    revision: u64,
    last_reconciled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkspaceReconciliationResponse {
    applied: bool,
    inserted_documents: usize,
    updated_documents: usize,
    unchanged_documents: usize,
    renamed_documents: usize,
    projection: WorkspaceProjectionOutcome,
    diagnostics: Vec<mv_engine::workspace::WorkspaceScanDiagnostic>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MountWorkspaceResponse {
    workspace: WorkspaceSummary,
    reconciliation: WorkspaceReconciliationResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReconcileWorkspaceResponse {
    workspace: WorkspaceSummary,
    reconciliation: WorkspaceReconciliationResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct RebuildWorkspaceProjectionsResponse {
    workspace: WorkspaceSummary,
    projection: WorkspaceProjectionOutcome,
}

pub(crate) async fn list_workspaces(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListWorkspacesQuery>,
) -> Result<Json<Vec<WorkspaceSummary>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let namespace = scoped_namespace(&auth, query.namespace)?;
    let workspaces = state
        .engine
        .list_knowledge_workspaces(namespace.as_deref())
        .await
        .map_err(map_workspace_error)?;
    let summaries = workspaces
        .into_iter()
        .map(workspace_summary)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(summaries))
}

pub(crate) async fn mount_workspace(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<MountWorkspaceRequest>,
) -> Result<(StatusCode, Json<MountWorkspaceResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;
    if !auth.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            "administrator permission is required to mount local filesystem roots".into(),
        ));
    }
    let namespace = request.namespace.unwrap_or_else(|| "default".into());
    authorize_namespace(&auth, &namespace)?;
    let root = state
        .workspace_root_policy
        .authorize(FsPath::new(&request.root_path))
        .map_err(|message| (StatusCode::FORBIDDEN, message))?;
    let display_name = request
        .display_name
        .unwrap_or_else(|| default_workspace_name(&root));

    let mounted = state
        .engine
        .mount_knowledge_workspace(&root, namespace, display_name)
        .await
        .map_err(map_workspace_error)?;
    let response = MountWorkspaceResponse {
        workspace: workspace_summary(mounted.workspace)?,
        reconciliation: reconciliation_response(mounted.reconciliation),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub(crate) async fn get_workspace_tree(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceTree>, (StatusCode, String)> {
    let workspace = authorized_workspace(&auth, &state, workspace_id).await?;
    let tree = state
        .engine
        .knowledge_workspace_tree(workspace.id)
        .await
        .map_err(map_workspace_error)?;
    Ok(Json(tree))
}

pub(crate) async fn reconcile_workspace(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<ReconcileWorkspaceResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let workspace = authorized_workspace(&auth, &state, workspace_id).await?;
    let reconciliation = state
        .engine
        .reconcile_knowledge_workspace(workspace.id)
        .await
        .map_err(map_workspace_error)?;
    if !reconciliation.applied {
        return Err((
            StatusCode::CONFLICT,
            "workspace changed concurrently; retry reconciliation".into(),
        ));
    }
    let workspace = state
        .engine
        .get_knowledge_workspace(workspace.id)
        .await
        .map_err(map_workspace_error)?;
    Ok(Json(ReconcileWorkspaceResponse {
        workspace: workspace_summary(workspace)?,
        reconciliation: reconciliation_response(reconciliation),
    }))
}

pub(crate) async fn rebuild_workspace_projections(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<RebuildWorkspaceProjectionsResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let workspace = authorized_workspace(&auth, &state, workspace_id).await?;
    let projection = state
        .engine
        .rebuild_knowledge_workspace_projections(workspace.id)
        .await
        .map_err(map_workspace_error)?;
    let workspace = state
        .engine
        .get_knowledge_workspace(workspace.id)
        .await
        .map_err(map_workspace_error)?;
    Ok(Json(RebuildWorkspaceProjectionsResponse {
        workspace: workspace_summary(workspace)?,
        projection,
    }))
}

pub(crate) async fn read_workspace_document(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((workspace_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<WorkspaceDocumentRead>, (StatusCode, String)> {
    let workspace = authorized_workspace(&auth, &state, workspace_id).await?;
    let document = state
        .engine
        .read_knowledge_workspace_document(workspace.id, document_id)
        .await
        .map_err(map_workspace_error)?;
    Ok(Json(document))
}


#[derive(Debug, Serialize)]
pub(crate) struct WorkspaceConflictView {
    id: Uuid,
    workspace_id: Uuid,
    document_id: Option<Uuid>,
    conflict_kind: String,
    state: String,
    relative_path: Option<String>,
    expected_content_hash: Option<String>,
    observed_content_hash: Option<String>,
    canonical_content_hash: Option<String>,
    available_resolutions: Vec<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct WorkspaceConflictsResponse {
    conflicts: Vec<WorkspaceConflictView>,
    resolution_choices: Vec<&'static str>,
}

/// GET /api/v1/workspaces/:id/conflicts — list open conflicts for review.
pub(crate) async fn list_workspace_conflicts(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceConflictsResponse>, (StatusCode, String)> {
    let workspace = authorized_workspace(&auth, &state, workspace_id).await?;
    let conflicts = state
        .engine
        .list_workspace_conflicts(workspace.id, true)
        .await
        .map_err(map_workspace_error)?;
    let views = conflicts
        .into_iter()
        .map(conflict_view)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(WorkspaceConflictsResponse {
        conflicts: views,
        resolution_choices: WORKSPACE_CONFLICT_RESOLUTIONS.to_vec(),
    }))
}

fn conflict_view(conflict: WorkspaceConflict) -> Result<WorkspaceConflictView, (StatusCode, String)> {
    let payload: mv_core::WorkspaceConflictPayloadV1 = serde_json::from_slice(&conflict.conflict_payload)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("conflict payload is not valid JSON: {err}"),
            )
        })?;
    Ok(WorkspaceConflictView {
        id: conflict.id,
        workspace_id: conflict.workspace_id,
        document_id: conflict.document_id,
        conflict_kind: conflict.conflict_kind.as_str().to_string(),
        state: conflict.state.as_str().to_string(),
        relative_path: payload.relative_path,
        expected_content_hash: payload.expected_content_hash,
        observed_content_hash: payload.observed_content_hash,
        canonical_content_hash: payload.canonical_content_hash,
        available_resolutions: payload.available_resolutions,
        created_at: conflict.created_at,
    })
}

async fn authorized_workspace(
    auth: &AuthContext,
    state: &AppState,
    workspace_id: Uuid,
) -> Result<KnowledgeWorkspace, (StatusCode, String)> {
    authorize_read(auth)?;
    let workspace = state
        .engine
        .get_knowledge_workspace(workspace_id)
        .await
        .map_err(map_workspace_error)?;
    authorize_namespace(auth, &workspace.namespace)?;
    Ok(workspace)
}

fn workspace_summary(
    workspace: KnowledgeWorkspace,
) -> Result<WorkspaceSummary, (StatusCode, String)> {
    let descriptor = decode_workspace_descriptor(&workspace).map_err(map_workspace_error)?;
    let root_name = PathBuf::from(&descriptor.root_path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Workspace")
        .to_string();
    Ok(WorkspaceSummary {
        id: workspace.id,
        namespace: workspace.namespace,
        display_name: descriptor.display_name,
        root_name,
        mode: workspace.mode.as_str().into(),
        state: workspace.state.as_str().into(),
        revision: workspace.revision,
        last_reconciled_at: workspace.last_reconciled_at,
    })
}

fn reconciliation_response(
    outcome: WorkspaceReconciliationOutcome,
) -> WorkspaceReconciliationResponse {
    WorkspaceReconciliationResponse {
        applied: outcome.applied,
        inserted_documents: outcome.inserted_documents,
        updated_documents: outcome.updated_documents,
        unchanged_documents: outcome.unchanged_documents,
        renamed_documents: outcome.renamed_documents,
        projection: outcome.projection,
        diagnostics: outcome.diagnostics,
    }
}

fn default_workspace_name(root: &FsPath) -> String {
    root.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Workspace")
        .to_string()
}

fn map_workspace_error(error: MvError) -> (StatusCode, String) {
    let status = match &error {
        MvError::NodeNotFound(_) => StatusCode::NOT_FOUND,
        MvError::DuplicateNode(_)
        | MvError::IdempotencyConflict(_)
        | MvError::CanonicalSourceConflict(_) => StatusCode::CONFLICT,
        MvError::InvalidInput(_) | MvError::Serialization(_) => StatusCode::BAD_REQUEST,
        MvError::AccessDenied(_) | MvError::Auth(_) => StatusCode::FORBIDDEN,
        MvError::VaultSealed => StatusCode::LOCKED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, error.to_string())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthContext;
    use crate::state::WorkspaceRootPolicy;
    use axum::extract::{Path as AxumPath, State};
    use axum::Extension;
    use mv_engine::config::EngineConfig;
    use mv_engine::engine::MindVaultEngine;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn test_state(allowed_roots: Vec<PathBuf>) -> (Arc<AppState>, TempDir) {
        let tmp = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: tmp.path().join("data").to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.llm.auto_detect = false;
        let engine = MindVaultEngine::init(config).await.unwrap();
        let state = Arc::new(
            AppState::new(Arc::new(engine)).with_workspace_allowed_roots(allowed_roots),
        );
        (state, tmp)
    }

    #[tokio::test]
    async fn workspaces_mount_rejects_non_allowlisted_root() {
        let (state, tmp) = test_state(vec![]).await;
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let err = mount_workspace(
            Extension(AuthContext::system_admin()),
            State(state),
            Json(MountWorkspaceRequest {
                root_path: outside.to_string_lossy().into(),
                namespace: Some("default".into()),
                display_name: Some("Outside".into()),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn workspaces_tree_rejects_unknown_workspace_id() {
        let (state, _tmp) = test_state(vec![std::env::temp_dir()]).await;
        let err = get_workspace_tree(
            Extension(AuthContext::system_admin()),
            State(state),
            AxumPath(Uuid::now_v7()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn workspaces_document_read_rejects_unknown_document_id() {
        let allowed = TempDir::new().unwrap();
        let root = allowed.path().join("knowledge");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Note.md"), "# note\n").unwrap();
        let (state, _tmp) = test_state(vec![allowed.path().to_path_buf()]).await;
        let mounted = state
            .engine
            .mount_knowledge_workspace(&root, "default", "Docs")
            .await
            .unwrap();
        let err = read_workspace_document(
            Extension(AuthContext::system_admin()),
            State(state),
            AxumPath((mounted.workspace.id, Uuid::now_v7())),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[test]
    fn workspaces_root_policy_is_disabled_without_allowlist() {
        let policy = WorkspaceRootPolicy::default();
        let err = policy.authorize(FsPath::new("/tmp")).unwrap_err();
        assert!(err.contains("disabled"));
    }
}
