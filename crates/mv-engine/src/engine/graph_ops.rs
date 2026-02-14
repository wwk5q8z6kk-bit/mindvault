use mv_core::*;

use crate::backlinks::{
    extract_reference_targets_with_kind, resolve_reference_targets_with_kind,
    ContentReferenceSourceKind, KnowledgeVaultBacklinkResolutionIndex,
    ResolvedContentReferenceTarget,
};

use super::{
    is_auto_backlink_relationship, is_daily_link_candidate, is_daily_note, is_template_node,
    KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus, MindVaultEngine,
    AUTO_BACKLINK_METADATA_KEY, AUTO_BACKLINK_SOURCE_METADATA_KEY,
};

impl MindVaultEngine {
    // ── Graph & Relationships ─────────────────────────────────────────

    /// Add a relationship between nodes.
    pub async fn add_relationship(&self, rel: Relationship) -> MvResult<()> {
        self.graph.add_relationship(&rel).await
    }

    /// Get graph neighbors.
    pub async fn get_neighbors(
        &self,
        node_id: uuid::Uuid,
        depth: usize,
    ) -> MvResult<Vec<uuid::Uuid>> {
        self.graph.get_neighbors(node_id, depth).await
    }

    /// Return the active embedding provider diagnostics for observability.
    pub fn embedding_runtime_status(
        &self,
    ) -> KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus {
        self.embedding_runtime_status.clone()
    }

    pub(crate) async fn auto_link_node_to_daily_note_best_effort(&self, node: &KnowledgeNode) {
        if !self.config.daily_notes.enabled {
            return;
        }
        if is_template_node(node) {
            return;
        }
        if is_daily_note(node) {
            return;
        }
        if !is_daily_link_candidate(node) {
            return;
        }

        let day = node.temporal.created_at.date_naive();
        let daily_note = match self.find_daily_note(day, &node.namespace).await {
            Ok(Some(existing)) => existing,
            Ok(None) => match self
                .ensure_daily_note(day, Some(node.namespace.clone()))
                .await
            {
                Ok((created, _created)) => created,
                Err(err) => {
                    tracing::warn!(
                        node_id = %node.id,
                        namespace = %node.namespace,
                        date = %day,
                        error = %err,
                        "mindvault_daily_note_auto_link_ensure_failed"
                    );
                    return;
                }
            },
            Err(err) => {
                tracing::warn!(
                    node_id = %node.id,
                    namespace = %node.namespace,
                    date = %day,
                    error = %err,
                    "mindvault_daily_note_auto_link_lookup_failed"
                );
                return;
            }
        };

        let existing_relationships = match self.graph.get_relationships_from(daily_note.id).await {
            Ok(relationships) => relationships,
            Err(err) => {
                tracing::warn!(
                    node_id = %node.id,
                    daily_note_id = %daily_note.id,
                    error = %err,
                    "mindvault_daily_note_auto_link_relationship_scan_failed"
                );
                return;
            }
        };

        let already_linked = existing_relationships.iter().any(|rel| {
            rel.to_node == node.id
                && matches!(
                    rel.kind,
                    RelationKind::Contains | RelationKind::References | RelationKind::PartOf
                )
        });
        if already_linked {
            return;
        }

        let relationship = Relationship::new(daily_note.id, node.id, RelationKind::Contains);
        if let Err(err) = self.graph.add_relationship(&relationship).await {
            tracing::warn!(
                node_id = %node.id,
                daily_note_id = %daily_note.id,
                error = %err,
                "mindvault_daily_note_auto_link_insert_failed"
            );
            return;
        }

        tracing::info!(
            node_id = %node.id,
            daily_note_id = %daily_note.id,
            namespace = %node.namespace,
            date = %day,
            "mindvault_daily_note_auto_linked"
        );
    }

    pub(crate) async fn auto_backlink_node_references_best_effort(&self, node: &KnowledgeNode) {
        if !self.config.linking.auto_backlinks_enabled {
            return;
        }
        if is_template_node(node) {
            return;
        }

        let link_targets = extract_reference_targets_with_kind(
            &node.content,
            self.config.linking.auto_backlinks_max_targets,
        );

        let resolved_target_ids = if link_targets.is_empty() {
            Vec::new()
        } else {
            let index = match self
                .build_backlink_resolution_index(
                    &node.namespace,
                    self.config.linking.auto_backlinks_scan_limit,
                )
                .await
            {
                Ok(index) => index,
                Err(err) => {
                    tracing::warn!(
                        node_id = %node.id,
                        namespace = %node.namespace,
                        error = %err,
                        "mindvault_backlink_auto_index_build_failed"
                    );
                    return;
                }
            };
            resolve_reference_targets_with_kind(&link_targets, node.id, &index)
        };

        if let Err(err) = self
            .sync_auto_backlink_references(node.id, &resolved_target_ids)
            .await
        {
            tracing::warn!(
                node_id = %node.id,
                namespace = %node.namespace,
                error = %err,
                "mindvault_backlink_auto_sync_failed"
            );
            return;
        }

        tracing::debug!(
            node_id = %node.id,
            namespace = %node.namespace,
            resolved_links = resolved_target_ids.len(),
            extracted_targets = link_targets.len(),
            "mindvault_backlink_auto_sync_applied"
        );
    }

    async fn build_backlink_resolution_index(
        &self,
        namespace: &str,
        scan_limit: usize,
    ) -> MvResult<KnowledgeVaultBacklinkResolutionIndex> {
        if scan_limit == 0 {
            return Ok(KnowledgeVaultBacklinkResolutionIndex::default());
        }

        let mut index = KnowledgeVaultBacklinkResolutionIndex::default();
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            ..Default::default()
        };
        let page_size = scan_limit.min(200);
        let mut offset = 0usize;
        let mut scanned = 0usize;

        while scanned < scan_limit {
            let remaining = scan_limit - scanned;
            let limit = remaining.min(page_size);
            let batch = self.store.nodes.list(&filters, limit, offset).await?;
            if batch.is_empty() {
                break;
            }
            offset += batch.len();
            scanned += batch.len();
            for node in batch {
                index.insert_node(node.id, node.title.as_deref(), node.source.as_deref());
            }
        }

        if scanned == scan_limit {
            tracing::debug!(
                namespace = %namespace,
                scan_limit,
                "mindvault_backlink_auto_index_scan_capped"
            );
        }

        Ok(index)
    }

    async fn sync_auto_backlink_references(
        &self,
        from_node_id: uuid::Uuid,
        desired_target_ids: &[ResolvedContentReferenceTarget],
    ) -> MvResult<()> {
        let mut desired =
            std::collections::HashMap::<uuid::Uuid, ContentReferenceSourceKind>::new();
        for target in desired_target_ids {
            desired.entry(target.node_id).or_insert(target.source_kind);
        }
        let desired_ids: std::collections::HashSet<uuid::Uuid> = desired.keys().copied().collect();
        let existing = self.graph.get_relationships_from(from_node_id).await?;

        let mut existing_auto_map = std::collections::HashMap::<uuid::Uuid, Relationship>::new();
        for rel in &existing {
            if rel.kind == RelationKind::References && is_auto_backlink_relationship(rel) {
                existing_auto_map.insert(rel.to_node, rel.clone());
            }
        }

        for (target_id, source_kind) in desired {
            let desired_source = source_kind.as_str();
            if let Some(existing_rel) = existing_auto_map.get(&target_id) {
                let existing_source = existing_rel
                    .metadata
                    .get(AUTO_BACKLINK_SOURCE_METADATA_KEY)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if existing_source == desired_source {
                    continue;
                }
                self.graph.remove_relationship(existing_rel.id).await?;
            }

            let mut relationship =
                Relationship::new(from_node_id, target_id, RelationKind::References);
            relationship.metadata.insert(
                AUTO_BACKLINK_METADATA_KEY.to_string(),
                serde_json::Value::Bool(true),
            );
            relationship.metadata.insert(
                AUTO_BACKLINK_SOURCE_METADATA_KEY.to_string(),
                serde_json::Value::String(desired_source.to_string()),
            );
            self.graph.add_relationship(&relationship).await?;
        }

        for rel in &existing {
            if rel.kind != RelationKind::References || !is_auto_backlink_relationship(rel) {
                continue;
            }
            if desired_ids.contains(&rel.to_node) {
                continue;
            }
            self.graph.remove_relationship(rel.id).await?;
        }

        Ok(())
    }
}
