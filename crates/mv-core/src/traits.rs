use async_trait::async_trait;
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
    async fn upsert(&self, id: Uuid, embedding: Vec<f32>, content: &str) -> MvResult<()>;
    async fn search(
        &self,
        embedding: Vec<f32>,
        limit: usize,
        min_score: f64,
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
    async fn list_audit_entries(&self, limit: usize, offset: usize) -> MvResult<Vec<KeychainAuditEntry>>;
    async fn get_latest_audit_entry(&self) -> MvResult<Option<KeychainAuditEntry>>;
    async fn verify_audit_chain(&self) -> MvResult<bool>;

    // --- Breach Detection ---
    async fn record_access_pattern(&self, pattern: &AccessPattern) -> MvResult<()>;
    async fn get_access_patterns(&self, credential_id: Uuid, limit: usize) -> MvResult<Vec<AccessPattern>>;
    async fn insert_breach_alert(&self, alert: &BreachAlert) -> MvResult<()>;
    async fn list_breach_alerts(&self, limit: usize, offset: usize) -> MvResult<Vec<BreachAlert>>;
    async fn acknowledge_breach_alert(&self, id: Uuid) -> MvResult<()>;

    // --- Tags ---
    async fn get_credential_tags(&self, credential_id: Uuid) -> MvResult<Vec<String>>;
    async fn save_credential_tags(&self, credential_id: Uuid, tags: &[String]) -> MvResult<()>;
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

// Legacy aliases for backward compatibility with proactive.rs
pub trait InsightStore: AgenticStore {}
impl<T: AgenticStore> InsightStore for T {}

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
