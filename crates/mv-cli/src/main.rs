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

        Commands::Import { from, path } => commands::import::run(from, path, &cli.config).await,

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
    }
}
