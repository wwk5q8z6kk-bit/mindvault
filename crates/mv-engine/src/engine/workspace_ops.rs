use std::fs;
use std::path::{Path, PathBuf};

use mv_core::{
    KnowledgeWorkspace, KnowledgeWorkspaceManifestStore, KnowledgeWorkspaceMode,
    KnowledgeWorkspaceState, MvError, MvResult, WorkspaceConflict,
    WorkspaceConflictPayloadV1, WorkspaceDescriptorPayloadV1, WorkspaceEvent,
    WorkspaceEventActorKind, WorkspaceEventOperation, WorkspaceEventPayloadV1,
    WorkspaceManifestPayloadFormat, WORKSPACE_CONFLICT_RESOLUTIONS,
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

        let correlation_id = Uuid::now_v7();
        self.journal_workspace_phase(
            workspace.id,
            correlation_id,
            WorkspaceEventOperation::Mount,
            None,
            Some(workspace.revision),
            Some(workspace.revision),
            "mount",
        )
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
        let after = self
            .store
            .nodes
            .get_knowledge_workspace(workspace.id)
            .await?
            .ok_or(MvError::NodeNotFound(workspace.id))?;
        self.journal_workspace_phase(
            workspace.id,
            correlation_id,
            WorkspaceEventOperation::Scan,
            None,
            Some(workspace.revision),
            Some(after.revision),
            "reconcile",
        )
        .await?;
        reconciliation.projection = self
            .project_knowledge_workspace(workspace.id, false, Some(correlation_id))
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
        let workspace = after;

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
        let correlation_id = Uuid::now_v7();
        let reconciler =
            WorkspaceReconciler::new(self.store.nodes.as_ref(), WorkspaceScanner::default());
        let mut outcome = reconciler.reconcile(&workspace, &root).await?;
        if outcome.applied {
            let after = self.get_knowledge_workspace(workspace_id).await?;
            self.journal_workspace_phase(
                workspace.id,
                correlation_id,
                WorkspaceEventOperation::Scan,
                None,
                Some(workspace.revision),
                Some(after.revision),
                "reconcile",
            )
            .await?;
            outcome.projection = self
                .project_knowledge_workspace(workspace.id, false, Some(correlation_id))
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

    /// Record a stale expected-hash write refusal without mutating canonical bytes.
    ///
    /// Stage-2 PUT will call this when `expected_content_hash` does not match the
    /// on-disk document. The conflict carries the four documented resolutions.
    pub async fn record_stale_write_conflict(
        &self,
        workspace_id: Uuid,
        document_id: Uuid,
        expected_content_hash: impl Into<String>,
        attempted_content_hash: impl Into<String>,
    ) -> MvResult<WorkspaceConflict> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        let document = self
            .store
            .nodes
            .get_workspace_document(document_id)
            .await?
            .ok_or(MvError::NodeNotFound(document_id))?;
        if document.workspace_id != workspace.id {
            return Err(MvError::NodeNotFound(document_id));
        }

        let root = workspace_root(&workspace)?;
        let read = self
            .read_knowledge_workspace_document(workspace.id, document.id)
            .await?;
        let expected = expected_content_hash.into();
        let attempted = attempted_content_hash.into();
        if expected == read.content_hash {
            return Err(MvError::InvalidInput(
                "expected_content_hash matches the canonical document; not a stale write".into(),
            ));
        }

        // Prove the filesystem bytes are left untouched by this refusal path:
        // re-read after recording must match the pre-record snapshot.
        let relative = PathBuf::from(&read.relative_path);
        let absolute = root.join(&relative);
        let before = std::fs::read(&absolute).map_err(|err| {
            MvError::Storage(format!("canonical document unavailable for conflict proof: {err}"))
        })?;

        let payload = WorkspaceConflictPayloadV1::stale_write(
            read.relative_path.clone(),
            expected,
            attempted,
            read.content_hash.clone(),
        );
        let conflict = WorkspaceConflict::open_stale_write(workspace.id, document.id, payload)
            .map_err(|err| MvError::InvalidInput(format!("conflict payload: {err}")))?;
        self.store.nodes.insert_workspace_conflict(&conflict).await?;

        let after = std::fs::read(&absolute).map_err(|err| {
            MvError::Storage(format!("canonical document unavailable after conflict: {err}"))
        })?;
        if before != after {
            return Err(MvError::Internal(
                "stale-write conflict path mutated canonical bytes".into(),
            ));
        }
        let _ = WORKSPACE_CONFLICT_RESOLUTIONS;
        Ok(conflict)
    }

    pub async fn list_workspace_conflicts(
        &self,
        workspace_id: Uuid,
        open_only: bool,
    ) -> MvResult<Vec<WorkspaceConflict>> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        self.store
            .nodes
            .list_workspace_conflicts(workspace.id, open_only)
            .await
    }

}


impl MindVaultEngine {
    pub(crate) async fn journal_workspace_phase(
        &self,
        workspace_id: Uuid,
        correlation_id: Uuid,
        operation: WorkspaceEventOperation,
        content_hash: Option<&str>,
        expected_old_revision: Option<u64>,
        intended_new_revision: Option<u64>,
        phase: &str,
    ) -> MvResult<WorkspaceEvent> {
        let mut payload = WorkspaceEventPayloadV1::new();
        payload.phase = Some(phase.to_string());
        payload.expected_old_revision = expected_old_revision;
        payload.intended_new_revision = intended_new_revision;
        if let Some(hash) = content_hash {
            payload.intended_new_content_hash = Some(hash.to_string());
            payload.expected_old_content_hash = Some(hash.to_string());
        }
        let event = WorkspaceEvent::completed(
            workspace_id,
            correlation_id,
            operation,
            WorkspaceEventActorKind::System,
            payload,
        )
        .map_err(|err| MvError::InvalidInput(format!("workspace event payload: {err}")))?;
        self.store.nodes.append_workspace_event(&event).await
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
