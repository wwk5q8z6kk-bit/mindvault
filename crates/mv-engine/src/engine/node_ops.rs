use mv_core::*;

use super::{MindVaultEngine, WORKSPACE_PROJECTION_METADATA_KEY};

impl MindVaultEngine {
    /// Store a knowledge node.
    pub async fn store_node(&self, node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        self.ensure_unsealed_for_node_io().await?;
        reject_reserved_workspace_projection_marker(&node)?;
        let stored = self.ingest.ingest(node).await?;
        self.auto_link_node_to_daily_note_best_effort(&stored).await;
        self.auto_backlink_node_references_best_effort(&stored)
            .await;
        Ok(stored)
    }

    /// Return the durable identifier of this local context node.
    pub async fn local_context_node_id(&self) -> MvResult<uuid::Uuid> {
        self.store.nodes.local_context_node_id().await
    }

    /// Resolve a durable node-create replay before mutable admission checks.
    pub async fn find_node_create_replay(
        &self,
        source: &StableUri,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
        payload_digest: &str,
    ) -> MvResult<Option<IdempotentNodeCommit>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store
            .nodes
            .find_node_create_replay(source, principal, idempotency_key, payload_digest)
            .await
    }

    /// Store a node and its interoperability event atomically.
    pub async fn store_node_with_event(
        &self,
        node: KnowledgeNode,
        event: EventEnvelope,
    ) -> MvResult<IdempotentNodeCommit> {
        self.ensure_unsealed_for_node_io().await?;
        reject_reserved_workspace_projection_marker(&node)?;
        let commit = self.ingest.ingest_with_event(node, event).await?;
        if !commit.replayed {
            self.auto_link_node_to_daily_note_best_effort(&commit.node)
                .await;
            self.auto_backlink_node_references_best_effort(&commit.node)
                .await;
        }
        Ok(commit)
    }

    /// Store a node with relationships.
    pub async fn store_with_relations(
        &self,
        node: KnowledgeNode,
        relations: Vec<Relationship>,
    ) -> MvResult<KnowledgeNode> {
        self.ensure_unsealed_for_node_io().await?;
        reject_reserved_workspace_projection_marker(&node)?;
        self.ingest.ingest_with_relations(node, relations).await
    }

    /// Recall knowledge matching a query.
    pub async fn recall(&self, query: &MemoryQuery) -> MvResult<Vec<SearchResult>> {
        self.ensure_unsealed_for_node_io().await?;
        self.recall.recall(query).await
    }

    /// Get a node by ID.
    pub async fn get_node(&self, id: uuid::Uuid) -> MvResult<Option<KnowledgeNode>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.get(id).await
    }

    /// Update an existing node.
    pub async fn update_node(&self, node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        self.ensure_unsealed_for_node_io().await?;
        if let Some(existing) = self.store.nodes.get(node.id).await? {
            reject_workspace_projection_mutation(&existing)?;
        }
        reject_reserved_workspace_projection_marker(&node)?;
        let updated = self.ingest.update(node).await?;
        self.auto_link_node_to_daily_note_best_effort(&updated)
            .await;
        self.auto_backlink_node_references_best_effort(&updated)
            .await;
        Ok(updated)
    }

    /// Delete a node.
    pub async fn delete_node(&self, id: uuid::Uuid) -> MvResult<bool> {
        self.ensure_unsealed_for_node_io().await?;
        if let Some(existing) = self.store.nodes.get(id).await? {
            reject_workspace_projection_mutation(&existing)?;
        }
        self.ingest.delete(id).await
    }

    /// List nodes with filters.
    pub async fn list_nodes(
        &self,
        filters: &QueryFilters,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.list(filters, limit, offset).await
    }

    /// Get node count.
    pub async fn node_count(&self) -> MvResult<usize> {
        self.store.nodes.count(&QueryFilters::default()).await
    }
}

pub(crate) fn is_workspace_projection_node(node: &KnowledgeNode) -> bool {
    node.metadata
        .get(WORKSPACE_PROJECTION_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
        .is_some_and(|projection| {
            projection.get("schema").and_then(serde_json::Value::as_str)
                == Some("mindvault.workspace-projection/v1")
        })
}

fn reject_reserved_workspace_projection_marker(node: &KnowledgeNode) -> MvResult<()> {
    if node
        .metadata
        .contains_key(WORKSPACE_PROJECTION_METADATA_KEY)
    {
        return Err(MvError::InvalidInput(
            "workspace projection metadata is reserved for canonical Markdown documents".into(),
        ));
    }
    Ok(())
}

fn reject_workspace_projection_mutation(node: &KnowledgeNode) -> MvResult<()> {
    if is_workspace_projection_node(node) {
        return Err(MvError::CanonicalSourceConflict(
            "this node is a read-only projection; mutate its canonical document through the workspace API"
                .into(),
        ));
    }
    Ok(())
}
