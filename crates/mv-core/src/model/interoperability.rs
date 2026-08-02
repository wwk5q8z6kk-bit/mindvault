use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use uuid::Uuid;

use super::KnowledgeNode;

pub const EVENT_ENVELOPE_V1: &str = "mindvault.event-envelope/v1";
pub const ACTION_RECEIPT_V1: &str = "mindvault.action-receipt/v1";
pub const ACTION_ENVELOPE_V1: &str = "mindvault.action-envelope/v1";
pub const CONSUMER_APPLICATION_RECEIPT_V1: &str = "mindvault.consumer-application-receipt/v1";
pub const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
pub const EVENT_TYPE_SCHEMA_EXTENSION: &str = "x-mindvault-event-type";
const MAX_PUBLIC_SCHEMA_BYTES: usize = 1024 * 1024;
pub const KNOWLEDGE_NODE_CREATED_V1: &str = "dev.mindvault.knowledge.node.created.v1";
pub const PUBLIC_SCHEMA_REGISTERED_V1: &str = "dev.mindvault.schema.registered.v1";
pub const SOURCE_BINDING_REGISTERED_V1: &str = "dev.mindvault.source-binding.registered.v1";
pub const SOURCE_BINDING_REBOUND_V1: &str = "dev.mindvault.source-binding.rebound.v1";
pub const CONTEXT_NODE_REGISTERED_V1: &str = "dev.mindvault.context-node.registered.v1";
pub const CONTEXT_NODE_DESCRIPTOR_UPDATED_V1: &str =
    "dev.mindvault.context-node.descriptor.updated.v1";
pub const CONTEXT_NODE_LIFECYCLE_TRANSITIONED_V1: &str =
    "dev.mindvault.context-node.lifecycle.transitioned.v1";
pub const CONTEXT_CAPABILITY_MANIFEST_V1: &str = "mindvault.context-capability-manifest/v1";
pub const AUTHORITY_GRANT_ISSUED_V1: &str = "dev.mindvault.authority-grant.issued.v1";
pub const AUTHORITY_GRANT_LIFECYCLE_TRANSITIONED_V1: &str =
    "dev.mindvault.authority-grant.lifecycle.transitioned.v1";
pub const WORK_ORDER_ADMITTED_V1: &str = "dev.mindvault.work-order.admitted.v1";
pub const WORK_ORDER_LIFECYCLE_TRANSITIONED_V1: &str =
    "dev.mindvault.work-order.lifecycle.transitioned.v1";
pub const AGENT_RUN_STARTED_V1: &str = "dev.mindvault.agent-run.started.v1";
pub const AGENT_RUN_LIFECYCLE_TRANSITIONED_V1: &str =
    "dev.mindvault.agent-run.lifecycle.transitioned.v1";

/// A portable MindVault identifier.
///
/// Stable identifiers are absolute, credential-free `mindvault://` URIs.
/// Transport adapters may map them to protocol-specific identifiers, but the
/// canonical identifier does not change with a database path or API endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StableUri(String);

impl StableUri {
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.trim() != value || value.len() > 2048 {
            return Err("stable URI must be trimmed and at most 2048 bytes".into());
        }
        if value
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        {
            return Err("stable URI must not contain relative path segments".into());
        }
        if value
            .strip_prefix("mindvault://")
            .and_then(|rest| rest.split('/').next())
            .is_some_and(|authority| authority.bytes().any(|byte| byte.is_ascii_uppercase()))
        {
            return Err("stable URI authority must use lowercase canonical form".into());
        }

        let parsed = url::Url::parse(&value).map_err(|err| format!("invalid stable URI: {err}"))?;
        if parsed.scheme() != "mindvault" {
            return Err("stable URI scheme must be mindvault".into());
        }
        let _host = parsed
            .host_str()
            .ok_or_else(|| "stable URI must include an authority".to_string())?;
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err("stable URI must not contain credentials".into());
        }
        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err("stable URI must not contain a query or fragment".into());
        }
        if parsed.as_str() != value {
            return Err("stable URI must use canonical URL encoding and casing".into());
        }

        Ok(Self(value))
    }

    pub fn node(node_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/node"))
    }

    pub fn knowledge_node(node_id: Uuid, resource_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/knowledge/{resource_id}"))
    }

    pub fn principal(node_id: Uuid, principal_id: Uuid) -> Self {
        Self(format!(
            "mindvault://{node_id}/identity/principal/{principal_id}"
        ))
    }

    pub fn schema(name: &str) -> Result<Self, String> {
        if !is_token(name, 128) {
            return Err("schema name must be a non-empty portable token".into());
        }
        Self::parse(format!("mindvault://schemas/{name}"))
    }

    pub fn schema_version(schema: &StableUri, version: &str) -> Result<Self, String> {
        if !is_token(version, 64) {
            return Err("schema version must be a non-empty portable token".into());
        }
        Self::parse(format!("{schema}/version/{version}"))
    }

    pub fn source_binding(node_id: Uuid, binding_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/source-binding/{binding_id}"))
    }

    pub fn authority_grant(node_id: Uuid, grant_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/grant/{grant_id}"))
    }

    pub fn action_receipt(node_id: Uuid, receipt_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/action-receipt/{receipt_id}"))
    }

    pub fn work_order(node_id: Uuid, work_order_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/work-order/{work_order_id}"))
    }

    pub fn work_order_node(node_id: Uuid, work_order_id: Uuid, contract_id: Uuid) -> Self {
        Self(format!(
            "mindvault://{node_id}/work-order/{work_order_id}/contract/{contract_id}"
        ))
    }

    pub fn agent_run(node_id: Uuid, run_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/agent-run/{run_id}"))
    }

    pub fn run_artifact(node_id: Uuid, artifact_id: Uuid) -> Self {
        Self(format!("mindvault://{node_id}/artifact/{artifact_id}"))
    }

    pub fn consumer_application_receipt(node_id: Uuid, receipt_id: Uuid) -> Self {
        Self(format!(
            "mindvault://{node_id}/consumer-application-receipt/{receipt_id}"
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn trailing_uuid(&self) -> Option<Uuid> {
        self.0
            .rsplit('/')
            .next()
            .and_then(|value| Uuid::parse_str(value).ok())
    }

    pub fn context_node_uuid(&self) -> Option<Uuid> {
        let parsed = url::Url::parse(&self.0).ok()?;
        if parsed.path() != "/node" {
            return None;
        }
        parsed
            .host_str()
            .and_then(|value| Uuid::parse_str(value).ok())
    }

    pub fn principal_context_node_uuid(&self) -> Option<Uuid> {
        let parsed = url::Url::parse(&self.0).ok()?;
        let segments = parsed.path_segments()?.collect::<Vec<_>>();
        if segments.len() != 3
            || segments[0] != "identity"
            || segments[1] != "principal"
            || Uuid::parse_str(segments[2]).is_err()
        {
            return None;
        }
        parsed
            .host_str()
            .and_then(|value| Uuid::parse_str(value).ok())
    }
}

impl std::fmt::Display for StableUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for StableUri {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<StableUri> for String {
    fn from(value: StableUri) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 200
            || !value.bytes().all(|byte| (b'!'..=b'~').contains(&byte))
        {
            return Err(
                "idempotency key must contain 1-200 visible ASCII characters without spaces".into(),
            );
        }
        Ok(Self(value))
    }

    pub fn generated() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaReference {
    pub uri: StableUri,
    pub version: String,
}

impl SchemaReference {
    pub fn new(uri: StableUri, version: impl Into<String>) -> Result<Self, String> {
        let version = version.into();
        if !is_token(&version, 64) {
            return Err("schema version must be a non-empty portable token".into());
        }
        Ok(Self { uri, version })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Internal,
    Confidential,
    Restricted,
}

impl Sensitivity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
            Self::Restricted => "restricted",
        }
    }

    /// Ordering position, low to high. A grant admits a request only when the
    /// request's rank is at or below the grant's ceiling.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Public => 0,
            Self::Internal => 1,
            Self::Confidential => 2,
            Self::Restricted => 3,
        }
    }
}

impl FromStr for Sensitivity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public" => Ok(Self::Public),
            "internal" => Ok(Self::Internal),
            "confidential" => Ok(Self::Confidential),
            "restricted" => Ok(Self::Restricted),
            _ => Err(format!("invalid Sensitivity value: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    Ephemeral,
    Operational,
    Durable,
    LegalHold,
}

impl RetentionClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::Operational => "operational",
            Self::Durable => "durable",
            Self::LegalHold => "legal_hold",
        }
    }

    /// Ordering position, low to high. A grant admits a request only when the
    /// request's rank is at or below the grant's ceiling.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Ephemeral => 0,
            Self::Operational => 1,
            Self::Durable => 2,
            Self::LegalHold => 3,
        }
    }
}

impl FromStr for RetentionClass {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ephemeral" => Ok(Self::Ephemeral),
            "operational" => Ok(Self::Operational),
            "durable" => Ok(Self::Durable),
            "legal_hold" => Ok(Self::LegalHold),
            _ => Err(format!("invalid RetentionClass value: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceRelation {
    WasDerivedFrom,
    WasAttributedTo,
    WasGeneratedBy,
    PrimarySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceReference {
    pub resource: StableUri,
    pub relation: ProvenanceRelation,
}

/// Internal event contract. Protocol adapters may translate this to
/// CloudEvents, AsyncAPI, MCP notifications, or A2A messages without dropping
/// any security-relevant field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub envelope_version: String,
    pub id: Uuid,
    pub event_type: String,
    pub source: StableUri,
    pub subject: StableUri,
    pub occurred_at: DateTime<Utc>,
    pub schema: SchemaReference,
    pub principal: StableUri,
    pub actor: StableUri,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub idempotency_key: IdempotencyKey,
    pub payload_digest: String,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub provenance: Vec<ProvenanceReference>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct NewEventEnvelope {
    pub event_type: String,
    pub source: StableUri,
    pub subject: StableUri,
    pub schema: SchemaReference,
    pub principal: StableUri,
    pub actor: StableUri,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub idempotency_key: IdempotencyKey,
    pub payload_digest: String,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub provenance: Vec<ProvenanceReference>,
    pub data: serde_json::Value,
}

impl EventEnvelope {
    pub fn new(input: NewEventEnvelope) -> Result<Self, String> {
        let envelope = Self {
            envelope_version: EVENT_ENVELOPE_V1.into(),
            id: Uuid::now_v7(),
            event_type: input.event_type,
            source: input.source,
            subject: input.subject,
            occurred_at: Utc::now(),
            schema: input.schema,
            principal: input.principal,
            actor: input.actor,
            correlation_id: input.correlation_id,
            causation_id: input.causation_id,
            idempotency_key: input.idempotency_key,
            payload_digest: input.payload_digest,
            sensitivity: input.sensitivity,
            retention: input.retention,
            provenance: input.provenance,
            data: input.data,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.envelope_version != EVENT_ENVELOPE_V1 {
            return Err(format!(
                "unsupported event envelope version: {}",
                self.envelope_version
            ));
        }
        if !is_event_type(&self.event_type) {
            return Err("event type must be a dotted portable identifier".into());
        }
        if !is_token(&self.schema.version, 64) {
            return Err("schema version must be a non-empty portable token".into());
        }
        if self.payload_digest.len() != 64
            || !self
                .payload_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("payload digest must be 64 lowercase hexadecimal characters".into());
        }
        if self.provenance.is_empty() {
            return Err("event provenance must not be empty".into());
        }
        if !self.data.is_object() {
            return Err("event data must be a JSON object".into());
        }
        Ok(())
    }

    pub fn content_digest(&self) -> String {
        canonical_json_sha256(
            &serde_json::to_value(self)
                .expect("serializing an in-memory event envelope cannot fail"),
        )
    }
}

macro_rules! interoperability_string_enum {
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

// Shared with sibling model modules that define governed string enums with the
// same serialization contract.
pub(crate) use interoperability_string_enum;

interoperability_string_enum! {
    /// Publication state of an immutable public schema version.
    pub enum PublicSchemaLifecycle {
        Active => "active",
        Deprecated => "deprecated",
        Withdrawn => "withdrawn",
    }
}

/// Immutable, content-addressed definition for one public schema version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublicSchemaRecord {
    pub schema: SchemaReference,
    pub media_type: String,
    pub definition: serde_json::Value,
    pub content_digest: String,
    pub lifecycle: PublicSchemaLifecycle,
    pub owner: StableUri,
    pub created_at: DateTime<Utc>,
    pub deprecated_at: Option<DateTime<Utc>>,
}

impl PublicSchemaRecord {
    pub fn new(
        schema: SchemaReference,
        definition: serde_json::Value,
        owner: StableUri,
    ) -> Result<Self, String> {
        let record = Self {
            schema,
            media_type: "application/schema+json".into(),
            content_digest: canonical_json_sha256(&definition),
            definition,
            lifecycle: PublicSchemaLifecycle::Active,
            owner,
            created_at: Utc::now(),
            deprecated_at: None,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !is_token(&self.schema.version, 64) {
            return Err("schema version must be a non-empty portable token".into());
        }
        if self.media_type != "application/schema+json" {
            return Err("public schemas must use application/schema+json".into());
        }
        if !self.definition.is_object() {
            return Err("public schema definition must be a JSON object".into());
        }
        if self
            .definition
            .get("$schema")
            .and_then(|value| value.as_str())
            != Some(JSON_SCHEMA_DRAFT_2020_12)
        {
            return Err("public schemas must declare JSON Schema draft 2020-12".into());
        }
        if serde_json::to_vec(&self.definition)
            .map_err(|err| format!("serialize public schema definition: {err}"))?
            .len()
            > MAX_PUBLIC_SCHEMA_BYTES
        {
            return Err("public schema definition must not exceed 1 MiB".into());
        }
        if let Some(event_type) = self
            .definition
            .get(EVENT_TYPE_SCHEMA_EXTENSION)
            .and_then(|value| value.as_str())
        {
            if !is_event_type(event_type) {
                return Err(format!(
                    "{EVENT_TYPE_SCHEMA_EXTENSION} must be a dotted portable event type"
                ));
            }
        } else if self.definition.get(EVENT_TYPE_SCHEMA_EXTENSION).is_some() {
            return Err(format!(
                "{EVENT_TYPE_SCHEMA_EXTENSION} must be a string when present"
            ));
        }
        validate_sha256(&self.content_digest, "schema content digest")?;
        if canonical_json_sha256(&self.definition) != self.content_digest {
            return Err("schema content digest does not match its canonical definition".into());
        }
        match (self.lifecycle, self.deprecated_at) {
            (PublicSchemaLifecycle::Active, Some(_)) => {
                return Err("an active public schema cannot have a deprecation time".into())
            }
            (PublicSchemaLifecycle::Deprecated | PublicSchemaLifecycle::Withdrawn, None) => {
                return Err("a non-active public schema requires a deprecation time".into())
            }
            _ => {}
        }
        Ok(())
    }
}

interoperability_string_enum! {
    pub enum SyncDirection {
        None => "none",
        Inbound => "inbound",
        Outbound => "outbound",
        Bidirectional => "bidirectional",
    }
}

interoperability_string_enum! {
    pub enum MaterializationMode {
        ReferenceOnly => "reference_only",
        MetadataMirror => "metadata_mirror",
        SearchProjection => "search_projection",
        CachedExcerpt => "cached_excerpt",
        FullReplica => "full_replica",
        CanonicalImport => "canonical_import",
        DerivedKnowledge => "derived_knowledge",
    }
}

interoperability_string_enum! {
    pub enum SourceFreshness {
        Fresh => "fresh",
        Stale => "stale",
        Unavailable => "unavailable",
        Conflicted => "conflicted",
        Unknown => "unknown",
    }
}

interoperability_string_enum! {
    /// Explicit authority rule; last-write-wins is intentionally absent.
    pub enum SourceConflictPolicy {
        RequireReview => "require_review",
        SourceAuthority => "source_authority",
        MindVaultAuthority => "mindvault_authority",
        FieldAuthority => "field_authority",
    }
}

interoperability_string_enum! {
    pub enum SourceDeletionPolicy {
        Tombstone => "tombstone",
        PurgeMaterialization => "purge_materialization",
        RetainLegalHold => "retain_legal_hold",
        RequireReview => "require_review",
    }
}

interoperability_string_enum! {
    pub enum SourceBindingStatus {
        Active => "active",
        Suspended => "suspended",
        Revoked => "revoked",
        Migrated => "migrated",
    }
}

interoperability_string_enum! {
    /// Portable role of an independently addressable context participant.
    pub enum ContextNodeType {
        Personal => "personal",
        Device => "device",
        Space => "space",
        Organization => "organization",
        Application => "application",
        Agent => "agent",
        Storage => "storage",
        Execution => "execution",
        Relay => "relay",
        Index => "index",
    }
}

interoperability_string_enum! {
    /// Advertised operation class. Advertisement never grants authorization.
    pub enum ContextCapability {
        Discover => "discover",
        Query => "query",
        Read => "read",
        Subscribe => "subscribe",
        Propose => "propose",
        Command => "command",
        Execute => "execute",
        Notify => "notify",
        Synchronize => "synchronize",
        Export => "export",
        Import => "import",
        Health => "health",
        Revoke => "revoke",
    }
}

interoperability_string_enum! {
    pub enum ContextNodeStatus {
        Discovered => "discovered",
        PendingTrust => "pending_trust",
        Active => "active",
        Suspended => "suspended",
        Revoked => "revoked",
        Retired => "retired",
    }
}

interoperability_string_enum! {
    /// Durable state of one outbox event's publication boundary.
    pub enum OutboxDeliveryState {
        Pending => "pending",
        Published => "published",
        DeadLetter => "dead_letter",
    }
}

interoperability_string_enum! {
    /// Result recorded for one claimed publication attempt.
    pub enum ActionReceiptOutcome {
        Published => "published",
        RetryScheduled => "retry_scheduled",
        DeadLettered => "dead_lettered",
    }
}

/// Exclusive, time-bounded right for one dispatcher to publish one event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboxDeliveryClaim {
    pub event: EventEnvelope,
    pub lease_id: Uuid,
    pub attempt: u32,
    pub executor: StableUri,
    pub destination: StableUri,
    pub claimed_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
}

impl OutboxDeliveryClaim {
    pub fn validate(&self) -> Result<(), String> {
        self.event.validate()?;
        if self.attempt == 0 {
            return Err("outbox delivery attempt must be at least one".into());
        }
        if self.lease_expires_at <= self.claimed_at {
            return Err("outbox delivery lease must expire after it is claimed".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxDeliveryStatus {
    pub event_id: Uuid,
    pub state: OutboxDeliveryState,
    pub attempts: u32,
    pub next_attempt_at: DateTime<Utc>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Terminal or retry result supplied by a dispatcher after one claimed
/// publication attempt. Provider responses remain opaque edge data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum OutboxDeliveryResult {
    Published {
        delivery_reference: String,
        response_digest: Option<String>,
    },
    RetryScheduled {
        retry_at: DateTime<Utc>,
        error_code: String,
        error_summary: String,
    },
    DeadLettered {
        error_code: String,
        error_summary: String,
    },
}

impl OutboxDeliveryResult {
    pub const fn outcome(&self) -> ActionReceiptOutcome {
        match self {
            Self::Published { .. } => ActionReceiptOutcome::Published,
            Self::RetryScheduled { .. } => ActionReceiptOutcome::RetryScheduled,
            Self::DeadLettered { .. } => ActionReceiptOutcome::DeadLettered,
        }
    }

    fn validate(&self, completed_at: DateTime<Utc>) -> Result<(), String> {
        match self {
            Self::Published {
                delivery_reference,
                response_digest,
            } => {
                validate_opaque_identifier(delivery_reference, "delivery reference", 2048)?;
                if let Some(digest) = response_digest {
                    validate_sha256(digest, "delivery response digest")?;
                }
            }
            Self::RetryScheduled {
                retry_at,
                error_code,
                error_summary,
            } => {
                if *retry_at <= completed_at {
                    return Err("outbox retry time must follow attempt completion".into());
                }
                validate_delivery_error(error_code, error_summary)?;
            }
            Self::DeadLettered {
                error_code,
                error_summary,
            } => validate_delivery_error(error_code, error_summary)?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboxDeliveryCompletion {
    pub event_id: Uuid,
    pub lease_id: Uuid,
    pub attempt: u32,
    pub completed_at: DateTime<Utc>,
    pub result: OutboxDeliveryResult,
}

impl OutboxDeliveryCompletion {
    pub fn validate_for(&self, claim: &OutboxDeliveryClaim) -> Result<(), String> {
        claim.validate()?;
        if self.event_id != claim.event.id
            || self.lease_id != claim.lease_id
            || self.attempt != claim.attempt
        {
            return Err("outbox completion does not match its delivery claim".into());
        }
        if self.completed_at < claim.claimed_at {
            return Err("outbox completion cannot precede its claim".into());
        }
        if self.completed_at >= claim.lease_expires_at {
            return Err("outbox completion cannot use an expired lease".into());
        }
        self.result.validate(self.completed_at)
    }
}

/// Immutable evidence for one durable publication attempt.
///
/// A published receipt proves acknowledgement by the declared destination. It
/// does not prove application by every downstream consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReceipt {
    pub receipt_version: String,
    pub receipt_id: Uuid,
    pub event_id: Uuid,
    pub claim_id: Uuid,
    pub attempt: u32,
    pub outcome: ActionReceiptOutcome,
    pub executor: StableUri,
    pub destination: StableUri,
    pub subject: StableUri,
    pub principal: StableUri,
    pub actor: StableUri,
    pub correlation_id: Uuid,
    pub request_digest: String,
    pub response_digest: Option<String>,
    pub delivery_reference: Option<String>,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub provenance: Vec<ProvenanceReference>,
}

impl ActionReceipt {
    pub fn from_outbox_delivery(
        claim: &OutboxDeliveryClaim,
        completion: &OutboxDeliveryCompletion,
    ) -> Result<Self, String> {
        completion.validate_for(claim)?;
        let (response_digest, delivery_reference, error_code, error_summary) =
            match &completion.result {
                OutboxDeliveryResult::Published {
                    delivery_reference,
                    response_digest,
                } => (
                    response_digest.clone(),
                    Some(delivery_reference.clone()),
                    None,
                    None,
                ),
                OutboxDeliveryResult::RetryScheduled {
                    error_code,
                    error_summary,
                    ..
                }
                | OutboxDeliveryResult::DeadLettered {
                    error_code,
                    error_summary,
                } => (
                    None,
                    None,
                    Some(error_code.clone()),
                    Some(error_summary.clone()),
                ),
            };
        let receipt = Self {
            receipt_version: ACTION_RECEIPT_V1.into(),
            receipt_id: Uuid::now_v7(),
            event_id: claim.event.id,
            claim_id: claim.lease_id,
            attempt: claim.attempt,
            outcome: completion.result.outcome(),
            executor: claim.executor.clone(),
            destination: claim.destination.clone(),
            subject: claim.event.subject.clone(),
            principal: claim.event.principal.clone(),
            actor: claim.event.actor.clone(),
            correlation_id: claim.event.correlation_id,
            request_digest: claim.event.content_digest(),
            response_digest,
            delivery_reference,
            error_code,
            error_summary,
            started_at: claim.claimed_at,
            completed_at: completion.completed_at,
            sensitivity: claim.event.sensitivity,
            retention: claim.event.retention,
            provenance: claim.event.provenance.clone(),
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.receipt_version != ACTION_RECEIPT_V1 {
            return Err(format!(
                "unsupported action receipt version: {}",
                self.receipt_version
            ));
        }
        if self.receipt_id.is_nil()
            || self.event_id.is_nil()
            || self.claim_id.is_nil()
            || self.correlation_id.is_nil()
        {
            return Err("action receipt identifiers must not be nil".into());
        }
        if self.attempt == 0 {
            return Err("action receipt attempt must be at least one".into());
        }
        validate_sha256(&self.request_digest, "action request digest")?;
        if let Some(digest) = &self.response_digest {
            validate_sha256(digest, "action response digest")?;
        }
        if self.completed_at < self.started_at {
            return Err("action receipt completion cannot precede its start".into());
        }
        if self.provenance.is_empty() {
            return Err("action receipt provenance must not be empty".into());
        }
        match self.outcome {
            ActionReceiptOutcome::Published => {
                let reference = self.delivery_reference.as_deref().ok_or_else(|| {
                    "published action receipt requires a delivery reference".to_string()
                })?;
                validate_opaque_identifier(reference, "delivery reference", 2048)?;
                if self.error_code.is_some() || self.error_summary.is_some() {
                    return Err("published action receipt cannot contain an error".into());
                }
            }
            ActionReceiptOutcome::RetryScheduled | ActionReceiptOutcome::DeadLettered => {
                if self.delivery_reference.is_some() || self.response_digest.is_some() {
                    return Err("failed action receipt cannot contain delivery evidence".into());
                }
                validate_delivery_error(
                    self.error_code.as_deref().ok_or_else(|| {
                        "failed action receipt requires an error code".to_string()
                    })?,
                    self.error_summary.as_deref().ok_or_else(|| {
                        "failed action receipt requires an error summary".to_string()
                    })?,
                )?;
            }
        }
        Ok(())
    }

    pub fn matches_delivery(
        &self,
        claim: &OutboxDeliveryClaim,
        completion: &OutboxDeliveryCompletion,
    ) -> bool {
        ActionReceipt::from_outbox_delivery(claim, completion)
            .map(|expected| {
                self.receipt_version == expected.receipt_version
                    && self.event_id == expected.event_id
                    && self.claim_id == expected.claim_id
                    && self.attempt == expected.attempt
                    && self.outcome == expected.outcome
                    && self.executor == expected.executor
                    && self.destination == expected.destination
                    && self.subject == expected.subject
                    && self.principal == expected.principal
                    && self.actor == expected.actor
                    && self.correlation_id == expected.correlation_id
                    && self.request_digest == expected.request_digest
                    && self.response_digest == expected.response_digest
                    && self.delivery_reference == expected.delivery_reference
                    && self.error_code == expected.error_code
                    && self.error_summary == expected.error_summary
                    && self.started_at == expected.started_at
                    && self.completed_at == expected.completed_at
                    && self.sensitivity == expected.sensitivity
                    && self.retention == expected.retention
                    && self.provenance == expected.provenance
            })
            .unwrap_or(false)
    }
}

interoperability_string_enum! {
    /// Durable local disposition of one admitted consumer event.
    pub enum ConsumerInboxState {
        Pending => "pending",
        Applied => "applied",
        DeadLetter => "dead_letter",
    }
}

interoperability_string_enum! {
    /// Result recorded for one claimed consumer application attempt.
    pub enum ConsumerApplicationOutcome {
        Applied => "applied",
        RetryScheduled => "retry_scheduled",
        DeadLettered => "dead_lettered",
    }
}

/// Result of admitting an event to one consumer's durable inbox.
///
/// `replayed` means the same consumer, event ID, and complete envelope digest
/// were already admitted. It does not mean the event was applied again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumerInboxAdmission {
    pub inbox_sequence: u64,
    pub consumer: StableUri,
    pub event: EventEnvelope,
    pub received_at: DateTime<Utc>,
    pub replayed: bool,
}

impl ConsumerInboxAdmission {
    pub fn validate(&self) -> Result<(), String> {
        if self.inbox_sequence == 0 {
            return Err("consumer inbox sequence must be at least one".into());
        }
        self.event.validate()
    }
}

/// Exclusive, time-bounded right to apply one consumer inbox event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumerInboxClaim {
    pub inbox_sequence: u64,
    pub consumer: StableUri,
    pub event: EventEnvelope,
    pub lease_id: Uuid,
    pub attempt: u32,
    pub processor: StableUri,
    pub claimed_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
}

impl ConsumerInboxClaim {
    pub fn validate(&self) -> Result<(), String> {
        if self.inbox_sequence == 0 {
            return Err("consumer inbox sequence must be at least one".into());
        }
        self.event.validate()?;
        if self.attempt == 0 {
            return Err("consumer application attempt must be at least one".into());
        }
        if self.lease_expires_at <= self.claimed_at {
            return Err("consumer application lease must expire after it is claimed".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerInboxStatus {
    pub inbox_sequence: u64,
    pub consumer: StableUri,
    pub event_id: Uuid,
    pub source: StableUri,
    pub state: ConsumerInboxState,
    pub attempts: u32,
    pub next_attempt_at: DateTime<Utc>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub applied_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub received_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Durable local stream checkpoint.
///
/// The sequence is assigned at local admission time. It proves local
/// disposition order only; it cannot detect gaps in a remote issuer stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerCheckpoint {
    pub consumer: StableUri,
    pub source: StableUri,
    pub last_dispositioned_sequence: u64,
    pub last_dispositioned_event_id: Uuid,
    pub last_applied_sequence: Option<u64>,
    pub last_applied_event_id: Option<Uuid>,
    pub applied_count: u64,
    pub dead_letter_count: u64,
    pub updated_at: DateTime<Utc>,
}

impl ConsumerCheckpoint {
    pub fn validate(&self) -> Result<(), String> {
        if self.last_dispositioned_sequence == 0 {
            return Err("consumer checkpoint sequence must be at least one".into());
        }
        match (
            self.last_applied_sequence,
            self.last_applied_event_id,
            self.applied_count,
        ) {
            (None, None, 0) => {}
            (Some(sequence), Some(_), count)
                if count > 0 && sequence <= self.last_dispositioned_sequence => {}
            _ => {
                return Err(
                    "consumer checkpoint application fields must form a consistent pair".into(),
                )
            }
        }
        if self
            .applied_count
            .checked_add(self.dead_letter_count)
            .is_none_or(|count| count == 0)
        {
            return Err("consumer checkpoint must record a terminal disposition".into());
        }
        Ok(())
    }
}

/// Result supplied by a processor after one claimed application attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ConsumerApplicationResult {
    Applied {
        application_reference: String,
        effect_digest: Option<String>,
    },
    RetryScheduled {
        retry_at: DateTime<Utc>,
        error_code: String,
        error_summary: String,
    },
    DeadLettered {
        error_code: String,
        error_summary: String,
    },
}

impl ConsumerApplicationResult {
    pub const fn outcome(&self) -> ConsumerApplicationOutcome {
        match self {
            Self::Applied { .. } => ConsumerApplicationOutcome::Applied,
            Self::RetryScheduled { .. } => ConsumerApplicationOutcome::RetryScheduled,
            Self::DeadLettered { .. } => ConsumerApplicationOutcome::DeadLettered,
        }
    }

    fn validate(&self, completed_at: DateTime<Utc>) -> Result<(), String> {
        match self {
            Self::Applied {
                application_reference,
                effect_digest,
            } => {
                validate_opaque_identifier(
                    application_reference,
                    "consumer application reference",
                    2048,
                )?;
                if let Some(digest) = effect_digest {
                    validate_sha256(digest, "consumer application effect digest")?;
                }
            }
            Self::RetryScheduled {
                retry_at,
                error_code,
                error_summary,
            } => {
                if *retry_at <= completed_at {
                    return Err(
                        "consumer application retry time must follow attempt completion".into(),
                    );
                }
                validate_delivery_error(error_code, error_summary)?;
            }
            Self::DeadLettered {
                error_code,
                error_summary,
            } => validate_delivery_error(error_code, error_summary)?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumerApplicationCompletion {
    pub inbox_sequence: u64,
    pub event_id: Uuid,
    pub lease_id: Uuid,
    pub attempt: u32,
    pub completed_at: DateTime<Utc>,
    pub result: ConsumerApplicationResult,
}

impl ConsumerApplicationCompletion {
    pub fn validate_for(&self, claim: &ConsumerInboxClaim) -> Result<(), String> {
        claim.validate()?;
        if self.inbox_sequence != claim.inbox_sequence
            || self.event_id != claim.event.id
            || self.lease_id != claim.lease_id
            || self.attempt != claim.attempt
        {
            return Err("consumer completion does not match its inbox claim".into());
        }
        if self.completed_at < claim.claimed_at {
            return Err("consumer completion cannot precede its claim".into());
        }
        if self.completed_at >= claim.lease_expires_at {
            return Err("consumer completion cannot use an expired lease".into());
        }
        self.result.validate(self.completed_at)
    }
}

/// Immutable evidence for one consumer application attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumerApplicationReceipt {
    pub receipt_version: String,
    pub receipt_id: Uuid,
    pub inbox_sequence: u64,
    pub event_id: Uuid,
    pub claim_id: Uuid,
    pub attempt: u32,
    pub outcome: ConsumerApplicationOutcome,
    pub consumer: StableUri,
    pub processor: StableUri,
    pub source: StableUri,
    pub subject: StableUri,
    pub principal: StableUri,
    pub actor: StableUri,
    pub correlation_id: Uuid,
    pub request_digest: String,
    pub effect_digest: Option<String>,
    pub application_reference: Option<String>,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub provenance: Vec<ProvenanceReference>,
}

impl ConsumerApplicationReceipt {
    pub fn from_application(
        claim: &ConsumerInboxClaim,
        completion: &ConsumerApplicationCompletion,
    ) -> Result<Self, String> {
        completion.validate_for(claim)?;
        let (effect_digest, application_reference, error_code, error_summary) =
            match &completion.result {
                ConsumerApplicationResult::Applied {
                    application_reference,
                    effect_digest,
                } => (
                    effect_digest.clone(),
                    Some(application_reference.clone()),
                    None,
                    None,
                ),
                ConsumerApplicationResult::RetryScheduled {
                    error_code,
                    error_summary,
                    ..
                }
                | ConsumerApplicationResult::DeadLettered {
                    error_code,
                    error_summary,
                } => (
                    None,
                    None,
                    Some(error_code.clone()),
                    Some(error_summary.clone()),
                ),
            };
        let receipt = Self {
            receipt_version: CONSUMER_APPLICATION_RECEIPT_V1.into(),
            receipt_id: Uuid::now_v7(),
            inbox_sequence: claim.inbox_sequence,
            event_id: claim.event.id,
            claim_id: claim.lease_id,
            attempt: claim.attempt,
            outcome: completion.result.outcome(),
            consumer: claim.consumer.clone(),
            processor: claim.processor.clone(),
            source: claim.event.source.clone(),
            subject: claim.event.subject.clone(),
            principal: claim.event.principal.clone(),
            actor: claim.event.actor.clone(),
            correlation_id: claim.event.correlation_id,
            request_digest: claim.event.content_digest(),
            effect_digest,
            application_reference,
            error_code,
            error_summary,
            started_at: claim.claimed_at,
            completed_at: completion.completed_at,
            sensitivity: claim.event.sensitivity,
            retention: claim.event.retention,
            provenance: claim.event.provenance.clone(),
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.receipt_version != CONSUMER_APPLICATION_RECEIPT_V1 {
            return Err(format!(
                "unsupported consumer application receipt version: {}",
                self.receipt_version
            ));
        }
        if self.inbox_sequence == 0 || self.attempt == 0 {
            return Err(
                "consumer application receipt sequence and attempt must be positive".into(),
            );
        }
        if self.receipt_id.is_nil()
            || self.event_id.is_nil()
            || self.claim_id.is_nil()
            || self.correlation_id.is_nil()
        {
            return Err("consumer application receipt identifiers must not be nil".into());
        }
        validate_sha256(&self.request_digest, "consumer application request digest")?;
        if let Some(digest) = &self.effect_digest {
            validate_sha256(digest, "consumer application effect digest")?;
        }
        if self.completed_at < self.started_at {
            return Err("consumer application receipt completion cannot precede its start".into());
        }
        if self.provenance.is_empty() {
            return Err("consumer application receipt provenance must not be empty".into());
        }
        match self.outcome {
            ConsumerApplicationOutcome::Applied => {
                let reference = self.application_reference.as_deref().ok_or_else(|| {
                    "applied consumer receipt requires an application reference".to_string()
                })?;
                validate_opaque_identifier(reference, "consumer application reference", 2048)?;
                if self.error_code.is_some() || self.error_summary.is_some() {
                    return Err("applied consumer receipt cannot contain an error".into());
                }
            }
            ConsumerApplicationOutcome::RetryScheduled
            | ConsumerApplicationOutcome::DeadLettered => {
                if self.application_reference.is_some() || self.effect_digest.is_some() {
                    return Err(
                        "failed consumer receipt cannot contain application evidence".into(),
                    );
                }
                validate_delivery_error(
                    self.error_code.as_deref().ok_or_else(|| {
                        "failed consumer receipt requires an error code".to_string()
                    })?,
                    self.error_summary.as_deref().ok_or_else(|| {
                        "failed consumer receipt requires an error summary".to_string()
                    })?,
                )?;
            }
        }
        Ok(())
    }

    pub fn matches_application(
        &self,
        claim: &ConsumerInboxClaim,
        completion: &ConsumerApplicationCompletion,
    ) -> bool {
        Self::from_application(claim, completion)
            .map(|expected| {
                self.receipt_version == expected.receipt_version
                    && self.inbox_sequence == expected.inbox_sequence
                    && self.event_id == expected.event_id
                    && self.claim_id == expected.claim_id
                    && self.attempt == expected.attempt
                    && self.outcome == expected.outcome
                    && self.consumer == expected.consumer
                    && self.processor == expected.processor
                    && self.source == expected.source
                    && self.subject == expected.subject
                    && self.principal == expected.principal
                    && self.actor == expected.actor
                    && self.correlation_id == expected.correlation_id
                    && self.request_digest == expected.request_digest
                    && self.effect_digest == expected.effect_digest
                    && self.application_reference == expected.application_reference
                    && self.error_code == expected.error_code
                    && self.error_summary == expected.error_summary
                    && self.started_at == expected.started_at
                    && self.completed_at == expected.completed_at
                    && self.sensitivity == expected.sensitivity
                    && self.retention == expected.retention
                    && self.provenance == expected.provenance
            })
            .unwrap_or(false)
    }
}

impl ContextNodeStatus {
    pub fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Discovered, Self::PendingTrust)
                | (Self::PendingTrust, Self::Active)
                | (Self::PendingTrust, Self::Revoked)
                | (Self::Active, Self::Suspended)
                | (Self::Suspended, Self::Active)
                | (Self::Suspended, Self::Revoked)
                | (Self::Revoked, Self::Retired)
        )
    }
}

/// Policy-defined trust label. It informs evaluation but never grants access.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContextNodeTrustClass(String);

impl ContextNodeTrustClass {
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if !is_token(&value, 64) {
            return Err("context-node trust class must be a non-empty portable token".into());
        }
        Ok(Self(value))
    }

    pub fn untrusted() -> Self {
        Self("untrusted".into())
    }

    pub fn local() -> Self {
        Self("local".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ContextNodeTrustClass {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<ContextNodeTrustClass> for String {
    fn from(value: ContextNodeTrustClass) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextProtocolProfile {
    pub protocol: String,
    pub version: String,
    pub roles: Vec<String>,
}

impl ContextProtocolProfile {
    pub fn validate(&self) -> Result<(), String> {
        if !is_token(&self.protocol, 64) || !is_token(&self.version, 64) {
            return Err("protocol name and version must be portable tokens".into());
        }
        if self.roles.is_empty()
            || self.roles.len() > 16
            || self.roles.iter().any(|role| !is_token(role, 64))
            || !is_strictly_sorted_unique(self.roles.iter().map(String::as_str))
        {
            return Err("protocol roles must contain 1-16 sorted, unique portable tokens".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextNodePublicKey {
    pub key_id: String,
    pub algorithm: String,
    pub public_key_multibase: String,
}

impl ContextNodePublicKey {
    pub fn validate(&self) -> Result<(), String> {
        if !is_token(&self.key_id, 128) || !is_token(&self.algorithm, 64) {
            return Err("public key id and algorithm must be portable tokens".into());
        }
        if self.public_key_multibase.is_empty()
            || self.public_key_multibase.len() > 16 * 1024
            || !self
                .public_key_multibase
                .bytes()
                .all(|byte| (b'!'..=b'~').contains(&byte))
        {
            return Err(
                "public key material must be 1-16384 visible ASCII multibase characters".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextNodeEndpoint {
    pub protocol: String,
    pub uri: String,
}

impl ContextNodeEndpoint {
    pub fn validate(&self) -> Result<(), String> {
        if !is_token(&self.protocol, 64) || self.uri.len() > 2048 {
            return Err("endpoint protocol and URI must be bounded".into());
        }
        let endpoint =
            url::Url::parse(&self.uri).map_err(|err| format!("invalid endpoint URI: {err}"))?;
        if endpoint.scheme().is_empty()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(
                "endpoint URI must be absolute and contain no credentials, query, or fragment"
                    .into(),
            );
        }
        Ok(())
    }
}

/// Versioned capability advertisement for a Context Node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCapabilityManifest {
    pub manifest_version: String,
    pub revision: u64,
    pub capabilities: Vec<ContextCapability>,
    pub supported_protocols: Vec<ContextProtocolProfile>,
    pub supported_schema_versions: Vec<SchemaReference>,
    pub content_digest: String,
}

impl ContextCapabilityManifest {
    pub fn new(
        capabilities: Vec<ContextCapability>,
        supported_protocols: Vec<ContextProtocolProfile>,
        supported_schema_versions: Vec<SchemaReference>,
    ) -> Result<Self, String> {
        let mut manifest = Self {
            manifest_version: CONTEXT_CAPABILITY_MANIFEST_V1.into(),
            revision: 1,
            capabilities,
            supported_protocols,
            supported_schema_versions,
            content_digest: String::new(),
        };
        manifest.content_digest = manifest.expected_digest();
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn expected_digest(&self) -> String {
        canonical_json_sha256(&serde_json::json!({
            "capabilities": self.capabilities,
            "manifest_version": self.manifest_version,
            "revision": self.revision,
            "supported_protocols": self.supported_protocols,
            "supported_schema_versions": self.supported_schema_versions,
        }))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_version != CONTEXT_CAPABILITY_MANIFEST_V1 {
            return Err(format!(
                "unsupported context capability manifest version: {}",
                self.manifest_version
            ));
        }
        if self.revision == 0 {
            return Err("context capability manifest revision must be at least one".into());
        }
        if self.capabilities.len() > 64
            || !is_strictly_sorted_unique(self.capabilities.iter().map(|value| value.as_str()))
        {
            return Err("capabilities must be sorted, unique, and bounded to 64 entries".into());
        }
        if self.supported_protocols.len() > 64 {
            return Err("supported protocols must be bounded to 64 entries".into());
        }
        for protocol in &self.supported_protocols {
            protocol.validate()?;
        }
        if !is_strictly_sorted_unique(
            self.supported_protocols
                .iter()
                .map(|value| format!("{}\0{}", value.protocol, value.version)),
        ) {
            return Err("supported protocols must be sorted and unique by name and version".into());
        }
        if self.supported_schema_versions.len() > 256
            || !is_strictly_sorted_unique(
                self.supported_schema_versions
                    .iter()
                    .map(|value| format!("{}\0{}", value.uri, value.version)),
            )
        {
            return Err(
                "supported schema versions must be sorted, unique, and bounded to 256 entries"
                    .into(),
            );
        }
        validate_sha256(&self.content_digest, "capability manifest content digest")?;
        if self.expected_digest() != self.content_digest {
            return Err("capability manifest digest does not match its canonical content".into());
        }
        Ok(())
    }
}

/// Governed descriptor for one independently addressable context participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextNodeRecord {
    pub node_id: Uuid,
    pub revision: u64,
    pub node_uri: StableUri,
    pub node_type: ContextNodeType,
    pub owner_actor_id: StableUri,
    pub governing_node_id: StableUri,
    pub display_name: String,
    pub capability_manifest: ContextCapabilityManifest,
    pub trust_class: ContextNodeTrustClass,
    pub public_keys: Vec<ContextNodePublicKey>,
    pub endpoints: Vec<ContextNodeEndpoint>,
    pub data_residency: Vec<String>,
    pub status: ContextNodeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ContextNodeRecord {
    pub fn discovered(
        node_id: Uuid,
        node_type: ContextNodeType,
        owner_actor_id: StableUri,
        governing_node_id: StableUri,
        display_name: impl Into<String>,
        capability_manifest: ContextCapabilityManifest,
    ) -> Result<Self, String> {
        let now = Utc::now();
        let record = Self {
            node_id,
            revision: 1,
            node_uri: StableUri::node(node_id),
            node_type,
            owner_actor_id,
            governing_node_id,
            display_name: display_name.into(),
            capability_manifest,
            trust_class: ContextNodeTrustClass::untrusted(),
            public_keys: Vec::new(),
            endpoints: Vec::new(),
            data_residency: Vec::new(),
            status: ContextNodeStatus::Discovered,
            created_at: now,
            updated_at: now,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn semantic_digest(&self) -> String {
        canonical_json_sha256(
            &serde_json::to_value(self)
                .expect("serializing an in-memory ContextNodeRecord cannot fail"),
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.revision == 0 {
            return Err("context-node revision must be at least one".into());
        }
        if self.node_uri != StableUri::node(self.node_id) {
            return Err("context-node URI must be derived from its stable node id".into());
        }
        if self.governing_node_id.context_node_uuid().is_none() {
            return Err("governing node id must be a canonical Context Node URI".into());
        }
        validate_display_text(&self.display_name, "context-node display name", 256)?;
        self.capability_manifest.validate()?;
        if self.public_keys.len() > 32 {
            return Err("context-node public keys must be bounded to 32 entries".into());
        }
        for key in &self.public_keys {
            key.validate()?;
        }
        if !is_strictly_sorted_unique(self.public_keys.iter().map(|key| key.key_id.as_str())) {
            return Err("context-node public keys must be sorted and unique by key id".into());
        }
        if self.endpoints.len() > 32 {
            return Err("context-node endpoints must be bounded to 32 entries".into());
        }
        for endpoint in &self.endpoints {
            endpoint.validate()?;
            if !self
                .capability_manifest
                .supported_protocols
                .iter()
                .any(|profile| profile.protocol == endpoint.protocol)
            {
                return Err("every endpoint protocol must be advertised by the manifest".into());
            }
        }
        if !is_strictly_sorted_unique(
            self.endpoints
                .iter()
                .map(|endpoint| format!("{}\0{}", endpoint.protocol, endpoint.uri)),
        ) {
            return Err("context-node endpoints must be sorted and unique".into());
        }
        if self.data_residency.len() > 32
            || self
                .data_residency
                .iter()
                .any(|residency| !is_token(residency, 64))
            || !is_strictly_sorted_unique(self.data_residency.iter().map(String::as_str))
        {
            return Err(
                "data residency must contain at most 32 sorted, unique portable tokens".into(),
            );
        }
        if self.updated_at < self.created_at {
            return Err("context-node update time cannot precede creation".into());
        }
        Ok(())
    }
}

interoperability_string_enum! {
    /// Context grants authorize observation; tool grants authorize effects.
    pub enum AuthorityGrantKind {
        Context => "context",
        Tool => "tool",
    }
}

interoperability_string_enum! {
    pub enum AuthorityGrantStatus {
        Active => "active",
        Suspended => "suspended",
        Revoked => "revoked",
        Expired => "expired",
    }
}

impl AuthorityGrantStatus {
    pub fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Active, Self::Suspended)
                | (Self::Active, Self::Revoked)
                | (Self::Active, Self::Expired)
                | (Self::Suspended, Self::Active)
                | (Self::Suspended, Self::Revoked)
                | (Self::Suspended, Self::Expired)
        )
    }
}

/// A purpose-bound, time-bounded authorization issued by one governing node.
///
/// Context and Tool Grants deliberately share lifecycle and delegation
/// machinery but use disjoint capability sets. Possessing one kind never
/// implies the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityGrant {
    pub grant_id: Uuid,
    pub revision: u64,
    pub grant_uri: StableUri,
    pub kind: AuthorityGrantKind,
    pub grantor: StableUri,
    pub grantee: StableUri,
    pub governing_node: StableUri,
    pub targets: Vec<StableUri>,
    pub capabilities: Vec<ContextCapability>,
    pub sensitivity_ceiling: Sensitivity,
    pub retention_ceiling: RetentionClass,
    pub allow_redistribution: bool,
    pub allow_model_training: bool,
    pub purpose: String,
    pub parent_grant_id: Option<Uuid>,
    pub delegation_depth_remaining: u8,
    pub not_before: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: AuthorityGrantStatus,
    pub status_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AuthorityGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new_context(
        governing_node: StableUri,
        grantor: StableUri,
        grantee: StableUri,
        mut targets: Vec<StableUri>,
        mut capabilities: Vec<ContextCapability>,
        purpose: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        targets.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        capabilities.sort_by_key(|capability| capability.as_str());
        Self::new(
            AuthorityGrantKind::Context,
            governing_node,
            grantor,
            grantee,
            targets,
            capabilities,
            purpose,
            expires_at,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_tool(
        governing_node: StableUri,
        grantor: StableUri,
        grantee: StableUri,
        mut targets: Vec<StableUri>,
        mut capabilities: Vec<ContextCapability>,
        purpose: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        targets.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        capabilities.sort_by_key(|capability| capability.as_str());
        Self::new(
            AuthorityGrantKind::Tool,
            governing_node,
            grantor,
            grantee,
            targets,
            capabilities,
            purpose,
            expires_at,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: AuthorityGrantKind,
        governing_node: StableUri,
        grantor: StableUri,
        grantee: StableUri,
        targets: Vec<StableUri>,
        capabilities: Vec<ContextCapability>,
        purpose: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        let grant_id = Uuid::now_v7();
        let node_id = governing_node
            .context_node_uuid()
            .ok_or_else(|| "governing node must be a canonical Context Node URI".to_string())?;
        let now = Utc::now();
        let grant = Self {
            grant_id,
            revision: 1,
            grant_uri: StableUri::authority_grant(node_id, grant_id),
            kind,
            grantor,
            grantee,
            governing_node,
            targets,
            capabilities,
            sensitivity_ceiling: Sensitivity::Internal,
            retention_ceiling: RetentionClass::Operational,
            allow_redistribution: false,
            allow_model_training: false,
            purpose: purpose.into(),
            parent_grant_id: None,
            delegation_depth_remaining: 0,
            not_before: now,
            expires_at,
            status: AuthorityGrantStatus::Active,
            status_reason: None,
            created_at: now,
            updated_at: now,
        };
        grant.validate()?;
        Ok(grant)
    }

    pub fn semantic_digest(&self) -> String {
        canonical_json_sha256(
            &serde_json::to_value(self)
                .expect("serializing an in-memory AuthorityGrant cannot fail"),
        )
    }

    pub fn is_effective_at(&self, at: DateTime<Utc>) -> bool {
        self.status == AuthorityGrantStatus::Active && at >= self.not_before && at < self.expires_at
    }

    pub fn allows(
        &self,
        kind: AuthorityGrantKind,
        target: &StableUri,
        capability: ContextCapability,
        sensitivity: Sensitivity,
        retention: RetentionClass,
        at: DateTime<Utc>,
    ) -> bool {
        self.kind == kind
            && self.is_effective_at(at)
            && sensitivity.rank() <= self.sensitivity_ceiling.rank()
            && retention.rank() <= self.retention_ceiling.rank()
            && self.targets.iter().any(|candidate| candidate == target)
            && self.capabilities.contains(&capability)
    }

    pub fn is_delegation_subset_of(&self, parent: &Self) -> bool {
        self.parent_grant_id == Some(parent.grant_id)
            && self.kind == parent.kind
            && self.governing_node == parent.governing_node
            && self.grantor == parent.grantee
            && self.not_before >= parent.not_before
            && self.expires_at <= parent.expires_at
            && self.sensitivity_ceiling.rank() <= parent.sensitivity_ceiling.rank()
            && self.retention_ceiling.rank() <= parent.retention_ceiling.rank()
            && (!self.allow_redistribution || parent.allow_redistribution)
            && (!self.allow_model_training || parent.allow_model_training)
            && parent.delegation_depth_remaining > 0
            && self.delegation_depth_remaining < parent.delegation_depth_remaining
            && self
                .targets
                .iter()
                .all(|target| parent.targets.contains(target))
            && self
                .capabilities
                .iter()
                .all(|capability| parent.capabilities.contains(capability))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.revision == 0 {
            return Err("authority grant revision must be at least one".into());
        }
        let node_id = self
            .governing_node
            .context_node_uuid()
            .ok_or_else(|| "governing node must be a canonical Context Node URI".to_string())?;
        if self.grant_uri != StableUri::authority_grant(node_id, self.grant_id) {
            return Err("authority grant URI must derive from its governing node and id".into());
        }
        if self.grantor == self.grantee {
            return Err("authority grantor and grantee must be distinct".into());
        }
        if self.targets.is_empty()
            || self.targets.len() > 256
            || !is_strictly_sorted_unique(self.targets.iter().map(StableUri::as_str))
        {
            return Err(
                "authority grant targets must contain 1-256 sorted, unique stable URIs".into(),
            );
        }
        if self.capabilities.is_empty()
            || self.capabilities.len() > 16
            || !is_strictly_sorted_unique(
                self.capabilities
                    .iter()
                    .map(|capability| capability.as_str()),
            )
        {
            return Err(
                "authority grant capabilities must be sorted, unique, and bounded to 16 entries"
                    .into(),
            );
        }
        let valid_capabilities = match self.kind {
            AuthorityGrantKind::Context => self.capabilities.iter().all(|capability| {
                matches!(
                    capability,
                    ContextCapability::Discover
                        | ContextCapability::Query
                        | ContextCapability::Read
                        | ContextCapability::Subscribe
                        | ContextCapability::Export
                        | ContextCapability::Health
                )
            }),
            AuthorityGrantKind::Tool => self.capabilities.iter().all(|capability| {
                matches!(
                    capability,
                    ContextCapability::Propose
                        | ContextCapability::Command
                        | ContextCapability::Execute
                        | ContextCapability::Notify
                        | ContextCapability::Synchronize
                        | ContextCapability::Import
                        | ContextCapability::Revoke
                )
            }),
        };
        if !valid_capabilities {
            return Err("authority grant capability does not belong to its grant kind".into());
        }
        validate_display_text(&self.purpose, "authority grant purpose", 512)?;
        if self.delegation_depth_remaining > 16 {
            return Err("authority grant delegation depth must not exceed 16".into());
        }
        if self.expires_at <= self.not_before || self.expires_at <= self.created_at {
            return Err(
                "authority grant expiry must follow its validity start and creation".into(),
            );
        }
        if self.updated_at < self.created_at {
            return Err("authority grant update time cannot precede creation".into());
        }
        match (self.status, &self.status_reason) {
            (AuthorityGrantStatus::Active, None) => {}
            (AuthorityGrantStatus::Active, Some(_)) => {
                return Err("active authority grants cannot carry a status reason".into());
            }
            (_, Some(reason)) => {
                validate_display_text(reason, "authority grant status reason", 512)?;
            }
            (_, None) => {
                return Err("inactive authority grants require a status reason".into());
            }
        }
        Ok(())
    }
}

/// Governed mapping between a canonical resource and an external object.
///
/// Provider-specific payload types stop at adapters. The values here are
/// portable strings and stable MindVault URIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBinding {
    pub binding_id: Uuid,
    pub revision: u64,
    pub resource_uri: StableUri,
    pub context_node: StableUri,
    pub external_system: String,
    pub external_account_id: String,
    pub external_object_id: String,
    pub authoritative_source: StableUri,
    pub authoritative_fields: Vec<String>,
    pub sync_direction: SyncDirection,
    pub last_seen_version: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_sync_cursor: Option<String>,
    pub content_hash: Option<String>,
    pub materialization_mode: MaterializationMode,
    pub freshness: SourceFreshness,
    pub conflict_policy: SourceConflictPolicy,
    pub deletion_policy: SourceDeletionPolicy,
    pub retention_class: RetentionClass,
    pub sensitivity: Sensitivity,
    pub provenance_ref: StableUri,
    pub status: SourceBindingStatus,
    pub supersedes_binding_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SourceBinding {
    pub fn new(
        resource_uri: StableUri,
        context_node: StableUri,
        external_system: impl Into<String>,
        external_account_id: impl Into<String>,
        external_object_id: impl Into<String>,
        authoritative_source: StableUri,
        provenance_ref: StableUri,
    ) -> Result<Self, String> {
        let now = Utc::now();
        let binding = Self {
            binding_id: Uuid::now_v7(),
            revision: 1,
            resource_uri,
            context_node,
            external_system: external_system.into(),
            external_account_id: external_account_id.into(),
            external_object_id: external_object_id.into(),
            authoritative_source,
            authoritative_fields: vec!["*".into()],
            sync_direction: SyncDirection::Inbound,
            last_seen_version: None,
            last_seen_at: None,
            last_sync_cursor: None,
            content_hash: None,
            materialization_mode: MaterializationMode::ReferenceOnly,
            freshness: SourceFreshness::Unknown,
            conflict_policy: SourceConflictPolicy::RequireReview,
            deletion_policy: SourceDeletionPolicy::RequireReview,
            retention_class: RetentionClass::Operational,
            sensitivity: Sensitivity::Internal,
            provenance_ref,
            status: SourceBindingStatus::Active,
            supersedes_binding_id: None,
            created_at: now,
            updated_at: now,
        };
        binding.validate()?;
        Ok(binding)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.revision == 0 {
            return Err("source binding revision must be at least one".into());
        }
        if !is_token(&self.external_system, 128) {
            return Err("external system must be a non-empty portable token".into());
        }
        if self.context_node.context_node_uuid().is_none() {
            return Err("source binding context node must use a canonical node URI".into());
        }
        validate_opaque_identifier(&self.external_account_id, "external account id", 1024)?;
        validate_opaque_identifier(&self.external_object_id, "external object id", 2048)?;
        if self.authoritative_fields.is_empty()
            || self.authoritative_fields.len() > 256
            || self.authoritative_fields.iter().any(|field| {
                field.is_empty() || field.len() > 512 || field.chars().any(char::is_control)
            })
        {
            return Err("authoritative fields must contain 1-256 bounded field selectors".into());
        }
        let mut fields = self.authoritative_fields.clone();
        fields.sort();
        fields.dedup();
        if fields.len() != self.authoritative_fields.len() {
            return Err("authoritative fields must not contain duplicates".into());
        }
        for (label, value, limit) in [
            ("last seen version", self.last_seen_version.as_deref(), 1024),
            ("last sync cursor", self.last_sync_cursor.as_deref(), 4096),
            ("content hash", self.content_hash.as_deref(), 512),
        ] {
            if let Some(value) = value {
                validate_opaque_identifier(value, label, limit)?;
            }
        }
        if self.updated_at < self.created_at {
            return Err("source binding update time cannot precede creation".into());
        }
        if self.supersedes_binding_id == Some(self.binding_id) {
            return Err("a source binding cannot supersede itself".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IdempotentNodeCommit {
    pub node: KnowledgeNode,
    pub event: EventEnvelope,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct IdempotentSchemaCommit {
    pub schema: PublicSchemaRecord,
    pub event: EventEnvelope,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct IdempotentSourceBindingCommit {
    pub binding: SourceBinding,
    pub event: EventEnvelope,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct IdempotentContextNodeCommit {
    pub context_node: ContextNodeRecord,
    pub event: EventEnvelope,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct IdempotentAuthorityGrantCommit {
    pub grant: AuthorityGrant,
    pub event: EventEnvelope,
    pub replayed: bool,
}

pub fn canonical_json_sha256(value: &serde_json::Value) -> String {
    let canonical =
        serde_json::to_vec(value).expect("serializing an in-memory serde_json::Value cannot fail");
    let digest = Sha256::digest(canonical);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Command admission
//
// Vocabulary for wiring the grant resolver into public command admission, the
// slice named by `docs/architecture/interoperability-kernel-v1.md:283-284`.
// Constitutional law 7 requires an explicit grant for context access; law 8
// requires action authority to be a separate axis, so reading never implies
// authority to mutate, execute, transmit, or spend.
//
// `ActionEnvelope` (IK-002) is the versioned command-attribution record required
// before a public mutation proceeds under observe/enforce. ADR 010:136-152 also
// lists Space, work-order, approval, budget, and outcome fields; those remain
// deferred to Trust Ledger / Space slices (IK-020 and successors). This type
// validates the acceptance-required attribution fields now, without inventing
// the deferred foreign keys.
// ---------------------------------------------------------------------------

/// One authorization question, asked before a public command mutates anything.
///
/// `resource` is the exact grant target and is deliberately distinct from
/// `subject`, the resource the command affects. For a create the two differ
/// necessarily: grant targets are exact stable-URI matches with no wildcards or
/// prefixes (`docs/architecture/AUTHORITY_GRANT_MODEL.md:44-45`), while a
/// created resource's identifier is minted microseconds before admission, so no
/// pre-existing grant could name it. The only satisfiable target for a create is
/// therefore the governing node URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAdmissionRequest {
    pub request_id: Uuid,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    /// The accountable identity.
    pub principal: StableUri,
    /// The acting identity, and the grantee the resolver looks up.
    pub actor: StableUri,
    pub governing_node: StableUri,
    /// Exact grant target.
    pub resource: StableUri,
    /// The resource this command affects or creates.
    pub subject: StableUri,
    pub operation: ContextCapability,
    pub required_grant_kind: AuthorityGrantKind,
    pub idempotency_key: IdempotencyKey,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    pub requested_at: DateTime<Utc>,
}

impl CommandAdmissionRequest {
    /// Reject incoherent requests before any storage lookup happens.
    ///
    /// The load-bearing check is the law-8 one: Context and Tool Grants carry
    /// disjoint capability sets, so pairing a Context Grant with an effectful
    /// capability is a construction error, not an authorization failure.
    pub fn validate(&self) -> Result<(), String> {
        let observational = Self::is_context_capability(self.operation);
        match self.required_grant_kind {
            AuthorityGrantKind::Context if !observational => {
                return Err(format!(
                    "capability {} is effectful and cannot be authorized by a Context Grant; \
                     context access never implies action authority",
                    self.operation.as_str()
                ));
            }
            AuthorityGrantKind::Tool if observational => {
                return Err(format!(
                    "capability {} is observational and belongs to a Context Grant",
                    self.operation.as_str()
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// Capabilities that only observe. Everything else can cause an effect.
    const fn is_context_capability(capability: ContextCapability) -> bool {
        matches!(
            capability,
            ContextCapability::Discover
                | ContextCapability::Query
                | ContextCapability::Read
                | ContextCapability::Subscribe
                | ContextCapability::Health
        )
    }

    /// Stable digest of the admission question, for correlating a recorded
    /// decision with the request that produced it.
    pub fn admission_digest(&self) -> String {
        canonical_json_sha256(&serde_json::json!({
            "actor": self.actor.as_str(),
            "capability": self.operation.as_str(),
            "grant_kind": self.required_grant_kind.as_str(),
            "principal": self.principal.as_str(),
            "resource": self.resource.as_str(),
            "retention": self.retention.as_str(),
            "sensitivity": self.sensitivity.as_str(),
            "subject": self.subject.as_str(),
        }))
    }
}

interoperability_string_enum! {
    /// Why admission was refused.
    ///
    /// Recorded in the audit trail, never returned to the caller: the
    /// distinction between "you hold no grant" and "your grant expired" is a
    /// probing oracle. Law 15 requires a denial be recorded, not disclosed.
    pub enum AdmissionDenialReason {
        NoEffectiveGrant => "no_effective_grant",
        GrantKindMismatch => "grant_kind_mismatch",
        CapabilityNotGranted => "capability_not_granted",
        TargetNotGranted => "target_not_granted",
        GrantNotEffective => "grant_not_effective",
        SensitivityCeilingExceeded => "sensitivity_ceiling_exceeded",
        RetentionCeilingExceeded => "retention_ceiling_exceeded",
        DelegationChainIneffective => "delegation_chain_ineffective",
        ResolverUnavailable => "resolver_unavailable",
    }
}

/// The resolver's answer. Fail-closed: anything but `Admitted` denies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AdmissionDecision {
    Admitted {
        grant_id: Uuid,
        grant_uri: StableUri,
        grant_kind: AuthorityGrantKind,
        capability: ContextCapability,
        delegation_depth_remaining: u8,
        decided_at: DateTime<Utc>,
    },
    Denied {
        reason: AdmissionDenialReason,
        decided_at: DateTime<Utc>,
    },
}

impl AdmissionDecision {
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted { .. })
    }

    /// Policy metadata safe to embed in a durable event.
    ///
    /// Carries stable identities and enum tokens only. Grant terms — purpose,
    /// target list, grantor, grantee — stay out: the event records that a
    /// decision was reached and under which grant, never what the grant permits.
    pub fn policy_metadata(&self) -> serde_json::Value {
        match self {
            Self::Admitted {
                grant_uri,
                grant_kind,
                capability,
                delegation_depth_remaining,
                decided_at,
                ..
            } => serde_json::json!({
                "decision": "admitted",
                "grant_uri": grant_uri.as_str(),
                "grant_kind": grant_kind.as_str(),
                "capability": capability.as_str(),
                "delegation_depth_remaining": delegation_depth_remaining,
                "decided_at": decided_at.to_rfc3339(),
            }),
            Self::Denied { reason, decided_at } => serde_json::json!({
                "decision": "denied",
                "reason": reason.as_str(),
                "decided_at": decided_at.to_rfc3339(),
            }),
        }
    }
}

/// Durable record of one command-admission decision (IK-001c).
///
/// Append-only. Keyed for idempotent replay on `(principal, idempotency_key)`.
/// Denied commands have no mutation/event, so this is the Law 15 durability
/// surface for refusals. Admitted decisions may also be recorded here as the
/// first Trust Ledger brick; they continue to ride in the event envelope too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAdmissionDecisionRecord {
    pub decision_id: Uuid,
    pub principal: StableUri,
    pub actor: StableUri,
    pub idempotency_key: IdempotencyKey,
    pub correlation_id: Uuid,
    pub request_id: Uuid,
    pub resource: StableUri,
    pub subject: StableUri,
    pub operation: ContextCapability,
    pub required_grant_kind: AuthorityGrantKind,
    pub decision: AdmissionDecision,
    pub admission_digest: String,
    pub decided_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl CommandAdmissionDecisionRecord {
    pub fn from_request_and_decision(
        request: &CommandAdmissionRequest,
        decision: &AdmissionDecision,
    ) -> Self {
        let decided_at = match decision {
            AdmissionDecision::Admitted { decided_at, .. }
            | AdmissionDecision::Denied { decided_at, .. } => *decided_at,
        };
        Self {
            decision_id: Uuid::now_v7(),
            principal: request.principal.clone(),
            actor: request.actor.clone(),
            idempotency_key: request.idempotency_key.clone(),
            correlation_id: request.correlation_id,
            request_id: request.request_id,
            resource: request.resource.clone(),
            subject: request.subject.clone(),
            operation: request.operation,
            required_grant_kind: request.required_grant_kind,
            decision: decision.clone(),
            admission_digest: request.admission_digest(),
            decided_at,
            created_at: Utc::now(),
        }
    }

    pub fn is_denied(&self) -> bool {
        !self.decision.is_admitted()
    }
}

#[derive(Debug, Clone)]
pub struct IdempotentAdmissionDecisionCommit {
    pub record: CommandAdmissionDecisionRecord,
    pub replayed: bool,
}

/// Versioned attribution record for one public command (IK-002).
///
/// Built from a [`CommandAdmissionRequest`] and its [`AdmissionDecision`]. Fail
/// closed: construction rejects missing action ID, correlation ID, principal,
/// acting actor, resource, operation, grant IDs (when admitted), or policy
/// decision. Space / work-order / budget / outcome fields from ADR 010 stay out
/// until those registries exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub envelope_version: String,
    pub action_id: Uuid,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    /// Accountable principal.
    pub principal: StableUri,
    /// Acting actor (may equal principal until the identity registry ships).
    pub actor: StableUri,
    /// Exact grant target consulted for admission.
    pub resource: StableUri,
    /// Resource the command affects or creates.
    pub subject: StableUri,
    pub operation: ContextCapability,
    /// Grant IDs that authorized the command. Non-empty iff admitted.
    pub grant_ids: Vec<Uuid>,
    pub policy_decision: AdmissionDecision,
    pub idempotency_key: IdempotencyKey,
    pub requested_at: DateTime<Utc>,
}

/// Builder input that allows tests to omit required fields one at a time.
#[derive(Debug, Clone)]
pub struct NewActionEnvelope {
    pub action_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub principal: Option<StableUri>,
    pub actor: Option<StableUri>,
    pub resource: Option<StableUri>,
    pub subject: Option<StableUri>,
    pub operation: Option<ContextCapability>,
    pub grant_ids: Option<Vec<Uuid>>,
    pub policy_decision: Option<AdmissionDecision>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub requested_at: Option<DateTime<Utc>>,
}

impl ActionEnvelope {
    /// Construct from an admission question and its resolver answer.
    pub fn from_admission(
        request: &CommandAdmissionRequest,
        decision: &AdmissionDecision,
    ) -> Result<Self, String> {
        let grant_ids = match decision {
            AdmissionDecision::Admitted { grant_id, .. } => vec![*grant_id],
            AdmissionDecision::Denied { .. } => Vec::new(),
        };
        Self::try_new(NewActionEnvelope {
            action_id: Some(request.request_id),
            correlation_id: Some(request.correlation_id),
            causation_id: request.causation_id,
            principal: Some(request.principal.clone()),
            actor: Some(request.actor.clone()),
            resource: Some(request.resource.clone()),
            subject: Some(request.subject.clone()),
            operation: Some(request.operation),
            grant_ids: Some(grant_ids),
            policy_decision: Some(decision.clone()),
            idempotency_key: Some(request.idempotency_key.clone()),
            requested_at: Some(request.requested_at),
        })
    }

    /// Fail-closed constructor used by tests and callers that assemble fields
    /// manually. Nil UUIDs and absent Options count as missing.
    pub fn try_new(input: NewActionEnvelope) -> Result<Self, String> {
        let action_id = match input.action_id {
            Some(id) if !id.is_nil() => id,
            _ => return Err("action_id is required".into()),
        };
        let correlation_id = match input.correlation_id {
            Some(id) if !id.is_nil() => id,
            _ => return Err("correlation_id is required".into()),
        };
        let principal = input
            .principal
            .ok_or_else(|| "principal is required".to_string())?;
        let actor = input
            .actor
            .ok_or_else(|| "acting actor is required".to_string())?;
        let resource = input
            .resource
            .ok_or_else(|| "resource is required".to_string())?;
        let subject = input
            .subject
            .ok_or_else(|| "subject is required".to_string())?;
        let operation = input
            .operation
            .ok_or_else(|| "operation is required".to_string())?;
        let grant_ids = input
            .grant_ids
            .ok_or_else(|| "grant_ids are required".to_string())?;
        let policy_decision = input
            .policy_decision
            .ok_or_else(|| "policy_decision is required".to_string())?;
        let idempotency_key = input
            .idempotency_key
            .ok_or_else(|| "idempotency_key is required".to_string())?;
        let requested_at = input
            .requested_at
            .ok_or_else(|| "requested_at is required".to_string())?;

        match &policy_decision {
            AdmissionDecision::Admitted { grant_id, .. } => {
                if grant_ids.is_empty() {
                    return Err("grant_ids are required for an admitted action".into());
                }
                if !grant_ids.contains(grant_id) {
                    return Err("grant_ids must include the admitted grant".into());
                }
            }
            AdmissionDecision::Denied { .. } => {
                if !grant_ids.is_empty() {
                    return Err("denied actions must not carry grant_ids".into());
                }
            }
        }

        let envelope = Self {
            envelope_version: ACTION_ENVELOPE_V1.into(),
            action_id,
            correlation_id,
            causation_id: input.causation_id,
            principal,
            actor,
            resource,
            subject,
            operation,
            grant_ids,
            policy_decision,
            idempotency_key,
            requested_at,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.envelope_version != ACTION_ENVELOPE_V1 {
            return Err(format!(
                "unsupported action envelope version: {}",
                self.envelope_version
            ));
        }
        if self.action_id.is_nil() {
            return Err("action_id is required".into());
        }
        if self.correlation_id.is_nil() {
            return Err("correlation_id is required".into());
        }
        match &self.policy_decision {
            AdmissionDecision::Admitted { grant_id, .. } => {
                if self.grant_ids.is_empty() {
                    return Err("grant_ids are required for an admitted action".into());
                }
                if !self.grant_ids.contains(grant_id) {
                    return Err("grant_ids must include the admitted grant".into());
                }
            }
            AdmissionDecision::Denied { .. } => {
                if !self.grant_ids.is_empty() {
                    return Err("denied actions must not carry grant_ids".into());
                }
            }
        }
        Ok(())
    }

    /// Compact metadata safe to embed beside an event's admission block.
    pub fn attribution_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "envelope_version": self.envelope_version,
            "action_id": self.action_id,
            "correlation_id": self.correlation_id,
            "causation_id": self.causation_id,
            "principal": self.principal.as_str(),
            "actor": self.actor.as_str(),
            "resource": self.resource.as_str(),
            "subject": self.subject.as_str(),
            "operation": self.operation.as_str(),
            "grant_ids": self.grant_ids,
            "policy_decision": self.policy_decision.policy_metadata(),
            "idempotency_key": self.idempotency_key.as_str(),
            "requested_at": self.requested_at.to_rfc3339(),
        })
    }
}

fn validate_sha256(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn validate_opaque_identifier(value: &str, label: &str, max_len: usize) -> Result<(), String> {
    if value.is_empty()
        || value.len() > max_len
        || value.chars().any(|character| character.is_control())
    {
        return Err(format!(
            "{label} must be non-empty, bounded, and contain no control characters"
        ));
    }
    Ok(())
}

fn validate_delivery_error(error_code: &str, error_summary: &str) -> Result<(), String> {
    if !is_token(error_code, 128) {
        return Err("delivery error code must be a non-empty portable token".into());
    }
    validate_opaque_identifier(error_summary, "delivery error summary", 4096)
}

fn validate_display_text(value: &str, label: &str, max_len: usize) -> Result<(), String> {
    if value.trim() != value
        || value.is_empty()
        || value.len() > max_len
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "{label} must be trimmed, non-empty, bounded, and contain no control characters"
        ));
    }
    Ok(())
}

fn is_strictly_sorted_unique<I, T>(values: I) -> bool
where
    I: IntoIterator<Item = T>,
    T: Ord,
{
    let mut previous: Option<T> = None;
    for value in values {
        if previous.as_ref().is_some_and(|prior| prior >= &value) {
            return false;
        }
        previous = Some(value);
    }
    true
}

fn is_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn is_event_type(value: &str) -> bool {
    is_token(value, 200) && value.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> EventEnvelope {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let principal_id = Uuid::now_v7();
        EventEnvelope::new(NewEventEnvelope {
            event_type: KNOWLEDGE_NODE_CREATED_V1.into(),
            source: StableUri::node(node_id),
            subject: StableUri::knowledge_node(node_id, resource_id),
            schema: SchemaReference::new(
                StableUri::schema("knowledge-node-created").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: StableUri::principal(node_id, principal_id),
            actor: StableUri::principal(node_id, principal_id),
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse("request-123").unwrap(),
            payload_digest: "a".repeat(64),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: StableUri::knowledge_node(node_id, resource_id),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data: serde_json::json!({"namespace": "default"}),
        })
        .unwrap()
    }

    #[test]
    fn stable_uri_rejects_credentials_and_non_mindvault_schemes() {
        assert!(StableUri::parse("https://example.com/node/1").is_err());
        assert!(StableUri::parse("mindvault://user:secret@example/node/1").is_err());
        assert!(StableUri::parse("mindvault://node/path?secret=value").is_err());
        assert!(StableUri::parse("mindvault://NODE/node").is_err());
        assert!(StableUri::parse("mindvault://node/a/../b").is_err());

        let node_id = Uuid::now_v7();
        let principal = StableUri::principal(node_id, Uuid::now_v7());
        assert_eq!(principal.principal_context_node_uuid(), Some(node_id));
        assert_eq!(StableUri::node(node_id).principal_context_node_uuid(), None);
    }

    #[test]
    fn envelope_round_trips_without_losing_required_fields() {
        let event = sample_event();
        let json = serde_json::to_string(&event).unwrap();
        let decoded: EventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, event);
        assert_eq!(decoded.envelope_version, EVENT_ENVELOPE_V1);
        assert_eq!(
            decoded.subject.trailing_uuid(),
            event.subject.trailing_uuid()
        );
        decoded.validate().unwrap();
    }

    #[test]
    fn envelope_rejects_missing_provenance_and_invalid_digest() {
        let mut event = sample_event();
        event.provenance.clear();
        assert!(event.validate().unwrap_err().contains("provenance"));

        event.provenance.push(ProvenanceReference {
            resource: event.subject.clone(),
            relation: ProvenanceRelation::PrimarySource,
        });
        event.payload_digest = "not-a-digest".into();
        assert!(event.validate().unwrap_err().contains("payload digest"));

        event.payload_digest = "a".repeat(64);
        event.schema.version = "contains spaces".into();
        assert!(event.validate().unwrap_err().contains("schema version"));
    }

    #[test]
    fn idempotency_key_is_bounded_and_header_safe() {
        assert!(IdempotencyKey::parse("meeting:abc-123").is_ok());
        assert!(IdempotencyKey::parse("").is_err());
        assert!(IdempotencyKey::parse("contains spaces").is_err());
        assert!(IdempotencyKey::parse("x".repeat(201)).is_err());
    }

    #[test]
    fn public_schema_digest_covers_the_canonical_definition() {
        let owner = StableUri::parse("mindvault://governance/node").unwrap();
        let mut schema = PublicSchemaRecord::new(
            SchemaReference::new(StableUri::schema("portable-test").unwrap(), "1.0.0").unwrap(),
            serde_json::json!({
                "$schema": JSON_SCHEMA_DRAFT_2020_12,
                "type": "object",
                "required": ["resource_uri"],
            }),
            owner,
        )
        .unwrap();
        schema.definition["type"] = serde_json::json!("array");
        assert!(schema.validate().unwrap_err().contains("does not match"));
    }

    #[test]
    fn public_schema_requires_governed_dialect_and_valid_event_type_extension() {
        let owner = StableUri::parse("mindvault://governance/node").unwrap();
        let reference =
            SchemaReference::new(StableUri::schema("portable-test").unwrap(), "1.0.0").unwrap();

        let missing_dialect = PublicSchemaRecord::new(
            reference.clone(),
            serde_json::json!({"type": "object"}),
            owner.clone(),
        );
        assert!(missing_dialect.unwrap_err().contains("draft 2020-12"));

        let invalid_event_type = PublicSchemaRecord::new(
            reference,
            serde_json::json!({
                "$schema": JSON_SCHEMA_DRAFT_2020_12,
                "type": "object",
                (EVENT_TYPE_SCHEMA_EXTENSION): "not dotted",
            }),
            owner,
        );
        assert!(invalid_event_type
            .unwrap_err()
            .contains(EVENT_TYPE_SCHEMA_EXTENSION));
    }

    #[test]
    fn capability_manifest_requires_canonical_order_and_digest_integrity() {
        assert!(ContextCapabilityManifest::new(
            vec![ContextCapability::Read, ContextCapability::Discover],
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err()
        .contains("sorted"));

        let mut manifest = ContextCapabilityManifest::new(
            vec![ContextCapability::Discover, ContextCapability::Read],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        manifest.capabilities.push(ContextCapability::Subscribe);
        assert!(manifest.validate().unwrap_err().contains("digest"));
    }

    #[test]
    fn context_node_binds_identity_and_endpoints_to_advertised_protocols() {
        let node_id = Uuid::now_v7();
        let manifest = ContextCapabilityManifest::new(
            vec![ContextCapability::Discover],
            vec![ContextProtocolProfile {
                protocol: "mcp".into(),
                version: "2025-11-25".into(),
                roles: vec!["server".into()],
            }],
            Vec::new(),
        )
        .unwrap();
        let mut context_node = ContextNodeRecord::discovered(
            node_id,
            ContextNodeType::Application,
            StableUri::principal(node_id, Uuid::now_v7()),
            StableUri::node(node_id),
            "Portable App",
            manifest,
        )
        .unwrap();
        context_node.endpoints = vec![ContextNodeEndpoint {
            protocol: "a2a".into(),
            uri: "https://example.invalid/a2a".into(),
        }];
        assert!(context_node.validate().unwrap_err().contains("advertised"));

        context_node.endpoints[0].protocol = "mcp".into();
        context_node.node_uri = StableUri::node(Uuid::now_v7());
        assert!(context_node.validate().unwrap_err().contains("derived"));
    }

    #[test]
    fn context_node_lifecycle_rejects_skipped_trust_states() {
        assert!(ContextNodeStatus::Discovered.can_transition_to(ContextNodeStatus::PendingTrust));
        assert!(ContextNodeStatus::PendingTrust.can_transition_to(ContextNodeStatus::Active));
        assert!(!ContextNodeStatus::Discovered.can_transition_to(ContextNodeStatus::Active));
        assert!(!ContextNodeStatus::Active.can_transition_to(ContextNodeStatus::Retired));
    }

    #[test]
    fn authority_grants_keep_context_and_tool_capabilities_disjoint() {
        let node_id = Uuid::now_v7();
        let governing_node = StableUri::node(node_id);
        let grantor = StableUri::principal(node_id, Uuid::now_v7());
        let grantee = StableUri::principal(Uuid::now_v7(), Uuid::now_v7());
        let target = StableUri::knowledge_node(node_id, Uuid::now_v7());
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let context = AuthorityGrant::new_context(
            governing_node.clone(),
            grantor.clone(),
            grantee.clone(),
            vec![target.clone()],
            vec![ContextCapability::Read],
            "Read one governed resource",
            expires_at,
        )
        .unwrap();
        assert!(context.allows(
            AuthorityGrantKind::Context,
            &target,
            ContextCapability::Read,
            Sensitivity::Internal,
            RetentionClass::Operational,
            Utc::now(),
        ));
        assert!(!context.allows(
            AuthorityGrantKind::Tool,
            &target,
            ContextCapability::Read,
            Sensitivity::Internal,
            RetentionClass::Operational,
            Utc::now(),
        ));
        assert!(!context.allows(
            AuthorityGrantKind::Context,
            &target,
            ContextCapability::Read,
            Sensitivity::Internal,
            RetentionClass::Durable,
            Utc::now(),
        ));

        assert!(AuthorityGrant::new_context(
            governing_node,
            grantor,
            grantee,
            vec![target],
            vec![ContextCapability::Execute],
            "Invalid effectful context permission",
            expires_at,
        )
        .unwrap_err()
        .contains("grant kind"));
    }

    #[test]
    fn delegated_authority_must_be_a_strict_parent_subset() {
        let node_id = Uuid::now_v7();
        let governing_node = StableUri::node(node_id);
        let root_grantor = StableUri::principal(node_id, Uuid::now_v7());
        let delegate = StableUri::principal(Uuid::now_v7(), Uuid::now_v7());
        let recipient = StableUri::principal(Uuid::now_v7(), Uuid::now_v7());
        let target = StableUri::knowledge_node(node_id, Uuid::now_v7());
        let parent_expiry = Utc::now() + chrono::Duration::hours(2);
        let mut parent = AuthorityGrant::new_context(
            governing_node.clone(),
            root_grantor,
            delegate.clone(),
            vec![target.clone()],
            vec![ContextCapability::Query, ContextCapability::Read],
            "Delegate bounded read access",
            parent_expiry,
        )
        .unwrap();
        parent.delegation_depth_remaining = 2;
        parent.validate().unwrap();

        let mut child = AuthorityGrant::new_context(
            governing_node,
            delegate,
            recipient,
            vec![target],
            vec![ContextCapability::Read],
            "Read through one delegation",
            parent_expiry - chrono::Duration::minutes(30),
        )
        .unwrap();
        child.parent_grant_id = Some(parent.grant_id);
        child.delegation_depth_remaining = 1;
        assert!(child.is_delegation_subset_of(&parent));

        child.allow_model_training = true;
        assert!(!child.is_delegation_subset_of(&parent));
    }

    #[test]
    fn source_binding_defaults_to_least_retention_and_review() {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let binding = SourceBinding::new(
            StableUri::knowledge_node(node_id, resource_id),
            StableUri::node(node_id),
            "calendar",
            "account-1",
            "event-1",
            StableUri::parse("mindvault://sources/calendar").unwrap(),
            StableUri::knowledge_node(node_id, resource_id),
        )
        .unwrap();

        assert_eq!(
            binding.materialization_mode,
            MaterializationMode::ReferenceOnly
        );
        assert_eq!(binding.conflict_policy, SourceConflictPolicy::RequireReview);
        assert_eq!(binding.freshness, SourceFreshness::Unknown);
    }

    #[test]
    fn source_binding_rejects_duplicate_field_authority() {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let mut binding = SourceBinding::new(
            StableUri::knowledge_node(node_id, resource_id),
            StableUri::node(node_id),
            "calendar",
            "account-1",
            "event-1",
            StableUri::parse("mindvault://sources/calendar").unwrap(),
            StableUri::knowledge_node(node_id, resource_id),
        )
        .unwrap();
        binding.authoritative_fields = vec!["start_time".into(), "start_time".into()];
        assert!(binding.validate().unwrap_err().contains("duplicates"));
    }

    #[test]
    fn published_delivery_builds_attributed_immutable_evidence() {
        let event = sample_event();
        let claimed_at = Utc::now();
        let claim = OutboxDeliveryClaim {
            event: event.clone(),
            lease_id: Uuid::now_v7(),
            attempt: 1,
            executor: StableUri::parse("mindvault://dispatchers/local").unwrap(),
            destination: StableUri::parse("mindvault://destinations/test").unwrap(),
            claimed_at,
            lease_expires_at: claimed_at + chrono::Duration::minutes(5),
        };
        let completion = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: claim.lease_id,
            attempt: claim.attempt,
            completed_at: claimed_at + chrono::Duration::seconds(2),
            result: OutboxDeliveryResult::Published {
                delivery_reference: "provider-message-123".into(),
                response_digest: Some("b".repeat(64)),
            },
        };

        let receipt = ActionReceipt::from_outbox_delivery(&claim, &completion).unwrap();

        assert_eq!(receipt.outcome, ActionReceiptOutcome::Published);
        assert_eq!(receipt.event_id, event.id);
        assert_eq!(receipt.claim_id, claim.lease_id);
        assert_eq!(receipt.request_digest, event.content_digest());
        assert_eq!(receipt.principal, event.principal);
        assert_eq!(receipt.provenance, event.provenance);
        assert!(receipt.matches_delivery(&claim, &completion));
        receipt.validate().unwrap();
    }

    #[test]
    fn consumer_receipt_preserves_governance_and_claim_evidence() {
        let event = sample_event();
        let claimed_at = Utc::now();
        let claim = ConsumerInboxClaim {
            inbox_sequence: 1,
            consumer: StableUri::parse("mindvault://consumers/local-index").unwrap(),
            event: event.clone(),
            lease_id: Uuid::now_v7(),
            attempt: 1,
            processor: StableUri::parse("mindvault://processors/local-index").unwrap(),
            claimed_at,
            lease_expires_at: claimed_at + chrono::Duration::minutes(5),
        };
        let completion = ConsumerApplicationCompletion {
            inbox_sequence: claim.inbox_sequence,
            event_id: event.id,
            lease_id: claim.lease_id,
            attempt: claim.attempt,
            completed_at: claimed_at + chrono::Duration::seconds(2),
            result: ConsumerApplicationResult::Applied {
                application_reference: "local-index-entry-123".into(),
                effect_digest: Some("c".repeat(64)),
            },
        };

        let receipt = ConsumerApplicationReceipt::from_application(&claim, &completion).unwrap();

        assert_eq!(receipt.outcome, ConsumerApplicationOutcome::Applied);
        assert_eq!(receipt.event_id, event.id);
        assert_eq!(receipt.claim_id, claim.lease_id);
        assert_eq!(receipt.request_digest, event.content_digest());
        assert_eq!(receipt.principal, event.principal);
        assert_eq!(receipt.provenance, event.provenance);
        assert!(receipt.matches_application(&claim, &completion));
        receipt.validate().unwrap();
    }

    #[test]
    fn consumer_completion_rejects_expired_lease_boundary() {
        let event = sample_event();
        let claimed_at = Utc::now();
        let claim = ConsumerInboxClaim {
            inbox_sequence: 1,
            consumer: StableUri::parse("mindvault://consumers/local-index").unwrap(),
            event: event.clone(),
            lease_id: Uuid::now_v7(),
            attempt: 1,
            processor: StableUri::parse("mindvault://processors/local-index").unwrap(),
            claimed_at,
            lease_expires_at: claimed_at + chrono::Duration::minutes(5),
        };
        let completion = ConsumerApplicationCompletion {
            inbox_sequence: claim.inbox_sequence,
            event_id: event.id,
            lease_id: claim.lease_id,
            attempt: claim.attempt,
            completed_at: claim.lease_expires_at,
            result: ConsumerApplicationResult::DeadLettered {
                error_code: "invalid_payload".into(),
                error_summary: "payload cannot be applied".into(),
            },
        };

        assert!(completion
            .validate_for(&claim)
            .unwrap_err()
            .contains("expired lease"));
    }

    #[test]
    fn delivery_completion_rejects_invalid_retry_and_expired_lease() {
        let event = sample_event();
        let claimed_at = Utc::now();
        let claim = OutboxDeliveryClaim {
            event: event.clone(),
            lease_id: Uuid::now_v7(),
            attempt: 1,
            executor: StableUri::parse("mindvault://dispatchers/local").unwrap(),
            destination: StableUri::parse("mindvault://destinations/test").unwrap(),
            claimed_at,
            lease_expires_at: claimed_at + chrono::Duration::minutes(5),
        };
        let mut completion = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: claim.lease_id,
            attempt: claim.attempt,
            completed_at: claimed_at + chrono::Duration::seconds(2),
            result: OutboxDeliveryResult::RetryScheduled {
                retry_at: claimed_at + chrono::Duration::seconds(2),
                error_code: "destination_unavailable".into(),
                error_summary: "temporary failure".into(),
            },
        };

        assert!(completion
            .validate_for(&claim)
            .unwrap_err()
            .contains("retry time"));

        completion.completed_at = claim.lease_expires_at;
        completion.result = OutboxDeliveryResult::DeadLettered {
            error_code: "invalid_destination".into(),
            error_summary: "permanent failure".into(),
        };
        assert!(completion
            .validate_for(&claim)
            .unwrap_err()
            .contains("expired lease"));
    }

    fn admission_request(
        kind: AuthorityGrantKind,
        operation: ContextCapability,
    ) -> CommandAdmissionRequest {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let principal_id = Uuid::now_v7();
        CommandAdmissionRequest {
            request_id: Uuid::now_v7(),
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            principal: StableUri::principal(node_id, principal_id),
            actor: StableUri::principal(node_id, principal_id),
            governing_node: StableUri::node(node_id),
            resource: StableUri::node(node_id),
            subject: StableUri::knowledge_node(node_id, resource_id),
            operation,
            required_grant_kind: kind,
            idempotency_key: IdempotencyKey::parse("admission-test").expect("idempotency key"),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            requested_at: Utc::now(),
        }
    }

    /// Law 8: context access and action authority are separate axes.
    #[test]
    fn a_context_grant_cannot_be_requested_for_an_effectful_capability() {
        let request = admission_request(AuthorityGrantKind::Context, ContextCapability::Command);
        let error = request
            .validate()
            .expect_err("a mutation is not observational");
        assert!(error.contains("effectful"), "unexpected error: {error}");

        let request = admission_request(AuthorityGrantKind::Tool, ContextCapability::Read);
        let error = request.validate().expect_err("a read is not effectful");
        assert!(error.contains("observational"), "unexpected error: {error}");

        admission_request(AuthorityGrantKind::Tool, ContextCapability::Command)
            .validate()
            .expect("a tool grant may carry a command capability");
        admission_request(AuthorityGrantKind::Context, ContextCapability::Read)
            .validate()
            .expect("a context grant may carry a read capability");
    }

    /// A recorded decision must not leak the terms of the grant that allowed it.
    #[test]
    fn admission_metadata_records_the_decision_without_grant_terms() {
        let node_id = Uuid::now_v7();
        let grant_id = Uuid::now_v7();
        let decision = AdmissionDecision::Admitted {
            grant_id,
            grant_uri: StableUri::authority_grant(node_id, grant_id),
            grant_kind: AuthorityGrantKind::Tool,
            capability: ContextCapability::Command,
            delegation_depth_remaining: 0,
            decided_at: Utc::now(),
        };

        let metadata = decision.policy_metadata().to_string();
        assert!(decision.is_admitted());
        assert!(metadata.contains("admitted"));
        assert!(metadata.contains("grant_uri"));
        for leaked in ["purpose", "targets", "grantee", "grantor", "capabilities"] {
            assert!(
                !metadata.contains(leaked),
                "policy metadata must not carry grant terms, found {leaked}: {metadata}"
            );
        }

        let denied = AdmissionDecision::Denied {
            reason: AdmissionDenialReason::NoEffectiveGrant,
            decided_at: Utc::now(),
        };
        assert!(!denied.is_admitted());
        assert!(denied
            .policy_metadata()
            .to_string()
            .contains("no_effective_grant"));
    }

    /// IK-002: ActionEnvelope rejects each required attribution field when missing.
    #[test]
    fn action_envelope_rejects_missing_required_fields() {
        let node_id = Uuid::now_v7();
        let grant_id = Uuid::now_v7();
        let principal = StableUri::principal(node_id, Uuid::now_v7());
        let resource = StableUri::node(node_id);
        let subject = StableUri::knowledge_node(node_id, Uuid::now_v7());
        let decision = AdmissionDecision::Admitted {
            grant_id,
            grant_uri: StableUri::authority_grant(node_id, grant_id),
            grant_kind: AuthorityGrantKind::Tool,
            capability: ContextCapability::Command,
            delegation_depth_remaining: 0,
            decided_at: Utc::now(),
        };
        let complete = NewActionEnvelope {
            action_id: Some(Uuid::now_v7()),
            correlation_id: Some(Uuid::now_v7()),
            causation_id: None,
            principal: Some(principal.clone()),
            actor: Some(principal.clone()),
            resource: Some(resource.clone()),
            subject: Some(subject.clone()),
            operation: Some(ContextCapability::Command),
            grant_ids: Some(vec![grant_id]),
            policy_decision: Some(decision),
            idempotency_key: Some(IdempotencyKey::parse("action-envelope").unwrap()),
            requested_at: Some(Utc::now()),
        };
        ActionEnvelope::try_new(complete.clone()).expect("complete envelope must validate");

        let mut missing_action = complete.clone();
        missing_action.action_id = None;
        assert!(ActionEnvelope::try_new(missing_action)
            .unwrap_err()
            .contains("action_id"));

        let mut missing_correlation = complete.clone();
        missing_correlation.correlation_id = None;
        assert!(ActionEnvelope::try_new(missing_correlation)
            .unwrap_err()
            .contains("correlation_id"));

        let mut missing_principal = complete.clone();
        missing_principal.principal = None;
        assert!(ActionEnvelope::try_new(missing_principal)
            .unwrap_err()
            .contains("principal"));

        let mut missing_actor = complete.clone();
        missing_actor.actor = None;
        assert!(ActionEnvelope::try_new(missing_actor)
            .unwrap_err()
            .contains("acting actor"));

        let mut missing_resource = complete.clone();
        missing_resource.resource = None;
        assert!(ActionEnvelope::try_new(missing_resource)
            .unwrap_err()
            .contains("resource"));

        let mut missing_operation = complete.clone();
        missing_operation.operation = None;
        assert!(ActionEnvelope::try_new(missing_operation)
            .unwrap_err()
            .contains("operation"));

        let mut missing_grants = complete.clone();
        missing_grants.grant_ids = None;
        assert!(ActionEnvelope::try_new(missing_grants)
            .unwrap_err()
            .contains("grant_ids"));

        let mut empty_grants = complete.clone();
        empty_grants.grant_ids = Some(Vec::new());
        assert!(ActionEnvelope::try_new(empty_grants)
            .unwrap_err()
            .contains("grant_ids"));

        let mut missing_policy = complete;
        missing_policy.policy_decision = None;
        assert!(ActionEnvelope::try_new(missing_policy)
            .unwrap_err()
            .contains("policy_decision"));
    }

    /// Nil UUIDs count as missing; denied decisions carry empty grant_ids.
    #[test]
    fn action_envelope_rejects_nil_ids_and_builds_from_admission() {
        let request = admission_request(AuthorityGrantKind::Tool, ContextCapability::Command);
        let node_id = Uuid::now_v7();
        let grant_id = Uuid::now_v7();
        let principal = StableUri::principal(node_id, Uuid::now_v7());
        let decision = AdmissionDecision::Admitted {
            grant_id,
            grant_uri: StableUri::authority_grant(node_id, grant_id),
            grant_kind: AuthorityGrantKind::Tool,
            capability: ContextCapability::Command,
            delegation_depth_remaining: 0,
            decided_at: Utc::now(),
        };
        let mut nil_action = NewActionEnvelope {
            action_id: Some(Uuid::nil()),
            correlation_id: Some(Uuid::now_v7()),
            causation_id: None,
            principal: Some(principal.clone()),
            actor: Some(principal.clone()),
            resource: Some(StableUri::node(node_id)),
            subject: Some(StableUri::knowledge_node(node_id, Uuid::now_v7())),
            operation: Some(ContextCapability::Command),
            grant_ids: Some(vec![grant_id]),
            policy_decision: Some(decision.clone()),
            idempotency_key: Some(IdempotencyKey::parse("nil-action").unwrap()),
            requested_at: Some(Utc::now()),
        };
        assert!(ActionEnvelope::try_new(nil_action.clone())
            .unwrap_err()
            .contains("action_id"));
        nil_action.action_id = Some(Uuid::now_v7());
        nil_action.correlation_id = Some(Uuid::nil());
        assert!(ActionEnvelope::try_new(nil_action)
            .unwrap_err()
            .contains("correlation_id"));

        let admitted = ActionEnvelope::from_admission(&request, &decision).unwrap();
        assert_eq!(admitted.grant_ids, vec![grant_id]);
        assert_eq!(admitted.envelope_version, ACTION_ENVELOPE_V1);
        assert!(admitted.attribution_metadata()["policy_decision"]["decision"] == "admitted");

        let denied = AdmissionDecision::Denied {
            reason: AdmissionDenialReason::NoEffectiveGrant,
            decided_at: Utc::now(),
        };
        let envelope = ActionEnvelope::from_admission(&request, &denied).unwrap();
        assert!(envelope.grant_ids.is_empty());
        assert!(!envelope.policy_decision.is_admitted());
    }

    /// A default-constructed Tool Grant denies a durable node create.
    ///
    /// `new_tool` defaults `retention_ceiling` to `Operational` while the
    /// node-create envelope declares `Durable`, so the issuer must raise the
    /// ceiling explicitly. Pinned because it is the least obvious way for a
    /// correctly-targeted grant to refuse.
    #[test]
    fn default_tool_grant_ceilings_deny_durable_retention() {
        let node_id = Uuid::now_v7();
        let node_uri = StableUri::node(node_id);
        let grantee = StableUri::principal(node_id, Uuid::now_v7());
        let mut grant = AuthorityGrant::new_tool(
            node_uri.clone(),
            node_uri.clone(),
            grantee,
            vec![node_uri.clone()],
            vec![ContextCapability::Command],
            "admission test",
            Utc::now() + chrono::Duration::hours(1),
        )
        .expect("tool grant should construct");

        let at = Utc::now();
        assert!(
            !grant.allows(
                AuthorityGrantKind::Tool,
                &node_uri,
                ContextCapability::Command,
                Sensitivity::Internal,
                RetentionClass::Durable,
                at,
            ),
            "the default Operational retention ceiling must refuse a Durable command"
        );

        grant.retention_ceiling = RetentionClass::Durable;
        assert!(
            grant.allows(
                AuthorityGrantKind::Tool,
                &node_uri,
                ContextCapability::Command,
                Sensitivity::Internal,
                RetentionClass::Durable,
                at,
            ),
            "raising the ceiling should admit the same command"
        );
    }

    /// The admission digest is stable and distinguishes different questions.
    #[test]
    fn admission_digest_is_stable_and_question_specific() {
        let request = admission_request(AuthorityGrantKind::Tool, ContextCapability::Command);
        assert_eq!(request.admission_digest(), request.admission_digest());

        let mut other = request.clone();
        other.operation = ContextCapability::Execute;
        assert_ne!(
            request.admission_digest(),
            other.admission_digest(),
            "a different capability is a different admission question"
        );
    }
}
