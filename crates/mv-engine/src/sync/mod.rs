//! Device-to-device sync via encrypted snapshots.
//! Uses vector clocks for conflict detection and the Exchange Inbox for conflict resolution.

pub mod clock;
pub mod snapshot;

use chrono::{DateTime, Utc};
use mv_core::*;
use mv_storage::unified::UnifiedStore;
use std::sync::Arc;

pub use clock::VectorClock;
pub use snapshot::{SyncConflict, SyncSnapshot, SyncStats};

/// Sync engine orchestrates import/export of vault snapshots.
pub struct SyncEngine {
    store: Arc<UnifiedStore>,
    device_id: String,
}

impl SyncEngine {
    pub fn new(store: Arc<UnifiedStore>, device_id: String) -> Self {
        Self { store, device_id }
    }

    /// Export a snapshot of nodes modified since `since`.
    pub async fn export_snapshot(
        &self,
        since: Option<DateTime<Utc>>,
        namespace: Option<&str>,
    ) -> MvResult<SyncSnapshot> {
        let filters = QueryFilters {
            namespace: namespace.map(|s| s.to_string()),
            ..Default::default()
        };
        let nodes = self.store.nodes.list(&filters, 10000, 0).await?;

        let filtered: Vec<KnowledgeNode> = if let Some(since) = since {
            nodes
                .into_iter()
                .filter(|n| n.temporal.updated_at >= since)
                .collect()
        } else {
            nodes
        };

        Ok(SyncSnapshot {
            device_id: self.device_id.clone(),
            exported_at: Utc::now(),
            node_count: filtered.len(),
            nodes: filtered,
            clock: VectorClock::new(&self.device_id),
        })
    }

    /// Import a snapshot from another device, detecting conflicts.
    pub async fn import_snapshot(&self, snapshot: SyncSnapshot) -> MvResult<SyncStats> {
        let mut stats = SyncStats::default();

        for incoming_node in &snapshot.nodes {
            stats.scanned += 1;

            match self.store.nodes.get(incoming_node.id).await? {
                Some(local_node) => {
                    // Node exists locally -- check for conflict
                    if local_node.temporal.updated_at > incoming_node.temporal.updated_at {
                        // Local is strictly newer -- skip
                        stats.skipped += 1;
                    } else if local_node.temporal.updated_at == incoming_node.temporal.updated_at {
                        // Same timestamp -- true conflict
                        stats.conflicts += 1;
                        // Create a conflict proposal in the Exchange Inbox
                        let diff = format!(
                            "Local updated: {}\nRemote updated: {}\nRemote content preview: {}",
                            local_node.temporal.updated_at,
                            incoming_node.temporal.updated_at,
                            incoming_node.content.chars().take(200).collect::<String>()
                        );
                        let proposal =
                            Proposal::new(ProposalSender::Relay, ProposalAction::UpdateNode)
                                .with_target(incoming_node.id)
                                .with_diff(diff)
                                .with_confidence(0.5);
                        self.store.nodes.submit_proposal(&proposal).await?;
                    } else {
                        // Remote is newer -- update local
                        self.store.nodes.update(incoming_node).await?;
                        stats.updated += 1;
                    }
                }
                None => {
                    // New node -- insert
                    self.store.nodes.insert(incoming_node).await?;
                    stats.inserted += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Get the current device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}
