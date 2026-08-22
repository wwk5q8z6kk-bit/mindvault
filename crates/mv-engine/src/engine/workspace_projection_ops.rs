use std::collections::{HashMap, HashSet};

use chrono::Utc;
use mv_core::{
    KnowledgeNode, KnowledgeWorkspace, KnowledgeWorkspaceDocument,
    KnowledgeWorkspaceManifestStore, MvError, MvResult, NodeStore, WorkspaceDocumentLifecycle,
    WorkspaceDocumentPayloadV1, WorkspaceEvent, WorkspaceEventActorKind,
    WorkspaceEventDocumentDelta, WorkspaceEventOperation, WorkspaceEventPayloadV1,
    WorkspaceEventStatus, WorkspaceManifestPayloadFormat, WorkspaceProjectionState,
};
use serde::Serialize;
use serde_json::{Map, Value};
use url::form_urlencoded::byte_serialize;
use uuid::Uuid;

use super::{
    node_ops::is_workspace_projection_node, workspace_ops::workspace_root, MindVaultEngine,
    WORKSPACE_PROJECTION_METADATA_KEY,
};
use crate::import::obsidian::{determine_kind, extract_frontmatter_tags, parse_frontmatter};
use crate::workspace::{ScannedWorkspaceDocument, WorkspaceScanner};

const WORKSPACE_PROJECTION_SCHEMA: &str = "mindvault.workspace-projection/v1";

#[derive(Debug, Clone, Default, Serialize)]
pub struct WorkspaceProjectionOutcome {
    pub attempted_documents: usize,
    pub projected_documents: usize,
    pub removed_documents: usize,
    pub unchanged_documents: usize,
    pub failed_documents: usize,
}

impl MindVaultEngine {
    /// Rebuild every disposable node, search, vector, and Markdown-link graph
    /// projection for one workspace from canonical files plus its manifest.
    pub async fn rebuild_knowledge_workspace_projections(
        &self,
        workspace_id: Uuid,
    ) -> MvResult<WorkspaceProjectionOutcome> {
        self.project_knowledge_workspace(workspace_id, true, Uuid::now_v7())
            .await
    }

    pub(crate) async fn project_knowledge_workspace(
        &self,
        workspace_id: Uuid,
        force: bool,
        correlation_id: Uuid,
    ) -> MvResult<WorkspaceProjectionOutcome> {
        let workspace = self.get_knowledge_workspace(workspace_id).await?;
        let root = workspace_root(&workspace)?;
        let scan = WorkspaceScanner::default().scan(&workspace, &root)?;
        let observations = scan
            .documents
            .into_iter()
            .map(|document| (document.path_token.clone(), document))
            .collect::<HashMap<_, _>>();
        let documents = self
            .store
            .nodes
            .list_workspace_documents(workspace.id)
            .await?;
        let mut outcome = WorkspaceProjectionOutcome::default();
        let mut projected = Vec::<(KnowledgeWorkspaceDocument, KnowledgeNode)>::new();
        let mut failed_ids = HashSet::new();
        let documents_by_id = documents
            .iter()
            .map(|document| (document.id, document))
            .collect::<HashMap<_, _>>();
        let refresh_all_graph_links = force
            || documents.iter().any(|document| {
                document.lifecycle_state == WorkspaceDocumentLifecycle::Active
                    && document.projected_node_id.is_none()
            });

        for document in &documents {
            if document.lifecycle_state != WorkspaceDocumentLifecycle::Active {
                if let Err(error) = self
                    .remove_inactive_workspace_projection(document, &mut outcome)
                    .await
                {
                    failed_ids.insert(document.id);
                    outcome.failed_documents += 1;
                    tracing::warn!(
                        workspace_id = %workspace.id,
                        document_id = %document.id,
                        error = %error,
                        "workspace inactive projection removal failed"
                    );
                    let _ = self
                        .set_workspace_projection_state(
                            document,
                            WorkspaceProjectionState::Failed,
                            document.projected_node_id,
                        )
                        .await;
                }
                continue;
            }

            if !force && document.projection_state == WorkspaceProjectionState::Ready {
                outcome.unchanged_documents += 1;
                continue;
            }
            outcome.attempted_documents += 1;

            let Some(observation) = observations.get(&document.path_token) else {
                outcome.failed_documents += 1;
                failed_ids.insert(document.id);
                let _ = self
                    .set_workspace_projection_state(
                        document,
                        WorkspaceProjectionState::Stale,
                        document.projected_node_id,
                    )
                    .await;
                continue;
            };

            match self
                .build_workspace_projection_node(&workspace, document, observation)
                .await
            {
                Ok(node) => match self.ingest.upsert_projection(node).await {
                    Ok(node) => projected.push((document.clone(), node)),
                    Err(error) => {
                        outcome.failed_documents += 1;
                        failed_ids.insert(document.id);
                        tracing::warn!(
                            workspace_id = %workspace.id,
                            document_id = %document.id,
                            error = %error,
                            "workspace node/search projection failed"
                        );
                        let _ = self
                            .set_workspace_projection_state(
                                document,
                                WorkspaceProjectionState::Failed,
                                Some(document.id),
                            )
                            .await;
                    }
                },
                Err(error) => {
                    outcome.failed_documents += 1;
                    failed_ids.insert(document.id);
                    tracing::warn!(
                        workspace_id = %workspace.id,
                        document_id = %document.id,
                        error = %error,
                        "workspace canonical document changed during projection"
                    );
                    let _ = self
                        .set_workspace_projection_state(
                            document,
                            WorkspaceProjectionState::Stale,
                            document.projected_node_id,
                        )
                        .await;
                }
            }
        }

        let graph_nodes = if refresh_all_graph_links && !projected.is_empty() {
            self.active_workspace_projection_nodes(&documents).await?
        } else {
            projected
                .iter()
                .map(|(_, node)| node.clone())
                .collect::<Vec<_>>()
        };
        let projected_ids = projected
            .iter()
            .map(|(document, _)| document.id)
            .collect::<HashSet<_>>();
        for node in graph_nodes {
            if let Err(error) = self.sync_node_references(&node).await {
                let newly_failed = failed_ids.insert(node.id);
                tracing::warn!(
                    workspace_id = %workspace.id,
                    document_id = %node.id,
                    error = %error,
                    "workspace graph projection failed"
                );
                if newly_failed && !projected_ids.contains(&node.id) {
                    outcome.attempted_documents += 1;
                    outcome.failed_documents += 1;
                    outcome.unchanged_documents = outcome.unchanged_documents.saturating_sub(1);
                    if let Some(document) = documents_by_id.get(&node.id) {
                        let _ = self
                            .set_workspace_projection_state(
                                document,
                                WorkspaceProjectionState::Failed,
                                Some(node.id),
                            )
                            .await;
                    }
                }
            }
        }

        let mut journal_deltas = Vec::new();
        for (document, _node) in projected {
            if failed_ids.contains(&document.id) {
                outcome.failed_documents += 1;
                let _ = self
                    .set_workspace_projection_state(
                        &document,
                        WorkspaceProjectionState::Failed,
                        Some(document.id),
                    )
                    .await;
                continue;
            }
            match self
                .set_workspace_projection_state(
                    &document,
                    WorkspaceProjectionState::Ready,
                    Some(document.id),
                )
                .await
            {
                Ok(true) => {
                    outcome.projected_documents += 1;
                    let manifest_hash =
                        decode_workspace_document_payload(&document)
                            .ok()
                            .and_then(|payload| payload.content_hash);
                    let observed = observations.get(&document.path_token);
                    journal_deltas.push(WorkspaceEventDocumentDelta {
                        document_id: Some(document.id),
                        relative_path: observed
                            .map(|document| document.payload.relative_path.clone()),
                        before_hash: manifest_hash,
                        after_hash: observed
                            .and_then(|document| document.payload.content_hash.clone()),
                    });
                }
                Ok(false) => {
                    outcome.failed_documents += 1;
                    tracing::warn!(
                        workspace_id = %workspace.id,
                        document_id = %document.id,
                        "workspace projection state lost an optimistic revision; later reconciliation will retry"
                    );
                }
                Err(error) => {
                    outcome.failed_documents += 1;
                    tracing::warn!(
                        workspace_id = %workspace.id,
                        document_id = %document.id,
                        error = %error,
                        "workspace projection state persistence failed"
                    );
                }
            }
        }

        tracing::info!(
            workspace_id = %workspace.id,
            attempted_documents = outcome.attempted_documents,
            projected_documents = outcome.projected_documents,
            removed_documents = outcome.removed_documents,
            unchanged_documents = outcome.unchanged_documents,
            failed_documents = outcome.failed_documents,
            force,
            "knowledge workspace projections completed"
        );
        self.journal_workspace_projection(
            &workspace,
            correlation_id,
            force,
            &outcome,
            journal_deltas,
        )
        .await;
        Ok(outcome)
    }

    /// Append one durable journal row summarizing a projection run. The
    /// per-document before/after hashes prove the projected bytes are the
    /// reconciled canonical bytes. Projection is a rebuildable derived-state
    /// sync, so a journal failure is logged rather than failing the run.
    async fn journal_workspace_projection(
        &self,
        workspace: &KnowledgeWorkspace,
        correlation_id: Uuid,
        force: bool,
        outcome: &WorkspaceProjectionOutcome,
        journal_deltas: Vec<WorkspaceEventDocumentDelta>,
    ) {
        let mut payload = WorkspaceEventPayloadV1::new(
            "projected workspace documents into node, search, and graph stores",
        );
        payload.documents = journal_deltas;
        payload
            .attributes
            .insert("detail".to_string(), "projection".to_string());
        payload
            .attributes
            .insert("force".to_string(), force.to_string());
        payload.attributes.insert(
            "attempted_documents".to_string(),
            outcome.attempted_documents.to_string(),
        );
        payload.attributes.insert(
            "projected_documents".to_string(),
            outcome.projected_documents.to_string(),
        );
        payload.attributes.insert(
            "removed_documents".to_string(),
            outcome.removed_documents.to_string(),
        );
        payload.attributes.insert(
            "unchanged_documents".to_string(),
            outcome.unchanged_documents.to_string(),
        );
        payload.attributes.insert(
            "failed_documents".to_string(),
            outcome.failed_documents.to_string(),
        );
        let event_payload = match serde_json::to_vec(&payload) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(
                    workspace_id = %workspace.id,
                    error = %error,
                    "workspace projection journal payload failed to serialize"
                );
                return;
            }
        };
        let event = WorkspaceEvent::new(
            workspace.id,
            None,
            correlation_id,
            WorkspaceEventActorKind::System,
            WorkspaceEventOperation::Update,
            WorkspaceEventStatus::Completed,
            event_payload,
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        if let Err(error) = self.store.nodes.append_workspace_event(&event).await {
            tracing::warn!(
                workspace_id = %workspace.id,
                error = %error,
                "workspace projection journal append failed"
            );
        }
    }

    async fn build_workspace_projection_node(
        &self,
        workspace: &mv_core::KnowledgeWorkspace,
        document: &KnowledgeWorkspaceDocument,
        observation: &ScannedWorkspaceDocument,
    ) -> MvResult<KnowledgeNode> {
        if observation.lifecycle != WorkspaceDocumentLifecycle::Active {
            return Err(MvError::InvalidInput(
                "only active UTF-8 documents can be projected".into(),
            ));
        }
        let manifest_payload = decode_workspace_document_payload(document)?;
        if manifest_payload.content_hash != observation.payload.content_hash {
            return Err(MvError::CanonicalSourceConflict(
                "canonical content changed after manifest reconciliation".into(),
            ));
        }
        let content = observation.content.clone().ok_or_else(|| {
            MvError::InvalidInput("active workspace document has no UTF-8 content".into())
        })?;
        let content_hash = observation.payload.content_hash.clone().ok_or_else(|| {
            MvError::Internal("active workspace document has no content hash".into())
        })?;
        let (properties, _) = parse_frontmatter(&content);
        let title = projection_title(&properties, &content, &observation.payload.relative_path);
        let tags = extract_frontmatter_tags(&properties);
        let kind = determine_kind(&properties);
        let projected_at = Utc::now();
        let parent_path = observation
            .payload
            .relative_path
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_string());
        let encoded_path =
            byte_serialize(observation.payload.relative_path.as_bytes()).collect::<String>();
        let source = format!("mindvault-workspace://{}/{encoded_path}", workspace.id);

        let mut projection = Map::new();
        projection.insert(
            "schema".into(),
            Value::String(WORKSPACE_PROJECTION_SCHEMA.into()),
        );
        projection.insert(
            "workspace_id".into(),
            Value::String(workspace.id.to_string()),
        );
        projection.insert("document_id".into(), Value::String(document.id.to_string()));
        projection.insert(
            "relative_path".into(),
            Value::String(observation.payload.relative_path.clone()),
        );
        projection.insert(
            "parent_path".into(),
            parent_path.map(Value::String).unwrap_or(Value::Null),
        );
        projection.insert("content_hash".into(), Value::String(content_hash));
        projection.insert(
            "projected_at".into(),
            Value::String(projected_at.to_rfc3339()),
        );
        projection.insert(
            "properties".into(),
            serde_json::to_value(&properties).unwrap_or_else(|_| Value::Object(Map::new())),
        );

        let mut node = KnowledgeNode::new(kind, content)
            .with_title(title)
            .with_source(source)
            .with_namespace(workspace.namespace.clone())
            .with_tags(tags);
        node.id = document.id;
        node.temporal.created_at = document.created_at;
        node.temporal.updated_at = observation
            .payload
            .modified_at
            .unwrap_or(document.updated_at);
        node.temporal.last_accessed_at = node.temporal.updated_at;
        node.temporal.version = u32::try_from(document.revision).unwrap_or(u32::MAX);
        node.metadata.insert(
            WORKSPACE_PROJECTION_METADATA_KEY.into(),
            Value::Object(projection),
        );
        Ok(node)
    }

    async fn remove_inactive_workspace_projection(
        &self,
        document: &KnowledgeWorkspaceDocument,
        outcome: &mut WorkspaceProjectionOutcome,
    ) -> MvResult<()> {
        if let Some(node_id) = document.projected_node_id {
            if self.ingest.remove_projection(node_id).await? {
                outcome.removed_documents += 1;
            }
        } else {
            outcome.unchanged_documents += 1;
        }
        if !self
            .set_workspace_projection_state(
                document,
                WorkspaceProjectionState::Ready,
                document.projected_node_id,
            )
            .await?
        {
            return Err(MvError::Storage(
                "workspace projection removal lost an optimistic manifest revision".into(),
            ));
        }
        Ok(())
    }

    async fn active_workspace_projection_nodes(
        &self,
        documents: &[KnowledgeWorkspaceDocument],
    ) -> MvResult<Vec<KnowledgeNode>> {
        let mut nodes = Vec::new();
        for document in documents {
            if document.lifecycle_state != WorkspaceDocumentLifecycle::Active {
                continue;
            }
            let node_id = document.projected_node_id.unwrap_or(document.id);
            if let Some(node) = self.store.nodes.get(node_id).await? {
                if is_workspace_projection_node(&node) {
                    nodes.push(node);
                }
            }
        }
        Ok(nodes)
    }

    async fn set_workspace_projection_state(
        &self,
        document: &KnowledgeWorkspaceDocument,
        state: WorkspaceProjectionState,
        projected_node_id: Option<Uuid>,
    ) -> MvResult<bool> {
        if document.projection_state == state && document.projected_node_id == projected_node_id {
            return Ok(true);
        }
        let mut replacement = document.clone();
        replacement.projection_state = state;
        replacement.projected_node_id = projected_node_id;
        replacement.revision = replacement
            .revision
            .checked_add(1)
            .ok_or_else(|| MvError::InvalidInput("workspace document revision overflow".into()))?;
        replacement.updated_at = Utc::now();
        self.store
            .nodes
            .update_workspace_document(&replacement, document.revision)
            .await
    }
}

fn decode_workspace_document_payload(
    document: &KnowledgeWorkspaceDocument,
) -> MvResult<WorkspaceDocumentPayloadV1> {
    let payload = serde_json::from_slice::<WorkspaceDocumentPayloadV1>(&document.document_payload)
        .map_err(|error| {
            MvError::InvalidInput(format!("workspace document payload is invalid: {error}"))
        })?;
    if !payload.has_supported_schema() {
        return Err(MvError::InvalidInput(
            "workspace document payload schema is unsupported".into(),
        ));
    }
    Ok(payload)
}

fn projection_title(
    properties: &HashMap<String, Value>,
    content: &str,
    relative_path: &str,
) -> String {
    if let Some(title) = properties
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        return title.to_string();
    }
    if let Some(heading) = content.lines().find_map(|line| {
        line.strip_prefix("# ")
            .map(str::trim)
            .filter(|heading| !heading.is_empty())
    }) {
        return heading.to_string();
    }
    relative_path
        .rsplit('/')
        .next()
        .unwrap_or(relative_path)
        .trim_end_matches(".markdown")
        .trim_end_matches(".md")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_title_prefers_frontmatter_then_heading_then_filename() {
        let properties = HashMap::from([(
            "title".to_string(),
            Value::String("Frontmatter title".into()),
        )]);
        assert_eq!(
            projection_title(&properties, "# Heading", "Folder/File.md"),
            "Frontmatter title"
        );
        assert_eq!(
            projection_title(&HashMap::new(), "# Heading", "Folder/File.md"),
            "Heading"
        );
        assert_eq!(
            projection_title(&HashMap::new(), "Body", "Folder/File.markdown"),
            "File"
        );
    }
}
