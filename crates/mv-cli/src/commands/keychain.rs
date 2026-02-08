//! Sovereign Keychain CLI commands for MindVault.

use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::sync::Arc;

use mv_engine::keychain::KeychainEngine;
use mv_storage::keychain::SqliteKeychainStore;

use super::{load_config, shellexpand};

/// Build a `KeychainEngine` from the config file path.
async fn build_engine(config_path: &str) -> Result<Arc<KeychainEngine>> {
    build_engine_with_timeout(config_path, None).await
}

async fn build_engine_with_timeout(
    config_path: &str,
    timeout: Option<std::time::Duration>,
) -> Result<Arc<KeychainEngine>> {
    let config = load_config(config_path)?;
    let data_dir = shellexpand(&config.data_dir);
    std::fs::create_dir_all(&data_dir).with_context(|| format!("create data dir: {data_dir}"))?;

    let keychain_path = format!("{data_dir}/keychain.sqlite");
    let store = Arc::new(
        SqliteKeychainStore::open(std::path::Path::new(&keychain_path))
            .map_err(|e| anyhow::anyhow!("open keychain db: {e}"))?,
    );

    let cred_store = Arc::new(mv_core::credentials::CredentialStore::new(
        "mindvault-keychain",
    ));
    let engine = KeychainEngine::new(
        store,
        cred_store,
        timeout,
        Some(std::path::PathBuf::from(&keychain_path)),
    )
    .await
    .map_err(|e| anyhow::anyhow!("init keychain engine: {e}"))?;
    Ok(Arc::new(engine))
}

fn prompt_password(prompt: &str) -> Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let stdin_fd = io::stdin().as_raw_fd();
        if let Ok(mut t) = termios::Termios::from_fd(stdin_fd) {
            let original = t;
            t.c_lflag &= !termios::ECHO;
            let _ = termios::tcsetattr(stdin_fd, termios::TCSANOW, &t);

            let mut password = String::new();
            io::stdin().read_line(&mut password)?;
            eprintln!();

            let _ = termios::tcsetattr(stdin_fd, termios::TCSANOW, &original);
            return Ok(password.trim().to_string());
        }
    }

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    Ok(password.trim().to_string())
}

/// Resolve a domain identifier (UUID or name) to a UUID.
async fn resolve_domain_id(engine: &KeychainEngine, domain: &str) -> Result<uuid::Uuid> {
    // Try UUID first
    if let Ok(uuid) = uuid::Uuid::parse_str(domain) {
        return Ok(uuid);
    }
    // Fall back to name lookup
    let domains = engine
        .list_domains()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    domains
        .iter()
        .find(|d| d.name == domain)
        .map(|d| d.id)
        .ok_or_else(|| anyhow::anyhow!("domain not found: {domain}"))
}

// ---------------------------------------------------------------------------
// Vault lifecycle
// ---------------------------------------------------------------------------

pub async fn init_vault(from_env: bool, macos_bridge: bool, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;

    let password = if from_env {
        std::env::var("MINDVAULT_VAULT_PASSWORD").context("MINDVAULT_VAULT_PASSWORD not set")?
    } else {
        let pw = prompt_password("Enter vault password: ")?;
        if pw.len() < 8 {
            bail!("password must be at least 8 characters");
        }
        let confirm = prompt_password("Confirm vault password: ")?;
        if pw != confirm {
            bail!("passwords do not match");
        }
        pw
    };

    engine
        .initialize_vault(&password, macos_bridge)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("vault initialized (epoch 0)");
    if macos_bridge {
        println!("master password stored in macOS Keychain");
    }
    Ok(())
}

pub async fn unseal(
    from_env: bool,
    from_macos_keychain: bool,
    from_secure_enclave: bool,
    timeout: u64,
    config_path: &str,
) -> Result<()> {
    let timeout_dur = if timeout > 0 {
        Some(std::time::Duration::from_secs(timeout))
    } else {
        None
    };
    let engine = build_engine_with_timeout(config_path, timeout_dur).await?;

    if from_secure_enclave {
        #[cfg(target_os = "macos")]
        {
            engine
                .unseal_from_secure_enclave()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("vault unsealed (from Secure Enclave)");
        }
        #[cfg(not(target_os = "macos"))]
        {
            bail!("Secure Enclave is only available on macOS");
        }
    } else if from_macos_keychain {
        engine
            .unseal_from_macos_keychain()
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("vault unsealed (from macOS Keychain)");
    } else {
        let password = if from_env {
            std::env::var("MINDVAULT_VAULT_PASSWORD").context("MINDVAULT_VAULT_PASSWORD not set")?
        } else {
            prompt_password("Enter vault password: ")?
        };
        engine
            .unseal(&password)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("vault unsealed");
    }

    if timeout > 0 {
        println!("auto-seal timeout: {timeout}s");
    }
    Ok(())
}

pub async fn seal(config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    engine.seal().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("vault sealed");
    Ok(())
}

pub async fn status(config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let (state, meta) = engine
        .vault_status()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("Vault Status");
    println!("============");
    println!("State: {state}");

    if let Some(m) = meta {
        println!("Key epoch:      {}", m.key_epoch);
        println!("Schema version: {}", m.schema_version);
        println!(
            "Created:        {}",
            m.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        if let Some(rotated) = m.last_rotated_at {
            println!(
                "Last rotated:   {}",
                rotated.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
        if m.macos_keychain_service.is_some() {
            println!("macOS bridge:   enabled");
        }
    }
    Ok(())
}

pub async fn rotate_key(grace_hours: u32, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;

    let new_password = prompt_password("Enter NEW vault password: ")?;
    if new_password.len() < 8 {
        bail!("password must be at least 8 characters");
    }
    let confirm = prompt_password("Confirm NEW vault password: ")?;
    if new_password != confirm {
        bail!("passwords do not match");
    }

    engine
        .rotate_master_key(&new_password, grace_hours)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("master key rotated (grace period: {grace_hours}h)");
    Ok(())
}

// ---------------------------------------------------------------------------
// Domains
// ---------------------------------------------------------------------------

pub async fn domain_create(name: &str, description: Option<&str>, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let domain = engine
        .create_domain(name, description)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("domain created: {} ({})", domain.name, domain.id);
    Ok(())
}

pub async fn domain_list(config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let domains = engine
        .list_domains()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if domains.is_empty() {
        println!("no domains");
        return Ok(());
    }

    println!("{:<38} {:<20} {:<8} {}", "ID", "Name", "Creds", "Status");
    println!("{}", "─".repeat(76));
    for d in &domains {
        let status = if d.revoked_at.is_some() {
            "revoked"
        } else {
            "active"
        };
        println!(
            "{:<38} {:<20} {:<8} {}",
            d.id, d.name, d.credential_count, status
        );
    }
    Ok(())
}

pub async fn domain_revoke(id: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let uuid = uuid::Uuid::parse_str(id).context("invalid UUID")?;
    engine
        .revoke_domain(uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("domain {id} revoked");
    Ok(())
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

pub async fn store_credential(
    domain: &str,
    name: &str,
    kind: &str,
    tags: &[String],
    expires_at: Option<&str>,
    config_path: &str,
) -> Result<()> {
    let engine = build_engine(config_path).await?;

    // Read value from stdin
    eprintln!("Enter credential value (then press Enter):");
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim_end();
    if value.is_empty() {
        bail!("credential value cannot be empty");
    }

    let expires = if let Some(s) = expires_at {
        Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .context("invalid ISO 8601 date")?
                .with_timezone(&chrono::Utc),
        )
    } else {
        None
    };

    let domain_id = resolve_domain_id(&engine, domain).await?;
    let cred = engine
        .store_credential(
            domain_id,
            name,
            kind,
            value.as_bytes(),
            tags.to_vec(),
            expires,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("credential stored: {} ({})", cred.name, cred.id);
    Ok(())
}

pub async fn get_credential(id: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let uuid = uuid::Uuid::parse_str(id).context("invalid UUID")?;
    let (_cred, plaintext) = engine
        .read_credential(uuid, "cli")
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let value = String::from_utf8_lossy(&plaintext);
    println!("{value}");
    Ok(())
}

pub async fn list_credentials(
    domain: Option<&str>,
    state: Option<&str>,
    limit: u32,
    config_path: &str,
) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let domain_id = if let Some(d) = domain {
        Some(resolve_domain_id(&engine, d).await?)
    } else {
        None
    };
    let cred_state = if let Some(s) = state {
        Some(
            s.parse::<mv_core::CredentialState>()
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
    } else {
        None
    };
    let creds = engine
        .list_credentials(domain_id, cred_state, limit as usize, 0)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if creds.is_empty() {
        println!("no credentials");
        return Ok(());
    }

    println!(
        "{:<38} {:<20} {:<12} {:<10} {}",
        "ID", "Name", "Kind", "State", "Epoch"
    );
    println!("{}", "─".repeat(90));
    for c in &creds {
        println!(
            "{:<38} {:<20} {:<12} {:<10} {}",
            c.id,
            c.name,
            c.kind,
            c.state.as_str(),
            c.epoch
        );
    }
    Ok(())
}

pub async fn archive_credential(id: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let uuid = uuid::Uuid::parse_str(id).context("invalid UUID")?;
    engine
        .archive_credential(uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("credential {id} archived");
    Ok(())
}

pub async fn destroy_credential(id: &str, confirm: bool, config_path: &str) -> Result<()> {
    if !confirm {
        bail!("pass --confirm to permanently destroy this credential");
    }
    let engine = build_engine(config_path).await?;
    let uuid = uuid::Uuid::parse_str(id).context("invalid UUID")?;
    engine
        .destroy_credential(uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("credential {id} destroyed (cryptographic shred)");
    Ok(())
}

// ---------------------------------------------------------------------------
// Delegations
// ---------------------------------------------------------------------------

pub async fn delegate_create(
    credential_id: &str,
    delegatee: &str,
    expires_at: &str,
    max_depth: u32,
    config_path: &str,
) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let cred_uuid = uuid::Uuid::parse_str(credential_id).context("invalid credential UUID")?;
    let expires = chrono::DateTime::parse_from_rfc3339(expires_at)
        .context("invalid ISO 8601 date")?
        .with_timezone(&chrono::Utc);

    let perms = mv_core::DelegationPermissions {
        can_read: true,
        can_use: true,
        can_delegate: max_depth > 1,
    };

    let delegation = engine
        .create_delegation(cred_uuid, delegatee, perms, Some(expires), max_depth)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("delegation created: {}", delegation.id);
    Ok(())
}

pub async fn delegate_list(credential_id: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let cred_uuid = uuid::Uuid::parse_str(credential_id).context("invalid credential UUID")?;
    let delegations = engine
        .list_delegations(cred_uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if delegations.is_empty() {
        println!("no delegations");
        return Ok(());
    }

    println!(
        "{:<38} {:<20} {:<8} {:<8} {}",
        "ID", "Delegatee", "Depth", "Max", "Status"
    );
    println!("{}", "─".repeat(82));
    for d in &delegations {
        let status = if d.revoked_at.is_some() {
            "revoked"
        } else if d.expires_at.map_or(false, |exp| exp < chrono::Utc::now()) {
            "expired"
        } else {
            "active"
        };
        println!(
            "{:<38} {:<20} {:<8} {:<8} {}",
            d.id, d.delegatee, d.depth, d.max_depth, status
        );
    }
    Ok(())
}

pub async fn delegate_revoke(id: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let uuid = uuid::Uuid::parse_str(id).context("invalid UUID")?;
    engine
        .revoke_delegation(uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("delegation {id} revoked");
    Ok(())
}

// ---------------------------------------------------------------------------
// Zero-knowledge proofs
// ---------------------------------------------------------------------------

pub async fn prove(credential_id: &str, nonce: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let cred_uuid = uuid::Uuid::parse_str(credential_id).context("invalid credential UUID")?;
    let proof = engine
        .generate_proof(cred_uuid, nonce)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("proof:      {}", proof.proof);
    println!("generated:  {}", proof.generated_at.to_rfc3339());
    println!("expires:    {}", proof.expires_at.to_rfc3339());
    Ok(())
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

pub async fn audit_verify(config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let ok = engine
        .verify_audit_integrity()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if ok {
        println!("audit chain integrity: OK");
    } else {
        println!("audit chain integrity: FAILED");
        std::process::exit(1);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Alerts
// ---------------------------------------------------------------------------

pub async fn alerts(limit: u32, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let alerts = engine
        .list_breach_alerts(limit as usize, 0)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if alerts.is_empty() {
        println!("no breach alerts");
        return Ok(());
    }

    println!(
        "{:<38} {:<22} {:<10} {}",
        "ID", "Type", "Severity", "Timestamp"
    );
    println!("{}", "─".repeat(90));
    for a in &alerts {
        println!(
            "{:<38} {:<22} {:<10} {}",
            a.id,
            format!("{:?}", a.alert_type),
            format!("{:?}", a.severity),
            a.timestamp.format("%Y-%m-%d %H:%M:%S"),
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Vault backup / restore
// ---------------------------------------------------------------------------

pub async fn vault_backup(output: &str, config_path: &str) -> Result<()> {
    let engine = build_engine(config_path).await?;
    let password = prompt_password("Enter backup password: ")?;
    if password.len() < 8 {
        bail!("password must be at least 8 characters");
    }
    let confirm = prompt_password("Confirm backup password: ")?;
    if password != confirm {
        bail!("passwords do not match");
    }

    let data = engine
        .backup_vault(&password)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    std::fs::write(output, &data).with_context(|| format!("write backup to {output}"))?;

    println!("vault backed up to {output} ({} bytes)", data.len());
    Ok(())
}

pub async fn vault_restore(input: &str, config_path: &str) -> Result<()> {
    let data = std::fs::read(input).with_context(|| format!("read backup from {input}"))?;

    let password = prompt_password("Enter backup password: ")?;

    let engine = build_engine(config_path).await?;
    engine
        .restore_vault(&data, &password)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("vault restored from {input}");
    Ok(())
}
