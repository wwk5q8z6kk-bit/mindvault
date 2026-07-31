use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::MvResult;
use crate::model::*;

/// Storage backend for knowledge nodes (metadata + content).
#[async_trait]
pub trait NodeStore: Send + Sync {
    async fn insert(&self, node: &KnowledgeNode) -> MvResult<()>;
    async fn get(&self, id: Uuid) -> MvResult<Option<KnowledgeNode>>;
    async fn update(&self, node: &KnowledgeNode) -> MvResult<()>;
    async fn delete(&self, id: Uuid) -> MvResult<bool>;
    async fn list(
        &self,
        filters: &QueryFilters,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>>;
    async fn touch(&self, id: Uuid) -> MvResult<()>;
    async fn count(&self, filters: &QueryFilters) -> MvResult<usize>;
}

/// Vector embedding storage + similarity search.
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(
        &self,
        id: Uuid,
        embedding: Vec<f32>,
        content: &str,
        namespace: Option<&str>,
    ) -> MvResult<()>;
    async fn search(
        &self,
        embedding: Vec<f32>,
        limit: usize,
        min_score: f64,
        namespace: Option<&str>,
    ) -> MvResult<Vec<(Uuid, f64)>>;
    async fn delete(&self, id: Uuid) -> MvResult<()>;
}

/// Full-text search index.
pub trait FullTextIndex: Send + Sync {
    fn index_node(&self, node: &KnowledgeNode) -> MvResult<()>;
    fn remove_node(&self, id: Uuid) -> MvResult<()>;
    fn search(&self, query: &str, limit: usize) -> MvResult<Vec<(Uuid, f64)>>;
    fn commit(&self) -> MvResult<()>;
}

/// Knowledge graph storage.
#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn add_relationship(&self, rel: &Relationship) -> MvResult<()>;
    async fn get_relationship(&self, id: Uuid) -> MvResult<Option<Relationship>>;
    async fn remove_relationship(&self, id: Uuid) -> MvResult<bool>;
    async fn get_relationships_from(&self, node_id: Uuid) -> MvResult<Vec<Relationship>>;
    async fn get_relationships_to(&self, node_id: Uuid) -> MvResult<Vec<Relationship>>;
    async fn get_neighbors(&self, node_id: Uuid, depth: usize) -> MvResult<Vec<Uuid>>;
    async fn remove_node_relationships(&self, node_id: Uuid) -> MvResult<usize>;
}

/// Embedding provider.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> MvResult<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> MvResult<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

/// Keychain storage backend for vault metadata, credentials, delegations, and audit.
#[async_trait]
pub trait KeychainStore: Send + Sync {
    // --- Vault Meta ---
    async fn get_vault_meta(&self) -> MvResult<Option<VaultMeta>>;
    async fn save_vault_meta(&self, meta: &VaultMeta) -> MvResult<()>;

    // --- Key Epochs ---
    async fn insert_key_epoch(&self, epoch: &KeyEpoch) -> MvResult<()>;
    async fn get_key_epoch(&self, epoch: u64) -> MvResult<Option<KeyEpoch>>;
    async fn list_key_epochs(&self) -> MvResult<Vec<KeyEpoch>>;
    async fn retire_key_epoch(&self, epoch: u64) -> MvResult<()>;
    /// Mark an epoch's re-encryption as complete (all credentials migrated to a newer epoch).
    async fn mark_epoch_re_encryption_complete(&self, epoch: u64) -> MvResult<()>;
    /// Delete key epoch rows that are expired and fully re-encrypted.
    async fn delete_expired_epochs(&self) -> MvResult<u64>;

    // --- Domains ---
    async fn insert_domain(&self, domain: &DomainKey) -> MvResult<()>;
    async fn get_domain(&self, id: Uuid) -> MvResult<Option<DomainKey>>;
    async fn get_domain_by_name(&self, name: &str) -> MvResult<Option<DomainKey>>;
    async fn list_domains(&self) -> MvResult<Vec<DomainKey>>;
    async fn revoke_domain(&self, id: Uuid) -> MvResult<()>;

    // --- Credentials ---
    async fn insert_credential(&self, cred: &StoredCredential) -> MvResult<()>;
    async fn get_credential(&self, id: Uuid) -> MvResult<Option<StoredCredential>>;
    async fn update_credential(&self, cred: &StoredCredential) -> MvResult<()>;
    async fn list_credentials(
        &self,
        domain_id: Option<Uuid>,
        state: Option<CredentialState>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<StoredCredential>>;
    async fn count_credentials(&self, domain_id: Option<Uuid>) -> MvResult<usize>;
    async fn shred_credential(&self, id: Uuid) -> MvResult<()>;
    async fn touch_credential(&self, id: Uuid) -> MvResult<()>;

    // --- Delegations ---
    async fn insert_delegation(&self, delegation: &Delegation) -> MvResult<()>;
    async fn get_delegation(&self, id: Uuid) -> MvResult<Option<Delegation>>;
    async fn list_delegations(&self, credential_id: Uuid) -> MvResult<Vec<Delegation>>;
    async fn revoke_delegation(&self, id: Uuid) -> MvResult<()>;
    async fn revoke_delegations_for_credential(&self, credential_id: Uuid) -> MvResult<()>;

    // --- Audit ---
    async fn append_audit_entry(&self, entry: &KeychainAuditEntry) -> MvResult<()>;
    async fn list_audit_entries(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KeychainAuditEntry>>;
    async fn get_latest_audit_entry(&self) -> MvResult<Option<KeychainAuditEntry>>;
    async fn verify_audit_chain(&self) -> MvResult<bool>;

    // --- Breach Detection ---
    async fn record_access_pattern(&self, pattern: &AccessPattern) -> MvResult<()>;
    async fn get_access_patterns(
        &self,
        credential_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<AccessPattern>>;
    async fn insert_breach_alert(&self, alert: &BreachAlert) -> MvResult<()>;
    async fn list_breach_alerts(&self, limit: usize, offset: usize) -> MvResult<Vec<BreachAlert>>;
    async fn acknowledge_breach_alert(&self, id: Uuid) -> MvResult<()>;
    /// Check if a breach alert of the same type was already recorded for this
    /// credential within the last `within_secs` seconds (dedup window).
    async fn has_recent_breach_alert(
        &self,
        credential_id: Uuid,
        alert_type: &str,
        within_secs: u64,
    ) -> MvResult<bool>;

    // --- Tags ---
    async fn get_credential_tags(&self, credential_id: Uuid) -> MvResult<Vec<String>>;
    async fn save_credential_tags(&self, credential_id: Uuid, tags: &[String]) -> MvResult<()>;

    // --- Lockout State ---
    async fn set_lockout_state(&self, attempts: u32, locked_until: Option<String>) -> MvResult<()>;
    async fn get_lockout_state(&self) -> MvResult<(u32, Option<String>)>;

    // --- Domain ACLs ---
    async fn insert_acl(&self, acl: &DomainAcl) -> MvResult<()>;
    async fn get_acls_for_domain(&self, domain_id: Uuid) -> MvResult<Vec<DomainAcl>>;
    async fn get_acl_for_subject(
        &self,
        domain_id: Uuid,
        subject: &str,
    ) -> MvResult<Option<DomainAcl>>;
    async fn delete_acl(&self, id: Uuid) -> MvResult<()>;
}

fn _assert_keychain_store_object_safe(_: &dyn KeychainStore) {}

/// Storage backend for agentic intents, insights, and chronicle entries.
#[async_trait]
pub trait AgenticStore: Send + Sync {
    // --- Intents ---
    async fn log_intent(&self, intent: &CapturedIntent) -> MvResult<()>;
    async fn get_intent(&self, id: Uuid) -> MvResult<Option<CapturedIntent>>;
    async fn list_intents(
        &self,
        node_id: Option<Uuid>,
        status: Option<IntentStatus>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<CapturedIntent>>;
    async fn update_intent_status(&self, id: Uuid, status: IntentStatus) -> MvResult<bool>;

    // --- Insights ---
    async fn log_insight(&self, insight: &ProactiveInsight) -> MvResult<()>;
    async fn list_insights(&self, limit: usize, offset: usize) -> MvResult<Vec<ProactiveInsight>>;
    async fn delete_insight(&self, id: Uuid) -> MvResult<bool>;

    // --- Chronicle ---
    async fn log_chronicle(&self, entry: &ChronicleEntry) -> MvResult<()>;
    async fn list_chronicles(
        &self,
        node_id: Option<Uuid>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ChronicleEntry>>;
}

fn _assert_agentic_store_object_safe(_: &dyn AgenticStore) {}

/// Storage for exchange inbox proposals.
#[async_trait]
pub trait ExchangeStore: Send + Sync {
    async fn submit_proposal(&self, proposal: &Proposal) -> MvResult<()>;
    async fn get_proposal(&self, id: Uuid) -> MvResult<Option<Proposal>>;
    async fn list_proposals(
        &self,
        state: Option<ProposalState>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<Proposal>>;
    async fn resolve_proposal(&self, id: Uuid, state: ProposalState) -> MvResult<bool>;
    async fn count_proposals(&self, state: Option<ProposalState>) -> MvResult<usize>;
    async fn expire_proposals(&self, before: DateTime<Utc>) -> MvResult<usize>;
}

fn _assert_exchange_store_object_safe(_: &dyn ExchangeStore) {}

/// Storage for relay safeguards: blocked senders, auto-approve rules, undo snapshots.
#[async_trait]
pub trait SafeguardStore: Send + Sync {
    // Blocked senders
    async fn add_blocked_sender(&self, sender: &BlockedSender) -> MvResult<()>;
    async fn remove_blocked_sender(&self, id: Uuid) -> MvResult<bool>;
    async fn list_blocked_senders(&self) -> MvResult<Vec<BlockedSender>>;
    async fn is_sender_blocked(&self, sender_type: &str, sender_name: &str) -> MvResult<bool>;

    // Auto-approve rules
    async fn add_auto_approve_rule(&self, rule: &AutoApproveRule) -> MvResult<()>;
    async fn remove_auto_approve_rule(&self, id: Uuid) -> MvResult<bool>;
    async fn list_auto_approve_rules(&self) -> MvResult<Vec<AutoApproveRule>>;
    async fn update_auto_approve_rule(&self, rule: &AutoApproveRule) -> MvResult<bool>;

    // Undo snapshots
    async fn save_undo_snapshot(&self, snapshot: &UndoSnapshot) -> MvResult<()>;
    async fn get_undo_snapshot(&self, proposal_id: Uuid) -> MvResult<Option<UndoSnapshot>>;
    async fn mark_undo_used(&self, id: Uuid) -> MvResult<bool>;
    async fn cleanup_expired_snapshots(&self) -> MvResult<usize>;
}

fn _assert_safeguard_store_object_safe(_: &dyn SafeguardStore) {}

/// Storage for agent feedback and confidence overrides (reflection / feedback loop).
#[async_trait]
pub trait FeedbackStore: Send + Sync {
    async fn record_feedback(&self, fb: &AgentFeedback) -> MvResult<()>;
    async fn list_feedback(
        &self,
        intent_type: Option<&str>,
        limit: usize,
    ) -> MvResult<Vec<AgentFeedback>>;
    async fn get_acceptance_rate(&self, intent_type: &str) -> MvResult<(usize, usize)>;
    async fn set_confidence_override(&self, override_: &ConfidenceOverride) -> MvResult<()>;
    async fn get_confidence_override(
        &self,
        intent_type: &str,
    ) -> MvResult<Option<ConfidenceOverride>>;
    async fn list_confidence_overrides(&self) -> MvResult<Vec<ConfidenceOverride>>;
}

fn _assert_feedback_store_object_safe(_: &dyn FeedbackStore) {}

/// Storage for autonomy rules and action logs (Phase 3.1 — Autonomy & Precision Controls).
#[async_trait]
pub trait AutonomyStore: Send + Sync {
    async fn add_autonomy_rule(&self, rule: &AutonomyRule) -> MvResult<()>;
    async fn get_autonomy_rule(&self, id: Uuid) -> MvResult<Option<AutonomyRule>>;
    async fn list_autonomy_rules(&self) -> MvResult<Vec<AutonomyRule>>;
    async fn update_autonomy_rule(&self, rule: &AutonomyRule) -> MvResult<bool>;
    async fn delete_autonomy_rule(&self, id: Uuid) -> MvResult<bool>;
    async fn log_autonomy_action(&self, log: &AutonomyActionLog) -> MvResult<()>;
    async fn count_recent_actions(
        &self,
        rule_id: Option<Uuid>,
        since: DateTime<Utc>,
    ) -> MvResult<usize>;
    async fn list_autonomy_action_log(&self, limit: usize) -> MvResult<Vec<AutonomyActionLog>>;
}

fn _assert_autonomy_store_object_safe(_: &dyn AutonomyStore) {}

/// Storage for communication relay: contacts, channels, and messages.
#[async_trait]
pub trait RelayStore: Send + Sync {
    // Contacts
    async fn add_relay_contact(&self, contact: &RelayContact) -> MvResult<()>;
    async fn get_relay_contact(&self, id: Uuid) -> MvResult<Option<RelayContact>>;
    async fn list_relay_contacts(&self) -> MvResult<Vec<RelayContact>>;
    async fn update_relay_contact(&self, contact: &RelayContact) -> MvResult<bool>;
    async fn delete_relay_contact(&self, id: Uuid) -> MvResult<bool>;

    // Channels
    async fn add_relay_channel(&self, channel: &RelayChannel) -> MvResult<()>;
    async fn get_relay_channel(&self, id: Uuid) -> MvResult<Option<RelayChannel>>;
    async fn list_relay_channels(&self) -> MvResult<Vec<RelayChannel>>;
    async fn delete_relay_channel(&self, id: Uuid) -> MvResult<bool>;

    // Messages
    async fn add_relay_message(&self, message: &RelayMessage) -> MvResult<()>;
    async fn get_relay_message(&self, id: Uuid) -> MvResult<Option<RelayMessage>>;
    async fn list_relay_messages(
        &self,
        channel_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<RelayMessage>>;
    async fn update_message_status(&self, id: Uuid, status: MessageStatus) -> MvResult<bool>;
    /// Bind or clear the optional vault knowledge node for a relay message.
    ///
    /// Communication storage is independent of canonical knowledge. A vault
    /// node ID is set only after explicit or policy-approved promotion.
    async fn bind_relay_message_vault_node(
        &self,
        message_id: Uuid,
        vault_node_id: Option<Uuid>,
    ) -> MvResult<bool>;
    async fn update_relay_message_metadata(
        &self,
        message_id: Uuid,
        metadata: &std::collections::HashMap<String, serde_json::Value>,
    ) -> MvResult<bool>;
    async fn list_thread_messages(
        &self,
        thread_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<RelayMessage>>;
    async fn count_unread_messages(&self, channel_id: Option<Uuid>) -> MvResult<usize>;
}

fn _assert_relay_store_object_safe(_: &dyn RelayStore) {}

// Legacy aliases for backward compatibility with proactive.rs
pub trait InsightStore: AgenticStore {}
impl<T: AgenticStore> InsightStore for T {}

/// Storage for knowledge conflict alerts.
#[async_trait]
pub trait ConflictStore: Send + Sync {
    async fn insert_conflict(&self, alert: &ConflictAlert) -> MvResult<()>;
    async fn get_conflict(&self, id: Uuid) -> MvResult<Option<ConflictAlert>>;
    async fn list_conflicts(
        &self,
        resolved: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ConflictAlert>>;
    async fn resolve_conflict(&self, id: Uuid) -> MvResult<bool>;
}

fn _assert_conflict_store_object_safe(_: &dyn ConflictStore) {}

/// Storage for contact identities and trust models.
#[async_trait]
pub trait ContactIdentityStore: Send + Sync {
    // Identities
    async fn add_contact_identity(&self, identity: &ContactIdentity) -> MvResult<()>;
    async fn list_contact_identities(&self, contact_id: Uuid) -> MvResult<Vec<ContactIdentity>>;
    async fn delete_contact_identity(&self, id: Uuid) -> MvResult<bool>;
    async fn verify_contact_identity(&self, id: Uuid) -> MvResult<bool>;

    // Trust models
    async fn get_trust_model(&self, contact_id: Uuid) -> MvResult<Option<TrustModel>>;
    async fn set_trust_model(&self, model: &TrustModel) -> MvResult<()>;
}

fn _assert_contact_identity_store_object_safe(_: &dyn ContactIdentityStore) {}

/// Storage for owner profile.
#[async_trait]
pub trait ProfileStore: Send + Sync {
    async fn get_profile(&self) -> MvResult<OwnerProfile>;
    async fn update_profile(&self, req: &UpdateProfileRequest) -> MvResult<OwnerProfile>;
}

fn _assert_profile_store_object_safe(_: &dyn ProfileStore) {}

/// Storage for consumer profiles (AI consumer identities).
#[async_trait]
pub trait ConsumerStore: Send + Sync {
    async fn create_consumer(&self, profile: &ConsumerProfile) -> MvResult<()>;
    async fn get_consumer(&self, id: Uuid) -> MvResult<Option<ConsumerProfile>>;
    async fn get_consumer_by_name(&self, name: &str) -> MvResult<Option<ConsumerProfile>>;
    async fn get_consumer_by_token_hash(
        &self,
        token_hash: &str,
    ) -> MvResult<Option<ConsumerProfile>>;
    async fn list_consumers(&self) -> MvResult<Vec<ConsumerProfile>>;
    async fn revoke_consumer(&self, id: Uuid) -> MvResult<bool>;
    async fn touch_consumer(&self, id: Uuid) -> MvResult<()>;
}

fn _assert_consumer_store_object_safe(_: &dyn ConsumerStore) {}

/// Storage for access policies (ABAC with default-deny).
#[async_trait]
pub trait PolicyStore: Send + Sync {
    async fn set_policy(&self, policy: &AccessPolicy) -> MvResult<()>;
    async fn get_policy(&self, id: Uuid) -> MvResult<Option<AccessPolicy>>;
    async fn get_policy_for(
        &self,
        secret_key: &str,
        consumer: &str,
    ) -> MvResult<Option<AccessPolicy>>;
    async fn list_policies(
        &self,
        secret_key: Option<&str>,
        consumer: Option<&str>,
    ) -> MvResult<Vec<AccessPolicy>>;
    async fn delete_policy(&self, id: Uuid) -> MvResult<bool>;
}

fn _assert_policy_store_object_safe(_: &dyn PolicyStore) {}

/// Storage for public share links.
#[async_trait]
pub trait ShareStore: Send + Sync {
    async fn insert_public_share(&self, share: &PublicShare) -> MvResult<()>;
    async fn get_public_share(&self, id: Uuid) -> MvResult<Option<PublicShare>>;
    async fn get_public_share_by_hash(&self, token_hash: &str) -> MvResult<Option<PublicShare>>;
    async fn list_public_shares(
        &self,
        node_id: Option<Uuid>,
        include_revoked: bool,
    ) -> MvResult<Vec<PublicShare>>;
    async fn revoke_public_share(&self, id: Uuid, revoked_at: DateTime<Utc>) -> MvResult<bool>;
}

fn _assert_share_store_object_safe(_: &dyn ShareStore) {}

/// Storage for node comments and annotations.
#[async_trait]
pub trait CommentStore: Send + Sync {
    async fn insert_comment(&self, comment: &NodeComment) -> MvResult<()>;
    async fn get_comment(&self, id: Uuid) -> MvResult<Option<NodeComment>>;
    async fn list_comments(
        &self,
        node_id: Uuid,
        include_resolved: bool,
    ) -> MvResult<Vec<NodeComment>>;
    async fn resolve_comment(&self, id: Uuid, resolved_at: DateTime<Utc>) -> MvResult<bool>;
    async fn delete_comment(&self, id: Uuid) -> MvResult<bool>;
}

fn _assert_comment_store_object_safe(_: &dyn CommentStore) {}

/// Registry for MCP connectors (marketplace catalog).
#[async_trait]
pub trait McpConnectorStore: Send + Sync {
    async fn insert_mcp_connector(&self, connector: &McpConnector) -> MvResult<()>;
    async fn get_mcp_connector(&self, id: Uuid) -> MvResult<Option<McpConnector>>;
    async fn list_mcp_connectors(
        &self,
        publisher: Option<&str>,
        verified: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<McpConnector>>;
    async fn update_mcp_connector(&self, connector: &McpConnector) -> MvResult<bool>;
    async fn delete_mcp_connector(&self, id: Uuid) -> MvResult<bool>;
}

fn _assert_mcp_connector_store_object_safe(_: &dyn McpConnectorStore) {}

/// Storage for proxy audit log entries.
#[async_trait]
pub trait ProxyAuditStore: Send + Sync {
    async fn log_proxy_audit(&self, entry: &ProxyAuditEntry) -> MvResult<()>;
    async fn update_proxy_audit(
        &self,
        id: Uuid,
        success: bool,
        sanitized: bool,
        error: Option<&str>,
        response_status: Option<i32>,
    ) -> MvResult<()>;
    async fn list_proxy_audit(
        &self,
        consumer: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ProxyAuditEntry>>;
}

fn _assert_proxy_audit_store_object_safe(_: &dyn ProxyAuditStore) {}

/// Storage for HITL approval queue entries.
#[async_trait]
pub trait ApprovalStore: Send + Sync {
    async fn create_approval(&self, request: &ApprovalRequest) -> MvResult<()>;
    async fn get_approval(&self, id: Uuid) -> MvResult<Option<ApprovalRequest>>;
    async fn list_pending_approvals(
        &self,
        consumer: Option<&str>,
    ) -> MvResult<Vec<ApprovalRequest>>;
    async fn decide_approval(
        &self,
        id: Uuid,
        approved: bool,
        decided_by: Option<&str>,
        deny_reason: Option<&str>,
    ) -> MvResult<bool>;
    async fn expire_approvals(&self) -> MvResult<usize>;
    /// Check if there's an approved (non-expired) approval for a consumer+secret pair.
    async fn find_active_approval(
        &self,
        consumer: &str,
        secret_key: &str,
    ) -> MvResult<Option<ApprovalRequest>>;
}

fn _assert_approval_store_object_safe(_: &dyn ApprovalStore) {}

/// Cross-encoder reranker for improving search result ordering.
///
/// Takes a query and a set of candidate documents, returns relevance scores.
/// Implementations may use ONNX models, LLM-based scoring, or heuristics.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Score each (query, document) pair. Returns scores in the same order as documents.
    async fn rerank(&self, query: &str, documents: &[String]) -> MvResult<Vec<f64>>;

    /// Name of the reranker for logging/diagnostics.
    fn name(&self) -> &str;

    /// Whether the reranker is ready (model loaded, etc.).
    fn is_ready(&self) -> bool;
}

fn _assert_reranker_object_safe(_: &dyn Reranker) {}

/// Session memory store for conversation context in follow-up queries.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Record a query+result turn in the session.
    async fn add_turn(&self, session_id: &str, query: &str, result_summary: &str) -> MvResult<()>;

    /// Get recent turns for a session.
    async fn get_turns(&self, session_id: &str, limit: usize) -> MvResult<Vec<(String, String)>>;

    /// Clear a session.
    async fn clear_session(&self, session_id: &str) -> MvResult<()>;

    /// Expire sessions older than the given duration.
    async fn expire_sessions(&self, max_age_secs: u64) -> MvResult<usize>;
}

fn _assert_session_store_object_safe(_: &dyn SessionStore) {}

/// Conversation store for persistent multi-turn dialogues.
#[async_trait]
pub trait ConversationStore: Send + Sync {
    async fn create_conversation(&self, id: Uuid, title: Option<&str>) -> MvResult<()>;

    async fn add_message(
        &self,
        conversation_id: Uuid,
        role: &str,
        content: &str,
        sources_json: Option<&str>,
    ) -> MvResult<Uuid>;

    /// Returns `(id, role, content, sources_json, created_at)`.
    async fn get_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<(Uuid, String, String, Option<String>, DateTime<Utc>)>>;

    async fn delete_conversation(&self, id: Uuid) -> MvResult<bool>;

    async fn list_conversations(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<(Uuid, Option<String>, DateTime<Utc>)>>;

    async fn expire_conversations(&self, max_age_secs: u64) -> MvResult<usize>;
}

fn _assert_conversation_store_object_safe(_: &dyn ConversationStore) {}

/// Adapter poll state for cursor persistence across adapter polling cycles.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdapterPollState {
    pub adapter_name: String,
    pub cursor: String,
    pub last_poll_at: String,
    pub messages_received: u64,
}

/// Storage for adapter poll state (cursor persistence).
#[async_trait]
pub trait AdapterPollStore: Send + Sync {
    async fn get_poll_state(&self, adapter_name: &str) -> MvResult<Option<AdapterPollState>>;
    async fn upsert_poll_state(
        &self,
        adapter_name: &str,
        cursor: &str,
        messages_received: u64,
    ) -> MvResult<()>;
    async fn list_poll_states(&self) -> MvResult<Vec<AdapterPollState>>;
    async fn delete_poll_state(&self, adapter_name: &str) -> MvResult<bool>;
}

fn _assert_adapter_poll_store_object_safe(_: &dyn AdapterPollStore) {}

/// Persistence boundary for the managed knowledge-workspace manifest.
///
/// This contract intentionally has no filesystem mutation methods. Canonical
/// file writes belong to the separately authorized workspace mutation service.
#[async_trait]
pub trait KnowledgeWorkspaceManifestStore: Send + Sync {
    async fn insert_knowledge_workspace(&self, workspace: &KnowledgeWorkspace) -> MvResult<()>;
    async fn get_knowledge_workspace(&self, id: Uuid) -> MvResult<Option<KnowledgeWorkspace>>;
    async fn list_knowledge_workspaces(
        &self,
        namespace: Option<&str>,
    ) -> MvResult<Vec<KnowledgeWorkspace>>;

    /// Replace a workspace record only when its persisted revision matches
    /// `expected_revision`. The replacement must advance the revision by one.
    async fn update_knowledge_workspace(
        &self,
        workspace: &KnowledgeWorkspace,
        expected_revision: u64,
    ) -> MvResult<bool>;

    async fn insert_workspace_document(
        &self,
        document: &KnowledgeWorkspaceDocument,
    ) -> MvResult<()>;
    async fn get_workspace_document(
        &self,
        id: Uuid,
    ) -> MvResult<Option<KnowledgeWorkspaceDocument>>;
    async fn get_workspace_document_by_path_token(
        &self,
        workspace_id: Uuid,
        path_token: &str,
    ) -> MvResult<Option<KnowledgeWorkspaceDocument>>;
    async fn list_workspace_documents(
        &self,
        workspace_id: Uuid,
    ) -> MvResult<Vec<KnowledgeWorkspaceDocument>>;

    /// Replace a document record only when its persisted revision matches
    /// `expected_revision`. The replacement must advance the revision by one.
    async fn update_workspace_document(
        &self,
        document: &KnowledgeWorkspaceDocument,
        expected_revision: u64,
    ) -> MvResult<bool>;

    /// Apply one authoritative reconciliation as a single transaction.
    ///
    /// Returns `false` without committing any mutation when the workspace or
    /// any document revision is stale.
    async fn apply_workspace_reconciliation(
        &self,
        reconciliation: &WorkspaceManifestReconciliation,
    ) -> MvResult<bool>;

    /// Append one durable journal row. Assigns the next `event_seq` for the
    /// workspace and returns the persisted record.
    async fn append_workspace_event(&self, event: &WorkspaceEvent) -> MvResult<WorkspaceEvent>;

    async fn get_workspace_event(&self, id: Uuid) -> MvResult<Option<WorkspaceEvent>>;

    async fn list_workspace_events(
        &self,
        workspace_id: Uuid,
        after_seq: Option<u64>,
        limit: usize,
    ) -> MvResult<Vec<WorkspaceEvent>>;

    async fn list_workspace_events_by_correlation(
        &self,
        correlation_id: Uuid,
    ) -> MvResult<Vec<WorkspaceEvent>>;

    async fn insert_workspace_conflict(&self, conflict: &WorkspaceConflict) -> MvResult<()>;

    async fn get_workspace_conflict(&self, id: Uuid) -> MvResult<Option<WorkspaceConflict>>;

    async fn list_workspace_conflicts(
        &self,
        workspace_id: Uuid,
        open_only: bool,
    ) -> MvResult<Vec<WorkspaceConflict>>;
}

fn _assert_knowledge_workspace_manifest_store_object_safe(_: &dyn KnowledgeWorkspaceManifestStore) {
}

/// One authorization question: who is asking, to do what, against which target,
/// under which handling class, at what moment.
///
/// Grouped into a single type rather than passed as seven positional arguments
/// because every field is load-bearing and several share a type — two
/// `StableUri` values and two handling classes. Positionally, transposing
/// `grantee` and `target`, or `sensitivity` and `retention`, still compiles and
/// silently asks a different question of a fail-closed authorization check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantQuery<'a> {
    /// The principal whose authority is being resolved.
    pub grantee: &'a StableUri,
    /// Context grants authorize reads; Tool grants authorize effects. The two
    /// capability sets are disjoint.
    pub kind: AuthorityGrantKind,
    /// Exact stable-URI target. Wildcards and prefixes are not grant targets.
    pub target: &'a StableUri,
    pub capability: ContextCapability,
    pub sensitivity: Sensitivity,
    pub retention: RetentionClass,
    /// Evaluation instant, checked against the grant's validity window.
    pub at: DateTime<Utc>,
}

/// Atomic persistence boundary for interoperable mutations.
///
/// Implementations must commit the domain mutation and event envelope in the
/// same transaction. A replay with the same source, principal, and idempotency
/// key returns the original result; reusing that scope with a different payload
/// digest fails.
#[async_trait]
pub trait InteroperabilityStore: Send + Sync {
    /// Return the persistent identity of this local MindVault context node.
    async fn local_context_node_id(&self) -> MvResult<Uuid>;

    /// Atomically register a governed Context Node descriptor and its event.
    async fn commit_context_node_with_event(
        &self,
        context_node: &ContextNodeRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentContextNodeCommit>;

    async fn get_context_node(&self, node_id: Uuid) -> MvResult<Option<ContextNodeRecord>>;

    async fn list_context_nodes(
        &self,
        status: Option<ContextNodeStatus>,
    ) -> MvResult<Vec<ContextNodeRecord>>;

    /// Replace an existing descriptor while preserving stable identity and
    /// lifecycle state.
    async fn update_context_node_descriptor_with_event(
        &self,
        expected_revision: u64,
        replacement: &ContextNodeRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentContextNodeCommit>;

    /// Apply one allowed lifecycle transition without changing the descriptor.
    async fn transition_context_node_with_event(
        &self,
        expected_revision: u64,
        replacement: &ContextNodeRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentContextNodeCommit>;

    /// Atomically issue one active Context or Tool Grant and its event.
    async fn commit_authority_grant_with_event(
        &self,
        grant: &AuthorityGrant,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAuthorityGrantCommit>;

    async fn get_authority_grant(&self, grant_id: Uuid) -> MvResult<Option<AuthorityGrant>>;

    async fn list_authority_grants(
        &self,
        grantee: Option<&StableUri>,
        kind: Option<AuthorityGrantKind>,
        status: Option<AuthorityGrantStatus>,
    ) -> MvResult<Vec<AuthorityGrant>>;

    /// Resolve one effective grant, including its complete parent chain.
    async fn find_authorizing_grant(
        &self,
        query: GrantQuery<'_>,
    ) -> MvResult<Option<AuthorityGrant>>;

    /// Apply one lifecycle transition without changing immutable grant terms.
    async fn transition_authority_grant_with_event(
        &self,
        expected_revision: u64,
        replacement: &AuthorityGrant,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAuthorityGrantCommit>;

    /// Persist one immutable command-admission decision.
    ///
    /// Idempotent on `(principal, idempotency_key)`. A matching digest replays;
    /// a conflicting digest is `IdempotencyConflict`.
    async fn commit_command_admission_decision(
        &self,
        record: &CommandAdmissionDecisionRecord,
    ) -> MvResult<IdempotentAdmissionDecisionCommit>;

    async fn get_command_admission_decision(
        &self,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
    ) -> MvResult<Option<CommandAdmissionDecisionRecord>>;

    /// Insert or replace (revision+1) a governed identity record.
    async fn upsert_identity_record(&self, record: &IdentityRecord) -> MvResult<IdentityRecord>;

    async fn get_identity_record(&self, principal_id: Uuid) -> MvResult<Option<IdentityRecord>>;

    async fn get_identity_by_principal_uri(
        &self,
        principal_uri: &StableUri,
    ) -> MvResult<Option<IdentityRecord>>;

    /// Resolve a principal by governing node + external auth subject.
    async fn resolve_identity_by_subject(
        &self,
        governing_node_uri: &StableUri,
        external_subject: &str,
    ) -> MvResult<Option<IdentityRecord>>;

    async fn list_identity_records(
        &self,
        status: Option<IdentityStatus>,
    ) -> MvResult<Vec<IdentityRecord>>;

    /// Atomically register an immutable public schema and its event.
    async fn commit_public_schema_with_event(
        &self,
        schema: &PublicSchemaRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentSchemaCommit>;

    async fn get_public_schema(
        &self,
        reference: &SchemaReference,
    ) -> MvResult<Option<PublicSchemaRecord>>;

    async fn list_public_schema_versions(
        &self,
        schema_uri: &StableUri,
    ) -> MvResult<Vec<PublicSchemaRecord>>;

    /// Atomically register one active source binding and its event.
    async fn commit_source_binding_with_event(
        &self,
        binding: &SourceBinding,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentSourceBindingCommit>;

    async fn get_source_binding(&self, binding_id: Uuid) -> MvResult<Option<SourceBinding>>;

    async fn find_active_source_binding(
        &self,
        context_node: &StableUri,
        external_account_id: &str,
        external_object_id: &str,
    ) -> MvResult<Option<SourceBinding>>;

    /// Retire one active binding and install its explicit successor atomically.
    async fn rebind_source_with_event(
        &self,
        previous_binding_id: Uuid,
        replacement: &SourceBinding,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentSourceBindingCommit>;

    /// Atomically create a knowledge node and enqueue its event envelope.
    async fn commit_node_create_with_event(
        &self,
        node: &KnowledgeNode,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentNodeCommit>;

    /// Resolve a previously committed node-create command before mutable
    /// admission checks such as quotas are applied.
    async fn find_node_create_replay(
        &self,
        source: &StableUri,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
        payload_digest: &str,
    ) -> MvResult<Option<IdempotentNodeCommit>>;

    async fn get_outbox_event(&self, event_id: Uuid) -> MvResult<Option<EventEnvelope>>;

    /// Return all nonterminal events in stable creation order for inspection.
    ///
    /// This administrative view includes scheduled and currently leased
    /// events. Dispatchers must use `claim_outbox_events`.
    async fn list_pending_outbox_events(&self, limit: usize) -> MvResult<Vec<EventEnvelope>>;

    async fn get_outbox_delivery_status(
        &self,
        event_id: Uuid,
    ) -> MvResult<Option<OutboxDeliveryStatus>>;

    /// Atomically lease dispatchable events to one executor and destination.
    async fn claim_outbox_events(
        &self,
        executor: &StableUri,
        destination: &StableUri,
        claimed_at: DateTime<Utc>,
        lease_expires_at: DateTime<Utc>,
        limit: usize,
    ) -> MvResult<Vec<OutboxDeliveryClaim>>;

    /// Atomically complete one lease and append its immutable action receipt.
    async fn complete_outbox_delivery(
        &self,
        claim: &OutboxDeliveryClaim,
        completion: &OutboxDeliveryCompletion,
    ) -> MvResult<ActionReceipt>;

    async fn get_action_receipt(&self, receipt_id: Uuid) -> MvResult<Option<ActionReceipt>>;

    async fn list_action_receipts(
        &self,
        event_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<ActionReceipt>>;

    /// Durably admit one event for a consumer. An exact redelivery returns the
    /// original local sequence with `replayed = true`; the same event ID with
    /// different envelope content fails closed.
    async fn admit_consumer_event(
        &self,
        consumer: &StableUri,
        event: &EventEnvelope,
        received_at: DateTime<Utc>,
    ) -> MvResult<ConsumerInboxAdmission>;

    async fn get_consumer_inbox_status(
        &self,
        consumer: &StableUri,
        event_id: Uuid,
    ) -> MvResult<Option<ConsumerInboxStatus>>;

    /// Claim at most one eligible event from each consumer/source stream.
    /// Local admission order is preserved within a stream.
    async fn claim_consumer_events(
        &self,
        consumer: &StableUri,
        processor: &StableUri,
        claimed_at: DateTime<Utc>,
        lease_expires_at: DateTime<Utc>,
        limit: usize,
    ) -> MvResult<Vec<ConsumerInboxClaim>>;

    async fn complete_consumer_event(
        &self,
        claim: &ConsumerInboxClaim,
        completion: &ConsumerApplicationCompletion,
    ) -> MvResult<ConsumerApplicationReceipt>;

    async fn get_consumer_checkpoint(
        &self,
        consumer: &StableUri,
        source: &StableUri,
    ) -> MvResult<Option<ConsumerCheckpoint>>;

    async fn get_consumer_application_receipt(
        &self,
        receipt_id: Uuid,
    ) -> MvResult<Option<ConsumerApplicationReceipt>>;

    async fn list_consumer_application_receipts(
        &self,
        consumer: &StableUri,
        event_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<ConsumerApplicationReceipt>>;

    // -----------------------------------------------------------------------
    // Governed agent execution graph
    //
    // Contract: `docs/architecture/WORK_ORDER_MODEL.md`.
    // Isolation: `docs/architecture/EXECUTION_ISOLATION_MODEL.md`.
    // -----------------------------------------------------------------------

    /// Atomically admit one Work Order with its node contracts, derived
    /// conflict edges, authored edges, and event.
    ///
    /// Admission is all-or-nothing: a Work Order is never partially admitted.
    /// The caller must have resolved every node's declared write scope against
    /// an effective Tool Grant first; this method records the outcome and does
    /// not itself authorize scope.
    async fn commit_work_order_with_event(
        &self,
        work_order: &WorkOrder,
        nodes: &[WorkOrderNode],
        edges: &[WorkOrderEdge],
        event: &EventEnvelope,
    ) -> MvResult<IdempotentWorkOrderCommit>;

    async fn get_work_order(&self, work_order_id: Uuid) -> MvResult<Option<WorkOrder>>;

    async fn list_work_orders(
        &self,
        status: Option<WorkOrderStatus>,
        limit: usize,
    ) -> MvResult<Vec<WorkOrder>>;

    async fn list_work_order_nodes(&self, work_order_id: Uuid) -> MvResult<Vec<WorkOrderNode>>;

    async fn list_work_order_edges(&self, work_order_id: Uuid) -> MvResult<Vec<WorkOrderEdge>>;

    /// Apply one allowed Work Order lifecycle transition.
    async fn transition_work_order_with_event(
        &self,
        expected_revision: u64,
        replacement: &WorkOrder,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentWorkOrderCommit>;

    /// Start one bounded execution attempt, decrementing the Work Order budget
    /// in the same immediate transaction as the run insertion and its event.
    ///
    /// A `CHECK` constraint prevents a negative balance; only this shared
    /// transaction prevents two concurrent runs from both passing a read-time
    /// check and both committing.
    async fn commit_agent_run_with_event(
        &self,
        run: &AgentRun,
        spend: &WorkOrderSpend,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAgentRunCommit>;

    async fn get_agent_run(&self, run_id: Uuid) -> MvResult<Option<AgentRun>>;

    async fn list_agent_runs(&self, work_order_id: Uuid, limit: usize) -> MvResult<Vec<AgentRun>>;

    /// Apply one allowed run lifecycle transition.
    ///
    /// Entering `awaiting_approval` releases every write lease the run holds,
    /// because approval is unbounded and a parked run must not block others.
    async fn transition_agent_run_with_event(
        &self,
        run: &AgentRun,
        release_reason: Option<LeaseReleaseReason>,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAgentRunCommit>;

    /// Atomically claim exclusive write leases for every declared target.
    ///
    /// All-or-nothing: a partial claim would let a run begin writing part of
    /// its scope while another run holds the rest. Expired leases on the same
    /// targets are released and replaced with an advancing attempt number.
    async fn claim_write_leases(
        &self,
        run_id: Uuid,
        target_digests: &[String],
        claimed_at: DateTime<Utc>,
        lease_expires_at: DateTime<Utc>,
    ) -> MvResult<Vec<WriteLease>>;

    async fn release_write_leases(
        &self,
        run_id: Uuid,
        reason: LeaseReleaseReason,
        released_at: DateTime<Utc>,
    ) -> MvResult<usize>;

    async fn list_write_leases(&self, run_id: Uuid) -> MvResult<Vec<WriteLease>>;

    /// Target digests currently held by an unreleased, unexpired lease held by
    /// some run other than `excluding_run`. Used to evaluate conflict edges.
    async fn conflicting_write_targets(
        &self,
        excluding_run: Uuid,
        target_digests: &[String],
        at: DateTime<Utc>,
    ) -> MvResult<Vec<String>>;

    /// Record immutable gate evidence. A run can never satisfy its own G5.
    async fn record_gate_result(&self, result: &GateResult) -> MvResult<GateResult>;

    async fn list_gate_results(&self, run_id: Uuid) -> MvResult<Vec<GateResult>>;

    /// Record an immutable artifact and its provenance references.
    async fn record_run_artifact(
        &self,
        artifact: &RunArtifact,
        payload: &[u8],
    ) -> MvResult<RunArtifact>;

    async fn get_run_artifact(&self, artifact_id: Uuid) -> MvResult<Option<RunArtifact>>;

    async fn list_run_artifacts(&self, work_order_id: Uuid) -> MvResult<Vec<RunArtifact>>;

    /// Read back artifact content, verifying it against the recorded digest.
    ///
    /// Without this, [`Self::record_run_artifact`] would be write-only: gate G2
    /// could compare a recorded digest against a recorded digest but never
    /// against retrievable content, and a portable export could not round-trip
    /// the bytes it claims to carry.
    ///
    /// Implementations MUST re-verify the digest on read rather than trusting
    /// the stored column, so silent corruption surfaces as an error instead of
    /// as a passing gate.
    async fn read_run_artifact_payload(&self, artifact_id: Uuid) -> MvResult<Option<Vec<u8>>>;

    /// Restore a complete exported Work Order graph in one transaction.
    ///
    /// `artifact_payloads` holds the decoded bytes for `export.artifacts`, in
    /// the same order; the caller verifies each against its recorded digest
    /// before calling.
    ///
    /// All-or-nothing by construction: a partially restored graph would leave
    /// gate evidence referring to runs that do not exist, which is worse than
    /// no restore at all.
    async fn restore_work_order_graph(
        &self,
        export: &WorkOrderExport,
        artifact_payloads: &[Vec<u8>],
    ) -> MvResult<()>;
}

fn _assert_interoperability_store_object_safe(_: &dyn InteroperabilityStore) {}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure traits are object-safe
    fn _assert_node_store_object_safe(_: &dyn NodeStore) {}
    fn _assert_vector_store_object_safe(_: &dyn VectorStore) {}
    fn _assert_full_text_index_object_safe(_: &dyn FullTextIndex) {}
    fn _assert_graph_store_object_safe(_: &dyn GraphStore) {}
    fn _assert_embedder_object_safe(_: &dyn Embedder) {}
}
