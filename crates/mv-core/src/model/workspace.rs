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
