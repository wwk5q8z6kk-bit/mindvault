use std::fs;
use std::path::{Path, PathBuf};

use mv_core::{
    KnowledgeWorkspace, KnowledgeWorkspaceManifestStore, KnowledgeWorkspaceMode,
    KnowledgeWorkspaceState, MvError, MvResult, WorkspaceDescriptorPayloadV1,
    WorkspaceManifestPayloadFormat,
};
use uuid::Uuid;

use super::MindVaultEngine;
use super::WorkspaceProjectionOutcome;
use crate::workspace::{
    build_workspace_tree, decode_workspace_descriptor, read_workspace_document_from_scan,
    WorkspaceDocumentRead, WorkspaceReconciler, WorkspaceReconciliationOutcome, WorkspaceScanner,
    WorkspaceTree,
};

#[derive(Debug, Clone)]
pub struct WorkspaceMountResult {
    pub workspace: KnowledgeWorkspace,
    pub reconciliation: WorkspaceReconciliationOutcome,
}

impl MindVaultEngine {
    /// Register a local Markdown root and perform its first authoritative scan.
    ///
    /// Filesystem authorization belongs to the caller. The REST boundary uses
    /// an explicit root allowlist before invoking this trusted engine method.
    pub async fn mount_knowledge_workspace(
        &self,
        root: &Path,
        namespace: impl Into<String>,
        display_name: impl Into<String>,
    ) -> MvResult<WorkspaceMountResult> {
        if self.config.sealed_mode {
            return Err(MvError::InvalidInput(
                "mounted workspaces in sealed mode require the encrypted descriptor bridge".into(),
            ));
        }

        let canonical_root = canonical_workspace_root(root)?;
        let root_text = canonical_root.to_str().ok_or_else(|| {
            MvError::InvalidInput("workspace roots must be valid UTF-8 in this release".into())
        })?;
        let namespace = namespace.into();
        if namespace.trim().is_empty() {
            return Err(MvError::InvalidInput(
                "workspace namespace must not be empty".into(),
            ));
        }
        let display_name = display_name.into();
        if display_name.trim().is_empty() || display_name.len() > 120 {
            return Err(MvError::InvalidInput(
                "workspace display name must contain 1 to 120 bytes".into(),
            ));
        }

        for existing in self.store.nodes.list_knowledge_workspaces(None).await? {
            if let Ok(descriptor) = decode_workspace_descriptor(&existing) {
                if descriptor.root_path == root_text {
                    return Err(MvError::DuplicateNode(existing.id));
                }
            }
        }

        let descriptor = WorkspaceDescriptorPayloadV1::new(display_name, root_text);
        let workspace = KnowledgeWorkspace::new(
            namespace,
            KnowledgeWorkspaceMode::Mounted,
            serde_json::to_vec(&descriptor)?,
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        self.store
            .nodes
            .insert_knowledge_workspace(&workspace)
            .await?;

        let reconciler =
            WorkspaceReconciler::new(self.store.nodes.as_ref(), WorkspaceScanner::default());
        let mut reconciliation = match reconciler.reconcile(&workspace, &canonical_root).await {
            Ok(outcome) if outcome.applied => outcome,
            Ok(_) => {
                mark_initial_workspace_error(self, &workspace).await?;
                return Err(MvError::Storage(
                    "initial workspace reconciliation lost its optimistic revision".into(),
                ));
            }
            Err(error) => {
                mark_initial_workspace_error(self, &workspace).await?;
                return Err(error);
            }
        };
        reconciliation.projection = self
            .project_knowledge_workspace(workspace.id, false)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(
                    workspace_id = %workspace.id,
                    error = %error,
                    "initial workspace projection failed after manifest reconciliation"
                );
                WorkspaceProjectionOutcome {
                    failed_documents: reconciliation.inserted_documents,
                    ..WorkspaceProjectionOutcome::default()
                }
            });
        let workspace = self
            .store
            .nodes
            .get_knowledge_workspace(workspace.id)
            .await?
            .ok_or(MvError::NodeNotFound(workspace.id))?;

        Ok(WorkspaceMountResult {
            workspace,
            reconciliation,
        })
    }

    pub async fn list_knowledge_workspaces(
        &self,
        namespace: Option<&str>,
    ) -> MvResult<Vec<KnowledgeWorkspace>> {
        self.store.nodes.list_knowledge_workspaces(namespace).await
    }

    pub async fn get_knowledge_workspace(&self, id: Uuid) -> MvResult<KnowledgeWorkspace> {
        self.store
            .nodes
            .get_knowledge_workspace(id)
            .await?
            .ok_or(MvError::NodeNotFound(id))
    }

    pub async fn reconcile_knowledge_workspace(
        &self,
        workspace_id: Uuid,
    ) -> MvResult<WorkspaceReconciliationOutcome> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        let root = workspace_root(&workspace)?;
        let reconciler =
            WorkspaceReconciler::new(self.store.nodes.as_ref(), WorkspaceScanner::default());
        let mut outcome = reconciler.reconcile(&workspace, &root).await?;
        if outcome.applied {
            outcome.projection = self
                .project_knowledge_workspace(workspace.id, false)
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(
                        workspace_id = %workspace.id,
                        error = %error,
                        "workspace projection failed after manifest reconciliation"
                    );
                    WorkspaceProjectionOutcome {
                        failed_documents: outcome
                            .inserted_documents
                            .saturating_add(outcome.updated_documents),
                        ..WorkspaceProjectionOutcome::default()
                    }
                });
        }
        Ok(outcome)
    }

    pub async fn knowledge_workspace_tree(&self, workspace_id: Uuid) -> MvResult<WorkspaceTree> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        let root = workspace_root(&workspace)?;
        let scan = WorkspaceScanner::default().scan(&workspace, &root)?;
        let documents = self
            .store
            .nodes
            .list_workspace_documents(workspace.id)
            .await?;
        Ok(build_workspace_tree(&workspace, &documents, scan))
    }

    pub async fn read_knowledge_workspace_document(
        &self,
        workspace_id: Uuid,
        document_id: Uuid,
    ) -> MvResult<WorkspaceDocumentRead> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        let document = self
            .store
            .nodes
            .get_workspace_document(document_id)
            .await?
            .ok_or(MvError::NodeNotFound(document_id))?;
        if document.workspace_id != workspace.id {
            // Keep the mismatch indistinguishable from a missing document so
            // callers cannot use this route as a cross-workspace ID oracle.
            return Err(MvError::NodeNotFound(document_id));
        }
        let root = workspace_root(&workspace)?;
        let scan = WorkspaceScanner::default().scan(&workspace, &root)?;
        read_workspace_document_from_scan(&workspace, &document, scan)
    }
}

fn canonical_workspace_root(root: &Path) -> MvResult<PathBuf> {
    let canonical = fs::canonicalize(root)
        .map_err(|error| MvError::InvalidInput(format!("workspace root unavailable: {error}")))?;
    if !canonical.is_dir() {
        return Err(MvError::InvalidInput(
            "workspace root must be a directory".into(),
        ));
    }
    Ok(canonical)
}

pub(super) fn workspace_root(workspace: &KnowledgeWorkspace) -> MvResult<PathBuf> {
    let descriptor = decode_workspace_descriptor(workspace)?;
    canonical_workspace_root(Path::new(&descriptor.root_path))
}

async fn mark_initial_workspace_error(
    engine: &MindVaultEngine,
    workspace: &KnowledgeWorkspace,
) -> MvResult<()> {
    let mut replacement = workspace.clone();
    replacement.state = KnowledgeWorkspaceState::Error;
    replacement.revision = replacement
        .revision
        .checked_add(1)
        .ok_or_else(|| MvError::InvalidInput("workspace revision overflow".into()))?;
    replacement.updated_at = chrono::Utc::now();
    let _ = engine
        .store
        .nodes
        .update_knowledge_workspace(&replacement, workspace.revision)
        .await?;
    Ok(())
}
