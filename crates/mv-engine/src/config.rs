use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub data_dir: String,
    pub profile: OwnerProfileConfig,
    pub embedding: EmbeddingConfig,
    pub search: SearchConfig,
    pub graph: GraphConfig,
    pub ai: AiConfig,
    pub llm: LlmConfig,
    pub email: EmailAdapterConfig,
    pub linking: LinkingConfig,
    pub daily_notes: DailyNotesConfig,
    pub recurrence: RecurrenceConfig,
    pub encryption: EncryptionConfig,
    pub watcher: WatcherConfig,
}

/// Configuration for the Watcher Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Whether the watcher agent is enabled.
    pub enabled: bool,
    /// Interval between watcher cycles in seconds.
    pub interval_secs: u64,
    /// How far back to look for recently modified nodes (hours).
    pub lookback_hours: u64,
    /// Maximum nodes to scan per cycle.
    pub max_nodes_per_cycle: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 300,
            lookback_hours: 24,
            max_nodes_per_cycle: 50,
        }
    }
}

/// Configuration for encryption at rest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Whether encryption is enabled.
    pub enabled: bool,
    /// Argon2 memory parameter in KiB.
    pub argon2_memory_kib: u32,
    /// Argon2 iteration count.
    pub argon2_iterations: u32,
    /// Argon2 parallelism.
    pub argon2_parallelism: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            argon2_memory_kib: 65536,
            argon2_iterations: 3,
            argon2_parallelism: 4,
        }
    }
}

impl EncryptionConfig {
    /// Create from environment variables.
    pub fn from_env() -> Self {
        let enabled = std::env::var("MINDVAULT_ENCRYPTION_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let argon2_memory_kib = std::env::var("MINDVAULT_ENCRYPTION_ARGON2_MEMORY_KIB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(65536);

        let argon2_iterations = std::env::var("MINDVAULT_ENCRYPTION_ARGON2_ITERATIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        let argon2_parallelism = std::env::var("MINDVAULT_ENCRYPTION_ARGON2_PARALLELISM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);

        Self {
            enabled,
            argon2_memory_kib,
            argon2_iterations,
            argon2_parallelism,
        }
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            data_dir: shellexpand("~/.mindvault/data"),
            profile: OwnerProfileConfig::default(),
            embedding: EmbeddingConfig::default(),
            search: SearchConfig::default(),
            graph: GraphConfig::default(),
            ai: AiConfig::default(),
            llm: LlmConfig::default(),
            email: EmailAdapterConfig::default(),
            linking: LinkingConfig::default(),
            daily_notes: DailyNotesConfig::default(),
            recurrence: RecurrenceConfig::default(),
            encryption: EncryptionConfig::default(),
            watcher: WatcherConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerProfileConfig {
    pub display_name: String,
    pub primary_email: Option<String>,
    pub timezone: String,
    pub signature: Option<String>,
}

impl Default for OwnerProfileConfig {
    fn default() -> Self {
        Self {
            display_name: "MindVault Owner".into(),
            primary_email: None,
            timezone: "UTC".into(),
            signature: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAdapterConfig {
    pub enabled: bool,
    pub namespace: String,
    pub poll_interval_secs: u64,
    pub max_fetch: usize,
    pub max_attachment_bytes: usize,
    pub mark_seen: bool,
    pub imap_host: Option<String>,
    pub imap_port: u16,
    pub imap_username: Option<String>,
    pub imap_folder: String,
    pub imap_starttls: bool,
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_from: Option<String>,
    pub smtp_starttls: bool,
}

impl Default for EmailAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            namespace: "default".into(),
            poll_interval_secs: 120,
            max_fetch: 20,
            max_attachment_bytes: 5 * 1024 * 1024,
            mark_seen: false,
            imap_host: None,
            imap_port: 993,
            imap_username: None,
            imap_folder: "INBOX".into(),
            imap_starttls: true,
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_from: None,
            smtp_starttls: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimensions: usize,
    /// Base URL for OpenAI-compatible embedding APIs.
    pub base_url: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "local".into(),
            model: "bge-small-en-v1.5".into(),
            dimensions: 384,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub default_limit: usize,
    pub default_strategy: String,
    pub min_score: f64,
    pub vector_weight: f64,
    pub fulltext_weight: f64,
    pub rrf_k: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: 10,
            default_strategy: "hybrid".into(),
            min_score: 0.1,
            vector_weight: 0.6,
            fulltext_weight: 0.4,
            rrf_k: 60.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    pub default_traversal_depth: usize,
    pub graph_boost_factor: f64,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            default_traversal_depth: 2,
            graph_boost_factor: 0.15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub auto_tagging_enabled: bool,
    pub auto_tagging_max_generated_tags: usize,
    pub auto_tagging_max_total_tags: usize,
    pub auto_tagging_similarity_seed_limit: usize,
    pub auto_tagging_min_token_length: usize,
    pub enrichment_enabled: bool,
    pub enrichment_model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            auto_tagging_enabled: false,
            auto_tagging_max_generated_tags: 6,
            auto_tagging_max_total_tags: 12,
            auto_tagging_similarity_seed_limit: 8,
            auto_tagging_min_token_length: 4,
            enrichment_enabled: false,
            enrichment_model: "gpt-4o-mini".into(),
        }
    }
}

/// Configuration for the LLM provider used by assist/briefing endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    pub auto_detect: bool,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_detect: true,
            base_url: "http://localhost:11434/v1".into(),
            model: "llama3.2".into(),
            max_tokens: 512,
            temperature: 0.3,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkingConfig {
    pub auto_backlinks_enabled: bool,
    pub auto_backlinks_scan_limit: usize,
    pub auto_backlinks_max_targets: usize,
}

impl Default for LinkingConfig {
    fn default() -> Self {
        Self {
            auto_backlinks_enabled: true,
            auto_backlinks_scan_limit: 5_000,
            auto_backlinks_max_targets: 64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyNotesConfig {
    pub enabled: bool,
    pub midnight_scheduler_enabled: bool,
    pub namespace: String,
    pub title_template: String,
    pub content_template: String,
    pub default_importance: f64,
}

impl Default for DailyNotesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            midnight_scheduler_enabled: true,
            namespace: "journal".into(),
            title_template: "Daily Note {{date}}".into(),
            content_template: "\
## Top Priorities
- [ ] 

## Schedule
- Morning:
- Afternoon:
- Evening:

## Notes

## Wins
- 

## Blockers
- 

## Follow-up
- "
            .into(),
            default_importance: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceConfig {
    pub enabled: bool,
    pub scheduler_interval_secs: u64,
    pub max_instances_per_template: usize,
}

impl Default for RecurrenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scheduler_interval_secs: 300,
            max_instances_per_template: 8,
        }
    }
}

fn shellexpand(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    s.to_string()
}
