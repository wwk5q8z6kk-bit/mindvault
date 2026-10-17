use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::workspace_path::WorkspacePathIssue;

pub const WORKSPACE_DOCUMENT_PAYLOAD_SCHEMA_V1: &str = "mindvault.workspace-document/v1";
pub const WORKSPACE_DESCRIPTOR_PAYLOAD_SCHEMA_V1: &str = "mindvault.workspace-descriptor/v1";

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($variant:ident => $value:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant)),+,
                    _ => Err(format!("invalid {} value: {value}", stringify!($name))),
                }
            }
        }
    };
}

string_enum! {
    /// How MindVault relates to the canonical files in a knowledge workspace.
    pub enum KnowledgeWorkspaceMode {
        Mounted => "mounted",
        ManagedPlaintext => "managed_plaintext",
    }
}

string_enum! {
    /// Result of inspecting a canonical document's bytes.
    pub enum WorkspaceDocumentContentStatus {
        Utf8 => "utf8",
        InvalidUtf8 => "invalid_utf8",
        TooLarge => "too_large",
    }
}

/// Sensitive root locator and display metadata for a mounted workspace.
///
/// The serialized descriptor is stored only in `descriptor_payload`. Sealed
/// storage must encrypt the serialized bytes before persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDescriptorPayloadV1 {
    pub schema: String,
    pub display_name: String,
    pub root_path: String,
}

impl WorkspaceDescriptorPayloadV1 {
    pub fn new(display_name: impl Into<String>, root_path: impl Into<String>) -> Self {
        Self {
            schema: WORKSPACE_DESCRIPTOR_PAYLOAD_SCHEMA_V1.to_string(),
            display_name: display_name.into(),
            root_path: root_path.into(),
        }
    }

    pub fn has_supported_schema(&self) -> bool {
        self.schema == WORKSPACE_DESCRIPTOR_PAYLOAD_SCHEMA_V1
    }
}

/// Cross-platform identity hint captured from filesystem metadata.
///
/// This value is only a rename heuristic. It is stored inside the protected
/// document payload and is never treated as canonical identity by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileIdentityHint {
    pub scheme: String,
    pub value: String,
}

/// Reversible v1 payload for one canonical workspace document.
///
/// In sealed mode the serialized value must be encrypted before it crosses
/// the manifest-store boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDocumentPayloadV1 {
    pub schema: String,
    pub relative_path: String,
    pub content_hash: Option<String>,
    pub byte_size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub content_status: WorkspaceDocumentContentStatus,
    pub file_identity_hint: Option<WorkspaceFileIdentityHint>,
    pub portability_issues: Vec<WorkspacePathIssue>,
    pub reconciliation_note: Option<String>,
}

impl WorkspaceDocumentPayloadV1 {
    pub fn new(relative_path: impl Into<String>, observed_at: DateTime<Utc>) -> Self {
        Self {
            schema: WORKSPACE_DOCUMENT_PAYLOAD_SCHEMA_V1.to_string(),
            relative_path: relative_path.into(),
            content_hash: None,
            byte_size: 0,
            modified_at: None,
            observed_at,
            content_status: WorkspaceDocumentContentStatus::Utf8,
            file_identity_hint: None,
            portability_issues: Vec::new(),
            reconciliation_note: None,
        }
    }

    pub fn has_supported_schema(&self) -> bool {
        self.schema == WORKSPACE_DOCUMENT_PAYLOAD_SCHEMA_V1
    }
}

string_enum! {
    /// Current reconciliation health of a knowledge workspace.
    pub enum KnowledgeWorkspaceState {
        Indexing => "indexing",
        Ready => "ready",
        Degraded => "degraded",
        Offline => "offline",
        Error => "error",
    }
}

string_enum! {
    /// Encoding of an opaque managed-manifest payload.
    pub enum WorkspaceManifestPayloadFormat {
        JsonV1 => "json-v1",
        MvencV1 => "mvenc-v1",
    }
}

string_enum! {
    /// Observed lifecycle of a canonical workspace document.
    pub enum WorkspaceDocumentLifecycle {
        Active => "active",
        Missing => "missing",
        Trashed => "trashed",
        Conflict => "conflict",
        Unsupported => "unsupported",
    }
}

string_enum! {
    /// State of the derived knowledge-node projection for a document.
    pub enum WorkspaceProjectionState {
        Pending => "pending",
        Ready => "ready",
        Failed => "failed",
        Stale => "stale",
    }
}

/// Managed database record for a file-first knowledge workspace.
///
/// Root locators and other sensitive descriptors remain inside
/// `descriptor_payload`; they must not be copied into plaintext columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeWorkspace {
    pub id: Uuid,
    pub namespace: String,
    pub mode: KnowledgeWorkspaceMode,
    pub state: KnowledgeWorkspaceState,
    pub descriptor_payload: Vec<u8>,
    pub payload_format: WorkspaceManifestPayloadFormat,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_reconciled_at: Option<DateTime<Utc>>,
}

impl KnowledgeWorkspace {
    pub fn new(
        namespace: impl Into<String>,
        mode: KnowledgeWorkspaceMode,
        descriptor_payload: Vec<u8>,
        payload_format: WorkspaceManifestPayloadFormat,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            namespace: namespace.into(),
            mode,
            state: KnowledgeWorkspaceState::Indexing,
            descriptor_payload,
            payload_format,
            revision: 1,
            created_at: now,
            updated_at: now,
            last_reconciled_at: None,
        }
    }
}

/// Managed database record for one canonical file in a knowledge workspace.
///
/// The reversible relative path and exact content hash remain inside
/// `document_payload`. `path_token` is a non-reversible lookup token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeWorkspaceDocument {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub path_token: String,
    pub document_payload: Vec<u8>,
    pub payload_format: WorkspaceManifestPayloadFormat,
    pub lifecycle_state: WorkspaceDocumentLifecycle,
    pub projection_state: WorkspaceProjectionState,
    pub projected_node_id: Option<Uuid>,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeWorkspaceDocument {
    pub fn new(
        workspace_id: Uuid,
        path_token: impl Into<String>,
        document_payload: Vec<u8>,
        payload_format: WorkspaceManifestPayloadFormat,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            workspace_id,
            path_token: path_token.into(),
            document_payload,
            payload_format,
            lifecycle_state: WorkspaceDocumentLifecycle::Active,
            projection_state: WorkspaceProjectionState::Pending,
            projected_node_id: None,
            revision: 1,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Optimistic replacement used inside one manifest reconciliation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDocumentManifestUpdate {
    pub expected_revision: u64,
    pub replacement: KnowledgeWorkspaceDocument,
}

/// Atomic manifest mutations produced by one authoritative filesystem scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifestReconciliation {
    pub expected_workspace_revision: u64,
    pub workspace_replacement: KnowledgeWorkspace,
    pub document_inserts: Vec<KnowledgeWorkspaceDocument>,
    pub document_updates: Vec<WorkspaceDocumentManifestUpdate>,
}

pub const WORKSPACE_EVENT_PAYLOAD_SCHEMA_V1: &str = "mindvault.workspace-event/v1";

string_enum! {
    /// Who initiated a workspace journal entry.
    pub enum WorkspaceEventActorKind {
        User => "user",
        System => "system",
        Migration => "migration",
        External => "external",
        Plugin => "plugin",
        Mcp => "mcp",
        Automation => "automation",
        AiProposal => "ai_proposal",
    }
}

string_enum! {
    /// Operation recorded in the workspace mutation journal.
    pub enum WorkspaceEventOperation {
        Mount => "mount",
        Scan => "scan",
        Create => "create",
        Update => "update",
        Move => "move",
        Trash => "trash",
        Restore => "restore",
        ExternalCreate => "external_create",
        ExternalUpdate => "external_update",
        ExternalMove => "external_move",
        ExternalDelete => "external_delete",
        ConflictResolve => "conflict_resolve",
        MigrationStage => "migration_stage",
        MigrationCommit => "migration_commit",
        MigrationRollback => "migration_rollback",
    }
}

string_enum! {
    /// Lifecycle status of a journaled workspace event.
    pub enum WorkspaceEventStatus {
        Prepared => "prepared",
        Completed => "completed",
        Aborted => "aborted",
        Conflict => "conflict",
    }
}

/// Versioned journal payload: expected-old / intended-new hashes and paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceEventPayloadV1 {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_old_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intended_new_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_old_content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intended_new_content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_old_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intended_new_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WorkspaceEventPayloadV1 {
    pub fn new() -> Self {
        Self {
            schema: WORKSPACE_EVENT_PAYLOAD_SCHEMA_V1.to_string(),
            expected_old_path: None,
            intended_new_path: None,
            expected_old_content_hash: None,
            intended_new_content_hash: None,
            expected_old_revision: None,
            intended_new_revision: None,
            phase: None,
            result: None,
            error: None,
        }
    }

    pub fn has_supported_schema(&self) -> bool {
        self.schema == WORKSPACE_EVENT_PAYLOAD_SCHEMA_V1
    }
}

impl Default for WorkspaceEventPayloadV1 {
    fn default() -> Self {
        Self::new()
    }
}

/// Durable workspace mutation journal row (migration 031 `workspace_events`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEvent {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub document_id: Option<Uuid>,
    pub event_seq: u64,
    pub correlation_id: Uuid,
    pub actor_kind: WorkspaceEventActorKind,
    pub actor_id: Option<String>,
    pub operation: WorkspaceEventOperation,
    pub status: WorkspaceEventStatus,
    pub event_payload: Vec<u8>,
    pub payload_format: WorkspaceManifestPayloadFormat,
    pub prepared_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl WorkspaceEvent {
    /// Build a completed journal entry. `event_seq` is assigned on append.
    pub fn completed(
        workspace_id: Uuid,
        correlation_id: Uuid,
        operation: WorkspaceEventOperation,
        actor_kind: WorkspaceEventActorKind,
        payload: WorkspaceEventPayloadV1,
    ) -> Result<Self, serde_json::Error> {
        let now = Utc::now();
        Ok(Self {
            id: Uuid::now_v7(),
            workspace_id,
            document_id: None,
            event_seq: 0,
            correlation_id,
            actor_kind,
            actor_id: None,
            operation,
            status: WorkspaceEventStatus::Completed,
            event_payload: serde_json::to_vec(&payload)?,
            payload_format: WorkspaceManifestPayloadFormat::JsonV1,
            prepared_at: now,
            completed_at: Some(now),
        })
    }
}


pub const WORKSPACE_CONFLICT_PAYLOAD_SCHEMA_V1: &str = "mindvault.workspace-conflict/v1";

/// Documented Stage-1 conflict resolution choices
/// (`knowledge-workspace-document-contract.md:169-171`).
pub const WORKSPACE_CONFLICT_RESOLUTIONS: [&str; 4] = [
    "keep_current",
    "restore_known_revision",
    "save_competing_to_new_path",
    "merge_via_reviewed_proposal",
];

string_enum! {
    /// Kind of workspace conflict recorded for review.
    pub enum WorkspaceConflictKind {
        StaleWrite => "stale_write",
        PathCollision => "path_collision",
        AmbiguousRename => "ambiguous_rename",
        ExternalDivergence => "external_divergence",
        UnsafePath => "unsafe_path",
        RestoreCollision => "restore_collision",
    }
}

string_enum! {
    /// Lifecycle of a recorded workspace conflict.
    pub enum WorkspaceConflictState {
        Open => "open",
        Resolved => "resolved",
        Dismissed => "dismissed",
    }
}

/// Versioned conflict payload carried in `workspace_conflicts.conflict_payload`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConflictPayloadV1 {
    pub schema: String,
    pub kind: WorkspaceConflictKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_content_hash: Option<String>,
    pub available_resolutions: Vec<String>,
}

impl WorkspaceConflictPayloadV1 {
    pub fn stale_write(
        relative_path: impl Into<String>,
        expected_content_hash: impl Into<String>,
        observed_content_hash: impl Into<String>,
        canonical_content_hash: impl Into<String>,
    ) -> Self {
        Self {
            schema: WORKSPACE_CONFLICT_PAYLOAD_SCHEMA_V1.to_string(),
            kind: WorkspaceConflictKind::StaleWrite,
            relative_path: Some(relative_path.into()),
            expected_content_hash: Some(expected_content_hash.into()),
            observed_content_hash: Some(observed_content_hash.into()),
            canonical_content_hash: Some(canonical_content_hash.into()),
            available_resolutions: WORKSPACE_CONFLICT_RESOLUTIONS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        }
    }

    pub fn has_supported_schema(&self) -> bool {
        self.schema == WORKSPACE_CONFLICT_PAYLOAD_SCHEMA_V1
    }
}

/// Durable workspace conflict review row (migration 031 `workspace_conflicts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConflict {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub document_id: Option<Uuid>,
    pub source_event_id: Option<Uuid>,
    pub conflict_kind: WorkspaceConflictKind,
    pub state: WorkspaceConflictState,
    pub conflict_payload: Vec<u8>,
    pub payload_format: WorkspaceManifestPayloadFormat,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl WorkspaceConflict {
    pub fn open_stale_write(
        workspace_id: Uuid,
        document_id: Uuid,
        payload: WorkspaceConflictPayloadV1,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: Uuid::now_v7(),
            workspace_id,
            document_id: Some(document_id),
            source_event_id: None,
            conflict_kind: WorkspaceConflictKind::StaleWrite,
            state: WorkspaceConflictState::Open,
            conflict_payload: serde_json::to_vec(&payload)?,
            payload_format: WorkspaceManifestPayloadFormat::JsonV1,
            created_at: Utc::now(),
            resolved_at: None,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_enums_round_trip_database_values() {
        assert_eq!(
            KnowledgeWorkspaceMode::from_str("managed_plaintext").unwrap(),
            KnowledgeWorkspaceMode::ManagedPlaintext
        );
        assert_eq!(WorkspaceManifestPayloadFormat::MvencV1.as_str(), "mvenc-v1");
        assert!(WorkspaceDocumentLifecycle::from_str("deleted").is_err());
    }

    #[test]
    fn workspace_descriptor_has_an_explicit_versioned_schema() {
        let descriptor = WorkspaceDescriptorPayloadV1::new("Research", "/vault/research");
        assert!(descriptor.has_supported_schema());
        assert_eq!(descriptor.display_name, "Research");
        assert_eq!(descriptor.root_path, "/vault/research");
    }
}
