use mv_core::*;

use super::MindVaultEngine;

impl MindVaultEngine {
    /// Store a knowledge node.
    pub async fn store_node(&self, node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        self.ensure_unsealed_for_node_io().await?;
        let stored = self.ingest.ingest(node).await?;
        self.auto_link_node_to_daily_note_best_effort(&stored).await;
        self.auto_backlink_node_references_best_effort(&stored)
            .await;
        Ok(stored)
    }

    /// Store a node with relationships.
    pub(crate) async fn store_with_relations(
        &self,
        node: KnowledgeNode,
        relations: Vec<Relationship>,
    ) -> MvResult<KnowledgeNode> {
        self.ensure_unsealed_for_node_io().await?;
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
