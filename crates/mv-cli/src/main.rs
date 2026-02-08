use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(
    name = "mv",
    about = "MindVault — shared brain for human and AI",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(long, global = true, default_value = "~/.mindvault/config.toml")]
    config: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a knowledge node
    Store {
        /// The content to store
        content: String,

        /// Node kind
        #[arg(long, short, default_value = "fact")]
        kind: String,

        /// Title
        #[arg(long, short)]
        title: Option<String>,

        /// Source
        #[arg(long, short)]
        source: Option<String>,

        /// Tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Namespace
        #[arg(long, short, default_value = "default")]
        ns: String,

        /// Importance (0.0 - 1.0)
        #[arg(long, short, default_value = "0.5")]
        importance: f64,
    },

    /// Recall knowledge
    Recall {
        /// The query text
        query: String,

        /// Max results
        #[arg(long, short, default_value = "5")]
        limit: usize,

        /// Search strategy (hybrid, vector, fulltext, graph)
        #[arg(long, default_value = "hybrid")]
        strategy: String,

        /// Min score threshold
        #[arg(long, default_value = "0.0")]
        min_score: f64,

        /// Namespace filter
        #[arg(long, short)]
        ns: Option<String>,
    },

    /// Full-text search
    Search {
        /// The search query
        query: String,

        /// Max results
        #[arg(long, short, default_value = "10")]
        limit: usize,

        /// Search type (fulltext, vector, hybrid)
        #[arg(long, short = 't', default_value = "fulltext")]
        r#type: String,
    },

    /// Knowledge graph operations
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },

    /// Import data from external sources
    Import {
        /// Source format
        #[arg(long)]
        from: String,

        /// Source path
        path: String,

        /// Target namespace (default varies by format)
        #[arg(long, short)]
        namespace: Option<String>,

        /// Show what would be imported without actually importing
        #[arg(long)]
        dry_run: bool,
    },

    /// Server management
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

    /// Configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show stats
    Stats,

    /// Encryption management
    Encrypt {
        #[command(subcommand)]
        action: EncryptAction,
    },

    /// Backup and restore
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },

    /// Export data
    Export {
        #[command(subcommand)]
        action: ExportAction,
    },

    /// Database maintenance
    Db {
        #[command(subcommand)]
        action: DbAction,
    },

    /// Manage secrets (API keys, tokens)
    Secret {
        #[command(subcommand)]
        action: SecretAction,
    },

    /// Sovereign keychain (sealed vault, credentials, delegations)
    Keychain {
        #[command(subcommand)]
        action: KeychainAction,
    },

    /// Start the MCP (Model Context Protocol) server on stdio
    Mcp {
        /// Access key for scoped MCP access (or set MINDVAULT_MCP_ACCESS_KEY)
        #[arg(long)]
        access_key: Option<String>,
        /// Allow unscoped read-only access (or set MINDVAULT_MCP_ALLOW_UNSCOPED=true)
        #[arg(long)]
        allow_unscoped: bool,
    },
}

#[derive(Subcommand)]
enum DbAction {
    /// Run vacuum to reclaim space
    Vacuum,
    /// Check database integrity
    Check,
    /// Analyze for query optimization
    Analyze,
    /// Show database info and statistics
    Info,
    /// Rebuild indexes
    Reindex,
}

#[derive(Subcommand)]
enum EncryptAction {
    /// Initialize encryption for the vault
    Init {
        /// Use password from environment variable MINDVAULT_ENCRYPTION_KEY
        #[arg(long)]
        from_env: bool,
    },
    /// Encrypt an existing unencrypted vault
    Migrate {
        /// Dry run (show what would be encrypted)
        #[arg(long)]
        dry_run: bool,
    },
    /// Decrypt vault (disable encryption)
    Decrypt {
        /// Confirm decryption
        #[arg(long)]
        confirm: bool,
    },
    /// Check encryption status
    Status,
}

#[derive(Subcommand)]
enum BackupAction {
    /// Create a backup
    Create {
        /// Output file path
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Restore from a backup
    Restore {
        /// Backup file path
        input: String,

        /// Force overwrite existing data
        #[arg(long)]
        force: bool,
    },
    /// List available backups
    List,
}

#[derive(Subcommand)]
enum ExportAction {
    /// Export to JSON
    Json {
        /// Output file path
        #[arg(long, short)]
        output: Option<String>,

        /// Namespace filter
        #[arg(long, short)]
        namespace: Option<String>,
    },
    /// Export to Markdown files
    Markdown {
        /// Output directory
        #[arg(long, short)]
        output: Option<String>,

        /// Namespace filter
        #[arg(long, short)]
        namespace: Option<String>,
    },
    /// Export to CSV
    Csv {
        /// Output file path
        #[arg(long, short)]
        output: Option<String>,

        /// Namespace filter
        #[arg(long, short)]
        namespace: Option<String>,
    },
}

#[derive(Subcommand)]
enum GraphAction {
    /// Show graph neighbors for a node
    Show {
        /// Node ID
        id: String,

        /// Traversal depth
        #[arg(long, short, default_value = "2")]
        depth: usize,
    },
    /// Add a relationship
    Link {
        /// Source node ID
        from: String,
        /// Target node ID
        to: String,
        /// Relationship kind
        #[arg(long, short, default_value = "relates_to")]
        kind: String,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Start the server
    Start {
        /// REST port
        #[arg(long)]
        port: Option<u16>,

        /// gRPC port
        #[arg(long)]
        grpc_port: Option<u16>,

        /// Run in foreground
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the server
    Stop,
    /// Check server status
    Status,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current config
    Show,
    /// Set a config value
    Set { key: String, value: String },
    /// Validate config
    Validate,
}

#[derive(Subcommand)]
enum SecretAction {
    /// Store a secret (prompts for value if not provided)
    Set {
        /// Secret key name (e.g., OPENAI_API_KEY)
        key: String,
        /// Secret value (omit to be prompted securely)
        value: Option<String>,
    },
    /// Retrieve a secret value
    Get {
        /// Secret key name
        key: String,
    },
    /// List stored secret names
    List,
    /// Delete a secret from all backends
    Delete {
        /// Secret key name
        key: String,
    },
    /// Show credential backend status
    Status,
}

#[derive(Subcommand)]
enum KeychainAction {
    /// Initialize the sealed vault
    Init {
        /// Read password from MINDVAULT_VAULT_PASSWORD env var
        #[arg(long)]
        from_env: bool,
        /// Store master password in macOS Keychain
        #[arg(long)]
        macos_bridge: bool,
    },
    /// Unseal (unlock) the vault
    Unseal {
        /// Read password from MINDVAULT_VAULT_PASSWORD env var
        #[arg(long)]
        from_env: bool,
        /// Unseal using password stored in macOS Keychain
        #[arg(long)]
        from_macos_keychain: bool,
        /// Unseal using macOS Secure Enclave
        #[arg(long)]
        from_secure_enclave: bool,
        /// Auto-seal timeout in seconds (default: 900)
        #[arg(long, default_value = "900")]
        timeout: u64,
    },
    /// Seal (lock) the vault
    Seal,
    /// Show vault status
    Status,
    /// Rotate the master key
    Rotate {
        /// Grace period in hours for old key
        #[arg(long, default_value = "24")]
        grace_hours: u32,
    },
    /// Create a credential domain
    #[command(name = "domain-create")]
    DomainCreate {
        /// Domain name
        name: String,
        /// Description
        #[arg(long)]
        description: Option<String>,
    },
    /// List credential domains
    #[command(name = "domain-list")]
    DomainList,
    /// Revoke a credential domain
    #[command(name = "domain-revoke")]
    DomainRevoke {
        /// Domain ID
        id: String,
    },
    /// Store a credential (reads value from stdin)
    Store {
        /// Domain name
        #[arg(long)]
        domain: String,
        /// Credential name
        #[arg(long)]
        name: String,
        /// Credential kind (e.g., api_key, token, password)
        #[arg(long)]
        kind: String,
        /// Tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Expiration date (ISO 8601)
        #[arg(long)]
        expires_at: Option<String>,
    },
    /// Decrypt and display a credential
    Get {
        /// Credential ID
        id: String,
    },
    /// List credentials (metadata only, no decryption)
    List {
        /// Filter by domain name
        #[arg(long)]
        domain: Option<String>,
        /// Filter by state (active, expiring, expired, archived, destroyed)
        #[arg(long)]
        state: Option<String>,
        /// Max results
        #[arg(long, default_value = "50")]
        limit: u32,
    },
    /// Archive a credential
    Archive {
        /// Credential ID
        id: String,
    },
    /// Permanently destroy a credential (cryptographic shred)
    Destroy {
        /// Credential ID
        id: String,
        /// Confirm destruction
        #[arg(long)]
        confirm: bool,
    },
    /// Create a delegation
    #[command(name = "delegate-create")]
    DelegateCreate {
        /// Credential ID
        #[arg(long)]
        credential_id: String,
        /// Delegatee name
        #[arg(long)]
        delegatee: String,
        /// Expiration (ISO 8601)
        #[arg(long)]
        expires_at: String,
        /// Maximum delegation depth
        #[arg(long, default_value = "1")]
        max_depth: u32,
    },
    /// List delegations for a credential
    #[command(name = "delegate-list")]
    DelegateList {
        /// Credential ID
        #[arg(long)]
        credential_id: String,
    },
    /// Revoke a delegation
    #[command(name = "delegate-revoke")]
    DelegateRevoke {
        /// Delegation ID
        id: String,
    },
    /// Generate a zero-knowledge proof
    Prove {
        /// Credential ID
        #[arg(long)]
        credential_id: String,
        /// Challenge nonce
        #[arg(long)]
        nonce: String,
    },
    /// Verify audit chain integrity
    #[command(name = "audit-verify")]
    AuditVerify,
    /// List breach alerts
    Alerts {
        /// Max results
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    /// Export an encrypted vault backup
    #[command(name = "vault-backup")]
    VaultBackup {
        /// Output file path
        #[arg(long, short)]
        output: String,
    },
    /// Restore vault from an encrypted backup
    #[command(name = "vault-restore")]
    VaultRestore {
        /// Backup file path
        input: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Store {
            content,
            kind,
            title,
            source,
            tags,
            ns,
            importance,
        } => {
            commands::store::run(
                content,
                kind,
                title,
                source,
                tags,
                ns,
                importance,
                &cli.config,
            )
            .await
        }

        Commands::Recall {
            query,
            limit,
            strategy,
            min_score,
            ns,
        } => commands::recall::run(query, limit, strategy, min_score, ns, &cli.config).await,

        Commands::Search {
            query,
            limit,
            r#type,
        } => commands::search::run(query, limit, r#type, &cli.config).await,

        Commands::Graph { action } => match action {
            GraphAction::Show { id, depth } => commands::graph::show(id, depth, &cli.config).await,
            GraphAction::Link { from, to, kind } => {
                commands::graph::link(from, to, kind, &cli.config).await
            }
        },

        Commands::Import {
            from,
            path,
            namespace,
            dry_run,
        } => commands::import::run(from, path, namespace, dry_run, &cli.config).await,

        Commands::Server { action } => match action {
            ServerAction::Start {
                port,
                grpc_port,
                foreground,
            } => commands::server::start(port, grpc_port, foreground, &cli.config).await,
            ServerAction::Stop => commands::server::stop(&cli.config).await,
            ServerAction::Status => commands::server::status(&cli.config).await,
        },

        Commands::Config { action } => match action {
            ConfigAction::Show => commands::config::show(&cli.config).await,
            ConfigAction::Set { key, value } => {
                commands::config::set(&cli.config, &key, &value).await
            }
            ConfigAction::Validate => commands::config::validate(&cli.config).await,
        },

        Commands::Stats => commands::stats::run(&cli.config).await,

        Commands::Encrypt { action } => match action {
            EncryptAction::Init { from_env } => {
                commands::encrypt::init(from_env, &cli.config).await
            }
            EncryptAction::Migrate { dry_run } => {
                commands::encrypt::migrate(dry_run, &cli.config).await
            }
            EncryptAction::Decrypt { confirm } => {
                commands::encrypt::decrypt(confirm, &cli.config).await
            }
            EncryptAction::Status => commands::encrypt::status(&cli.config).await,
        },

        Commands::Backup { action } => match action {
            BackupAction::Create { output } => commands::backup::create(output, &cli.config).await,
            BackupAction::Restore { input, force } => {
                commands::backup::restore(input, force, &cli.config).await
            }
            BackupAction::List => commands::backup::list(&cli.config).await,
        },

        Commands::Export { action } => match action {
            ExportAction::Json { output, namespace } => {
                commands::export::json(output, namespace, &cli.config).await
            }
            ExportAction::Markdown { output, namespace } => {
                commands::export::markdown(output, namespace, &cli.config).await
            }
            ExportAction::Csv { output, namespace } => {
                commands::export::csv(output, namespace, &cli.config).await
            }
        },

        Commands::Db { action } => match action {
            DbAction::Vacuum => commands::db::vacuum(&cli.config).await,
            DbAction::Check => commands::db::check(&cli.config).await,
            DbAction::Analyze => commands::db::analyze(&cli.config).await,
            DbAction::Info => commands::db::info(&cli.config).await,
            DbAction::Reindex => commands::db::reindex(&cli.config).await,
        },

        Commands::Secret { action } => match action {
            SecretAction::Set { key, value } => commands::secret::set(&key, value.as_deref()).await,
            SecretAction::Get { key } => commands::secret::get(&key).await,
            SecretAction::List => commands::secret::list().await,
            SecretAction::Delete { key } => commands::secret::delete(&key).await,
            SecretAction::Status => commands::secret::status().await,
        },

        Commands::Keychain { action } => match action {
            KeychainAction::Init {
                from_env,
                macos_bridge,
            } => commands::keychain::init_vault(from_env, macos_bridge, &cli.config).await,
            KeychainAction::Unseal {
                from_env,
                from_macos_keychain,
                from_secure_enclave,
                timeout,
            } => {
                commands::keychain::unseal(
                    from_env,
                    from_macos_keychain,
                    from_secure_enclave,
                    timeout,
                    &cli.config,
                )
                .await
            }
            KeychainAction::Seal => commands::keychain::seal(&cli.config).await,
            KeychainAction::Status => commands::keychain::status(&cli.config).await,
            KeychainAction::Rotate { grace_hours } => {
                commands::keychain::rotate_key(grace_hours, &cli.config).await
            }
            KeychainAction::DomainCreate { name, description } => {
                commands::keychain::domain_create(&name, description.as_deref(), &cli.config).await
            }
            KeychainAction::DomainList => commands::keychain::domain_list(&cli.config).await,
            KeychainAction::DomainRevoke { id } => {
                commands::keychain::domain_revoke(&id, &cli.config).await
            }
            KeychainAction::Store {
                domain,
                name,
                kind,
                tags,
                expires_at,
            } => {
                commands::keychain::store_credential(
                    &domain,
                    &name,
                    &kind,
                    &tags,
                    expires_at.as_deref(),
                    &cli.config,
                )
                .await
            }
            KeychainAction::Get { id } => {
                commands::keychain::get_credential(&id, &cli.config).await
            }
            KeychainAction::List {
                domain,
                state,
                limit,
            } => {
                commands::keychain::list_credentials(
                    domain.as_deref(),
                    state.as_deref(),
                    limit,
                    &cli.config,
                )
                .await
            }
            KeychainAction::Archive { id } => {
                commands::keychain::archive_credential(&id, &cli.config).await
            }
            KeychainAction::Destroy { id, confirm } => {
                commands::keychain::destroy_credential(&id, confirm, &cli.config).await
            }
            KeychainAction::DelegateCreate {
                credential_id,
                delegatee,
                expires_at,
                max_depth,
            } => {
                commands::keychain::delegate_create(
                    &credential_id,
                    &delegatee,
                    &expires_at,
                    max_depth,
                    &cli.config,
                )
                .await
            }
            KeychainAction::DelegateList { credential_id } => {
                commands::keychain::delegate_list(&credential_id, &cli.config).await
            }
            KeychainAction::DelegateRevoke { id } => {
                commands::keychain::delegate_revoke(&id, &cli.config).await
            }
            KeychainAction::Prove {
                credential_id,
                nonce,
            } => commands::keychain::prove(&credential_id, &nonce, &cli.config).await,
            KeychainAction::AuditVerify => commands::keychain::audit_verify(&cli.config).await,
            KeychainAction::Alerts { limit } => {
                commands::keychain::alerts(limit, &cli.config).await
            }
            KeychainAction::VaultBackup { output } => {
                commands::keychain::vault_backup(&output, &cli.config).await
            }
            KeychainAction::VaultRestore { input } => {
                commands::keychain::vault_restore(&input, &cli.config).await
            }
        },

        Commands::Mcp {
            access_key,
            allow_unscoped,
        } => commands::mcp::run(&cli.config, access_key, allow_unscoped).await,
    }
}
