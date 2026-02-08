use crate::config::WatcherConfig;
use crate::engine::MindVaultEngine;
use crate::intent::IntentEngine;
use crate::proactive::ProactiveEngine;
use chrono::{Duration, Utc};
use mv_core::{AgenticStore, ChronicleEntry, MvResult, QueryFilters};
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};

/// Report from a watcher cycle
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct WatcherReport {
    pub nodes_scanned: usize,
    pub intents_detected: usize,
    pub insights_generated: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// The Watcher Agent monitors vault changes and generates proposals.
pub struct WatcherAgent {
    engine: Arc<MindVaultEngine>,
    intent_engine: IntentEngine,
    proactive_engine: Arc<ProactiveEngine>,
    config: WatcherConfig,
}

impl WatcherAgent {
    pub fn new(
        engine: Arc<MindVaultEngine>,
        intent_engine: IntentEngine,
        proactive_engine: Arc<ProactiveEngine>,
        config: WatcherConfig,
    ) -> Self {
        Self {
            engine,
            intent_engine,
            proactive_engine,
            config,
        }
    }

    /// Run a single watcher cycle
    pub async fn run_cycle(&self) -> MvResult<WatcherReport> {
        let start = std::time::Instant::now();
        let mut report = WatcherReport::default();

        // Log cycle start to chronicle
        let entry = ChronicleEntry::new(
            "watcher_cycle_start",
            format!(
                "Starting watcher cycle. Lookback: {}h, Max nodes: {}",
                self.config.lookback_hours, self.config.max_nodes_per_cycle
            ),
        );
        self.engine.store.nodes.log_chronicle(&entry).await?;

        // 1. Get recently modified nodes
        let lookback = Utc::now() - Duration::hours(self.config.lookback_hours as i64);
        let filters = QueryFilters {
            created_after: Some(lookback),
            ..Default::default()
        };

        let recent_nodes = self
            .engine
            .list_nodes(&filters, self.config.max_nodes_per_cycle, 0)
            .await?;

        report.nodes_scanned = recent_nodes.len();

        // 2. Run intent detection on each node
        for node in &recent_nodes {
            match self.intent_engine.extract_intents_and_store(node).await {
                Ok(intents) => {
                    report.intents_detected += intents.len();

                    // Log each intent to chronicle
                    for intent in &intents {
                        let entry = ChronicleEntry::new(
                            "intent_detected",
                            format!(
                                "Detected {} intent with {:.0}% confidence",
                                intent.intent_type,
                                intent.confidence * 100.0
                            ),
                        )
                        .with_node(node.id);
                        let _ = self.engine.store.nodes.log_chronicle(&entry).await;
                    }
                }
                Err(e) => {
                    report
                        .errors
                        .push(format!("Intent detection failed for {}: {}", node.id, e));
                }
            }
        }

        // 3. Run insight generation (namespace-level analysis)
        // Get unique namespaces from scanned nodes
        let namespaces: std::collections::HashSet<_> =
            recent_nodes.iter().map(|n| n.namespace.clone()).collect();

        for namespace in namespaces {
            match self
                .proactive_engine
                .generate_insights(namespace.clone())
                .await
            {
                Ok(insights) => {
                    report.insights_generated += insights.len();

                    // Log each insight to chronicle
                    for insight in &insights {
                        let entry = ChronicleEntry::new(
                            "insight_generated",
                            format!("{}: {}", insight.insight_type, insight.title),
                        );
                        let _ = self.engine.store.nodes.log_chronicle(&entry).await;
                    }
                }
                Err(e) => {
                    report.errors.push(format!(
                        "Insight generation failed for namespace {}: {}",
                        namespace, e
                    ));
                }
            }
        }

        // 4. Log cycle completion
        report.duration_ms = start.elapsed().as_millis() as u64;

        let entry = ChronicleEntry::new(
            "watcher_cycle_complete",
            format!(
                "Cycle complete. Scanned {} nodes, found {} intents, {} insights in {}ms",
                report.nodes_scanned,
                report.intents_detected,
                report.insights_generated,
                report.duration_ms
            ),
        );
        self.engine.store.nodes.log_chronicle(&entry).await?;

        tracing::info!(
            nodes = report.nodes_scanned,
            intents = report.intents_detected,
            insights = report.insights_generated,
            duration_ms = report.duration_ms,
            "Watcher cycle complete"
        );

        Ok(report)
    }

    /// Start the watcher loop (runs until cancelled)
    pub async fn run_loop(self: Arc<Self>, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        if !self.config.enabled {
            tracing::info!("Watcher agent is disabled");
            return;
        }

        tracing::info!(
            interval_secs = self.config.interval_secs,
            "Starting watcher agent loop"
        );

        let mut ticker = interval(TokioDuration::from_secs(self.config.interval_secs));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.run_cycle().await {
                        Ok(report) => {
                            if !report.errors.is_empty() {
                                tracing::warn!(
                                    errors = ?report.errors,
                                    "Watcher cycle completed with errors"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Watcher cycle failed");
                        }
                    }
                }
                _ = shutdown.recv() => {
                    tracing::info!("Watcher agent shutting down");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_secs, 300);
        assert_eq!(config.lookback_hours, 24);
    }
}
