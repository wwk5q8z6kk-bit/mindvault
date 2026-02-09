pub mod backup;
pub mod config;
pub mod db;
pub mod encrypt;
pub mod export;
pub mod git_credential;
pub mod graph;
pub mod import;
pub mod keychain;
pub mod mcp;
pub mod profile;
pub mod recall;
pub mod search;
pub mod secret;
pub mod server;
pub mod stats;
pub mod store;

use anyhow::Context;
use mv_engine::config::EngineConfig;
use mv_engine::engine::MindVaultEngine;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ServerRuntimeConfig {
    pub bind_host: String,
    pub rest_port: u16,
    pub grpc_port: u16,
    pub socket_path: Option<String>,
    pub cors_allowed_origins: Vec<String>,
}

impl Default for ServerRuntimeConfig {
    fn default() -> Self {
        Self {
            bind_host: "127.0.0.1".into(),
            rest_port: 9470,
            grpc_port: 50051,
            socket_path: Some(shellexpand("~/.mindvault/mindvault.sock")),
            cors_allowed_origins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub engine: EngineConfig,
    pub server: ServerRuntimeConfig,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    server: Option<FileServerConfig>,
    storage: Option<FileStorageConfig>,
    profile: Option<FileProfileConfig>,
    embedding: Option<FileEmbeddingConfig>,
    search: Option<FileSearchConfig>,
    graph: Option<FileGraphConfig>,
    ai: Option<FileAiConfig>,
    linking: Option<FileLinkingConfig>,
    daily_notes: Option<FileDailyNotesConfig>,
    recurrence: Option<FileRecurrenceConfig>,
    encryption: Option<FileEncryptionConfig>,
    llm: Option<FileLlmConfig>,
    email: Option<FileEmailConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct FileServerConfig {
    bind_host: Option<String>,
    rest_port: Option<u16>,
    grpc_port: Option<u16>,
    socket_path: Option<String>,
    cors_allowed_origins: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct FileStorageConfig {
    data_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileProfileConfig {
    display_name: Option<String>,
    primary_email: Option<String>,
    timezone: Option<String>,
    signature: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileEmbeddingConfig {
    provider: Option<String>,
    model: Option<String>,
    dimensions: Option<usize>,
    base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileSearchConfig {
    default_limit: Option<usize>,
    default_strategy: Option<String>,
    min_score: Option<f64>,
    vector_weight: Option<f64>,
    fulltext_weight: Option<f64>,
    rrf_k: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct FileGraphConfig {
    default_traversal_depth: Option<usize>,
    graph_boost_factor: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct FileAiConfig {
    auto_tagging_enabled: Option<bool>,
    auto_tagging_max_generated_tags: Option<usize>,
    auto_tagging_max_total_tags: Option<usize>,
    auto_tagging_similarity_seed_limit: Option<usize>,
    auto_tagging_min_token_length: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct FileLinkingConfig {
    auto_backlinks_enabled: Option<bool>,
    auto_backlinks_scan_limit: Option<usize>,
    auto_backlinks_max_targets: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct FileDailyNotesConfig {
    enabled: Option<bool>,
    midnight_scheduler_enabled: Option<bool>,
    namespace: Option<String>,
    title_template: Option<String>,
    content_template: Option<String>,
    default_importance: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct FileRecurrenceConfig {
    enabled: Option<bool>,
    scheduler_interval_secs: Option<u64>,
    max_instances_per_template: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct FileEncryptionConfig {
    enabled: Option<bool>,
    argon2_memory_kib: Option<u32>,
    argon2_iterations: Option<u32>,
    argon2_parallelism: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
struct FileLlmConfig {
    enabled: Option<bool>,
    base_url: Option<String>,
    model: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    timeout_secs: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct FileEmailConfig {
    enabled: Option<bool>,
    namespace: Option<String>,
    poll_interval_secs: Option<u64>,
    max_fetch: Option<usize>,
    max_attachment_bytes: Option<usize>,
    mark_seen: Option<bool>,
    imap_host: Option<String>,
    imap_port: Option<u16>,
    imap_username: Option<String>,
    imap_folder: Option<String>,
    imap_starttls: Option<bool>,
    smtp_host: Option<String>,
    smtp_port: Option<u16>,
    smtp_username: Option<String>,
    smtp_from: Option<String>,
    smtp_starttls: Option<bool>,
}

/// Load engine from config file path (expanding ~).
pub async fn load_engine(config_path: &str) -> anyhow::Result<MindVaultEngine> {
    let config = load_config(config_path)?;
    let engine = MindVaultEngine::init(config).await?;
    Ok(engine)
}

pub fn load_runtime_config(config_path: &str) -> anyhow::Result<RuntimeConfig> {
    let path = shellexpand(config_path);

    let mut engine = EngineConfig::default();
    let mut server = ServerRuntimeConfig::default();

    if std::path::Path::new(&path).exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file {path}"))?;
        let file_config: FileConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse TOML config {path}"))?;

        if let Some(storage) = file_config.storage {
            if let Some(data_dir) = storage.data_dir {
                engine.data_dir = shellexpand(&data_dir);
            }
        }

        if let Some(profile) = file_config.profile {
            if let Some(display_name) = profile.display_name {
                engine.profile.display_name = display_name;
            }
            if let Some(primary_email) = profile.primary_email {
                if primary_email.trim().is_empty() {
                    engine.profile.primary_email = None;
                } else {
                    engine.profile.primary_email = Some(primary_email);
                }
            }
            if let Some(timezone) = profile.timezone {
                engine.profile.timezone = timezone;
            }
            if let Some(signature) = profile.signature {
                if signature.trim().is_empty() {
                    engine.profile.signature = None;
                } else {
                    engine.profile.signature = Some(signature);
                }
            }
        }

        if let Some(embedding) = file_config.embedding {
            if let Some(provider) = embedding.provider {
                engine.embedding.provider = provider;
            }
            if let Some(model) = embedding.model {
                engine.embedding.model = model;
            }
            if let Some(dimensions) = embedding.dimensions {
                engine.embedding.dimensions = dimensions;
            }
            if let Some(base_url) = embedding.base_url {
                engine.embedding.base_url = Some(base_url);
            }
        }

        if let Some(llm) = file_config.llm {
            if let Some(enabled) = llm.enabled {
                engine.llm.enabled = enabled;
            }
            if let Some(base_url) = llm.base_url {
                engine.llm.base_url = base_url;
            }
            if let Some(model) = llm.model {
                engine.llm.model = model;
            }
            if let Some(max_tokens) = llm.max_tokens {
                engine.llm.max_tokens = max_tokens;
            }
            if let Some(temperature) = llm.temperature {
                engine.llm.temperature = temperature;
            }
            if let Some(timeout_secs) = llm.timeout_secs {
                engine.llm.timeout_secs = timeout_secs;
            }
        }

        if let Some(email) = file_config.email {
            if let Some(enabled) = email.enabled {
                engine.email.enabled = enabled;
            }
            if let Some(namespace) = email.namespace {
                engine.email.namespace = namespace;
            }
            if let Some(poll_interval_secs) = email.poll_interval_secs {
                engine.email.poll_interval_secs = poll_interval_secs;
            }
            if let Some(max_fetch) = email.max_fetch {
                engine.email.max_fetch = max_fetch;
            }
            if let Some(max_attachment_bytes) = email.max_attachment_bytes {
                engine.email.max_attachment_bytes = max_attachment_bytes;
            }
            if let Some(mark_seen) = email.mark_seen {
                engine.email.mark_seen = mark_seen;
            }
            if let Some(imap_host) = email.imap_host {
                if imap_host.trim().is_empty() {
                    engine.email.imap_host = None;
                } else {
                    engine.email.imap_host = Some(imap_host);
                }
            }
            if let Some(imap_port) = email.imap_port {
                engine.email.imap_port = imap_port;
            }
            if let Some(imap_username) = email.imap_username {
                if imap_username.trim().is_empty() {
                    engine.email.imap_username = None;
                } else {
                    engine.email.imap_username = Some(imap_username);
                }
            }
            if let Some(imap_folder) = email.imap_folder {
                engine.email.imap_folder = imap_folder;
            }
            if let Some(imap_starttls) = email.imap_starttls {
                engine.email.imap_starttls = imap_starttls;
            }
            if let Some(smtp_host) = email.smtp_host {
                if smtp_host.trim().is_empty() {
                    engine.email.smtp_host = None;
                } else {
                    engine.email.smtp_host = Some(smtp_host);
                }
            }
            if let Some(smtp_port) = email.smtp_port {
                engine.email.smtp_port = smtp_port;
            }
            if let Some(smtp_username) = email.smtp_username {
                if smtp_username.trim().is_empty() {
                    engine.email.smtp_username = None;
                } else {
                    engine.email.smtp_username = Some(smtp_username);
                }
            }
            if let Some(smtp_from) = email.smtp_from {
                if smtp_from.trim().is_empty() {
                    engine.email.smtp_from = None;
                } else {
                    engine.email.smtp_from = Some(smtp_from);
                }
            }
            if let Some(smtp_starttls) = email.smtp_starttls {
                engine.email.smtp_starttls = smtp_starttls;
            }
        }

        if let Some(search) = file_config.search {
            if let Some(default_limit) = search.default_limit {
                engine.search.default_limit = default_limit;
            }
            if let Some(default_strategy) = search.default_strategy {
                engine.search.default_strategy = default_strategy;
            }
            if let Some(min_score) = search.min_score {
                engine.search.min_score = min_score;
            }
            if let Some(vector_weight) = search.vector_weight {
                engine.search.vector_weight = vector_weight;
            }
            if let Some(fulltext_weight) = search.fulltext_weight {
                engine.search.fulltext_weight = fulltext_weight;
            }
            if let Some(rrf_k) = search.rrf_k {
                engine.search.rrf_k = rrf_k;
            }
        }

        if let Some(graph) = file_config.graph {
            if let Some(default_traversal_depth) = graph.default_traversal_depth {
                engine.graph.default_traversal_depth = default_traversal_depth;
            }
            if let Some(graph_boost_factor) = graph.graph_boost_factor {
                engine.graph.graph_boost_factor = graph_boost_factor;
            }
        }

        if let Some(ai) = file_config.ai {
            if let Some(auto_tagging_enabled) = ai.auto_tagging_enabled {
                engine.ai.auto_tagging_enabled = auto_tagging_enabled;
            }
            if let Some(max_generated_tags) = ai.auto_tagging_max_generated_tags {
                engine.ai.auto_tagging_max_generated_tags = max_generated_tags;
            }
            if let Some(max_total_tags) = ai.auto_tagging_max_total_tags {
                engine.ai.auto_tagging_max_total_tags = max_total_tags;
            }
            if let Some(similarity_seed_limit) = ai.auto_tagging_similarity_seed_limit {
                engine.ai.auto_tagging_similarity_seed_limit = similarity_seed_limit;
            }
            if let Some(min_token_length) = ai.auto_tagging_min_token_length {
                engine.ai.auto_tagging_min_token_length = min_token_length;
            }
        }

        if let Some(linking) = file_config.linking {
            if let Some(enabled) = linking.auto_backlinks_enabled {
                engine.linking.auto_backlinks_enabled = enabled;
            }
            if let Some(scan_limit) = linking.auto_backlinks_scan_limit {
                engine.linking.auto_backlinks_scan_limit = scan_limit;
            }
            if let Some(max_targets) = linking.auto_backlinks_max_targets {
                engine.linking.auto_backlinks_max_targets = max_targets;
            }
        }

        if let Some(daily_notes) = file_config.daily_notes {
            if let Some(enabled) = daily_notes.enabled {
                engine.daily_notes.enabled = enabled;
            }
            if let Some(enabled) = daily_notes.midnight_scheduler_enabled {
                engine.daily_notes.midnight_scheduler_enabled = enabled;
            }
            if let Some(namespace) = daily_notes.namespace {
                engine.daily_notes.namespace = namespace;
            }
            if let Some(title_template) = daily_notes.title_template {
                engine.daily_notes.title_template = title_template;
            }
            if let Some(content_template) = daily_notes.content_template {
                engine.daily_notes.content_template = content_template;
            }
            if let Some(default_importance) = daily_notes.default_importance {
                engine.daily_notes.default_importance = default_importance;
            }
        }

        if let Some(recurrence) = file_config.recurrence {
            if let Some(enabled) = recurrence.enabled {
                engine.recurrence.enabled = enabled;
            }
            if let Some(scheduler_interval_secs) = recurrence.scheduler_interval_secs {
                engine.recurrence.scheduler_interval_secs = scheduler_interval_secs;
            }
            if let Some(max_instances_per_template) = recurrence.max_instances_per_template {
                engine.recurrence.max_instances_per_template = max_instances_per_template;
            }
        }

        if let Some(encryption) = file_config.encryption {
            if let Some(enabled) = encryption.enabled {
                engine.encryption.enabled = enabled;
            }
            if let Some(argon2_memory_kib) = encryption.argon2_memory_kib {
                engine.encryption.argon2_memory_kib = argon2_memory_kib;
            }
            if let Some(argon2_iterations) = encryption.argon2_iterations {
                engine.encryption.argon2_iterations = argon2_iterations;
            }
            if let Some(argon2_parallelism) = encryption.argon2_parallelism {
                engine.encryption.argon2_parallelism = argon2_parallelism;
            }
        }

        if let Some(file_server) = file_config.server {
            if let Some(bind_host) = file_server.bind_host {
                server.bind_host = bind_host;
            }
            if let Some(rest_port) = file_server.rest_port {
                server.rest_port = rest_port;
            }
            if let Some(grpc_port) = file_server.grpc_port {
                server.grpc_port = grpc_port;
            }
            if let Some(socket_path) = file_server.socket_path {
                server.socket_path = Some(shellexpand(&socket_path));
            }
            if let Some(cors_allowed_origins) = file_server.cors_allowed_origins {
                server.cors_allowed_origins = cors_allowed_origins;
            }
        }
    }

    apply_env_overrides(&mut engine, &mut server);

    Ok(RuntimeConfig { engine, server })
}

pub fn load_config(config_path: &str) -> anyhow::Result<EngineConfig> {
    Ok(load_runtime_config(config_path)?.engine)
}

fn apply_env_overrides(engine: &mut EngineConfig, server: &mut ServerRuntimeConfig) {
    if let Ok(value) = std::env::var("MINDVAULT_DATA_DIR") {
        if !value.is_empty() {
            engine.data_dir = shellexpand(&value);
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_PROFILE_DISPLAY_NAME") {
        if !value.is_empty() {
            engine.profile.display_name = value;
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_PROFILE_PRIMARY_EMAIL") {
        if value.trim().is_empty() {
            engine.profile.primary_email = None;
        } else {
            engine.profile.primary_email = Some(value);
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_PROFILE_TIMEZONE") {
        if !value.is_empty() {
            engine.profile.timezone = value;
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_PROFILE_SIGNATURE") {
        if value.trim().is_empty() {
            engine.profile.signature = None;
        } else {
            engine.profile.signature = Some(value);
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_EMBEDDING_PROVIDER") {
        if !value.is_empty() {
            engine.embedding.provider = value;
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_EMBEDDING_MODEL") {
        if !value.is_empty() {
            engine.embedding.model = value;
        }
    }

    if let Some(dimensions) = parse_env::<usize>("MINDVAULT_EMBEDDING_DIMENSIONS") {
        engine.embedding.dimensions = dimensions;
    }

    if let Ok(value) = std::env::var("MINDVAULT_EMBEDDING_BASE_URL") {
        if !value.is_empty() {
            engine.embedding.base_url = Some(value);
        }
    }

    // LLM env overrides
    if let Some(value) = parse_env_bool("MINDVAULT_LLM_ENABLED") {
        engine.llm.enabled = value;
    }
    if let Ok(value) = std::env::var("MINDVAULT_LLM_BASE_URL") {
        if !value.is_empty() {
            engine.llm.base_url = value;
        }
    }
    if let Ok(value) = std::env::var("MINDVAULT_LLM_MODEL") {
        if !value.is_empty() {
            engine.llm.model = value;
        }
    }
    if let Some(value) = parse_env::<u32>("MINDVAULT_LLM_MAX_TOKENS") {
        engine.llm.max_tokens = value;
    }
    if let Some(value) = parse_env::<f32>("MINDVAULT_LLM_TEMPERATURE") {
        engine.llm.temperature = value;
    }
    if let Some(value) = parse_env::<u64>("MINDVAULT_LLM_TIMEOUT_SECS") {
        engine.llm.timeout_secs = value;
    }

    if let Some(value) = parse_env_bool("MINDVAULT_EMAIL_ENABLED") {
        engine.email.enabled = value;
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_NAMESPACE") {
        if !value.is_empty() {
            engine.email.namespace = value;
        }
    }
    if let Some(value) = parse_env::<u64>("MINDVAULT_EMAIL_POLL_INTERVAL_SECS") {
        engine.email.poll_interval_secs = value;
    }
    if let Some(value) = parse_env::<usize>("MINDVAULT_EMAIL_MAX_FETCH") {
        engine.email.max_fetch = value;
    }
    if let Some(value) = parse_env::<usize>("MINDVAULT_EMAIL_MAX_ATTACHMENT_BYTES") {
        engine.email.max_attachment_bytes = value;
    }
    if let Some(value) = parse_env_bool("MINDVAULT_EMAIL_MARK_SEEN") {
        engine.email.mark_seen = value;
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_IMAP_HOST") {
        if value.trim().is_empty() {
            engine.email.imap_host = None;
        } else {
            engine.email.imap_host = Some(value);
        }
    }
    if let Some(value) = parse_env::<u16>("MINDVAULT_EMAIL_IMAP_PORT") {
        engine.email.imap_port = value;
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_IMAP_USERNAME") {
        if value.trim().is_empty() {
            engine.email.imap_username = None;
        } else {
            engine.email.imap_username = Some(value);
        }
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_IMAP_FOLDER") {
        if !value.is_empty() {
            engine.email.imap_folder = value;
        }
    }
    if let Some(value) = parse_env_bool("MINDVAULT_EMAIL_IMAP_STARTTLS") {
        engine.email.imap_starttls = value;
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_SMTP_HOST") {
        if value.trim().is_empty() {
            engine.email.smtp_host = None;
        } else {
            engine.email.smtp_host = Some(value);
        }
    }
    if let Some(value) = parse_env::<u16>("MINDVAULT_EMAIL_SMTP_PORT") {
        engine.email.smtp_port = value;
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_SMTP_USERNAME") {
        if value.trim().is_empty() {
            engine.email.smtp_username = None;
        } else {
            engine.email.smtp_username = Some(value);
        }
    }
    if let Ok(value) = std::env::var("MINDVAULT_EMAIL_SMTP_FROM") {
        if value.trim().is_empty() {
            engine.email.smtp_from = None;
        } else {
            engine.email.smtp_from = Some(value);
        }
    }
    if let Some(value) = parse_env_bool("MINDVAULT_EMAIL_SMTP_STARTTLS") {
        engine.email.smtp_starttls = value;
    }

    if let Some(default_limit) = parse_env::<usize>("MINDVAULT_SEARCH_DEFAULT_LIMIT") {
        engine.search.default_limit = default_limit;
    }

    if let Ok(value) = std::env::var("MINDVAULT_SEARCH_DEFAULT_STRATEGY") {
        if !value.is_empty() {
            engine.search.default_strategy = value;
        }
    }

    if let Some(min_score) = parse_env::<f64>("MINDVAULT_SEARCH_MIN_SCORE") {
        engine.search.min_score = min_score;
    }

    if let Some(default_depth) = parse_env::<usize>("MINDVAULT_GRAPH_DEFAULT_TRAVERSAL_DEPTH") {
        engine.graph.default_traversal_depth = default_depth;
    }

    if let Some(value) = parse_env_bool("MINDVAULT_AI_AUTO_TAGGING_ENABLED") {
        engine.ai.auto_tagging_enabled = value;
    }

    if let Some(value) = parse_env::<usize>("MINDVAULT_AI_AUTO_TAGGING_MAX_GENERATED_TAGS") {
        engine.ai.auto_tagging_max_generated_tags = value;
    }

    if let Some(value) = parse_env::<usize>("MINDVAULT_AI_AUTO_TAGGING_MAX_TOTAL_TAGS") {
        engine.ai.auto_tagging_max_total_tags = value;
    }

    if let Some(value) = parse_env::<usize>("MINDVAULT_AI_AUTO_TAGGING_SIMILARITY_SEED_LIMIT") {
        engine.ai.auto_tagging_similarity_seed_limit = value;
    }

    if let Some(value) = parse_env::<usize>("MINDVAULT_AI_AUTO_TAGGING_MIN_TOKEN_LENGTH") {
        engine.ai.auto_tagging_min_token_length = value;
    }

    if let Some(value) = parse_env_bool("MINDVAULT_LINKING_AUTO_BACKLINKS_ENABLED") {
        engine.linking.auto_backlinks_enabled = value;
    }
    if let Some(value) = parse_env::<usize>("MINDVAULT_LINKING_AUTO_BACKLINKS_SCAN_LIMIT") {
        engine.linking.auto_backlinks_scan_limit = value;
    }
    if let Some(value) = parse_env::<usize>("MINDVAULT_LINKING_AUTO_BACKLINKS_MAX_TARGETS") {
        engine.linking.auto_backlinks_max_targets = value;
    }

    if let Some(value) = parse_env_bool("MINDVAULT_DAILY_NOTES_ENABLED") {
        engine.daily_notes.enabled = value;
    }
    if let Some(value) = parse_env_bool("MINDVAULT_DAILY_NOTES_MIDNIGHT_SCHEDULER_ENABLED") {
        engine.daily_notes.midnight_scheduler_enabled = value;
    }

    if let Ok(value) = std::env::var("MINDVAULT_DAILY_NOTES_NAMESPACE") {
        if !value.is_empty() {
            engine.daily_notes.namespace = value;
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_DAILY_NOTES_TITLE_TEMPLATE") {
        if !value.is_empty() {
            engine.daily_notes.title_template = value;
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_DAILY_NOTES_CONTENT_TEMPLATE") {
        if !value.is_empty() {
            engine.daily_notes.content_template = value;
        }
    }

    if let Some(value) = parse_env::<f64>("MINDVAULT_DAILY_NOTES_DEFAULT_IMPORTANCE") {
        engine.daily_notes.default_importance = value;
    }

    if let Some(value) = parse_env_bool("MINDVAULT_RECURRENCE_ENABLED") {
        engine.recurrence.enabled = value;
    }
    if let Some(value) = parse_env::<u64>("MINDVAULT_RECURRENCE_SCHEDULER_INTERVAL_SECS") {
        engine.recurrence.scheduler_interval_secs = value;
    }
    if let Some(value) = parse_env::<usize>("MINDVAULT_RECURRENCE_MAX_INSTANCES_PER_TEMPLATE") {
        engine.recurrence.max_instances_per_template = value;
    }

    if let Some(value) = parse_env_bool("MINDVAULT_ENCRYPTION_ENABLED") {
        engine.encryption.enabled = value;
    }
    if let Some(value) = parse_env::<u32>("MINDVAULT_ENCRYPTION_ARGON2_MEMORY_KIB") {
        engine.encryption.argon2_memory_kib = value;
    }
    if let Some(value) = parse_env::<u32>("MINDVAULT_ENCRYPTION_ARGON2_ITERATIONS") {
        engine.encryption.argon2_iterations = value;
    }
    if let Some(value) = parse_env::<u32>("MINDVAULT_ENCRYPTION_ARGON2_PARALLELISM") {
        engine.encryption.argon2_parallelism = value;
    }

    if let Ok(value) = std::env::var("MINDVAULT_BIND_HOST") {
        if !value.is_empty() {
            server.bind_host = value;
        }
    }

    if let Some(rest_port) = parse_env::<u16>("MINDVAULT_REST_PORT") {
        server.rest_port = rest_port;
    }

    if let Some(grpc_port) = parse_env::<u16>("MINDVAULT_GRPC_PORT") {
        server.grpc_port = grpc_port;
    }

    if let Ok(value) = std::env::var("MINDVAULT_SOCKET_PATH") {
        if !value.is_empty() {
            server.socket_path = Some(shellexpand(&value));
        }
    }

    if let Ok(value) = std::env::var("MINDVAULT_CORS_ALLOWED_ORIGINS") {
        if !value.is_empty() {
            server.cors_allowed_origins = value
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect();
        }
    }
}

fn parse_env<T: std::str::FromStr>(key: &str) -> Option<T> {
    let raw = std::env::var(key).ok()?;
    if raw.trim().is_empty() {
        return None;
    }

    raw.parse().ok()
}

fn parse_env_bool(key: &str) -> Option<bool> {
    let raw = std::env::var(key).ok()?;
    let value = raw.trim().to_ascii_lowercase();
    match value.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub fn shellexpand(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    s.to_string()
}
