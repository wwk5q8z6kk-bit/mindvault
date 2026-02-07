//! Encryption management commands for MindVault CLI.

use anyhow::{bail, Context, Result};
use mv_storage::crypto::{EncryptionConfig, KeyManager};
use std::io::{self, Write};

use super::load_config;

const SQLITE_DB_FILE: &str = "mindvault.sqlite";
const NODE_TABLE: &str = "knowledge_nodes";
const METADATA_COLUMN: &str = "metadata_json";

/// Initialize encryption for the vault.
pub async fn init(from_env: bool, config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;

    // Check if encryption is already initialized
    let key_marker_path = format!("{data_dir}/.encryption_initialized");
    if std::path::Path::new(&key_marker_path).exists() {
        bail!("Encryption is already initialized. Use `mv encrypt status` to check status.");
    }

    let password = if from_env {
        std::env::var("MINDVAULT_ENCRYPTION_KEY")
            .context("MINDVAULT_ENCRYPTION_KEY environment variable not set")?
    } else {
        prompt_password("Enter encryption password: ")?
    };

    if password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    if !from_env {
        let confirm = prompt_password("Confirm encryption password: ")?;
        if password != confirm {
            bail!("Passwords do not match");
        }
    }

    // Generate a random salt and store it
    let salt = generate_salt();
    let salt_path = format!("{data_dir}/.encryption_salt");

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("Failed to create data directory: {data_dir}"))?;

    // Test key derivation
    let crypto_config = EncryptionConfig {
        enabled: true,
        argon2_memory_kib: config.encryption.argon2_memory_kib,
        argon2_iterations: config.encryption.argon2_iterations,
        argon2_parallelism: config.encryption.argon2_parallelism,
    };

    let mut key_manager = KeyManager::new(crypto_config);
    key_manager
        .derive_master_key(&password, salt.as_bytes())
        .context("Failed to derive encryption key")?;

    // Test encryption/decryption
    let test_data = "mindvault-encryption-test";
    let encrypted = key_manager
        .encrypt_string(test_data)
        .context("Failed to encrypt test data")?;
    let decrypted = key_manager
        .decrypt_string(&encrypted)
        .context("Failed to decrypt test data")?;

    if decrypted != test_data {
        bail!("Encryption verification failed");
    }

    // Save salt
    std::fs::write(&salt_path, &salt).context("Failed to save encryption salt")?;

    // Create marker file
    std::fs::write(&key_marker_path, "v1").context("Failed to create encryption marker")?;

    println!("Encryption initialized successfully.");
    println!();
    println!("IMPORTANT: Store your password securely. If lost, your data cannot be recovered.");
    println!();
    println!("To enable encryption, set these environment variables:");
    println!("  export MINDVAULT_ENCRYPTION_ENABLED=true");
    println!("  export MINDVAULT_ENCRYPTION_KEY=\"your-password\"");
    println!();
    println!("Or add to config.toml:");
    println!("  [encryption]");
    println!("  enabled = true");

    Ok(())
}

/// Migrate an existing unencrypted vault to encrypted storage.
pub async fn migrate(dry_run: bool, config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;

    // Check if encryption is initialized
    let key_marker_path = format!("{data_dir}/.encryption_initialized");
    if !std::path::Path::new(&key_marker_path).exists() {
        bail!("Encryption not initialized. Run `mv encrypt init` first.");
    }

    // Get the encryption key
    let password = std::env::var("MINDVAULT_ENCRYPTION_KEY")
        .context("MINDVAULT_ENCRYPTION_KEY environment variable required for migration")?;

    let salt_path = format!("{data_dir}/.encryption_salt");
    let salt = std::fs::read_to_string(&salt_path).context("Failed to read encryption salt")?;

    let crypto_config = EncryptionConfig {
        enabled: true,
        argon2_memory_kib: config.encryption.argon2_memory_kib,
        argon2_iterations: config.encryption.argon2_iterations,
        argon2_parallelism: config.encryption.argon2_parallelism,
    };

    let mut key_manager = KeyManager::new(crypto_config);
    key_manager
        .derive_master_key(&password, salt.as_bytes())
        .context("Failed to derive encryption key")?;

    // Find SQLite database
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");
    if !std::path::Path::new(&db_path).exists() {
        println!("No database found at {db_path}. Nothing to migrate.");
        return Ok(());
    }

    if dry_run {
        println!("Dry run mode - no changes will be made");
        println!();
    }

    // Connect to database and count records
    let conn = rusqlite::Connection::open(&db_path).context("Failed to open database")?;

    let node_count: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {NODE_TABLE}"), [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    println!("Found {node_count} nodes to migrate");

    if dry_run {
        println!();
        println!("Would encrypt:");
        println!("  - {node_count} node content fields");
        println!("  - Associated metadata fields");
        println!();
        println!("Run without --dry-run to perform migration.");
        return Ok(());
    }

    if node_count == 0 {
        println!("No nodes to migrate.");
        return Ok(());
    }

    // Confirm migration
    print!("Proceed with encryption migration? [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Migration cancelled.");
        return Ok(());
    }

    println!("Migrating nodes...");

    // Migrate nodes in batches
    let batch_size = 100;
    let mut migrated = 0;

    loop {
        let mut stmt = conn.prepare(
            &format!(
                "SELECT id, content, {METADATA_COLUMN} FROM {NODE_TABLE} WHERE content NOT LIKE 'enc:v1:%' LIMIT ?"
            ),
        )?;

        let rows: Vec<(String, String, Option<String>)> = stmt
            .query_map([batch_size], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        if rows.is_empty() {
            break;
        }

        for (id, content, metadata) in rows {
            let encrypted_content = key_manager
                .encrypt_string(&content)
                .context("Failed to encrypt content")?;
            let encrypted_content = format!("enc:v1:{encrypted_content}");

            let encrypted_metadata = if let Some(meta) = metadata {
                let encrypted = key_manager
                    .encrypt_string(&meta)
                    .context("Failed to encrypt metadata")?;
                Some(format!("enc:v1:{encrypted}"))
            } else {
                None
            };

            conn.execute(
                &format!("UPDATE {NODE_TABLE} SET content = ?, {METADATA_COLUMN} = ? WHERE id = ?"),
                rusqlite::params![encrypted_content, encrypted_metadata, id],
            )?;

            migrated += 1;
        }

        print!("\rMigrated {migrated}/{node_count} nodes...");
        io::stdout().flush()?;
    }

    println!("\rMigrated {migrated} nodes successfully.     ");

    // Create migration marker
    let migration_marker = format!("{data_dir}/.encryption_migrated");
    std::fs::write(&migration_marker, chrono::Utc::now().to_rfc3339())?;

    println!();
    println!("Migration complete. Your vault is now encrypted.");

    Ok(())
}

/// Decrypt the vault (disable encryption).
pub async fn decrypt(confirm: bool, config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;

    // Check if encryption is initialized
    let key_marker_path = format!("{data_dir}/.encryption_initialized");
    if !std::path::Path::new(&key_marker_path).exists() {
        println!("Encryption is not initialized.");
        return Ok(());
    }

    if !confirm {
        println!("This will decrypt all data in your vault.");
        println!("Run with --confirm to proceed.");
        return Ok(());
    }

    // Get the encryption key
    let password = std::env::var("MINDVAULT_ENCRYPTION_KEY")
        .context("MINDVAULT_ENCRYPTION_KEY environment variable required for decryption")?;

    let salt_path = format!("{data_dir}/.encryption_salt");
    let salt = std::fs::read_to_string(&salt_path).context("Failed to read encryption salt")?;

    let crypto_config = EncryptionConfig {
        enabled: true,
        argon2_memory_kib: config.encryption.argon2_memory_kib,
        argon2_iterations: config.encryption.argon2_iterations,
        argon2_parallelism: config.encryption.argon2_parallelism,
    };

    let mut key_manager = KeyManager::new(crypto_config);
    key_manager
        .derive_master_key(&password, salt.as_bytes())
        .context("Failed to derive encryption key")?;

    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");
    if !std::path::Path::new(&db_path).exists() {
        println!("No database found. Removing encryption markers.");
        std::fs::remove_file(&key_marker_path).ok();
        std::fs::remove_file(&salt_path).ok();
        return Ok(());
    }

    let conn = rusqlite::Connection::open(&db_path).context("Failed to open database")?;

    let encrypted_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {NODE_TABLE} WHERE content LIKE 'enc:v1:%'"),
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if encrypted_count == 0 {
        println!("No encrypted nodes found. Removing encryption markers.");
        std::fs::remove_file(&key_marker_path).ok();
        std::fs::remove_file(&salt_path).ok();
        return Ok(());
    }

    println!("Decrypting {encrypted_count} nodes...");

    let batch_size = 100;
    let mut decrypted = 0;

    loop {
        let mut stmt = conn.prepare(
            &format!(
                "SELECT id, content, {METADATA_COLUMN} FROM {NODE_TABLE} WHERE content LIKE 'enc:v1:%' LIMIT ?"
            ),
        )?;

        let rows: Vec<(String, String, Option<String>)> = stmt
            .query_map([batch_size], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        if rows.is_empty() {
            break;
        }

        for (id, content, metadata) in rows {
            let plain_content = if let Some(encrypted) = content.strip_prefix("enc:v1:") {
                key_manager
                    .decrypt_string(encrypted)
                    .context("Failed to decrypt content")?
            } else {
                content
            };

            let plain_metadata = if let Some(meta) = metadata {
                if let Some(encrypted) = meta.strip_prefix("enc:v1:") {
                    Some(
                        key_manager
                            .decrypt_string(encrypted)
                            .context("Failed to decrypt metadata")?,
                    )
                } else {
                    Some(meta)
                }
            } else {
                None
            };

            conn.execute(
                &format!("UPDATE {NODE_TABLE} SET content = ?, {METADATA_COLUMN} = ? WHERE id = ?"),
                rusqlite::params![plain_content, plain_metadata, id],
            )?;

            decrypted += 1;
        }

        print!("\rDecrypted {decrypted}/{encrypted_count} nodes...");
        io::stdout().flush()?;
    }

    println!("\rDecrypted {decrypted} nodes successfully.     ");

    // Remove encryption markers
    std::fs::remove_file(&key_marker_path).ok();
    std::fs::remove_file(&salt_path).ok();
    let migration_marker = format!("{data_dir}/.encryption_migrated");
    std::fs::remove_file(&migration_marker).ok();

    println!();
    println!("Vault decrypted. Encryption has been disabled.");

    Ok(())
}

/// Check encryption status.
pub async fn status(config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;

    let key_marker_path = format!("{data_dir}/.encryption_initialized");
    let salt_path = format!("{data_dir}/.encryption_salt");
    let migration_marker = format!("{data_dir}/.encryption_migrated");

    let initialized = std::path::Path::new(&key_marker_path).exists();
    let salt_exists = std::path::Path::new(&salt_path).exists();
    let migrated = std::path::Path::new(&migration_marker).exists();

    println!("Encryption Status");
    println!("=================");
    println!();
    println!("Data directory: {data_dir}");
    println!();

    if !initialized {
        println!("Status: NOT INITIALIZED");
        println!();
        println!("Run `mv encrypt init` to set up encryption.");
        return Ok(());
    }

    println!("Initialized: yes");
    println!(
        "Salt file:   {}",
        if salt_exists { "present" } else { "MISSING" }
    );
    println!("Migrated:    {}", if migrated { "yes" } else { "no" });

    // Check environment
    let env_enabled = std::env::var("MINDVAULT_ENCRYPTION_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);
    let env_key_set = std::env::var("MINDVAULT_ENCRYPTION_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    println!();
    println!("Environment:");
    println!(
        "  MINDVAULT_ENCRYPTION_ENABLED: {}",
        if env_enabled { "true" } else { "false" }
    );
    println!(
        "  MINDVAULT_ENCRYPTION_KEY:     {}",
        if env_key_set { "set" } else { "not set" }
    );

    // Check database
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");
    if std::path::Path::new(&db_path).exists() {
        let conn = rusqlite::Connection::open(&db_path)?;

        let total_nodes: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {NODE_TABLE}"), [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let encrypted_nodes: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {NODE_TABLE} WHERE content LIKE 'enc:v1:%'"),
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        println!();
        println!("Database:");
        println!("  Total nodes:     {total_nodes}");
        println!("  Encrypted nodes: {encrypted_nodes}");

        if total_nodes > 0 {
            let pct = (encrypted_nodes as f64 / total_nodes as f64) * 100.0;
            println!("  Encryption:      {pct:.1}%");
        }
    } else {
        println!();
        println!("Database: not found");
    }

    Ok(())
}

fn prompt_password(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    // Try to disable echo for password input
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let stdin_fd = io::stdin().as_raw_fd();
        let mut termios = termios::Termios::from_fd(stdin_fd).ok();
        if let Some(ref mut t) = termios {
            let original = *t;
            t.c_lflag &= !termios::ECHO;
            let _ = termios::tcsetattr(stdin_fd, termios::TCSANOW, t);

            let mut password = String::new();
            io::stdin().read_line(&mut password)?;
            println!();

            let _ = termios::tcsetattr(stdin_fd, termios::TCSANOW, &original);
            return Ok(password.trim().to_string());
        }
    }

    // Fallback: read with echo
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    Ok(password.trim().to_string())
}

fn generate_salt() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}
