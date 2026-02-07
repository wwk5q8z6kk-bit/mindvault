use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub data_dir: String,
    pub embedding: EmbeddingConfig,
    pub search: SearchConfig,
    pub graph: GraphConfig,
    pub ai: AiConfig,
    pub linking: LinkingConfig,
    pub daily_notes: DailyNotesConfig,
    pub recurrence: RecurrenceConfig,
    pub encryption: EncryptionConfig,
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
            embedding: EmbeddingConfig::default(),
            search: SearchConfig::default(),
            graph: GraphConfig::default(),
            ai: AiConfig::default(),
            linking: LinkingConfig::default(),
            daily_notes: DailyNotesConfig::default(),
            recurrence: RecurrenceConfig::default(),
            encryption: EncryptionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "openai".into(),
            model: "text-embedding-3-small".into(),
            dimensions: 1536,
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
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            auto_tagging_enabled: false,
            auto_tagging_max_generated_tags: 6,
            auto_tagging_max_total_tags: 12,
            auto_tagging_similarity_seed_limit: 8,
            auto_tagging_min_token_length: 4,
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
