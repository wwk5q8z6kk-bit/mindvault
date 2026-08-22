mod access_ops;
mod agent_run_executor;
mod consumer_ops;
mod graph_ops;
mod intent_ops;
mod interoperability_ops;
mod mcp_ops;
mod node_ops;
mod outbox_dispatch;
mod profile_ops;
mod proposal_ops;
mod relay_ops;
mod search_ops;
mod security_ops;
mod social_ops;
mod sync_ops;
mod work_order_ops;
mod workspace_ops;
mod workspace_projection_ops;

pub use interoperability_ops::{
    AuthorityGrantIssuance, AuthorityGrantTransition, IssueAuthorityGrantRequest,
    LocalContextNodeRegistration,
};
pub use outbox_dispatch::{
    spawn_outbox_dispatcher, HttpOutboxPublisher, LocalAckPublisher, OutboxDispatchTick,
    OutboxDispatcherConfig, OutboxPublisher,
};
pub use agent_run_executor::ExecutedRun;
pub use work_order_ops::{
    AdmissionRefusal, ProposedEdge, ProposedNode, ProposedWorkOrder, RunReadiness,
};
pub use workspace_ops::WorkspaceMountResult;
pub use workspace_projection_ops::WorkspaceProjectionOutcome;

use std::path::PathBuf;
use std::sync::Arc;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use mv_core::credentials::CredentialStore;
use mv_core::*;
use mv_graph::store::SqliteGraphStore;
use mv_index::tantivy_index::TantivyFullTextIndex;
use mv_storage::unified::UnifiedStore;
use mv_storage::vector::{KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder, OpenAiEmbedder};
use rand::RngCore;
use reqwest::StatusCode as HttpStatusCode;
use sha2::{Digest, Sha256};
use url::form_urlencoded::byte_serialize;
use uuid::Uuid;

use crate::config::EngineConfig;
use crate::ingest::IngestPipeline;
use crate::llm::{self, LlmProvider};
use crate::recall::RecallPipeline;
use crate::recurrence::RECURRING_INSTANCE_METADATA_KEY;

// ── Constants ────────────────────────────────────────────────────────

const DAILY_NOTE_TAG: &str = "daily-note";
const PROFILE_RELAY_CONTACT_ID_KEY: &str = "relay_contact_id";
const PROFILE_OWNER_CONTACT_NOTES: &str = "owner";
const DAILY_LINK_CANDIDATE_TAGS: &[&str] = &[
    "task", "tasks", "todo", "to-do", "event", "events", "meeting", "reminder",
];
const AUTO_BACKLINK_METADATA_KEY: &str = "auto_backlink";
const AUTO_BACKLINK_SOURCE_METADATA_KEY: &str = "source";
const TASK_AI_PRIORITY_METADATA_KEY: &str = "ai_priority";
const TASK_PRIORITY_METADATA_KEY: &str = "task_priority";
const TASK_PRIORITY_ALT_METADATA_KEY: &str = "priority";
const TASK_STATUS_METADATA_KEY: &str = "task_status";
const TASK_STATUS_ALT_METADATA_KEY: &str = "status";
const TASK_ESTIMATE_MINUTES_METADATA_KEY: &str = "task_estimate_minutes";
const TASK_ESTIMATE_MINUTES_ALT_METADATA_KEY: &str = "task_estimate_min";
const TASK_ESTIMATE_MIN_METADATA_KEY: &str = "estimate_min";
const SEALED_BLOB_MAGIC: &[u8; 4] = b"MVB1";
pub(crate) const WORKSPACE_PROJECTION_METADATA_KEY: &str = "mindvault.workspace_projection";

// ── Public Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus {
    pub configured_provider: String,
    pub configured_model: String,
    pub configured_dimensions: usize,
    pub effective_provider: String,
    pub effective_model: String,
    pub effective_dimensions: usize,
    pub fallback_to_noop: bool,
    pub reason: Option<String>,
    pub local_embeddings_feature_enabled: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TaskRecurrenceRollforwardStats {
    pub scanned_tasks: usize,
    pub recurring_templates: usize,
    pub generated_instances: usize,
    pub updated_templates: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TaskReminderDispatchStats {
    pub scanned_tasks: usize,
    pub due_tasks: usize,
    pub reminders_marked_sent: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GoogleCalendarSyncReport {
    pub calendar_id: String,
    pub fetched: usize,
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub skipped: usize,
    pub exported_created: usize,
    pub exported_updated: usize,
    pub next_sync_token: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RelayInboundOutcome {
    pub message: RelayMessage,
    pub auto_reply: Option<RelayMessage>,
    pub proposal_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
struct RelayReplySuggestion {
    content: String,
    confidence: f32,
    context_snippets: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrioritizedTask {
    pub task: KnowledgeNode,
    pub score: f64,
    pub rank: usize,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct TaskPrioritizationOptions {
    pub namespace: Option<String>,
    pub limit: usize,
    pub include_completed: bool,
    pub include_without_due: bool,
    pub persist: bool,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct TaskPriorityCandidate {
    task: KnowledgeNode,
    score: f64,
    reason: String,
    due_at: Option<DateTime<Utc>>,
}

struct EmbeddingProviderSelection {
    embedder: Option<Arc<dyn Embedder>>,
    vector_dimensions: usize,
    runtime_status: KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus,
}

/// Result of executing a proposal action.
#[derive(Debug, Default)]
pub struct ProposalActionResult {
    pub created_node_id: Option<Uuid>,
    pub updated_node_id: Option<Uuid>,
    pub deleted_node_id: Option<Uuid>,
    pub affected_namespace: Option<String>,
}

/// Result of applying an undo snapshot.
#[derive(Debug)]
pub struct UndoActionResult {
    pub action: String,
}

// ── Engine struct ────────────────────────────────────────────────────

/// The MindVault engine — orchestrates storage, indexing, and search.
pub struct MindVaultEngine {
    pub(crate) ingest: IngestPipeline,
    pub recall: RecallPipeline,
    pub store: Arc<UnifiedStore>,
    pub(crate) fts: Arc<TantivyFullTextIndex>,
    pub graph: Arc<SqliteGraphStore>,
    pub config: EngineConfig,
    pub credential_store: Arc<CredentialStore>,
    pub keychain: Arc<crate::keychain::KeychainEngine>,
    pub llm: Option<Arc<dyn LlmProvider>>,
    pub proactive: Arc<crate::proactive::ProactiveEngine>,
    pub(crate) enrichment: Option<crate::enrichment::EnrichmentPipeline>,
    pub reflection: crate::reflection::ReflectionEngine,
    pub autonomy: crate::autonomy::AutonomyGate,
    pub relay: crate::relay::RelayEngine,
    pub adapters: crate::adapters::AdapterRegistry,
    pub multimodal: crate::multimodal::MultiModalPipeline,
    pub sync: crate::sync::SyncEngine,
    pub federation: crate::federation::FederationEngine,
    pub metrics: crate::metrics_collector::MetricsCollector,
    pub insight: Arc<crate::insight::InsightEngine>,
    embedding_runtime_status: KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus,
}

impl MindVaultEngine {
    /// Initialize the engine from configuration.
    pub async fn init(config: EngineConfig) -> MvResult<Self> {
        let data_dir = PathBuf::from(&config.data_dir);
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| MvError::Storage(format!("create data dir: {e}")))?;

        // Base credential store (OS keyring + env) for KeychainEngine's macOS bridge
        let bridge_cred_store = Arc::new(CredentialStore::new("mindvault"));

        // Initialize keychain engine
        let keychain_path = data_dir.join("keychain.sqlite");
        let keychain_store: Arc<dyn mv_core::traits::KeychainStore> = Arc::new(
            mv_storage::keychain::SqliteKeychainStore::open(&keychain_path)
                .map_err(|e| MvError::Storage(format!("open keychain db: {e}")))?,
        );
        let keychain = Arc::new(
            crate::keychain::KeychainEngine::new(
                keychain_store,
                Arc::clone(&bridge_cred_store),
                None,
                Some(keychain_path.clone()),
            )
            .await?,
        );

        // Build the main credential store with Sovereign Keychain as highest-priority backend.
        // Resolution chain: Sovereign Keychain → OS Keyring → Environment Variables.
        let credential_store = {
            let mut store = CredentialStore::new("mindvault");
            store.insert_backend(
                0,
                Box::new(crate::keychain_backend::KeychainBackend::new(
                    Arc::clone(&keychain),
                    tokio::runtime::Handle::current(),
                )),
            );
            Arc::new(store)
        };

        let selection = select_embedding_provider(&config, &credential_store);

        // Initialize unified store
        let mut store = UnifiedStore::open_with_mode(
            &data_dir,
            selection.vector_dimensions,
            config.sealed_mode,
        )
        .await?;
        if let Some(embedder) = selection.embedder {
            store = store.with_embedder(embedder);
        }

        let store = Arc::new(store);

        let tantivy_path = if config.sealed_mode {
            data_dir.join("tantivy.sealed")
        } else {
            data_dir.join("tantivy")
        };
        let fts = Arc::new(TantivyFullTextIndex::open_with_mode(
            &tantivy_path,
            config.sealed_mode,
        )?);

        // Initialize graph store (shares SQLite connection via separate connection)
        let graph_conn = rusqlite::Connection::open(data_dir.join("mindvault.sqlite"))
            .map_err(|e| MvError::Graph(format!("open graph db: {e}")))?;
        graph_conn
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
            )
            .map_err(|e| MvError::Graph(format!("graph pragma: {e}")))?;
        let graph = Arc::new(SqliteGraphStore::open(graph_conn)?);

        let ingest = IngestPipeline::new(
            Arc::clone(&store),
            Arc::clone(&fts),
            Arc::clone(&graph),
            config.clone(),
        );

        // Initialize LLM provider (optional — heuristic fallback when disabled)
        let llm_api_key = credential_store
            .get_secret_string("MINDVAULT_LLM_API_KEY")
            .or_else(|| credential_store.get_secret_string("OPENAI_API_KEY"));
        let llm =
            llm::init_llm_provider_with_local(&config.llm, &config.local_llm, llm_api_key).await;

        let recall = RecallPipeline::new(
            Arc::clone(&store),
            Arc::clone(&fts),
            Arc::clone(&graph),
            config.clone(),
            llm.clone(),
        );

        // Initialize proactive engine (will be wired to engine after construction)
        let proactive = Arc::new(crate::proactive::ProactiveEngine::new());

        let reflection = crate::reflection::ReflectionEngine::new(Arc::clone(&store));
        let autonomy = crate::autonomy::AutonomyGate::new(Arc::clone(&store));
        let relay = crate::relay::RelayEngine::new(Arc::clone(&store));
        let federation = crate::federation::FederationEngine::new(Arc::clone(&store));
        let sync = crate::sync::SyncEngine::new(Arc::clone(&store), Uuid::now_v7().to_string());

        let engine = Self {
            ingest,
            recall,
            store,
            fts,
            graph,
            config,
            credential_store,
            keychain,
            llm,
            proactive,
            enrichment: None,
            reflection,
            autonomy,
            relay,
            adapters: crate::adapters::AdapterRegistry::new(),
            sync,
            federation,
            multimodal: {
                let mut pipeline = crate::multimodal::MultiModalPipeline::new();
                pipeline.register(Box::new(crate::multimodal::audio::AudioProcessor::new()));
                pipeline.register(Box::new(crate::multimodal::image::ImageProcessor::new()));
                pipeline.register(Box::new(crate::multimodal::pdf::PdfProcessor::new()));
                pipeline
            },
            metrics: crate::metrics_collector::MetricsCollector::new(),
            insight: Arc::new(crate::insight::InsightEngine::default()),
            embedding_runtime_status: selection.runtime_status,
        };

        engine.ensure_default_permission_templates().await?;

        Ok(engine)
    }

    /// Initialize the engine and return it wrapped in Arc, with proactive engine properly wired.
    /// Use this when you need proactive features.
    pub async fn init_arc(config: EngineConfig) -> MvResult<Arc<Self>> {
        let engine = Arc::new(Self::init(config).await?);
        engine.proactive.set_engine(Arc::clone(&engine));
        engine.insight.set_engine(Arc::clone(&engine));
        Ok(engine)
    }

    async fn ensure_default_permission_templates(&self) -> MvResult<()> {
        let owner_exists = self
            .store
            .nodes
            .get_permission_template_by_name("Owner")
            .await?
            .is_some();
        let assistant_exists = self
            .store
            .nodes
            .get_permission_template_by_name("Assistant")
            .await?
            .is_some();

        if !owner_exists {
            let now = Utc::now();
            let owner = PermissionTemplate {
                id: Uuid::now_v7(),
                name: "Owner".to_string(),
                description: Some("Full access template".to_string()),
                tier: PermissionTier::Admin,
                scope_namespace: None,
                scope_tags: Vec::new(),
                allow_kinds: Vec::new(),
                allow_actions: Vec::new(),
                created_at: now,
                updated_at: now,
            };
            self.store.nodes.insert_permission_template(&owner).await?;
        }

        if !assistant_exists {
            let now = Utc::now();
            let assistant = PermissionTemplate {
                id: Uuid::now_v7(),
                name: "Assistant".to_string(),
                description: Some("Scoped assistant template".to_string()),
                tier: PermissionTier::Action,
                scope_namespace: Some("assistant".to_string()),
                scope_tags: Vec::new(),
                allow_kinds: Vec::new(),
                allow_actions: Vec::new(),
                created_at: now,
                updated_at: now,
            };
            self.store
                .nodes
                .insert_permission_template(&assistant)
                .await?;
        }

        Ok(())
    }
}

// ── Private helper types and free functions ──────────────────────────

/// Deserialized proposal payload for node operations.
#[derive(serde::Deserialize)]
struct ProposalNodePayload {
    kind: Option<String>,
    content: Option<String>,
    title: Option<String>,
    source: Option<String>,
    namespace: Option<String>,
    tags: Option<Vec<String>>,
    importance: Option<f64>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

fn proposal_node_payload(
    payload: &std::collections::HashMap<String, serde_json::Value>,
) -> MvResult<ProposalNodePayload> {
    let value = serde_json::to_value(payload)
        .map_err(|e| MvError::InvalidInput(format!("invalid payload: {e}")))?;
    serde_json::from_value(value)
        .map_err(|e| MvError::InvalidInput(format!("invalid payload: {e}")))
}

/// Simple glob matching: supports `*` as wildcard prefix/suffix/full.
fn glob_match_simple(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    pattern == value
}

fn is_daily_note(node: &KnowledgeNode) -> bool {
    node.tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(DAILY_NOTE_TAG))
        || node
            .metadata
            .get("daily_note")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
}

fn is_template_node(node: &KnowledgeNode) -> bool {
    node.kind == NodeKind::Template
        || node
            .metadata
            .get("template")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        || node
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("template"))
}

fn is_recurring_instance(node: &KnowledgeNode) -> bool {
    node.metadata
        .get(RECURRING_INSTANCE_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn is_daily_link_candidate(node: &KnowledgeNode) -> bool {
    if matches!(node.kind, NodeKind::Task | NodeKind::Event) {
        return true;
    }

    node.tags.iter().any(|tag| {
        DAILY_LINK_CANDIDATE_TAGS
            .iter()
            .any(|candidate| tag.eq_ignore_ascii_case(candidate))
    })
}

fn is_auto_backlink_relationship(rel: &Relationship) -> bool {
    rel.metadata
        .get(AUTO_BACKLINK_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn select_embedding_provider(
    config: &EngineConfig,
    credentials: &CredentialStore,
) -> EmbeddingProviderSelection {
    let provider = config.embedding.provider.trim().to_ascii_lowercase();
    let configured_model = config.embedding.model.clone();
    let configured_dimensions = config.embedding.dimensions;

    let base_status =
        |effective_provider: &str,
         effective_model: String,
         effective_dimensions: usize,
         fallback_to_noop: bool,
         reason: Option<String>| KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus {
            configured_provider: config.embedding.provider.clone(),
            configured_model: configured_model.clone(),
            configured_dimensions,
            effective_provider: effective_provider.to_string(),
            effective_model,
            effective_dimensions,
            fallback_to_noop,
            reason,
            local_embeddings_feature_enabled: cfg!(feature = "local-embeddings"),
        };

    match provider.as_str() {
        "openai" => {
            let base_url = config
                .embedding
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let api_key = credentials.get_secret_string("OPENAI_API_KEY");
            if api_key.is_none() && base_url.contains("api.openai.com") {
                let reason = "OPENAI_API_KEY not found in any credential backend".to_string();
                tracing::warn!("mindvault_openai_embedder_unavailable_falling_back_to_noop");
                return EmbeddingProviderSelection {
                    embedder: None,
                    vector_dimensions: config.embedding.dimensions,
                    runtime_status: base_status(
                        "noop",
                        "noop".to_string(),
                        config.embedding.dimensions,
                        true,
                        Some(reason),
                    ),
                };
            }
            let embedder = OpenAiEmbedder::for_compatible(
                base_url,
                api_key,
                config.embedding.model.clone(),
                config.embedding.dimensions,
            );
            tracing::info!(
                provider = "openai",
                dimensions = config.embedding.dimensions,
                "mindvault_embedding_provider_initialized"
            );
            EmbeddingProviderSelection {
                embedder: Some(Arc::new(embedder)),
                vector_dimensions: config.embedding.dimensions,
                runtime_status: base_status(
                    "openai",
                    configured_model.clone(),
                    config.embedding.dimensions,
                    false,
                    None,
                ),
            }
        }
        "openai-compatible" | "openai_compatible" => {
            let base_url = config
                .embedding
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/v1".to_string());
            let api_key = credentials
                .get_secret_string("MINDVAULT_EMBEDDING_API_KEY")
                .or_else(|| credentials.get_secret_string("OPENAI_API_KEY"));
            let embedder = OpenAiEmbedder::for_compatible(
                base_url.clone(),
                api_key,
                config.embedding.model.clone(),
                config.embedding.dimensions,
            );
            tracing::info!(
                provider = "openai-compatible",
                base_url = %base_url,
                model = %config.embedding.model,
                dimensions = config.embedding.dimensions,
                "mindvault_embedding_provider_initialized"
            );
            EmbeddingProviderSelection {
                embedder: Some(Arc::new(embedder)),
                vector_dimensions: config.embedding.dimensions,
                runtime_status: base_status(
                    "openai-compatible",
                    configured_model.clone(),
                    config.embedding.dimensions,
                    false,
                    None,
                ),
            }
        }
        "ollama" => {
            let base_url = config
                .embedding
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434/v1".to_string());
            let model = if config.embedding.model.starts_with("text-embedding-") {
                "nomic-embed-text".to_string()
            } else {
                config.embedding.model.clone()
            };
            let embedder = OpenAiEmbedder::for_ollama(
                Some(base_url.clone()),
                model.clone(),
                config.embedding.dimensions,
            );
            tracing::info!(
                provider = "ollama",
                base_url = %base_url,
                model = %model,
                dimensions = config.embedding.dimensions,
                "mindvault_embedding_provider_initialized"
            );
            let reason = if model != configured_model {
                Some(format!(
                    "model '{configured_model}' auto-mapped to '{model}' for ollama"
                ))
            } else {
                None
            };
            EmbeddingProviderSelection {
                embedder: Some(Arc::new(embedder)),
                vector_dimensions: config.embedding.dimensions,
                runtime_status: base_status(
                    "ollama",
                    model,
                    config.embedding.dimensions,
                    false,
                    reason,
                ),
            }
        }
        "noop" | "none" | "disabled" => {
            tracing::info!(provider = "noop", "mindvault_embedding_provider_noop");
            EmbeddingProviderSelection {
                embedder: None,
                vector_dimensions: config.embedding.dimensions,
                runtime_status: base_status(
                    "noop",
                    "noop".to_string(),
                    config.embedding.dimensions,
                    false,
                    None,
                ),
            }
        }
        "local_fastembed" | "fastembed" | "local" => {
            let local_model = default_local_model_if_needed(&config.embedding.model);
            match KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder::try_new(&local_model) {
                Ok(embedder) => {
                    let dimensions = embedder.dimensions();
                    tracing::info!(
                        provider = "local_fastembed",
                        model = %embedder.model_name(),
                        dimensions,
                        "mindvault_embedding_provider_initialized"
                    );
                    let reason = if local_model != configured_model {
                        Some(format!(
                            "model '{configured_model}' auto-mapped to '{local_model}' for local_fastembed"
                        ))
                    } else {
                        None
                    };
                    EmbeddingProviderSelection {
                        embedder: Some(Arc::new(embedder)),
                        vector_dimensions: dimensions,
                        runtime_status: base_status(
                            "local_fastembed",
                            local_model,
                            dimensions,
                            false,
                            reason,
                        ),
                    }
                }
                Err(err) => {
                    let reason = format!("local_fastembed initialization failed: {err}");
                    tracing::warn!(
                        provider = "local_fastembed",
                        model = %local_model,
                        error = %err,
                        "mindvault_local_embedder_unavailable_falling_back_to_noop"
                    );
                    EmbeddingProviderSelection {
                        embedder: None,
                        vector_dimensions: config.embedding.dimensions,
                        runtime_status: base_status(
                            "noop",
                            "noop".to_string(),
                            config.embedding.dimensions,
                            true,
                            Some(reason),
                        ),
                    }
                }
            }
        }
        other => {
            let reason = format!("unknown embedding provider '{other}'");
            tracing::warn!(
                provider = %other,
                "mindvault_unknown_embedding_provider_falling_back_to_noop"
            );
            EmbeddingProviderSelection {
                embedder: None,
                vector_dimensions: config.embedding.dimensions,
                runtime_status: base_status(
                    "noop",
                    "noop".to_string(),
                    config.embedding.dimensions,
                    true,
                    Some(reason),
                ),
            }
        }
    }
}

fn default_local_model_if_needed(config_model: &str) -> String {
    if config_model.starts_with("text-embedding-") {
        tracing::info!(
            configured_model = %config_model,
            fallback_model = "bge-small-en-v1.5",
            "mindvault_local_embedder_model_auto_selected"
        );
        "bge-small-en-v1.5".to_string()
    } else {
        config_model.to_string()
    }
}

fn generate_access_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    format!("mvk_{encoded}")
}

fn generate_share_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    format!("mvs_{encoded}")
}

fn hash_access_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

fn hash_share_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

// ── Google Calendar helpers ──────────────────────────────────────────

#[derive(Debug)]
enum GoogleCalendarFetchError {
    SyncTokenExpired,
    RequestFailed(String),
}

#[derive(Debug, serde::Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleEventsResponse {
    items: Option<Vec<GoogleEvent>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "nextSyncToken")]
    next_sync_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleEvent {
    id: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    status: Option<String>,
    updated: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    start: Option<GoogleEventTime>,
    end: Option<GoogleEventTime>,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleEventTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

async fn google_refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> MvResult<String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| MvError::Storage(format!("google token request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(MvError::Storage(format!(
            "google token request failed ({status}): {body}"
        )));
    }

    let token = response
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|e| MvError::Storage(format!("google token response parse failed: {e}")))?;

    Ok(token.access_token)
}

async fn google_list_events(
    access_token: &str,
    calendar_id: &str,
    config: &crate::config::GoogleCalendarConfig,
    sync_token: Option<&str>,
) -> Result<(Vec<GoogleEvent>, Option<String>), GoogleCalendarFetchError> {
    let client = reqwest::Client::new();
    let encoded_calendar = byte_serialize(calendar_id.as_bytes()).collect::<String>();
    let url = format!("https://www.googleapis.com/calendar/v3/calendars/{encoded_calendar}/events");

    let max_results = config.max_results.to_string();
    let mut page_token: Option<String> = None;
    let mut events: Vec<GoogleEvent> = Vec::new();

    let next_sync_token = loop {
        let mut request = client.get(&url).bearer_auth(access_token).query(&[
            ("singleEvents", "true"),
            ("showDeleted", "true"),
            ("maxResults", max_results.as_str()),
        ]);

        if let Some(token) = sync_token {
            request = request.query(&[("syncToken", token)]);
        } else {
            let now = Utc::now();
            let time_min = (now - chrono::Duration::days(config.lookback_days)).to_rfc3339();
            let time_max = (now + chrono::Duration::days(config.lookahead_days)).to_rfc3339();
            request = request.query(&[
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
                ("orderBy", "startTime"),
            ]);
        }

        if let Some(ref token) = page_token {
            request = request.query(&[("pageToken", token.as_str())]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| GoogleCalendarFetchError::RequestFailed(e.to_string()))?;

        if response.status() == HttpStatusCode::GONE {
            return Err(GoogleCalendarFetchError::SyncTokenExpired);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(GoogleCalendarFetchError::RequestFailed(format!(
                "google events request failed ({status}): {body}"
            )));
        }

        let payload = response
            .json::<GoogleEventsResponse>()
            .await
            .map_err(|e| GoogleCalendarFetchError::RequestFailed(e.to_string()))?;

        if let Some(mut page_items) = payload.items {
            events.append(&mut page_items);
        }

        if payload.next_page_token.is_none() {
            break payload.next_sync_token;
        }

        page_token = payload.next_page_token;
    };

    Ok((events, next_sync_token))
}

fn event_source(calendar_id: &str, event_id: &str) -> String {
    format!("google-calendar:{calendar_id}:{event_id}")
}

fn parse_google_event_time(time: &GoogleEventTime) -> Option<DateTime<Utc>> {
    if let Some(ref dt) = time.date_time {
        return DateTime::parse_from_rfc3339(dt)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));
    }

    let date_str = time.date.as_ref()?;
    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn event_times(event: &GoogleEvent) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let start = event.start.as_ref().and_then(parse_google_event_time)?;
    let end = event
        .end
        .as_ref()
        .and_then(parse_google_event_time)
        .unwrap_or_else(|| start + chrono::Duration::hours(1));
    Some((start, end))
}

async fn google_export_events(
    engine: &MindVaultEngine,
    access_token: &str,
    calendar_id: &str,
    config: &crate::config::GoogleCalendarConfig,
) -> MvResult<(usize, usize)> {
    let mut exported_created = 0usize;
    let mut exported_updated = 0usize;

    let filters = QueryFilters {
        namespace: Some(config.namespace.clone()),
        kinds: Some(vec![NodeKind::Event]),
        tags: None,
        min_importance: None,
        created_after: None,
        created_before: None,
    };

    let nodes = engine.store.nodes.list(&filters, 1000, 0).await?;
    let client = reqwest::Client::new();
    let encoded_calendar = byte_serialize(calendar_id.as_bytes()).collect::<String>();
    let url = format!("https://www.googleapis.com/calendar/v3/calendars/{encoded_calendar}/events");

    for mut node in nodes {
        let start_at = node
            .metadata
            .get("event_start_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.to_rfc3339());
        let end_at = node
            .metadata
            .get("event_end_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.to_rfc3339());

        let (Some(start_at), Some(end_at)) = (start_at, end_at) else {
            continue;
        };

        let summary = node
            .title
            .clone()
            .unwrap_or_else(|| "MindVault Event".to_string());
        let description = node.content.clone();

        let mut payload = serde_json::json!({
            "summary": summary,
            "description": description,
            "start": { "dateTime": start_at },
            "end": { "dateTime": end_at }
        });

        if let Some(ref source) = node.source {
            payload["source"] = serde_json::json!({ "title": "MindVault", "url": source });
        }

        if let Some(event_id) = node
            .metadata
            .get("google_calendar_event_id")
            .and_then(|v| v.as_str())
        {
            let request = client
                .patch(format!("{url}/{event_id}"))
                .bearer_auth(access_token)
                .json(&payload);

            let response = request
                .send()
                .await
                .map_err(|e| MvError::Storage(format!("google event update failed: {e}")))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(MvError::Storage(format!(
                    "google event update failed ({status}): {body}"
                )));
            }

            exported_updated += 1;
            continue;
        }

        let response = client
            .post(&url)
            .bearer_auth(access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MvError::Storage(format!("google event create failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MvError::Storage(format!(
                "google event create failed ({status}): {body}"
            )));
        }

        let created_event = response
            .json::<GoogleEvent>()
            .await
            .map_err(|e| MvError::Storage(format!("google event response parse failed: {e}")))?;

        if let Some(event_id) = created_event.id {
            node.metadata.insert(
                "google_calendar_event_id".to_string(),
                serde_json::Value::String(event_id.clone()),
            );
            node.metadata.insert(
                "google_calendar_calendar_id".to_string(),
                serde_json::Value::String(calendar_id.to_string()),
            );
            if let Some(html_link) = created_event.html_link {
                node.metadata.insert(
                    "google_calendar_html_link".to_string(),
                    serde_json::Value::String(html_link),
                );
            }
            if node.source.is_none() {
                node.source = Some(event_source(calendar_id, &event_id));
            }
            node.temporal.updated_at = Utc::now();
            let _ = engine.update_node(node).await?;
            exported_created += 1;
        }
    }

    Ok((exported_created, exported_updated))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurrence::{
        RECURRING_INSTANCE_METADATA_KEY, RECURRING_PARENT_ID_METADATA_KEY,
        TASK_COMPLETED_METADATA_KEY, TASK_DUE_AT_METADATA_KEY,
        TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY, TASK_RECURRENCE_METADATA_KEY,
        TASK_REMINDER_SENT_AT_METADATA_KEY,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use mv_core::{
        ConflictAlert, ConflictType, ContactIdentity, GraphStore, IdentityType, InsightType,
        KnowledgeNode, MessageStatus, NodeKind, ProactiveInsight, ProposalAction, ProposalState,
        QueryFilters, RelationKind, Relationship, RelayChannel, RelayContact, RelayMessage,
        RelayPromotionRequest, SearchStrategy, TrustLevel, TrustModel,
    };
    use tempfile::TempDir;

    async fn create_test_engine() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_engine_with_ai_auto_tagging() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.ai.auto_tagging_enabled = true;
        config.ai.auto_tagging_similarity_seed_limit = 8;
        config.ai.auto_tagging_max_generated_tags = 6;
        config.ai.auto_tagging_max_total_tags = 12;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_engine_with_local_embedding_provider(
        provider: &str,
        model: &str,
    ) -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = provider.to_string();
        config.embedding.model = model.to_string();
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_engine_with_backlinks_disabled() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.linking.auto_backlinks_enabled = false;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_sealed_engine(unseal_vault: bool) -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.sealed_mode = true;
        let engine = MindVaultEngine::init(config).await.unwrap();
        engine
            .keychain
            .initialize_vault("test-password", false, "test-suite")
            .await
            .unwrap();
        if unseal_vault {
            engine
                .keychain
                .unseal("test-password", "test-suite")
                .await
                .unwrap();
        } else {
            engine.keychain.seal("test-suite").await.unwrap();
        }
        (engine, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve_node() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let node = KnowledgeNode::new(NodeKind::Fact, "Test content".to_string())
            .with_title("Test Title")
            .with_tags(vec!["test".to_string()]);

        let stored_node = engine.store_node(node.clone()).await.unwrap();
        assert_eq!(stored_node.content, "Test content");
        assert_eq!(stored_node.title, Some("Test Title".to_string()));

        let retrieved = engine.get_node(stored_node.id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test content");
        assert_eq!(retrieved.tags, vec!["test"]);
    }

    #[tokio::test]
    async fn test_update_node() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let node = KnowledgeNode::new(NodeKind::Fact, "Original content".to_string());
        let stored = engine.store_node(node).await.unwrap();

        let mut updated = stored.clone();
        updated.content = "Updated content".to_string();
        updated.title = Some("Updated title".to_string());

        let updated_node = engine.update_node(updated).await.unwrap();
        assert_eq!(updated_node.content, "Updated content");
        assert_eq!(updated_node.title, Some("Updated title".to_string()));
    }

    #[tokio::test]
    async fn test_delete_node() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let node = KnowledgeNode::new(NodeKind::Fact, "To delete".to_string());
        let stored = engine.store_node(node).await.unwrap();

        assert!(engine.delete_node(stored.id).await.unwrap());
        assert!(engine.get_node(stored.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_node_count() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let count = engine.node_count().await.unwrap();
        assert_eq!(count, 0);

        engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "First".to_string()))
            .await
            .unwrap();
        engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Second".to_string()))
            .await
            .unwrap();

        let count = engine.node_count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_sealed_engine_blocks_node_io_while_sealed() {
        let (engine, _tmp_dir) = create_test_sealed_engine(false).await;
        let err = engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "blocked".to_string()))
            .await
            .expect_err("sealed store_node must fail");
        assert!(matches!(err, MvError::VaultSealed));
    }

    #[tokio::test]
    async fn test_sealed_migrate_and_rebuild_require_unseal_then_succeed() {
        let (engine, _tmp_dir) = create_test_sealed_engine(false).await;

        let rebuild_err = engine
            .rebuild_runtime_indexes()
            .await
            .expect_err("sealed rebuild should fail");
        assert!(matches!(rebuild_err, MvError::VaultSealed));

        let migrate_err = engine
            .migrate_sealed_storage()
            .await
            .expect_err("sealed migrate should fail");
        assert!(matches!(migrate_err, MvError::VaultSealed));

        engine
            .keychain
            .unseal("test-password", "test-suite")
            .await
            .unwrap();

        let node = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "sealed migration validation".to_string(),
            ))
            .await
            .unwrap();

        engine
            .rebuild_runtime_indexes()
            .await
            .expect("rebuild after unseal");
        engine
            .migrate_sealed_storage()
            .await
            .expect("migrate after unseal");

        let loaded = engine
            .get_node(node.id)
            .await
            .expect("load node")
            .expect("node exists");
        assert_eq!(loaded.content, "sealed migration validation");
    }

    #[tokio::test]
    async fn test_sealed_restart_cycle_recovers_data_after_unseal_and_rebuild() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_string_lossy().to_string();

        let mut config = EngineConfig {
            data_dir: data_dir.clone(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.sealed_mode = true;

        let engine = MindVaultEngine::init(config.clone()).await.unwrap();
        engine
            .keychain
            .initialize_vault("restart-password", false, "test-suite")
            .await
            .unwrap();

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "restart lifecycle keeps encrypted knowledge".to_string(),
            ))
            .await
            .unwrap();

        engine.keychain.seal("test-suite").await.unwrap();
        drop(engine);

        let restarted = MindVaultEngine::init(config).await.unwrap();
        assert!(
            restarted.is_sealed(),
            "engine should start sealed after restart"
        );

        let sealed_err = restarted
            .get_node(stored.id)
            .await
            .expect_err("sealed restart should block node reads");
        assert!(matches!(sealed_err, MvError::VaultSealed));

        restarted
            .keychain
            .unseal("restart-password", "test-suite")
            .await
            .unwrap();
        restarted
            .migrate_sealed_storage()
            .await
            .expect("migrate after restart");
        restarted
            .rebuild_runtime_indexes()
            .await
            .expect("rebuild after restart");

        let loaded = restarted
            .get_node(stored.id)
            .await
            .expect("load node")
            .expect("node exists");
        assert_eq!(
            loaded.content,
            "restart lifecycle keeps encrypted knowledge"
        );

        let recall_results = restarted
            .recall(
                &MemoryQuery::new("restart lifecycle")
                    .with_strategy(SearchStrategy::FullText)
                    .with_limit(10)
                    .with_min_score(0.0),
            )
            .await
            .expect("recall should work after restart rebuild");
        assert!(
            recall_results
                .iter()
                .any(|result| result.node.id == stored.id),
            "recalled results should include the stored node"
        );
    }

    #[tokio::test]
    async fn test_relationships() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let node1 = engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Node 1".to_string()))
            .await
            .unwrap();
        let node2 = engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Node 2".to_string()))
            .await
            .unwrap();

        let relation = Relationship {
            id: uuid::Uuid::new_v4(),
            from_node: node1.id,
            to_node: node2.id,
            kind: RelationKind::RelatesTo,
            weight: 1.0,
            metadata: Default::default(),
            created_at: chrono::Utc::now(),
        };

        engine.add_relationship(relation.clone()).await.unwrap();

        let neighbors = engine.get_neighbors(node1.id, 1).await.unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], node2.id);
    }

    #[tokio::test]
    async fn test_update_profile_syncs_owner_relay_contact() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let updated = engine
            .update_profile(&UpdateProfileRequest {
                display_name: Some("Owner".to_string()),
                email: Some("owner@example.com".to_string()),
                signature_public_key: Some("pk-owner".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        let contact_id = updated
            .metadata
            .get("relay_contact_id")
            .and_then(|value| value.as_str())
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
            .expect("relay_contact_id set");

        let contact = engine.relay.get_contact(contact_id).await.unwrap().unwrap();
        assert_eq!(contact.display_name, "Owner");
        assert_eq!(contact.public_key, "pk-owner");
        assert_eq!(
            contact.vault_address.as_deref(),
            Some("mailto:owner@example.com")
        );
    }

    #[tokio::test]
    async fn test_relay_inbound_creates_reply_proposal() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let contact = RelayContact::new("Alice", "pk-alice").with_trust(TrustLevel::ContextInject);
        engine.relay.add_contact(&contact).await.unwrap();

        let channel = RelayChannel::direct(contact.id);
        engine.relay.create_channel(&channel).await.unwrap();

        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Project Atlas roadmap lives in the Q2 plan.".to_string(),
        );
        engine.store_node(node).await.unwrap();

        let message =
            RelayMessage::inbound(channel.id, contact.id, "Can you share the Atlas roadmap?");
        let outcome = engine
            .receive_relay_message(message, "default")
            .await
            .unwrap();

        assert!(outcome.proposal_id.is_some());
        assert!(outcome.auto_reply.is_none());
        assert_eq!(outcome.message.status, MessageStatus::Deferred);

        let proposals = engine
            .list_proposals(Some(ProposalState::Pending), 10, 0)
            .await
            .unwrap();
        assert!(proposals.iter().any(|proposal| {
            proposal.action == ProposalAction::Custom("relay.reply".to_string())
        }));
    }

    #[tokio::test]
    async fn promotion_boundary_relay_message_not_in_knowledge_graph() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let contact = RelayContact::new("Alice", "pk-alice");
        engine.relay.add_contact(&contact).await.unwrap();
        let channel = RelayChannel::direct(contact.id);
        engine.relay.create_channel(&channel).await.unwrap();

        let message = RelayMessage::inbound(channel.id, contact.id, "casual chat should stay out");
        let outcome = engine
            .receive_relay_message(message, "default")
            .await
            .unwrap();

        assert!(outcome.message.vault_node_id.is_none());
        let nodes = engine
            .store
            .nodes
            .list(&QueryFilters::default(), 100, 0)
            .await
            .unwrap();
        assert!(
            nodes.iter().all(|n| n.kind != NodeKind::Conversation),
            "relay receive must not create Conversation knowledge nodes"
        );
        assert!(
            !nodes.iter().any(|n| n.content.contains("casual chat")),
            "relay content must not enter the knowledge graph without promotion"
        );
    }

    #[tokio::test]
    async fn promotion_boundary_explicit_promotion_records_provenance() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let contact = RelayContact::new("Bob", "pk-bob");
        engine.relay.add_contact(&contact).await.unwrap();
        let channel = RelayChannel::direct(contact.id);
        engine.relay.create_channel(&channel).await.unwrap();

        let message = RelayMessage::inbound(channel.id, contact.id, "Decision: ship Friday");
        let stored = engine
            .relay
            .receive_message(message, "default")
            .await
            .unwrap();

        let request = RelayPromotionRequest::explicit(
            stored.id,
            "human:owner",
            "user selected message as a durable decision",
        )
        .with_policy("promotion.explicit")
        .with_approval("approval:test-1");

        let node = engine.promote_relay_message(request).await.unwrap();
        assert_eq!(node.kind, NodeKind::Conversation);
        assert_eq!(node.content, "Decision: ship Friday");

        let promotion = node
            .metadata
            .get("promotion")
            .expect("promotion metadata required");
        let source_id = stored.id.to_string();
        assert_eq!(
            promotion.get("source_message_id").and_then(|v| v.as_str()),
            Some(source_id.as_str())
        );
        assert_eq!(
            promotion.get("extractor").and_then(|v| v.as_str()),
            Some("explicit")
        );
        assert_eq!(
            promotion.get("actor").and_then(|v| v.as_str()),
            Some("human:owner")
        );
        assert_eq!(
            promotion.get("evidence").and_then(|v| v.as_str()),
            Some("user selected message as a durable decision")
        );
        assert_eq!(
            promotion.get("policy").and_then(|v| v.as_str()),
            Some("promotion.explicit")
        );
        assert_eq!(
            promotion.get("approval").and_then(|v| v.as_str()),
            Some("approval:test-1")
        );
        assert!(
            promotion
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                >= 1.0
        );

        let rebound = engine.relay.get_message(stored.id).await.unwrap().unwrap();
        assert_eq!(rebound.vault_node_id, Some(node.id));
    }

    #[tokio::test]
    async fn promotion_boundary_retraction_preserves_source_communication() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let contact = RelayContact::new("Carol", "pk-carol");
        engine.relay.add_contact(&contact).await.unwrap();
        let channel = RelayChannel::direct(contact.id);
        engine.relay.create_channel(&channel).await.unwrap();

        let message = RelayMessage::inbound(channel.id, contact.id, "Keep this exact text");
        let stored = engine
            .relay
            .receive_message(message, "default")
            .await
            .unwrap();
        let node = engine
            .promote_relay_message(RelayPromotionRequest::explicit(
                stored.id,
                "human:owner",
                "temporary promotion",
            ))
            .await
            .unwrap();

        let retained = engine.retract_relay_promotion(stored.id).await.unwrap();
        assert_eq!(retained.content, "Keep this exact text");
        assert!(retained.vault_node_id.is_none());
        assert!(engine.get_node(node.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ai_auto_tagging_enriches_from_content_and_neighbors() {
        let (engine, _tmp_dir) = create_test_engine_with_ai_auto_tagging().await;

        let seed = KnowledgeNode::new(
            NodeKind::Fact,
            "Rust async tokio memory pipeline for background jobs".to_string(),
        )
        .with_tags(vec![
            "rust".to_string(),
            "async".to_string(),
            "tokio".to_string(),
        ]);
        let _seed_node = engine.store_node(seed).await.unwrap();

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Building a memory pipeline in Rust for async workers".to_string(),
            ))
            .await
            .unwrap();

        let tags_lower: Vec<String> = stored
            .tags
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect();
        assert!(!tags_lower.is_empty(), "auto-tagging should add tags");
        assert!(
            tags_lower.iter().any(|tag| tag == "rust"),
            "expected generated tags to include rust"
        );
        assert!(
            tags_lower
                .iter()
                .any(|tag| tag == "memory" || tag == "pipeline"),
            "expected lexical tags to include memory/pipeline"
        );
    }

    #[tokio::test]
    async fn test_ai_auto_tagging_disabled_keeps_empty_tag_list() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Novel note without manual tags".to_string(),
            ))
            .await
            .unwrap();

        assert!(
            stored.tags.is_empty(),
            "tags should remain empty when auto-tagging is disabled"
        );
    }

    #[tokio::test]
    async fn test_unknown_embedding_provider_falls_back_without_breaking_ingest() {
        let (engine, _tmp_dir) =
            create_test_engine_with_local_embedding_provider("unknown-provider", "any").await;

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Provider fallback should keep ingest healthy".to_string(),
            ))
            .await
            .unwrap();

        assert_eq!(
            stored.content,
            "Provider fallback should keep ingest healthy"
        );

        let status = engine.embedding_runtime_status();
        assert_eq!(status.configured_provider, "unknown-provider");
        assert_eq!(status.effective_provider, "noop");
        assert!(status.fallback_to_noop);
        assert!(status
            .reason
            .as_deref()
            .is_some_and(|value| value.contains("unknown embedding provider")));
    }

    #[tokio::test]
    async fn test_local_fastembed_invalid_model_falls_back_without_breaking_ingest() {
        let (engine, _tmp_dir) =
            create_test_engine_with_local_embedding_provider("local_fastembed", "invalid-model")
                .await;

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Invalid local model should not break ingest".to_string(),
            ))
            .await
            .unwrap();

        assert_eq!(
            stored.content,
            "Invalid local model should not break ingest"
        );

        let status = engine.embedding_runtime_status();
        assert_eq!(status.configured_provider, "local_fastembed");
        assert_eq!(status.effective_provider, "noop");
        assert!(status.fallback_to_noop);
        assert!(status
            .reason
            .as_deref()
            .is_some_and(|value| value.contains("local_fastembed initialization failed")));
    }

    #[tokio::test]
    async fn test_ensure_daily_note_is_idempotent() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let date = NaiveDate::from_ymd_opt(2026, 2, 6).expect("valid date");

        let (created_note, created) = engine.ensure_daily_note(date, None).await.unwrap();
        assert!(created);
        assert_eq!(created_note.namespace, engine.config.daily_notes.namespace);
        assert!(created_note.tags.iter().any(|tag| tag == DAILY_NOTE_TAG));
        assert!(created_note.tags.iter().any(|tag| tag == "day:2026-02-06"));
        assert_eq!(
            created_note.metadata.get("daily_note_date"),
            Some(&serde_json::Value::String("2026-02-06".to_string()))
        );

        let (existing_note, created_again) = engine.ensure_daily_note(date, None).await.unwrap();
        assert!(!created_again);
        assert_eq!(existing_note.id, created_note.id);
    }

    #[tokio::test]
    async fn test_list_daily_notes_filters_non_daily_nodes() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let date = NaiveDate::from_ymd_opt(2026, 2, 6).expect("valid date");
        let namespace = engine.config.daily_notes.namespace.clone();

        let (_daily, created) = engine
            .ensure_daily_note(date, Some(namespace.clone()))
            .await
            .unwrap();
        assert!(created);

        let non_daily = KnowledgeNode::new(NodeKind::Fact, "not a daily note".to_string())
            .with_namespace(namespace.clone())
            .with_tags(vec!["journal".to_string()]);
        let stored_non_daily = engine.store_node(non_daily).await.unwrap();

        let notes = engine
            .list_daily_notes(Some(namespace), 50, 0)
            .await
            .unwrap();

        assert!(!notes.is_empty());
        assert!(notes
            .iter()
            .all(|node| node.tags.iter().any(|tag| tag == DAILY_NOTE_TAG)));
        assert!(notes.iter().all(|node| node.id != stored_non_daily.id));
    }

    #[tokio::test]
    async fn test_store_node_auto_links_task_to_daily_note() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = engine.config.daily_notes.namespace.clone();
        let task_node = KnowledgeNode::new(
            NodeKind::Task,
            "Ship release checklist and verify deployment timeline".to_string(),
        )
        .with_namespace(namespace.clone())
        .with_tags(vec!["release".to_string()]);

        let stored_task = engine.store_node(task_node).await.unwrap();
        let day = stored_task.temporal.created_at.date_naive();
        let daily_note = engine
            .find_daily_note(day, &namespace)
            .await
            .unwrap()
            .expect("daily note should be created");

        let outgoing = engine
            .graph
            .get_relationships_from(daily_note.id)
            .await
            .unwrap();
        assert!(outgoing
            .iter()
            .any(|rel| { rel.to_node == stored_task.id && rel.kind == RelationKind::Contains }));
    }

    #[tokio::test]
    async fn test_store_node_auto_links_event_kind_without_tags() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = engine.config.daily_notes.namespace.clone();
        let event_node = KnowledgeNode::new(
            NodeKind::Event,
            "Team planning sync at 10:00 UTC".to_string(),
        )
        .with_namespace(namespace.clone());

        let stored_event = engine.store_node(event_node).await.unwrap();
        let day = stored_event.temporal.created_at.date_naive();
        let daily_note = engine
            .find_daily_note(day, &namespace)
            .await
            .unwrap()
            .expect("daily note should be created");

        let outgoing = engine
            .graph
            .get_relationships_from(daily_note.id)
            .await
            .unwrap();
        assert!(outgoing
            .iter()
            .any(|rel| rel.to_node == stored_event.id && rel.kind == RelationKind::Contains));
    }

    #[tokio::test]
    async fn test_update_node_auto_link_does_not_duplicate_daily_edge() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = engine.config.daily_notes.namespace.clone();
        let task_node =
            KnowledgeNode::new(NodeKind::Event, "Capture meeting action items".to_string())
                .with_namespace(namespace.clone());
        let stored_task = engine.store_node(task_node).await.unwrap();
        let day = stored_task.temporal.created_at.date_naive();
        let daily_note = engine
            .find_daily_note(day, &namespace)
            .await
            .unwrap()
            .expect("daily note should exist");

        let mut updated_task = stored_task.clone();
        updated_task.content = "Capture meeting action items and owner assignments".to_string();
        let _updated = engine.update_node(updated_task).await.unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(daily_note.id)
            .await
            .unwrap();
        let contains_edges = outgoing
            .iter()
            .filter(|rel| rel.to_node == stored_task.id && rel.kind == RelationKind::Contains)
            .count();
        assert_eq!(contains_edges, 1);
    }

    #[tokio::test]
    async fn test_store_node_auto_backlinks_from_wikilinks() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Launch scope and milestones".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Review [[Project Alpha]] and prep a kickoff checklist.".to_string(),
                )
                .with_namespace(namespace.clone())
                .with_title("Kickoff Brief"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        let reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(reference.is_some());
        assert_eq!(
            reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("wikilink")
        );
    }

    #[tokio::test]
    async fn test_store_node_auto_backlinks_from_markdown_and_source_urls() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let title_target = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Project Beta release sequencing".to_string(),
                )
                .with_namespace(namespace.clone())
                .with_title("Project Beta"),
            )
            .await
            .unwrap();
        let source_target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Bookmark, "Spec reference".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Platform Spec")
                    .with_source("https://docs.example.com/spec"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Review [release scope](Project Beta#Milestones) and https://docs.example.com/spec#overview before kickoff.".to_string(),
                )
                .with_namespace(namespace.clone())
                .with_title("Kickoff Prep"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        let title_reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == title_target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(title_reference.is_some());
        assert_eq!(
            title_reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("markdown_link")
        );

        let source_reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == source_target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(source_reference.is_some());
        assert_eq!(
            source_reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("source_url")
        );
    }

    #[tokio::test]
    async fn test_store_node_auto_backlinks_from_mentions() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Project Alpha execution notes".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Align the launch checklist with @\"Project Alpha\" owners.".to_string(),
                )
                .with_namespace(namespace.clone())
                .with_title("Launch Checklist"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        let mention_reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(mention_reference.is_some());
        assert_eq!(
            mention_reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("mention")
        );
    }

    #[tokio::test]
    async fn test_update_node_auto_backlinks_removes_stale_targets() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let target_alpha = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Alpha details".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();
        let target_beta = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Beta details".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Project Beta"),
            )
            .await
            .unwrap();

        let mut source = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Draft [[Project Alpha]] notes".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Planning"),
            )
            .await
            .unwrap();

        source.content = "Finalize [[Project Beta]] notes".to_string();
        let source = engine.update_node(source).await.unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        assert!(outgoing.iter().any(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target_beta.id
                && is_auto_backlink_relationship(rel)
        }));
        assert!(!outgoing.iter().any(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target_alpha.id
                && is_auto_backlink_relationship(rel)
        }));
    }

    #[tokio::test]
    async fn test_auto_backlinks_can_be_disabled() {
        let (engine, _tmp_dir) = create_test_engine_with_backlinks_disabled().await;
        let namespace = "knowledge".to_string();

        let _target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Alpha details".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Draft [[Project Alpha]] notes".to_string())
                    .with_namespace(namespace.clone())
                    .with_title("Planning"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        assert!(!outgoing.iter().any(|rel| {
            rel.kind == RelationKind::References && is_auto_backlink_relationship(rel)
        }));
    }

    #[tokio::test]
    async fn test_rollforward_recurring_tasks_generates_due_instance() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "ops".to_string();
        let mut template =
            KnowledgeNode::new(NodeKind::Task, "Daily standup action checklist".to_string())
                .with_namespace(namespace.clone())
                .with_tags(vec!["ops".to_string()]);

        let last_generated = Utc
            .with_ymd_and_hms(2026, 2, 5, 9, 0, 0)
            .single()
            .expect("valid datetime");
        template.metadata.insert(
            TASK_RECURRENCE_METADATA_KEY.into(),
            serde_json::json!({
                "frequency": "daily",
                "interval": 1,
                "enabled": true
            }),
        );
        template.metadata.insert(
            TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY.into(),
            serde_json::Value::String(last_generated.to_rfc3339()),
        );

        let stored_template = engine.store_node(template).await.unwrap();
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 9, 0, 0)
            .single()
            .expect("valid datetime");
        let stats = engine.rollforward_recurring_tasks(now, 4).await.unwrap();
        assert_eq!(stats.generated_instances, 1);

        let tasks = engine
            .list_nodes(
                &QueryFilters {
                    namespace: Some(namespace.clone()),
                    kinds: Some(vec![NodeKind::Task]),
                    ..Default::default()
                },
                50,
                0,
            )
            .await
            .unwrap();

        let instance = tasks
            .iter()
            .find(|node| {
                node.metadata
                    .get(RECURRING_INSTANCE_METADATA_KEY)
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            })
            .expect("recurring instance should exist");
        assert_eq!(
            instance
                .metadata
                .get(RECURRING_PARENT_ID_METADATA_KEY)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            Some(stored_template.id.to_string())
        );

        let rels = engine
            .graph
            .get_relationships_from(stored_template.id)
            .await
            .unwrap();
        assert!(rels
            .iter()
            .any(|rel| { rel.to_node == instance.id && rel.kind == RelationKind::DerivedFrom }));
    }

    #[tokio::test]
    async fn test_rollforward_recurring_tasks_is_idempotent_for_same_instant() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let mut template =
            KnowledgeNode::new(NodeKind::Task, "Weekly planning template".to_string())
                .with_namespace("ops");
        template.metadata.insert(
            TASK_RECURRENCE_METADATA_KEY.into(),
            serde_json::json!({
                "frequency": "weekly",
                "interval": 1,
                "enabled": true
            }),
        );
        template.metadata.insert(
            TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 1, 30, 10, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let _stored = engine.store_node(template).await.unwrap();

        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 10, 0, 0)
            .single()
            .expect("valid datetime");
        let first = engine.rollforward_recurring_tasks(now, 4).await.unwrap();
        let second = engine.rollforward_recurring_tasks(now, 4).await.unwrap();

        assert!(first.generated_instances >= 1);
        assert_eq!(second.generated_instances, 0);
    }

    #[tokio::test]
    async fn test_list_due_tasks_filters_and_sorts() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let base = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut task_a = KnowledgeNode::new(NodeKind::Task, "A".to_string()).with_namespace("ops");
        task_a.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 10, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let mut task_b = KnowledgeNode::new(NodeKind::Task, "B".to_string()).with_namespace("ops");
        task_b.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 11, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let mut task_c = KnowledgeNode::new(NodeKind::Task, "C".to_string()).with_namespace("ops");
        task_c.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 9, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        task_c.metadata.insert(
            TASK_COMPLETED_METADATA_KEY.into(),
            serde_json::Value::Bool(true),
        );

        let _a = engine.store_node(task_a).await.unwrap();
        let _b = engine.store_node(task_b).await.unwrap();
        let _c = engine.store_node(task_c).await.unwrap();

        let due = engine
            .list_due_tasks(base, Some("ops".to_string()), 10, false)
            .await
            .unwrap();
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].content, "A");
        assert_eq!(due[1].content, "B");
    }

    #[tokio::test]
    async fn test_prioritize_tasks_ranks_by_due_and_importance() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut urgent = KnowledgeNode::new(NodeKind::Task, "Urgent".to_string())
            .with_namespace("ops")
            .with_importance(0.9);
        urgent.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 18, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );

        let important = KnowledgeNode::new(NodeKind::Task, "Important".to_string())
            .with_namespace("ops")
            .with_importance(0.8);

        let mut later = KnowledgeNode::new(NodeKind::Task, "Later".to_string())
            .with_namespace("ops")
            .with_importance(0.2);
        later.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );

        let _urgent = engine.store_node(urgent).await.unwrap();
        let _important = engine.store_node(important).await.unwrap();
        let _later = engine.store_node(later).await.unwrap();

        let prioritized = engine
            .prioritize_tasks(TaskPrioritizationOptions {
                namespace: Some("ops".to_string()),
                limit: 10,
                include_completed: false,
                include_without_due: true,
                persist: false,
                now,
            })
            .await
            .unwrap();

        assert_eq!(prioritized.len(), 3);
        assert_eq!(prioritized[0].task.content, "Urgent");
        assert_eq!(prioritized[1].task.content, "Important");
        assert_eq!(prioritized[2].task.content, "Later");
        assert_eq!(prioritized[0].rank, 1);
    }

    #[tokio::test]
    async fn test_dispatch_due_task_reminders_marks_once() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut task = KnowledgeNode::new(NodeKind::Task, "Follow up on incident".to_string())
            .with_namespace("ops");
        task.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 8, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let stored = engine.store_node(task).await.unwrap();

        let first = engine.dispatch_due_task_reminders(now, 20).await.unwrap();
        assert_eq!(first.reminders_marked_sent, 1);

        let second = engine.dispatch_due_task_reminders(now, 20).await.unwrap();
        assert_eq!(second.reminders_marked_sent, 0);

        let refreshed = engine
            .get_node(stored.id)
            .await
            .unwrap()
            .expect("task exists");
        assert!(refreshed
            .metadata
            .contains_key(TASK_REMINDER_SENT_AT_METADATA_KEY));
    }

    #[tokio::test]
    async fn test_default_permission_templates_seeded() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let templates = engine.list_permission_templates(10, 0).await.unwrap();
        let names: Vec<String> = templates.into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"Owner".to_string()));
        assert!(names.contains(&"Assistant".to_string()));
    }

    #[tokio::test]
    async fn test_access_key_round_trip() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let template = engine
            .create_permission_template(
                "Scoped".to_string(),
                Some("Test".to_string()),
                PermissionTier::Edit,
                Some("ops".to_string()),
                vec!["shared".to_string()],
                vec![NodeKind::Fact],
                vec!["transform".to_string()],
            )
            .await
            .unwrap();

        let (_key, token) = engine
            .create_access_key(template.id, Some("Key".to_string()), None)
            .await
            .unwrap();

        let resolved = engine.resolve_access_key(&token).await.unwrap();
        assert!(resolved.is_some());
        let (_key, resolved_template) = resolved.unwrap();
        assert_eq!(resolved_template.id, template.id);
    }

    #[tokio::test]
    async fn test_insight_generate_and_list() {
        let (engine, _tmp) = create_test_engine().await;

        let insight = ProactiveInsight {
            id: Uuid::now_v7(),
            title: "Test Insight".into(),
            content: "Something interesting".into(),
            insight_type: InsightType::General,
            related_node_ids: vec![],
            importance: 0.8,
            metadata: Default::default(),
            created_at: Utc::now(),
            dismissed_at: None,
        };
        engine.store.nodes.log_insight(&insight).await.unwrap();

        let insights = engine.list_insights(10, 0).await.unwrap();
        assert!(!insights.is_empty(), "should have at least one insight");
        assert_eq!(insights[0].title, "Test Insight");

        let deleted = engine.delete_insight(insight.id).await.unwrap();
        assert!(deleted);

        let after = engine.list_insights(10, 0).await.unwrap();
        assert!(
            after.iter().all(|i| i.id != insight.id),
            "insight should be dismissed"
        );
    }

    #[tokio::test]
    async fn test_conflict_detection_on_contradictory_nodes() {
        let (engine, _tmp) = create_test_engine().await;

        let node_a = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "The sky is blue").with_tags(vec!["sky".into()]),
            )
            .await
            .unwrap();
        let node_b = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "The sky is green")
                    .with_tags(vec!["sky".into()]),
            )
            .await
            .unwrap();

        let alert = ConflictAlert {
            id: Uuid::now_v7(),
            node_a: node_a.id,
            node_b: node_b.id,
            conflict_type: ConflictType::Contradiction,
            score: 0.95,
            explanation: "Contradictory claims about sky color".into(),
            resolved: false,
            created_at: Utc::now(),
        };
        engine.store.nodes.insert_conflict(&alert).await.unwrap();

        let conflicts = engine.list_conflicts(Some(false), 10, 0).await.unwrap();
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].node_a, node_a.id);

        let resolved = engine.resolve_conflict(alert.id).await.unwrap();
        assert!(resolved);

        let fetched = engine.get_conflict(alert.id).await.unwrap().unwrap();
        assert!(fetched.resolved, "conflict should be resolved");

        let re_resolved = engine.resolve_conflict(alert.id).await.unwrap();
        assert!(!re_resolved);
    }

    #[tokio::test]
    async fn test_contact_identity_crud() {
        let (engine, _tmp) = create_test_engine().await;

        let contact_id = Uuid::now_v7();
        let identity = ContactIdentity {
            id: Uuid::now_v7(),
            contact_id,
            identity_type: IdentityType::Email,
            identity_value: "alice@example.com".into(),
            verified: false,
            verified_at: None,
            created_at: Utc::now(),
        };

        engine.add_contact_identity(&identity).await.unwrap();

        let list = engine.list_contact_identities(contact_id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].identity_value, "alice@example.com");
        assert!(!list[0].verified);

        let verified = engine.verify_contact_identity(identity.id).await.unwrap();
        assert!(verified);

        let list = engine.list_contact_identities(contact_id).await.unwrap();
        assert!(list[0].verified);

        let deleted = engine.delete_contact_identity(identity.id).await.unwrap();
        assert!(deleted);

        let list = engine.list_contact_identities(contact_id).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_trust_model_defaults_and_update() {
        let (engine, _tmp) = create_test_engine().await;

        let contact_id = Uuid::now_v7();

        let model = engine.get_trust_model(contact_id).await.unwrap();
        assert!(model.is_none());

        let mut tm = TrustModel {
            contact_id,
            ..Default::default()
        };
        engine.set_trust_model(&tm).await.unwrap();

        let stored = engine.get_trust_model(contact_id).await.unwrap().unwrap();
        assert!(!stored.can_query);
        assert!(!stored.can_inject_context);
        assert!(!stored.can_auto_reply);
        assert!(stored.allowed_namespaces.is_empty());

        tm.can_query = true;
        tm.allowed_namespaces = vec!["research".into()];
        engine.set_trust_model(&tm).await.unwrap();

        let updated = engine.get_trust_model(contact_id).await.unwrap().unwrap();
        assert!(updated.can_query);
        assert_eq!(updated.allowed_namespaces, vec!["research"]);
    }

    #[tokio::test]
    async fn test_federation_peer_add_and_list() {
        let (engine, _tmp) = create_test_engine().await;

        let peer = crate::federation::FederationPeer {
            id: Uuid::now_v7(),
            vault_id: "vault-test-123".into(),
            display_name: "Test Vault".into(),
            endpoint: "http://127.0.0.1:19470".into(),
            public_key: None,
            allowed_namespaces: vec![],
            max_results: 10,
            enabled: true,
            last_seen: None,
            created_at: Utc::now(),
            shared_secret: Some("secret123".into()),
        };

        engine.federation.add_peer(peer.clone()).await;

        let peers = engine.federation.list_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].vault_id, "vault-test-123");

        let found = engine
            .federation
            .find_peer_by_vault_id("vault-test-123")
            .await;
        assert!(found.is_some());

        let removed = engine.federation.remove_peer(peer.id).await;
        assert!(removed);
        assert!(engine.federation.list_peers().await.is_empty());
    }

    #[tokio::test]
    async fn test_store_multiple_node_kinds_and_retrieve() {
        let (engine, _tmp) = create_test_engine().await;

        let obs = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Observation, "Observed: Rust async is fast")
                    .with_tags(vec!["rust".into(), "async".into()])
                    .with_namespace("default"),
            )
            .await
            .unwrap();
        let fact = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Tokio is the most popular async runtime")
                    .with_tags(vec!["rust".into(), "tokio".into()])
                    .with_namespace("default"),
            )
            .await
            .unwrap();

        let r_obs = engine.get_node(obs.id).await.unwrap();
        assert!(r_obs.is_some());
        assert_eq!(r_obs.unwrap().kind, NodeKind::Observation);

        let r_fact = engine.get_node(fact.id).await.unwrap();
        assert!(r_fact.is_some());
        assert_eq!(r_fact.unwrap().kind, NodeKind::Fact);

        let neighbors = engine.get_neighbors(obs.id, 1).await.unwrap();
        assert!(!neighbors.contains(&fact.id));
    }

    #[test]
    fn glob_wildcard_matches_everything() {
        assert!(glob_match_simple("*", "anything"));
        assert!(glob_match_simple("*", ""));
    }

    #[test]
    fn glob_prefix_wildcard_matches_suffix() {
        assert!(glob_match_simple("*@example.com", "user@example.com"));
        assert!(!glob_match_simple("*@example.com", "user@other.com"));
    }

    #[test]
    fn glob_suffix_wildcard_matches_prefix() {
        assert!(glob_match_simple("mcp-*", "mcp-agent"));
        assert!(!glob_match_simple("mcp-*", "other-agent"));
    }

    #[test]
    fn glob_exact_match() {
        assert!(glob_match_simple("exact", "exact"));
        assert!(!glob_match_simple("exact", "different"));
    }

    #[test]
    fn glob_empty_pattern_only_matches_empty() {
        assert!(glob_match_simple("", ""));
        assert!(!glob_match_simple("", "notempty"));
    }
}
