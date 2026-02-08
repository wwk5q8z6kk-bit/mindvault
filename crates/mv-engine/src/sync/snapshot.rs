//! Sync snapshot types for import/export.

use super::clock::VectorClock;
use chrono::{DateTime, Utc};
use mv_core::KnowledgeNode;
use serde::{Deserialize, Serialize};

/// An exported snapshot of vault data for sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshot {
    pub device_id: String,
    pub exported_at: DateTime<Utc>,
    pub node_count: usize,
    pub nodes: Vec<KnowledgeNode>,
    pub clock: VectorClock,
}

/// A sync conflict that couldn't be auto-resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub node_id: uuid::Uuid,
    pub local_updated_at: DateTime<Utc>,
    pub remote_updated_at: DateTime<Utc>,
    pub remote_content_preview: String,
    pub resolved: bool,
}

/// Statistics from a sync import operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStats {
    pub scanned: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
}
