//! Keychain engine — orchestrates vault lifecycle, credential CRUD,
//! delegations, ZK proofs, audit, and breach detection.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{Datelike, Timelike, Utc};
use rand::RngCore;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use uuid::Uuid;
use zeroize::Zeroizing;

use mv_core::credentials::CredentialStore;
use mv_core::error::{MvError, MvResult};
use mv_core::model::keychain::*;
use mv_core::traits::KeychainStore;
use mv_storage::crypto::EncryptionConfig;
use mv_storage::vault_crypto::{validate_argon2_params, VaultCrypto, VaultCryptoError};

// ---------------------------------------------------------------------------
// Breach Detector
// ---------------------------------------------------------------------------

pub struct BreachDetectorConfig {
    pub max_accesses_per_minute: u32,
    pub quiet_hours_start: u8,
    pub quiet_hours_end: u8,
    pub new_accessor_lookback_days: u32,
}

impl Default for BreachDetectorConfig {
    fn default() -> Self {
        Self {
            max_accesses_per_minute: 10,
            quiet_hours_start: 22,
            quiet_hours_end: 6,
            new_accessor_lookback_days: 30,
        }
    }
}

pub struct BreachDetector {
    config: BreachDetectorConfig,
}

impl BreachDetector {
    pub fn new(config: BreachDetectorConfig) -> Self {
        Self { config }
    }

    pub fn analyze(
        &self,
        patterns: &[AccessPattern],
        new_access: &AccessPattern,
    ) -> Vec<BreachAlert> {
        let mut alerts = Vec::new();

        // 1. Rapid sequential access — check accesses in the last minute
        let one_minute_ago = new_access.timestamp - chrono::Duration::seconds(60);
        let recent_count = patterns
            .iter()
            .filter(|p| p.timestamp > one_minute_ago)
            .count() as u32;
        if recent_count >= self.config.max_accesses_per_minute {
            alerts.push(BreachAlert {
                id: Uuid::now_v7(),
                credential_id: new_access.credential_id,
                alert_type: BreachAlertType::RapidSequentialAccess,
                severity: BreachSeverity::High,
                description: format!(
                    "{} accesses in the last minute (threshold: {})",
                    recent_count, self.config.max_accesses_per_minute
                ),
                details: None,
                timestamp: Utc::now(),
                acknowledged_at: None,
            });
        }

        // 2. Off-hours access
        let hour = new_access.hour_of_day;
        let in_quiet = if self.config.quiet_hours_start > self.config.quiet_hours_end {
            hour >= self.config.quiet_hours_start || hour < self.config.quiet_hours_end
        } else {
            hour >= self.config.quiet_hours_start && hour < self.config.quiet_hours_end
        };
        if in_quiet {
            alerts.push(BreachAlert {
                id: Uuid::now_v7(),
                credential_id: new_access.credential_id,
                alert_type: BreachAlertType::OffHoursAccess,
                severity: BreachSeverity::Medium,
                description: format!(
                    "Access at hour {} (quiet hours: {}-{})",
                    hour, self.config.quiet_hours_start, self.config.quiet_hours_end
                ),
                details: None,
                timestamp: Utc::now(),
                acknowledged_at: None,
            });
        }

        // 3. New accessor — check if this accessor has been seen before
        let lookback =
            Utc::now() - chrono::Duration::days(self.config.new_accessor_lookback_days as i64);
        let known = patterns
            .iter()
            .any(|p| p.accessor == new_access.accessor && p.timestamp > lookback);
        if !known && !patterns.is_empty() {
            alerts.push(BreachAlert {
                id: Uuid::now_v7(),
                credential_id: new_access.credential_id,
                alert_type: BreachAlertType::NewAccessor,
                severity: BreachSeverity::Medium,
                description: format!(
                    "New accessor '{}' not seen in the last {} days",
                    new_access.accessor, self.config.new_accessor_lookback_days
                ),
                details: None,
                timestamp: Utc::now(),
                acknowledged_at: None,
            });
        }

        alerts
    }
}

impl Default for BreachDetector {
    fn default() -> Self {
        Self::new(BreachDetectorConfig::default())
    }
}

// ---------------------------------------------------------------------------
// KeychainEngine
// ---------------------------------------------------------------------------

pub struct KeychainEngine {
    pub store: Arc<dyn KeychainStore>,
    crypto: RwLock<VaultCrypto>,
    cred_store: Arc<CredentialStore>,
    breach_detector: BreachDetector,
    failed_attempts: AtomicU32,
    locked_until: RwLock<Option<Instant>>,
    last_access: RwLock<Instant>,
    auto_seal_timeout: Duration,
    auto_seal_handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    keychain_db_path: Option<std::path::PathBuf>,
}

fn map_crypto_err(e: VaultCryptoError) -> MvError {
    match e {
        VaultCryptoError::Sealed => MvError::VaultSealed,
        other => MvError::Keychain(other.to_string()),
    }
}

impl KeychainEngine {
    pub async fn new(
        store: Arc<dyn KeychainStore>,
        cred_store: Arc<CredentialStore>,
        auto_seal_timeout: Option<Duration>,
        keychain_db_path: Option<std::path::PathBuf>,
    ) -> MvResult<Self> {
        // Disable core dumps to prevent leaking key material
        #[cfg(unix)]
        unsafe {
            let rlim = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &rlim);
        }

        Ok(Self {
            store,
            crypto: RwLock::new(VaultCrypto::new()),
            cred_store,
            breach_detector: BreachDetector::default(),
            failed_attempts: AtomicU32::new(0),
            locked_until: RwLock::new(None),
            last_access: RwLock::new(Instant::now()),
            auto_seal_timeout: auto_seal_timeout.unwrap_or(Duration::from_secs(900)),
            auto_seal_handle: tokio::sync::Mutex::new(None),
            keychain_db_path,
        })
    }

    fn touch_last_access(&self) {
        if let Ok(mut last) = self.last_access.try_write() {
            *last = Instant::now();
        }
    }

    // -----------------------------------------------------------------------
    // Vault lifecycle
    // -----------------------------------------------------------------------

    pub async fn initialize_vault(&self, password: &str, macos_bridge: bool) -> MvResult<()> {
        // Check if already initialized
        if self.store.get_vault_meta().await?.is_some() {
            return Err(MvError::Keychain("vault already initialized".into()));
        }

        // Generate salt
        let mut salt_bytes = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut salt_bytes);
        let salt = BASE64.encode(salt_bytes);

        // Derive master key
        let config = EncryptionConfig::default();
        validate_argon2_params(&config).map_err(map_crypto_err)?;
        {
            let mut crypto = self.crypto.write().await;
            crypto
                .unseal(password, salt_bytes.as_ref(), &config)
                .map_err(map_crypto_err)?;
        }

        // Create verification blob
        let verification_blob = {
            let crypto = self.crypto.read().await;
            crypto
                .generate_verification_blob()
                .map_err(map_crypto_err)?
        };

        // Create epoch 0
        let epoch = KeyEpoch {
            epoch: 0,
            wrapped_key: None,
            created_at: Utc::now(),
            grace_expires_at: None,
            retired_at: None,
        };
        self.store.insert_key_epoch(&epoch).await?;

        // Save vault metadata
        let macos_service = if macos_bridge {
            Some("mindvault-keychain".to_string())
        } else {
            None
        };
        let meta = VaultMeta {
            schema_version: 1,
            master_salt: salt,
            verification_blob,
            key_epoch: 0,
            created_at: Utc::now(),
            last_rotated_at: None,
            macos_keychain_service: macos_service.clone(),
        };
        self.store.save_vault_meta(&meta).await?;

        // macOS Keychain bridge
        if macos_bridge {
            self.store_to_macos_keychain(password)?;
        }

        self.audit_log(KeychainAuditAction::VaultInitialized, "system", None, None)
            .await?;

        Ok(())
    }

    pub async fn unseal(&self, password: &str) -> MvResult<()> {
        // Check lockout
        {
            let locked = self.locked_until.read().await;
            if let Some(until) = *locked {
                if Instant::now() < until {
                    return Err(MvError::Keychain(
                        "vault is temporarily locked due to failed attempts".into(),
                    ));
                }
            }
        }

        let attempts = self.failed_attempts.load(Ordering::SeqCst);
        if attempts >= 20 {
            return Err(MvError::Keychain(
                "vault is permanently locked after 20 failed attempts".into(),
            ));
        }

        let meta = self
            .store
            .get_vault_meta()
            .await?
            .ok_or_else(|| MvError::Keychain("vault not initialized".into()))?;

        let salt_bytes = BASE64
            .decode(&meta.master_salt)
            .map_err(|e| MvError::Keychain(format!("invalid salt: {e}")))?;

        let config = EncryptionConfig::default();
        validate_argon2_params(&config).map_err(map_crypto_err)?;

        {
            let mut crypto = self.crypto.write().await;
            crypto
                .unseal(password, &salt_bytes, &config)
                .map_err(map_crypto_err)?;
        }

        // Verify password
        let valid = {
            let crypto = self.crypto.read().await;
            crypto
                .verify_password(&meta.verification_blob)
                .map_err(map_crypto_err)?
        };

        if !valid {
            {
                let mut crypto = self.crypto.write().await;
                crypto.seal();
            }
            let new_attempts = self.failed_attempts.fetch_add(1, Ordering::SeqCst) + 1;
            // Exponential backoff: 1s, 2s, 4s, 8s... capped at 60s
            let backoff_secs = std::cmp::min(1u64 << (new_attempts - 1), 60);
            {
                let mut locked = self.locked_until.write().await;
                *locked = Some(Instant::now() + Duration::from_secs(backoff_secs));
            }
            // Persist lockout state
            let _ = self
                .store
                .set_lockout_state(new_attempts, Some(chrono::Utc::now().to_rfc3339()))
                .await;

            self.audit_log(
                KeychainAuditAction::VaultUnlockFailed,
                "system",
                None,
                Some(serde_json::json!({"attempts": new_attempts})),
            )
            .await?;
            return Err(MvError::Keychain("invalid password".into()));
        }

        // Success — reset lockout
        self.failed_attempts.store(0, Ordering::SeqCst);
        {
            let mut locked = self.locked_until.write().await;
            *locked = None;
        }
        let _ = self.store.set_lockout_state(0, None).await;

        // Load grace keys from wrapped key epochs
        let epochs = self.store.list_key_epochs().await?;
        let now = chrono::Utc::now();
        for epoch_entry in &epochs {
            if let Some(ref wrapped) = epoch_entry.wrapped_key {
                // Only load if grace period hasn't expired
                let grace_ok = epoch_entry.grace_expires_at.map_or(false, |exp| exp > now);
                if grace_ok {
                    let mut crypto = self.crypto.write().await;
                    if let Err(e) = crypto.unwrap_grace_key(epoch_entry.epoch, wrapped) {
                        tracing::warn!(epoch = epoch_entry.epoch, error = %e, "failed to unwrap grace key");
                    }
                }
            }
        }

        self.audit_log(KeychainAuditAction::VaultUnlocked, "system", None, None)
            .await?;

        Ok(())
    }

    pub async fn unseal_from_macos_keychain(&self) -> MvResult<()> {
        let password = self
            .cred_store
            .get_secret_string("MINDVAULT_VAULT_KEY")
            .ok_or_else(|| MvError::Keychain("vault key not found in macOS Keychain".into()))?;
        self.unseal(&password).await
    }

    pub fn store_to_macos_keychain(&self, password: &str) -> MvResult<()> {
        use mv_core::credentials::SecretSource;
        self.cred_store
            .set_in("MINDVAULT_VAULT_KEY", password, SecretSource::OsKeyring)
            .map_err(|e| MvError::Keychain(format!("macOS Keychain: {e}")))?;
        Ok(())
    }

    pub async fn seal(&self) -> MvResult<()> {
        // Abort auto-seal task if running
        {
            let mut handle = self.auto_seal_handle.lock().await;
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
        {
            let mut crypto = self.crypto.write().await;
            crypto.seal();
        }
        self.audit_log(KeychainAuditAction::VaultLocked, "system", None, None)
            .await?;
        Ok(())
    }

    pub async fn vault_status(&self) -> MvResult<(VaultState, Option<VaultMeta>)> {
        let meta = self.store.get_vault_meta().await?;
        let crypto = self.crypto.read().await;
        let state = match &meta {
            None => VaultState::Uninitialized,
            Some(_) if crypto.is_unsealed() => VaultState::Unsealed,
            Some(_) => VaultState::Sealed,
        };
        Ok((state, meta))
    }

    /// Start the auto-seal background task. Checks every 30s and seals on idle.
    pub async fn start_auto_seal(self: &Arc<Self>) {
        let mut handle = self.auto_seal_handle.lock().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
        let engine = Arc::clone(self);
        *handle = Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let elapsed = {
                    let last = engine.last_access.read().await;
                    last.elapsed()
                };
                if elapsed >= engine.auto_seal_timeout {
                    tracing::info!("auto-sealing vault after idle timeout");
                    let _ = engine.seal().await;
                    return;
                }
            }
        }));
    }

    /// Returns seconds until auto-seal, or None if not applicable.
    pub async fn auto_seal_remaining(&self) -> Option<u64> {
        let crypto = self.crypto.read().await;
        if !crypto.is_unsealed() {
            return None;
        }
        let last = self.last_access.read().await;
        let elapsed = last.elapsed();
        if elapsed >= self.auto_seal_timeout {
            Some(0)
        } else {
            Some((self.auto_seal_timeout - elapsed).as_secs())
        }
    }

    // -----------------------------------------------------------------------
    // Key rotation
    // -----------------------------------------------------------------------

    pub async fn rotate_master_key(
        &self,
        new_password: &str,
        grace_period_hours: u32,
    ) -> MvResult<()> {
        self.touch_last_access();

        let meta = self
            .store
            .get_vault_meta()
            .await?
            .ok_or_else(|| MvError::Keychain("vault not initialized".into()))?;

        let old_epoch = meta.key_epoch;
        let new_epoch = old_epoch + 1;

        // Extract old master key before re-keying
        let old_master = {
            let crypto = self.crypto.read().await;
            crypto.extract_master_key().map_err(map_crypto_err)?
        };

        // Generate new salt
        let mut new_salt_bytes = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut new_salt_bytes);
        let new_salt = BASE64.encode(new_salt_bytes);

        // Derive new master key
        {
            let mut crypto = self.crypto.write().await;
            let config = EncryptionConfig::default();
            *crypto = VaultCrypto::new();
            crypto
                .unseal(new_password, &new_salt_bytes, &config)
                .map_err(map_crypto_err)?;

            // Add old key as grace key for re-encryption
            crypto.add_grace_key(old_epoch, old_master.clone());
        }

        // Wrap old master key with new master key for storage
        let wrapped_old_key = {
            let crypto = self.crypto.read().await;
            let new_master = crypto.extract_master_key().map_err(map_crypto_err)?;
            let encrypted = VaultCrypto::aes_gcm_encrypt_pub(&*new_master, &*old_master)
                .map_err(map_crypto_err)?;
            BASE64.encode(encrypted)
        };

        // Create new verification blob
        let verification_blob = {
            let crypto = self.crypto.read().await;
            crypto
                .generate_verification_blob()
                .map_err(map_crypto_err)?
        };

        // Insert new epoch with wrapped old key
        let new_key_epoch = KeyEpoch {
            epoch: new_epoch,
            wrapped_key: Some(wrapped_old_key),
            created_at: Utc::now(),
            grace_expires_at: Some(chrono::DateTime::<Utc>::from(
                std::time::SystemTime::now()
                    + std::time::Duration::from_secs(grace_period_hours as u64 * 3600),
            )),
            retired_at: None,
        };
        self.store.insert_key_epoch(&new_key_epoch).await?;

        // Retire old epoch
        self.store.retire_key_epoch(old_epoch).await?;

        // Update vault meta
        let updated_meta = VaultMeta {
            schema_version: meta.schema_version,
            master_salt: new_salt,
            verification_blob,
            key_epoch: new_epoch,
            created_at: meta.created_at,
            last_rotated_at: Some(Utc::now()),
            macos_keychain_service: meta.macos_keychain_service,
        };
        self.store.save_vault_meta(&updated_meta).await?;

        // Re-encrypt all credentials: decrypt with OLD epoch grace key, encrypt with NEW master
        self.re_encrypt_all_credentials(old_epoch, new_epoch)
            .await?;

        self.audit_log(
            KeychainAuditAction::KeyRotated,
            "system",
            None,
            Some(serde_json::json!({
                "old_epoch": old_epoch,
                "new_epoch": new_epoch,
                "grace_period_hours": grace_period_hours,
            })),
        )
        .await?;

        Ok(())
    }

    async fn re_encrypt_all_credentials(&self, old_epoch: u64, new_epoch: u64) -> MvResult<()> {
        let creds = self
            .store
            .list_credentials(None, Some(CredentialState::Active), 10000, 0)
            .await?;

        let crypto = self.crypto.read().await;
        for mut cred in creds {
            let domain = self.store.get_domain(cred.domain_id).await?;
            if let Some(domain) = domain {
                // Decrypt with old epoch grace key
                let plaintext = crypto
                    .decrypt_credential_with_epoch(
                        &cred.encrypted_value,
                        &domain.derivation_info,
                        &cred.derivation_info,
                        old_epoch,
                    )
                    .map_err(map_crypto_err)?;

                // Re-encrypt with current (new) master key
                let encrypted = crypto
                    .encrypt_credential(&*plaintext, &domain.derivation_info, &cred.derivation_info)
                    .map_err(map_crypto_err)?;

                cred.encrypted_value = encrypted;
                cred.epoch = new_epoch;
                cred.updated_at = Utc::now();
                self.store.update_credential(&cred).await?;
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Domains
    // -----------------------------------------------------------------------

    pub async fn create_domain(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> MvResult<DomainKey> {
        let meta = self
            .store
            .get_vault_meta()
            .await?
            .ok_or_else(|| MvError::Keychain("vault not initialized".into()))?;

        let derivation_info = format!("domain:{name}");
        let mut domain = DomainKey::new(name, &derivation_info).with_epoch(meta.key_epoch);
        if let Some(desc) = description {
            domain = domain.with_description(desc);
        }

        self.store.insert_domain(&domain).await?;
        self.audit_log(
            KeychainAuditAction::DomainCreated,
            "system",
            Some(&domain.id.to_string()),
            Some(serde_json::json!({"name": name})),
        )
        .await?;

        Ok(domain)
    }

    pub async fn list_domains(&self) -> MvResult<Vec<DomainKey>> {
        self.store.list_domains().await
    }

    pub async fn revoke_domain(&self, id: Uuid) -> MvResult<()> {
        self.store.revoke_domain(id).await?;
        self.audit_log(
            KeychainAuditAction::DomainRevoked,
            "system",
            Some(&id.to_string()),
            None,
        )
        .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Credential CRUD
    // -----------------------------------------------------------------------

    pub async fn store_credential(
        &self,
        domain_id: Uuid,
        name: &str,
        kind: &str,
        value: &[u8],
        tags: Vec<String>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> MvResult<StoredCredential> {
        self.touch_last_access();

        let domain = self
            .store
            .get_domain(domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        if domain.revoked_at.is_some() {
            return Err(MvError::Keychain("domain is revoked".into()));
        }

        let cred_derivation = format!("cred:{name}:{}", Uuid::now_v7());

        // Encrypt
        let encrypted = {
            let crypto = self.crypto.read().await;
            crypto
                .encrypt_credential(value, &domain.derivation_info, &cred_derivation)
                .map_err(map_crypto_err)?
        };

        let meta = self
            .store
            .get_vault_meta()
            .await?
            .ok_or_else(|| MvError::Keychain("vault not initialized".into()))?;

        let mut cred = StoredCredential::new(domain_id, name, kind, encrypted, &cred_derivation)
            .with_tags(tags)
            .with_epoch(meta.key_epoch);

        if let Some(exp) = expires_at {
            cred = cred.with_expires_at(exp);
        }

        self.store.insert_credential(&cred).await?;
        self.audit_log(
            KeychainAuditAction::CredentialStored,
            "system",
            Some(&cred.id.to_string()),
            Some(
                serde_json::json!({"name": name, "kind": kind, "domain_id": domain_id.to_string()}),
            ),
        )
        .await?;

        Ok(cred)
    }

    pub async fn read_credential(
        &self,
        id: Uuid,
        subject: &str,
    ) -> MvResult<(StoredCredential, Zeroizing<Vec<u8>>)> {
        self.touch_last_access();

        let cred = self
            .store
            .get_credential(id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        if cred.state == CredentialState::Destroyed {
            return Err(MvError::Keychain("credential has been destroyed".into()));
        }

        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        // Decrypt
        let plaintext = {
            let crypto = self.crypto.read().await;
            crypto
                .decrypt_credential(
                    &cred.encrypted_value,
                    &domain.derivation_info,
                    &cred.derivation_info,
                )
                .map_err(map_crypto_err)?
        };

        // Touch credential
        self.store.touch_credential(id).await?;

        // Record access pattern + breach detection
        let now = Utc::now();
        let pattern = AccessPattern {
            credential_id: id,
            accessor: subject.to_string(),
            source_ip: None,
            timestamp: now,
            hour_of_day: now.hour() as u8,
            day_of_week: now.weekday().num_days_from_monday() as u8,
        };
        self.store.record_access_pattern(&pattern).await?;

        let recent_patterns = self.store.get_access_patterns(id, 100).await?;
        let alerts = self.breach_detector.analyze(&recent_patterns, &pattern);
        for alert in &alerts {
            self.store.insert_breach_alert(alert).await?;
            self.audit_log(
                KeychainAuditAction::BreachDetected,
                subject,
                Some(&id.to_string()),
                Some(serde_json::json!({
                    "alert_type": alert.alert_type.as_str(),
                    "severity": alert.severity.as_str(),
                })),
            )
            .await?;
        }

        self.audit_log(
            KeychainAuditAction::CredentialRead,
            subject,
            Some(&id.to_string()),
            None,
        )
        .await?;

        Ok((cred, plaintext))
    }

    pub async fn update_credential_value(
        &self,
        id: Uuid,
        new_value: &[u8],
    ) -> MvResult<StoredCredential> {
        self.touch_last_access();

        let mut cred = self
            .store
            .get_credential(id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        let encrypted = {
            let crypto = self.crypto.read().await;
            crypto
                .encrypt_credential(new_value, &domain.derivation_info, &cred.derivation_info)
                .map_err(map_crypto_err)?
        };

        cred.encrypted_value = encrypted;
        cred.version += 1;
        cred.updated_at = Utc::now();
        self.store.update_credential(&cred).await?;

        self.audit_log(
            KeychainAuditAction::CredentialUpdated,
            "system",
            Some(&id.to_string()),
            Some(serde_json::json!({"new_version": cred.version})),
        )
        .await?;

        Ok(cred)
    }

    pub async fn update_credential_metadata(
        &self,
        id: Uuid,
        description: Option<String>,
        tags: Option<Vec<String>>,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> MvResult<StoredCredential> {
        self.touch_last_access();

        let mut cred = self
            .store
            .get_credential(id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        if let Some(desc) = description {
            cred.description = Some(desc);
        }
        if let Some(tags) = tags {
            cred.tags = tags;
        }
        if let Some(metadata) = metadata {
            cred.metadata = metadata;
        }
        if let Some(expires_at) = expires_at {
            cred.expires_at = Some(expires_at);
        }

        cred.updated_at = Utc::now();
        cred.version += 1;
        self.store.update_credential(&cred).await?;

        self.audit_log(
            KeychainAuditAction::CredentialUpdated,
            "system",
            Some(&id.to_string()),
            Some(serde_json::json!({"new_version": cred.version})),
        )
        .await?;

        Ok(cred)
    }

    pub async fn archive_credential(&self, id: Uuid) -> MvResult<()> {
        self.touch_last_access();

        let mut cred = self
            .store
            .get_credential(id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        cred.state = CredentialState::Archived;
        cred.archived_at = Some(Utc::now());
        cred.updated_at = Utc::now();
        self.store.update_credential(&cred).await?;

        self.audit_log(
            KeychainAuditAction::CredentialArchived,
            "system",
            Some(&id.to_string()),
            None,
        )
        .await?;

        Ok(())
    }

    pub async fn destroy_credential(&self, id: Uuid) -> MvResult<()> {
        self.touch_last_access();

        // Revoke all delegations first
        self.store.revoke_delegations_for_credential(id).await?;

        // Shred the credential
        self.store.shred_credential(id).await?;

        self.audit_log(
            KeychainAuditAction::CredentialDestroyed,
            "system",
            Some(&id.to_string()),
            None,
        )
        .await?;

        Ok(())
    }

    pub async fn list_credentials(
        &self,
        domain_id: Option<Uuid>,
        state: Option<CredentialState>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<StoredCredential>> {
        self.store
            .list_credentials(domain_id, state, limit, offset)
            .await
    }

    // -----------------------------------------------------------------------
    // Lifecycle engine
    // -----------------------------------------------------------------------

    pub async fn run_lifecycle_transitions(&self) -> MvResult<u32> {
        let now = Utc::now();
        let mut transitioned = 0u32;

        // Check active credentials approaching expiry (within 7 days)
        let actives = self
            .store
            .list_credentials(None, Some(CredentialState::Active), 10000, 0)
            .await?;

        for mut cred in actives {
            if let Some(expires_at) = cred.expires_at {
                if expires_at <= now {
                    cred.state = CredentialState::Expired;
                    cred.updated_at = now;
                    self.store.update_credential(&cred).await?;
                    transitioned += 1;
                } else if expires_at <= now + chrono::Duration::days(7) {
                    cred.state = CredentialState::Expiring;
                    cred.updated_at = now;
                    self.store.update_credential(&cred).await?;
                    transitioned += 1;
                }
            }
        }

        // Check expiring credentials that have now expired
        let expiring = self
            .store
            .list_credentials(None, Some(CredentialState::Expiring), 10000, 0)
            .await?;

        for mut cred in expiring {
            if let Some(expires_at) = cred.expires_at {
                if expires_at <= now {
                    cred.state = CredentialState::Expired;
                    cred.updated_at = now;
                    self.store.update_credential(&cred).await?;
                    transitioned += 1;
                }
            }
        }

        if transitioned > 0 {
            self.audit_log(
                KeychainAuditAction::LifecycleTransition,
                "system",
                None,
                Some(serde_json::json!({"transitioned": transitioned})),
            )
            .await?;
        }

        Ok(transitioned)
    }

    // -----------------------------------------------------------------------
    // Delegations
    // -----------------------------------------------------------------------

    pub async fn create_delegation(
        &self,
        credential_id: Uuid,
        delegatee: &str,
        permissions: DelegationPermissions,
        expires_at: Option<chrono::DateTime<Utc>>,
        max_depth: u32,
    ) -> MvResult<Delegation> {
        self.touch_last_access();

        let cred = self
            .store
            .get_credential(credential_id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        let perms_str = format!(
            "r:{},u:{},d:{}",
            permissions.can_read, permissions.can_use, permissions.can_delegate
        );
        let chain_hash = {
            let crypto = self.crypto.read().await;
            crypto
                .compute_chain_hash(
                    &domain.derivation_info,
                    None,
                    &credential_id.to_string(),
                    delegatee,
                    &perms_str,
                    expires_at.as_ref().map(|dt| dt.to_rfc3339()).as_deref(),
                    0,
                    max_depth,
                )
                .map_err(map_crypto_err)?
        };

        let delegation = Delegation {
            id: Uuid::now_v7(),
            credential_id,
            delegatee: delegatee.to_string(),
            parent_id: None,
            permissions,
            chain_hash,
            created_at: Utc::now(),
            expires_at,
            revoked_at: None,
            max_depth,
            depth: 0,
        };

        self.store.insert_delegation(&delegation).await?;
        self.audit_log(
            KeychainAuditAction::DelegationCreated,
            "system",
            Some(&delegation.id.to_string()),
            Some(serde_json::json!({
                "credential_id": credential_id.to_string(),
                "delegatee": delegatee,
            })),
        )
        .await?;

        Ok(delegation)
    }

    pub async fn sub_delegate(
        &self,
        parent_id: Uuid,
        delegatee: &str,
        permissions: DelegationPermissions,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> MvResult<Delegation> {
        self.touch_last_access();

        let parent = self
            .store
            .get_delegation(parent_id)
            .await?
            .ok_or_else(|| MvError::Keychain("parent delegation not found".into()))?;

        if parent.revoked_at.is_some() {
            return Err(MvError::Keychain("parent delegation is revoked".into()));
        }
        if !parent.permissions.can_delegate {
            return Err(MvError::Keychain(
                "parent delegation does not allow sub-delegation".into(),
            ));
        }
        if parent.depth + 1 >= parent.max_depth {
            return Err(MvError::Keychain("delegation depth limit reached".into()));
        }

        let cred = self
            .store
            .get_credential(parent.credential_id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;
        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        let perms_str = format!(
            "r:{},u:{},d:{}",
            permissions.can_read, permissions.can_use, permissions.can_delegate
        );
        let chain_hash = {
            let crypto = self.crypto.read().await;
            crypto
                .compute_chain_hash(
                    &domain.derivation_info,
                    Some(&parent.chain_hash),
                    &parent.credential_id.to_string(),
                    delegatee,
                    &perms_str,
                    expires_at.as_ref().map(|dt| dt.to_rfc3339()).as_deref(),
                    parent.depth + 1,
                    parent.max_depth,
                )
                .map_err(map_crypto_err)?
        };

        let delegation = Delegation {
            id: Uuid::now_v7(),
            credential_id: parent.credential_id,
            delegatee: delegatee.to_string(),
            parent_id: Some(parent_id),
            permissions,
            chain_hash,
            created_at: Utc::now(),
            expires_at,
            revoked_at: None,
            max_depth: parent.max_depth,
            depth: parent.depth + 1,
        };

        self.store.insert_delegation(&delegation).await?;
        self.audit_log(
            KeychainAuditAction::DelegationCreated,
            "system",
            Some(&delegation.id.to_string()),
            Some(serde_json::json!({
                "parent_id": parent_id.to_string(),
                "delegatee": delegatee,
                "depth": delegation.depth,
            })),
        )
        .await?;

        Ok(delegation)
    }

    pub async fn revoke_delegation(&self, id: Uuid) -> MvResult<()> {
        self.store.revoke_delegation(id).await?;
        self.audit_log(
            KeychainAuditAction::DelegationRevoked,
            "system",
            Some(&id.to_string()),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn list_delegations(&self, credential_id: Uuid) -> MvResult<Vec<Delegation>> {
        self.store.list_delegations(credential_id).await
    }

    pub async fn read_credential_via_delegation(
        &self,
        delegation_id: Uuid,
        subject: &str,
    ) -> MvResult<(StoredCredential, Zeroizing<Vec<u8>>)> {
        self.touch_last_access();

        let delegation = self
            .store
            .get_delegation(delegation_id)
            .await?
            .ok_or_else(|| MvError::Keychain("delegation not found".into()))?;

        if delegation.revoked_at.is_some() {
            return Err(MvError::Keychain("delegation is revoked".into()));
        }
        if let Some(expires_at) = delegation.expires_at {
            if expires_at <= Utc::now() {
                return Err(MvError::Keychain("delegation has expired".into()));
            }
        }
        if !delegation.permissions.can_read {
            return Err(MvError::Keychain(
                "delegation does not grant read access".into(),
            ));
        }

        self.read_credential(delegation.credential_id, subject)
            .await
    }

    // -----------------------------------------------------------------------
    // Zero-knowledge proofs
    // -----------------------------------------------------------------------

    pub async fn generate_proof(
        &self,
        credential_id: Uuid,
        challenge_nonce: &str,
    ) -> MvResult<AccessProof> {
        self.touch_last_access();

        let cred = self
            .store
            .get_credential(credential_id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        // Decrypt the credential to get the raw value for proof generation
        let plaintext = {
            let crypto = self.crypto.read().await;
            crypto
                .decrypt_credential(
                    &cred.encrypted_value,
                    &domain.derivation_info,
                    &cred.derivation_info,
                )
                .map_err(map_crypto_err)?
        };

        let proof = {
            let crypto = self.crypto.read().await;
            crypto
                .generate_zk_proof(&*plaintext, challenge_nonce)
                .map_err(map_crypto_err)?
        };

        let now = Utc::now();
        let zk_proof = AccessProof {
            credential_id,
            challenge_nonce: challenge_nonce.to_string(),
            proof,
            generated_at: now,
            expires_at: now + chrono::Duration::minutes(5),
        };

        self.audit_log(
            KeychainAuditAction::ProofGenerated,
            "system",
            Some(&credential_id.to_string()),
            None,
        )
        .await?;

        Ok(zk_proof)
    }

    pub async fn verify_proof(&self, proof: &AccessProof) -> MvResult<bool> {
        self.touch_last_access();

        if proof.expires_at <= Utc::now() {
            return Ok(false);
        }

        let cred = self
            .store
            .get_credential(proof.credential_id)
            .await?
            .ok_or_else(|| MvError::Keychain("credential not found".into()))?;

        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        let plaintext = {
            let crypto = self.crypto.read().await;
            crypto
                .decrypt_credential(
                    &cred.encrypted_value,
                    &domain.derivation_info,
                    &cred.derivation_info,
                )
                .map_err(map_crypto_err)?
        };

        let valid = {
            let crypto = self.crypto.read().await;
            crypto
                .verify_zk_proof(&*plaintext, &proof.challenge_nonce, &proof.proof)
                .map_err(map_crypto_err)?
        };

        self.audit_log(
            KeychainAuditAction::ProofVerified,
            "system",
            Some(&proof.credential_id.to_string()),
            Some(serde_json::json!({"valid": valid})),
        )
        .await?;

        Ok(valid)
    }

    // -----------------------------------------------------------------------
    // Audit + Breach
    // -----------------------------------------------------------------------

    pub async fn list_audit_trail(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KeychainAuditEntry>> {
        self.store.list_audit_entries(limit, offset).await
    }

    pub async fn verify_audit_integrity(&self) -> MvResult<bool> {
        // First check the hash chain
        let chain_ok = self.store.verify_audit_chain().await?;
        if !chain_ok {
            return Ok(false);
        }

        // Then verify HMAC signatures on entries that have them
        let crypto = self.crypto.read().await;
        if !crypto.is_unsealed() {
            return Ok(chain_ok); // Can't verify signatures when sealed
        }

        let entries = self.store.list_audit_entries(100000, 0).await?;
        for entry in &entries {
            if let Some(ref sig) = entry.signature {
                let valid = crypto
                    .verify_audit_signature(
                        entry.sequence,
                        entry.action.as_str(),
                        &entry.subject,
                        entry.resource_id.as_deref(),
                        &entry.entry_hash,
                        &entry.timestamp.to_rfc3339(),
                        sig,
                    )
                    .map_err(map_crypto_err)?;
                if !valid {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    pub async fn backup_vault(&self, password: &str) -> MvResult<Vec<u8>> {
        let db_path = self
            .keychain_db_path
            .as_ref()
            .ok_or_else(|| MvError::Keychain("keychain db path not set".into()))?;
        crate::backup::export_vault(db_path, password)
            .map_err(|e| MvError::Keychain(format!("backup failed: {e}")))
    }

    pub async fn restore_vault(&self, data: &[u8], password: &str) -> MvResult<()> {
        let db_path = self
            .keychain_db_path
            .as_ref()
            .ok_or_else(|| MvError::Keychain("keychain db path not set".into()))?;
        crate::backup::import_vault(data, password, db_path)
            .map_err(|e| MvError::Keychain(format!("restore failed: {e}")))
    }

    #[cfg(target_os = "macos")]
    pub async fn unseal_from_secure_enclave(&self) -> MvResult<()> {
        let wrapped = self
            .cred_store
            .get_secret_string("MINDVAULT_SE_WRAPPED_KEY")
            .ok_or_else(|| MvError::Keychain("SE wrapped key not found".into()))?;
        let key_bytes = crate::secure_enclave::unwrap_key_from_se(&wrapped)
            .map_err(|e| MvError::Keychain(format!("Secure Enclave: {e}")))?;
        if key_bytes.len() != 32 {
            return Err(MvError::Keychain(
                "invalid key length from Secure Enclave".into(),
            ));
        }
        let mut key = zeroize::Zeroizing::new([0u8; 32]);
        key.copy_from_slice(&key_bytes);
        {
            let mut crypto = self.crypto.write().await;
            crypto.set_master_key(key);
        }
        self.audit_log(
            KeychainAuditAction::VaultUnlocked,
            "secure_enclave",
            None,
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn list_breach_alerts(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<BreachAlert>> {
        self.store.list_breach_alerts(limit, offset).await
    }

    pub async fn acknowledge_alert(&self, id: Uuid) -> MvResult<()> {
        self.store.acknowledge_breach_alert(id).await
    }

    // -----------------------------------------------------------------------
    // Private: audit logging
    // -----------------------------------------------------------------------

    async fn audit_log(
        &self,
        action: KeychainAuditAction,
        subject: &str,
        resource_id: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> MvResult<()> {
        let previous = self.store.get_latest_audit_entry().await?;
        let prev_hash = previous.as_ref().map(|e| e.entry_hash.as_str());
        let sequence = previous.as_ref().map(|e| e.sequence + 1).unwrap_or(1);
        let timestamp = Utc::now();

        let entry_hash = VaultCrypto::compute_audit_hash(
            prev_hash,
            sequence,
            action.as_str(),
            subject,
            resource_id,
            &timestamp.to_rfc3339(),
        );

        // Sign the entry if the vault is unsealed
        let signature = {
            let crypto = self.crypto.read().await;
            if crypto.is_unsealed() {
                crypto
                    .sign_audit_entry(
                        sequence,
                        action.as_str(),
                        subject,
                        resource_id,
                        &entry_hash,
                        &timestamp.to_rfc3339(),
                    )
                    .ok()
            } else {
                None
            }
        };

        let entry = KeychainAuditEntry {
            id: Uuid::now_v7(),
            sequence,
            action,
            subject: subject.to_string(),
            resource_id: resource_id.map(|s| s.to_string()),
            details,
            entry_hash,
            previous_hash: prev_hash.map(|s| s.to_string()),
            timestamp,
            source_ip: None,
            signature,
        };

        self.store.append_audit_entry(&entry).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Agent bridge helpers (used by KeychainBackend)
    // -----------------------------------------------------------------------

    /// Synchronous check whether the vault is currently unsealed.
    /// Uses `try_read` to avoid blocking — returns `false` if the lock is contended.
    pub fn is_unsealed_sync(&self) -> bool {
        match self.crypto.try_read() {
            Ok(guard) => guard.is_unsealed(),
            Err(_) => false,
        }
    }

    /// Find a credential by name within a specific domain and decrypt it.
    pub async fn read_credential_by_name(
        &self,
        domain_id: Uuid,
        name: &str,
    ) -> MvResult<Option<(StoredCredential, Zeroizing<Vec<u8>>)>> {
        let creds = self
            .store
            .list_credentials(Some(domain_id), Some(CredentialState::Active), 1000, 0)
            .await?;

        let cred = match creds.into_iter().find(|c| c.name == name) {
            Some(c) => c,
            None => return Ok(None),
        };

        let domain = self
            .store
            .get_domain(cred.domain_id)
            .await?
            .ok_or_else(|| MvError::Keychain("domain not found".into()))?;

        let plaintext = {
            let crypto = self.crypto.read().await;
            crypto
                .decrypt_credential(
                    &cred.encrypted_value,
                    &domain.derivation_info,
                    &cred.derivation_info,
                )
                .map_err(map_crypto_err)?
        };

        self.store.touch_credential(cred.id).await?;
        Ok(Some((cred, plaintext)))
    }

    /// Find a domain by name, or create it if it does not exist.
    pub async fn find_or_create_domain(&self, name: &str) -> MvResult<Uuid> {
        let domains = self.store.list_domains().await?;
        if let Some(d) = domains.iter().find(|d| d.name == name && d.revoked_at.is_none()) {
            return Ok(d.id);
        }
        let domain = self.create_domain(name, Some("Auto-created for agent bridge")).await?;
        Ok(domain.id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mv_storage::keychain::SqliteKeychainStore;

    async fn test_engine() -> KeychainEngine {
        let store = Arc::new(SqliteKeychainStore::open_in_memory().unwrap());
        let cred_store = Arc::new(CredentialStore::env_only());
        KeychainEngine::new(store, cred_store, None, None)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn vault_lifecycle() {
        let engine = test_engine().await;

        // Status should be uninitialized
        let (state, _) = engine.vault_status().await.unwrap();
        assert_eq!(state, VaultState::Uninitialized);

        // Initialize
        engine
            .initialize_vault("test-password", false)
            .await
            .unwrap();
        let (state, meta) = engine.vault_status().await.unwrap();
        assert_eq!(state, VaultState::Unsealed);
        assert!(meta.is_some());

        // Seal
        engine.seal().await.unwrap();
        let (state, _) = engine.vault_status().await.unwrap();
        assert_eq!(state, VaultState::Sealed);

        // Unseal
        engine.unseal("test-password").await.unwrap();
        let (state, _) = engine.vault_status().await.unwrap();
        assert_eq!(state, VaultState::Unsealed);
    }

    #[tokio::test]
    async fn update_credential_metadata_applies_fields() {
        let engine = test_engine().await;
        engine.initialize_vault("test-password", false).await.unwrap();
        engine.unseal_vault("test-password").await.unwrap();

        let domain_id = engine.find_or_create_domain("oauth-clients").await.unwrap();
        let stored = engine
            .store_credential(
                domain_id,
                "client-id",
                "oauth_client_secret",
                b"secret",
                vec!["oauth".into()],
                None,
            )
            .await
            .unwrap();

        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "template_id".to_string(),
            serde_json::Value::String("template-123".to_string()),
        );
        metadata.insert(
            "token_ttl_seconds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(3600u64)),
        );

        let updated = engine
            .update_credential_metadata(
                stored.id,
                Some("AI Manager".to_string()),
                Some(vec!["oauth".into(), "client".into()]),
                Some(metadata.clone()),
                None,
            )
            .await
            .unwrap();

        assert_eq!(updated.description.as_deref(), Some("AI Manager"));
        assert_eq!(updated.tags.len(), 2);
        assert_eq!(
            updated
                .metadata
                .get("template_id")
                .and_then(|v| v.as_str()),
            Some("template-123")
        );
    }

    #[tokio::test]
    async fn wrong_password_rejected() {
        let engine = test_engine().await;
        engine.initialize_vault("correct", false).await.unwrap();
        engine.seal().await.unwrap();
        let result = engine.unseal("wrong").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn credential_store_and_read() {
        let engine = test_engine().await;
        engine.initialize_vault("pass", false).await.unwrap();

        let domain = engine
            .create_domain("api-keys", Some("API key storage"))
            .await
            .unwrap();

        let cred = engine
            .store_credential(
                domain.id,
                "openai-key",
                "api_key",
                b"sk-12345",
                vec!["prod".into()],
                None,
            )
            .await
            .unwrap();

        let (loaded, plaintext) = engine.read_credential(cred.id, "admin").await.unwrap();
        assert_eq!(loaded.name, "openai-key");
        assert_eq!(&*plaintext, b"sk-12345");
    }

    #[tokio::test]
    async fn delegation_chain() {
        let engine = test_engine().await;
        engine.initialize_vault("pass", false).await.unwrap();
        let domain = engine.create_domain("test", None).await.unwrap();
        let cred = engine
            .store_credential(domain.id, "key1", "api_key", b"secret", vec![], None)
            .await
            .unwrap();

        let d1 = engine
            .create_delegation(
                cred.id,
                "alice",
                DelegationPermissions {
                    can_read: true,
                    can_use: false,
                    can_delegate: true,
                },
                None,
                3,
            )
            .await
            .unwrap();
        assert_eq!(d1.depth, 0);

        let d2 = engine
            .sub_delegate(
                d1.id,
                "bob",
                DelegationPermissions {
                    can_read: true,
                    can_use: false,
                    can_delegate: false,
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(d2.depth, 1);

        // Bob can read via delegation
        let (_, plaintext) = engine
            .read_credential_via_delegation(d2.id, "bob")
            .await
            .unwrap();
        assert_eq!(&*plaintext, b"secret");
    }

    #[tokio::test]
    async fn zk_proof_roundtrip() {
        let engine = test_engine().await;
        engine.initialize_vault("pass", false).await.unwrap();
        let domain = engine.create_domain("test", None).await.unwrap();
        let cred = engine
            .store_credential(domain.id, "key1", "api_key", b"secret-value", vec![], None)
            .await
            .unwrap();

        let proof = engine.generate_proof(cred.id, "nonce-123").await.unwrap();
        assert!(engine.verify_proof(&proof).await.unwrap());
    }

    #[tokio::test]
    async fn audit_chain_integrity() {
        let engine = test_engine().await;
        engine.initialize_vault("pass", false).await.unwrap();
        engine.create_domain("test", None).await.unwrap();

        assert!(engine.verify_audit_integrity().await.unwrap());

        let trail = engine.list_audit_trail(100, 0).await.unwrap();
        assert!(trail.len() >= 2); // vault_initialized + domain_created
    }
}
