use std::sync::Arc;

use mv_core::*;
use mv_graph::store::SqliteGraphStore;
use mv_index::hybrid::{apply_graph_boost, reciprocal_rank_fusion};
use mv_index::tantivy_index::TantivyFullTextIndex;
use mv_storage::unified::UnifiedStore;

use crate::config::EngineConfig;

/// Recall pipeline: searches across FTS + vector + graph and fuses results.
pub struct RecallPipeline {
    store: Arc<UnifiedStore>,
    fts: Arc<TantivyFullTextIndex>,
    graph: Arc<SqliteGraphStore>,
    config: EngineConfig,
}

impl RecallPipeline {
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
        }
    }

    /// Execute a memory query and return ranked results.
    pub async fn recall(&self, query: &MemoryQuery) -> MvResult<Vec<SearchResult>> {
        let limit = if query.limit > 0 {
            query.limit
        } else {
            self.config.search.default_limit
        };

        // Fetch extra results for post-filtering
        let fetch_limit = limit * 3;

        let mut result_lists: Vec<Vec<(uuid::Uuid, f64)>> = Vec::new();

        match query.strategy {
            SearchStrategy::FullText => {
                let fts_results = self.fts.search(&query.text, fetch_limit)?;
                result_lists.push(fts_results);
            }
            SearchStrategy::Vector => {
                if let Some(ref vectors) = self.store.vectors {
                    let embedding = self.store.embedder.embed(&query.text).await?;
                    let vec_results = vectors
                        .search(
                            embedding,
                            fetch_limit,
                            query.min_score,
                            query.filters.namespace.as_deref(),
                        )
                        .await?;
                    result_lists.push(vec_results);
                }
            }
            SearchStrategy::Hybrid => {
                // Full-text search
                let fts_results = self.fts.search(&query.text, fetch_limit)?;
                result_lists.push(fts_results);

                // Vector search (if available)
                if let Some(ref vectors) = self.store.vectors {
                    match self.store.embedder.embed(&query.text).await {
                        Ok(embedding) => {
                            let vec_results = vectors
                                .search(
                                    embedding,
                                    fetch_limit,
                                    query.min_score,
                                    query.filters.namespace.as_deref(),
                                )
                                .await?;
                            result_lists.push(vec_results);
                        }
                        Err(e) => {
                            tracing::warn!("vector search skipped (embedding failed): {e}");
                        }
                    }
                }
            }
            SearchStrategy::Graph => {
                // First do a text search to find seed nodes, then expand via graph
                let fts_results = self.fts.search(&query.text, 5)?;
                let mut all_neighbors = Vec::new();

                for (node_id, _score) in &fts_results {
                    let neighbors = self
                        .graph
                        .get_neighbors(*node_id, self.config.graph.default_traversal_depth)
                        .await?;
                    all_neighbors.extend(neighbors);
                }

                // Combine seed results with graph-expanded results
                let mut combined: Vec<(uuid::Uuid, f64)> = fts_results;
                for neighbor_id in all_neighbors {
                    if !combined.iter().any(|(id, _)| *id == neighbor_id) {
                        combined.push((neighbor_id, 0.3)); // base score for graph neighbors
                    }
                }
                result_lists.push(combined);
            }
        }

        // Fuse results using RRF
        let mut fused = if result_lists.len() > 1 {
            reciprocal_rank_fusion(&result_lists, self.config.search.rrf_k, fetch_limit)
        } else {
            result_lists.into_iter().next().unwrap_or_default()
        };

        // Apply graph boost for hybrid/fulltext/vector strategies
        if query.strategy != SearchStrategy::Graph {
            let mut all_neighbors = Vec::new();
            for (node_id, _) in fused.iter().take(5) {
                if let Ok(neighbors) = self
                    .graph
                    .get_neighbors(*node_id, self.config.graph.default_traversal_depth)
                    .await
                {
                    all_neighbors.extend(neighbors);
                }
            }
            if !all_neighbors.is_empty() {
                apply_graph_boost(
                    &mut fused,
                    &all_neighbors,
                    self.config.graph.graph_boost_factor,
                );
            }
        }

        // Filter by min_score
        fused.retain(|(_, score)| *score >= query.min_score);

        // Truncate to requested limit
        fused.truncate(limit);

        // Hydrate with full node data
        let mut results = Vec::new();
        for (node_id, score) in fused {
            // Touch for access tracking
            let _ = self.store.nodes.touch(node_id).await;

            if let Ok(Some(node)) = self.store.nodes.get(node_id).await {
                // Apply filter checks
                if !matches_filters(&node, &query.filters) {
                    continue;
                }
                results.push(SearchResult {
                    node,
                    score,
                    match_source: match query.strategy {
                        SearchStrategy::Vector => MatchSource::Vector,
                        SearchStrategy::FullText => MatchSource::FullText,
                        SearchStrategy::Hybrid => MatchSource::Hybrid,
                        SearchStrategy::Graph => MatchSource::Graph,
                    },
                });
            }
        }

        Ok(results)
    }
}

fn matches_filters(node: &KnowledgeNode, filters: &QueryFilters) -> bool {
    if let Some(ref ns) = filters.namespace {
        if node.namespace != *ns {
            return false;
        }
    }
    if let Some(ref kinds) = filters.kinds {
        if !kinds.contains(&node.kind) {
            return false;
        }
    }
    if let Some(ref tags) = filters.tags {
        if !tags.iter().any(|t| node.tags.contains(t)) {
            return false;
        }
    }
    if let Some(min_imp) = filters.min_importance {
        if node.importance < min_imp {
            return false;
        }
    }
    if let Some(ref after) = filters.created_after {
        if node.temporal.created_at < *after {
            return false;
        }
    }
    if let Some(ref before) = filters.created_before {
        if node.temporal.created_at > *before {
            return false;
        }
    }
    true
}
