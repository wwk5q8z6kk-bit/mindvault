use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use mv_core::{
    KnowledgeWorkspace, KnowledgeWorkspaceManifestStore, KnowledgeWorkspaceMode,
    KnowledgeWorkspaceState, MvError, MvResult, WorkspaceConflict, WorkspaceConflictKind,
    WorkspaceConflictPayloadV1, WorkspaceConflictState, WorkspaceDescriptorPayloadV1,
    WorkspaceDocumentPayloadV1, WorkspaceEvent, WorkspaceEventActorKind,
    WorkspaceEventDocumentDelta, WorkspaceEventOperation, WorkspaceEventPayloadV1,
    WorkspaceEventStatus, WorkspaceManifestPayloadFormat,
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

        // One correlation id ties the mount, its initial scan, and its first
        // projection together in the durable journal.
        let correlation_id = Uuid::now_v7();
        let mount_prepared_at = Utc::now();
        let reconciler =
            WorkspaceReconciler::new(self.store.nodes.as_ref(), WorkspaceScanner::default());
        let mut reconciliation = match reconciler
            .reconcile(&workspace, &canonical_root, correlation_id)
            .await
        {
            Ok(outcome) if outcome.applied => outcome,
            Ok(_) => {
                self.journal_mount_outcome(
                    &workspace,
                    correlation_id,
                    mount_prepared_at,
                    &descriptor.display_name,
                    WorkspaceEventStatus::Aborted,
                )
                .await;
                mark_initial_workspace_error(self, &workspace).await?;
                return Err(MvError::Storage(
                    "initial workspace reconciliation lost its optimistic revision".into(),
                ));
            }
            Err(error) => {
                self.journal_mount_outcome(
                    &workspace,
                    correlation_id,
                    mount_prepared_at,
                    &descriptor.display_name,
                    WorkspaceEventStatus::Aborted,
                )
                .await;
                mark_initial_workspace_error(self, &workspace).await?;
                return Err(error);
            }
        };
        reconciliation.projection = self
            .project_knowledge_workspace(workspace.id, false, correlation_id)
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
        self.journal_mount_outcome(
            &workspace,
            correlation_id,
            mount_prepared_at,
            &descriptor.display_name,
            WorkspaceEventStatus::Completed,
        )
        .await;
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

    /// Append the mount row to the durable journal. The row is a summary of
    /// the whole mount operation; the substantive manifest mutation is
    /// already journaled in-transaction by the initial scan, so a failure
    /// here is logged rather than failing the mount.
    async fn journal_mount_outcome(
        &self,
        workspace: &KnowledgeWorkspace,
        correlation_id: Uuid,
        prepared_at: DateTime<Utc>,
        display_name: &str,
        status: WorkspaceEventStatus,
    ) {
        let mut payload =
            WorkspaceEventPayloadV1::new(format!("mount workspace '{display_name}'"));
        if status == WorkspaceEventStatus::Completed {
            if let Ok(documents) = self.store.nodes.list_workspace_documents(workspace.id).await {
                payload.documents = documents
                    .iter()
                    .map(|document| {
                        let payload = serde_json::from_slice::<WorkspaceDocumentPayloadV1>(
                            &document.document_payload,
                        )
                        .ok();
                        WorkspaceEventDocumentDelta {
                            document_id: Some(document.id),
                            relative_path: payload
                                .as_ref()
                                .map(|payload| payload.relative_path.clone()),
                            before_hash: None,
                            after_hash: payload.and_then(|payload| payload.content_hash),
                        }
                    })
                    .collect();
            }
        }
        let event_payload = match serde_json::to_vec(&payload) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(
                    workspace_id = %workspace.id,
                    error = %error,
                    "workspace mount journal payload failed to serialize"
                );
                return;
            }
        };
        let mut event = WorkspaceEvent::new(
            workspace.id,
            None,
            correlation_id,
            WorkspaceEventActorKind::System,
            WorkspaceEventOperation::Mount,
            status,
            event_payload,
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        event.prepared_at = prepared_at;
        if let Err(error) = self.store.nodes.append_workspace_event(&event).await {
            tracing::warn!(
                workspace_id = %workspace.id,
                error = %error,
                "workspace mount journal append failed"
            );
        }
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
        let mut outcome = reconciler
            .reconcile(&workspace, &root, correlation_id)
            .await?;
        if outcome.applied {
            outcome.projection = self
                .project_knowledge_workspace(workspace.id, false, correlation_id)
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

    /// Expected-hash preflight for the guarded mutation boundary (ADR 009
    /// gate 4). Stage 2 writers must hold this guard before touching
    /// canonical bytes.
    ///
    /// A mismatch fails closed: no filesystem write is attempted, and a
    /// `stale_write` conflict plus its journal event are recorded in one
    /// transaction so reviewers can see exactly which hashes competed.
    pub async fn guard_workspace_document_write(
        &self,
        workspace_id: Uuid,
        document_id: Uuid,
        expected_content_hash: &str,
    ) -> MvResult<WorkspaceWriteGuard> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        let document = self
            .store
            .nodes
            .get_workspace_document(document_id)
            .await?
            .ok_or(MvError::NodeNotFound(document_id))?;
        if document.workspace_id != workspace.id {
            // Indistinguishable from missing, matching the read boundary.
            return Err(MvError::NodeNotFound(document_id));
        }
        let root = workspace_root(&workspace)?;
        let scan = WorkspaceScanner::default().scan(&workspace, &root)?;
        let read = read_workspace_document_from_scan(&workspace, &document, scan)?;

        if read.content_hash == expected_content_hash {
            return Ok(WorkspaceWriteGuard {
                workspace_id: workspace.id,
                document_id: document.id,
                observed_content_hash: read.content_hash,
                observed_revision: document.revision,
            });
        }

        let correlation_id = Uuid::now_v7();
        let mut event_payload = WorkspaceEventPayloadV1::new(
            "guarded write rejected: expected content hash did not match canonical bytes",
        );
        event_payload.documents.push(WorkspaceEventDocumentDelta {
            document_id: Some(document.id),
            relative_path: Some(read.relative_path.clone()),
            before_hash: Some(read.content_hash.clone()),
            // The canonical bytes are provably unchanged by the failed write.
            after_hash: Some(read.content_hash.clone()),
        });
        event_payload.attributes.insert(
            "expected_content_hash".to_string(),
            expected_content_hash.to_string(),
        );
        let event = WorkspaceEvent::new(
            workspace.id,
            Some(document.id),
            correlation_id,
            WorkspaceEventActorKind::System,
            WorkspaceEventOperation::Update,
            WorkspaceEventStatus::Conflict,
            serde_json::to_vec(&event_payload)?,
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        let mut conflict_payload = WorkspaceConflictPayloadV1::new(
            "guarded write preflight observed a different canonical content hash than the              caller expected; the canonical file was left unchanged",
        );
        conflict_payload.relative_path = Some(read.relative_path);
        conflict_payload.expected_hash = Some(expected_content_hash.to_string());
        conflict_payload.observed_hash = Some(read.content_hash.clone());
        let conflict = WorkspaceConflict::new(
            workspace.id,
            Some(document.id),
            Some(event.id),
            WorkspaceConflictKind::StaleWrite,
            serde_json::to_vec(&conflict_payload)?,
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        self.store
            .nodes
            .record_workspace_conflict(&conflict, &event)
            .await?;
        Err(MvError::CanonicalSourceConflict(format!(
            "expected content hash {expected_content_hash} does not match observed canonical              hash {}",
            read.content_hash
        )))
    }

    pub async fn list_knowledge_workspace_conflicts(
        &self,
        workspace_id: Uuid,
        state: Option<WorkspaceConflictState>,
    ) -> MvResult<Vec<WorkspaceConflict>> {
        self.store
            .nodes
            .list_workspace_conflicts(workspace_id, state)
            .await
    }
}

/// Proof that an expected-hash preflight passed for a guarded write. The
/// Stage 2 mutation routes must hold a fresh guard before applying any
/// canonical file change.
#[derive(Debug, Clone)]
pub struct WorkspaceWriteGuard {
    pub workspace_id: Uuid,
    pub document_id: Uuid,
    pub observed_content_hash: String,
    pub observed_revision: u64,
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
