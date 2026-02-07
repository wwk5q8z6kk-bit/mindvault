use crate::engine::MindVaultEngine;
use chrono::{Duration, Utc};
use mv_core::{AgenticStore, InsightType, KnowledgeNode, MemoryQuery, MvResult, ProactiveInsight, QueryFilters};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct ProactiveEngine {
    engine: tokio::sync::OnceCell<Weak<MindVaultEngine>>,
}

impl ProactiveEngine {
    pub fn new() -> Self {
        tracing::info!("Initializing ProactiveEngine");
        Self {
            engine: tokio::sync::OnceCell::new(),
        }
    }

    pub fn set_engine(&self, engine: Arc<MindVaultEngine>) {
        if self.engine.set(Arc::downgrade(&engine)).is_err() {
            tracing::warn!("ProactiveEngine engine already set");
        }
    }

    fn engine(&self) -> Arc<MindVaultEngine> {
        self.engine
            .get()
            .and_then(Weak::upgrade)
            .expect("ProactiveEngine::engine called before set_engine")
    }

    /// Identify nodes related to the given basis node using graph and semantic relevance.
    pub async fn find_related_context(
        &self,
        basis_node_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<KnowledgeNode>> {
        let span = tracing::info_span!("find_related_context", basis_node_id = %basis_node_id);
        let _enter = span.enter();

        let engine = self.engine();
        let basis_node = match engine.get_node(basis_node_id).await? {
            Some(node) => node,
            None => return Ok(Vec::new()),
        };

        // Find neighbors in the graph.
        let neighbors = engine.get_neighbors(basis_node.id, 1).await?;

        // Find semantically similar nodes (RAG).
        let query_text = basis_node.title.as_deref().unwrap_or(&basis_node.content);
        let recall_query = MemoryQuery::new(query_text.to_string())
            .with_namespace(basis_node.namespace.clone())
            .with_limit(limit.saturating_mul(2).max(limit));
        let recall_results = engine.recall(&recall_query).await?;

        let mut related = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        seen_ids.insert(basis_node.id);

        // Add neighbors first (strongest graph signal).
        for neighbor_id in neighbors {
            if seen_ids.insert(neighbor_id) {
                if let Some(neighbor_node) = engine.get_node(neighbor_id).await? {
                    related.push(neighbor_node);
                }
            }
        }

        // Add recall results
        for result in recall_results {
            if seen_ids.insert(result.node.id) {
                related.push(result.node);
            }
        }

        let results: Vec<_> = related.into_iter().take(limit).collect();
        tracing::info!(count = results.len(), "Context surfaced");
        Ok(results)
    }

    /// Generate a summary of the current executive context.
    pub async fn get_executive_summary(&self, namespace: String) -> MvResult<String> {
        let engine = self.engine();
        let filters = QueryFilters {
            namespace: Some(namespace),
            ..Default::default()
        };
        let recent = engine.list_nodes(&filters, 5, 0).await?;

        if recent.is_empty() {
            return Ok("The AI assistant is monitoring your active context. Related information will appear here.".to_string());
        }

        let mut summary = String::from("Recent activity focus:\n");
        for node in recent {
            summary.push_str(&format!(
                "- {}\n",
                node.title.as_deref().unwrap_or(&node.content)
            ));
        }

        Ok(summary)
    }

    /// Generate new proactive insights based on vault patterns.
    pub async fn generate_insights(&self, namespace: String) -> MvResult<Vec<ProactiveInsight>> {
        let engine = self.engine();
        let mut insights = Vec::new();

        // Pattern 1: Recent activity clustering
        if let Some(insight) = self.detect_activity_cluster(&engine, &namespace).await? {
            insights.push(insight);
        }

        // Pattern 2: Stale nodes that might need attention
        if let Some(insight) = self.detect_stale_nodes(&engine, &namespace).await? {
            insights.push(insight);
        }

        // Pattern 3: Trending topics (tags/keywords appearing frequently)
        if let Some(insight) = self.detect_trending_topics(&engine, &namespace).await? {
            insights.push(insight);
        }

        // Pattern 4: Connection opportunities (nodes with similar content but no links)
        if let Some(insight) = self.detect_connection_opportunities(&engine, &namespace).await? {
            insights.push(insight);
        }

        // Store generated insights
        for insight in &insights {
            engine.store.nodes.log_insight(insight).await?;
        }

        Ok(insights)
    }

    /// Detect clusters of recent activity
    async fn detect_activity_cluster(
        &self,
        engine: &MindVaultEngine,
        namespace: &str,
    ) -> MvResult<Option<ProactiveInsight>> {
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            ..Default::default()
        };
        let recent = engine.list_nodes(&filters, 10, 0).await?;

        if recent.len() < 3 {
            return Ok(None);
        }

        // Count tag frequencies
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for node in &recent {
            for tag in &node.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        // Find tags appearing in multiple nodes
        let common_tags: Vec<_> = tag_counts
            .iter()
            .filter(|(_, &count)| count >= 2)
            .map(|(tag, _)| tag.clone())
            .collect();

        if common_tags.is_empty() {
            return Ok(None);
        }

        let insight = ProactiveInsight::new(
            "Emerging Focus Area",
            format!(
                "You've been working on {} related entries. Common themes: {}",
                recent.len(),
                common_tags.join(", ")
            ),
            InsightType::Cluster,
        )
        .with_related_nodes(recent.iter().map(|n| n.id).collect())
        .with_importance(0.6);

        Ok(Some(insight))
    }

    /// Detect nodes that haven't been accessed in a while
    async fn detect_stale_nodes(
        &self,
        engine: &MindVaultEngine,
        namespace: &str,
    ) -> MvResult<Option<ProactiveInsight>> {
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            ..Default::default()
        };
        let nodes = engine.list_nodes(&filters, 50, 0).await?;

        let stale_threshold = Utc::now() - Duration::days(30);
        let stale_nodes: Vec<_> = nodes
            .into_iter()
            .filter(|n| {
                n.temporal.last_accessed_at < stale_threshold
                    && n.importance >= 0.5 // Only care about important nodes
            })
            .take(5)
            .collect();

        if stale_nodes.is_empty() {
            return Ok(None);
        }

        let titles: Vec<_> = stale_nodes
            .iter()
            .filter_map(|n| n.title.clone())
            .take(3)
            .collect();

        let insight = ProactiveInsight::new(
            "Notes Need Attention",
            format!(
                "{} important notes haven't been accessed in over a month{}",
                stale_nodes.len(),
                if !titles.is_empty() {
                    format!(", including: {}", titles.join(", "))
                } else {
                    String::new()
                }
            ),
            InsightType::Stale,
        )
        .with_related_nodes(stale_nodes.iter().map(|n| n.id).collect())
        .with_importance(0.5);

        Ok(Some(insight))
    }

    /// Detect trending topics based on recent tag usage
    async fn detect_trending_topics(
        &self,
        engine: &MindVaultEngine,
        namespace: &str,
    ) -> MvResult<Option<ProactiveInsight>> {
        let week_ago = Utc::now() - Duration::days(7);
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            created_after: Some(week_ago),
            ..Default::default()
        };
        let recent = engine.list_nodes(&filters, 20, 0).await?;

        if recent.len() < 5 {
            return Ok(None);
        }

        // Count tags
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for node in &recent {
            for tag in &node.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        // Find top trending tags (at least 3 occurrences)
        let mut trending: Vec<_> = tag_counts
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .collect();
        trending.sort_by(|a, b| b.1.cmp(&a.1));

        if trending.is_empty() {
            return Ok(None);
        }

        let top_tags: Vec<_> = trending.iter().take(3).map(|(t, _)| t.clone()).collect();

        let insight = ProactiveInsight::new(
            "Trending Topics This Week",
            format!(
                "Your most active topics: {}. Consider creating a summary or project note.",
                top_tags.join(", ")
            ),
            InsightType::Trend,
        )
        .with_importance(0.55);

        Ok(Some(insight))
    }

    /// Detect potential connections between unlinked similar nodes
    async fn detect_connection_opportunities(
        &self,
        engine: &MindVaultEngine,
        namespace: &str,
    ) -> MvResult<Option<ProactiveInsight>> {
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            ..Default::default()
        };
        let recent = engine.list_nodes(&filters, 10, 0).await?;

        if recent.len() < 2 {
            return Ok(None);
        }

        // Pick the most recent node and find similar ones
        let basis = &recent[0];
        let query_text = basis.title.as_deref().unwrap_or(&basis.content);
        let recall_query = MemoryQuery::new(query_text.to_string())
            .with_namespace(namespace.to_string())
            .with_limit(5)
            .with_min_score(0.7);

        let similar = engine.recall(&recall_query).await?;

        // Filter out nodes that are already connected
        let neighbors = engine.get_neighbors(basis.id, 1).await?;
        let neighbor_set: std::collections::HashSet<_> = neighbors.into_iter().collect();

        let unconnected: Vec<_> = similar
            .iter()
            .filter(|r| r.node.id != basis.id && !neighbor_set.contains(&r.node.id))
            .collect();

        if unconnected.is_empty() {
            return Ok(None);
        }

        let titles: Vec<_> = unconnected
            .iter()
            .filter_map(|r| r.node.title.clone())
            .take(2)
            .collect();

        let mut related = vec![basis.id];
        related.extend(unconnected.iter().map(|r| r.node.id));

        let insight = ProactiveInsight::new(
            "Potential Connections Found",
            format!(
                "\"{}\" might be related to {}. Consider linking them.",
                basis.title.as_deref().unwrap_or("Recent note"),
                if !titles.is_empty() {
                    titles.join(" and ")
                } else {
                    "similar notes".to_string()
                }
            ),
            InsightType::Connection,
        )
        .with_related_nodes(related)
        .with_importance(0.65);

        Ok(Some(insight))
    }
}

impl Default for ProactiveEngine {
    fn default() -> Self {
        Self::new()
    }
}
