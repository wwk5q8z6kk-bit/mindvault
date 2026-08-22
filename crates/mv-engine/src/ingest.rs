use std::sync::Arc;

use mv_core::*;
use mv_graph::store::SqliteGraphStore;
use mv_index::tantivy_index::TantivyFullTextIndex;
use mv_storage::unified::UnifiedStore;

use crate::ai_autotag::KnowledgeVaultIndexNoteEmbeddingAutoTagger;
use crate::config::EngineConfig;
use crate::conflict::ConflictDetector;
use crate::engine::WORKSPACE_PROJECTION_METADATA_KEY;

const WORKSPACE_PROJECTION_SCHEMA: &str = "mindvault.workspace-projection/v1";

/// Ingest pipeline: validates, stores, indexes, and embeds a knowledge node.
pub struct IngestPipeline {
    store: Arc<UnifiedStore>,
    fts: Arc<TantivyFullTextIndex>,
    graph: Arc<SqliteGraphStore>,
    config: EngineConfig,
    knowledge_vault_index_note_embedding_auto_tagger: KnowledgeVaultIndexNoteEmbeddingAutoTagger,
}

impl IngestPipeline {
    pub fn new(
        store: Arc<UnifiedStore>,
        fts: Arc<TantivyFullTextIndex>,
        graph: Arc<SqliteGraphStore>,
        config: EngineConfig,
    ) -> Self {
        Self {
            store,
            fts,
            graph,
            config,
            knowledge_vault_index_note_embedding_auto_tagger:
                KnowledgeVaultIndexNoteEmbeddingAutoTagger::new(),
        }
    }

    /// Ingest a node: store in SQLite, index in Tantivy, embed in LanceDB.
    pub async fn ingest(&self, mut node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        self.apply_ai_auto_tagging_if_enabled(&mut node).await;

        // 1. Store in SQLite
        self.store.nodes.insert(&node).await?;

        self.index_and_enrich(&node).await?;
        tracing::info!("ingested node {} ({})", node.id, node.kind);
        Ok(node)
    }

    /// Ingest through the atomic interoperability mutation boundary.
    ///
    /// A replay returns the originally committed node and deliberately skips
    /// indexing, embedding, and conflict-detection side effects.
    pub async fn ingest_with_event(
        &self,
        mut node: KnowledgeNode,
        event: EventEnvelope,
    ) -> MvResult<IdempotentNodeCommit> {
        self.apply_ai_auto_tagging_if_enabled(&mut node).await;
        let commit = self
            .store
            .nodes
            .commit_node_create_with_event(&node, &event)
            .await?;
        if !commit.replayed {
            if let Err(error) = self.index_and_enrich(&commit.node).await {
                tracing::error!(
                    node_id = %commit.node.id,
                    event_id = %commit.event.id,
                    error = %error,
                    "canonical node and outbox event committed, but a derived projection failed"
                );
            }
            tracing::info!(
                node_id = %commit.node.id,
                event_id = %commit.event.id,
                "ingested node through interoperability boundary"
            );
        }
        Ok(commit)
    }

    pub(crate) async fn index_and_enrich(&self, node: &KnowledgeNode) -> MvResult<()> {
        // 1. Index in Tantivy
        self.fts.index_node(node)?;
        self.fts.commit()?;

        // 2. Generate embedding and store in LanceDB (if vector store available)
        if let Some(ref vectors) = self.store.vectors {
            match self.store.embedder.embed(&node.content).await {
                Ok(embedding) => {
                    vectors
                        .upsert(node.id, embedding, &node.content, Some(&node.namespace))
                        .await?;
                }
                Err(e) => {
                    tracing::warn!("embedding failed for node {}: {e}", node.id);
                    // Continue without embedding — FTS still works
                }
            }
        }

        // 3. Conflict detection (best-effort, non-blocking)
        match ConflictDetector::detect_conflicts(&self.store, node, 0.5).await {
            Ok(alerts) => {
                for alert in &alerts {
                    if let Err(e) = self.store.nodes.insert_conflict(alert).await {
                        tracing::warn!("failed to store conflict alert: {e}");
                    }
                }
                if !alerts.is_empty() {
                    tracing::info!(
                        count = alerts.len(),
                        node_id = %node.id,
                        "conflict alerts generated"
                    );
                }
            }
            Err(e) => {
                tracing::warn!("conflict detection failed for node {}: {e}", node.id);
            }
        }

        Ok(())
    }

    /// Upsert a read-only node projection without applying authored-node
    /// enrichment or conflict detection.
    ///
    /// Workspace Markdown remains canonical on disk. This method only updates
    /// disposable SQLite, full-text, and vector representations.
    pub(crate) async fn upsert_projection(&self, node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        if !is_workspace_projection(&node) {
            return Err(MvError::InvalidInput(
                "workspace projection metadata is missing or invalid".into(),
            ));
        }

        if let Some(existing) = self.store.nodes.get(node.id).await? {
            if !is_workspace_projection(&existing) {
                return Err(MvError::CanonicalSourceConflict(format!(
                    "node {} already exists and is not a disposable workspace projection",
                    node.id
                )));
            }
            self.store.nodes.update(&node).await?;
        } else {
            self.store.nodes.insert(&node).await?;
        }

        self.fts.index_node(&node)?;
        self.fts.commit()?;

        if let Some(ref vectors) = self.store.vectors {
            match self.store.embedder.embed(&node.content).await {
                Ok(embedding) => {
                    vectors
                        .upsert(node.id, embedding, &node.content, Some(&node.namespace))
                        .await?;
                }
                Err(error) => {
                    tracing::warn!(
                        node_id = %node.id,
                        error = %error,
                        "workspace projection embedding failed; full-text projection remains available"
                    );
                }
            }
        }

        Ok(node)
    }

    /// Remove every disposable representation of a projected node.
    pub(crate) async fn remove_projection(&self, id: uuid::Uuid) -> MvResult<bool> {
        self.delete(id).await
    }

    /// Ingest a node with relationships.
    pub async fn ingest_with_relations(
        &self,
        node: KnowledgeNode,
        relations: Vec<Relationship>,
    ) -> MvResult<KnowledgeNode> {
        let node = self.ingest(node).await?;

        for rel in &relations {
            self.graph.add_relationship(rel).await?;
        }

        Ok(node)
    }

    /// Update an existing node, re-index and re-embed.
    pub async fn update(&self, mut node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        self.apply_ai_auto_tagging_if_enabled(&mut node).await;

        self.store.nodes.update(&node).await?;

        self.fts.index_node(&node)?;
        self.fts.commit()?;

        if let Some(ref vectors) = self.store.vectors {
            match self.store.embedder.embed(&node.content).await {
                Ok(embedding) => {
                    vectors
                        .upsert(node.id, embedding, &node.content, Some(&node.namespace))
                        .await?;
                }
                Err(e) => {
                    tracing::warn!("re-embedding failed for node {}: {e}", node.id);
                }
            }
        }

        Ok(node)
    }

    /// Delete a node and its index entries.
    pub async fn delete(&self, id: uuid::Uuid) -> MvResult<bool> {
        self.fts.remove_node(id)?;
        self.fts.commit()?;

        if let Some(ref vectors) = self.store.vectors {
            let _ = vectors.delete(id).await;
        }

        self.graph.remove_node_relationships(id).await?;

        // Clean up attachment blob files
        let blob_dir = std::path::PathBuf::from(&self.config.data_dir)
            .join("blobs")
            .join(id.to_string());
        if blob_dir.is_dir() {
            if let Err(e) = tokio::fs::remove_dir_all(&blob_dir).await {
                tracing::warn!(node_id = %id, error = %e, "failed to remove blob directory");
            }
        }

        self.store.nodes.delete(id).await
    }

    async fn apply_ai_auto_tagging_if_enabled(&self, node: &mut KnowledgeNode) {
        if !self.config.ai.auto_tagging_enabled {
            return;
        }

        self.knowledge_vault_index_note_embedding_auto_tagger
            .enrich_node_tags(node, &self.store, &self.fts, &self.config.ai)
            .await;
    }
}

fn is_workspace_projection(node: &KnowledgeNode) -> bool {
    node.metadata
        .get(WORKSPACE_PROJECTION_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
        .and_then(|projection| projection.get("schema"))
        .and_then(serde_json::Value::as_str)
        == Some(WORKSPACE_PROJECTION_SCHEMA)
}
