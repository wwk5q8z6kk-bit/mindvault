use std::fmt;

use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Where a secret was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSource {
    OsKeyring,
    EncryptedFile,
    EnvironmentVariable,
}

impl fmt::Display for SecretSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OsKeyring => write!(f, "os_keyring"),
            Self::EncryptedFile => write!(f, "encrypted_file"),
            Self::EnvironmentVariable => write!(f, "env"),
        }
    }
}

/// A resolved secret value. The inner string is zeroized on drop.
pub struct SecretValue {
    inner: Zeroizing<String>,
    source: SecretSource,
}

impl SecretValue {
    pub fn new(value: String, source: SecretSource) -> Self {
        Self {
            inner: Zeroizing::new(value),
            source,
        }
    }

    /// Expose the secret for use (e.g., passing to an HTTP header).
    pub fn expose(&self) -> &str {
        &self.inner
    }

    /// Which backend resolved this secret.
    pub fn source(&self) -> SecretSource {
        self.source
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretValue")
            .field("source", &self.source)
            .field("inner", &"[REDACTED]")
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("encrypted file error: {0}")]
    EncryptedFile(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Backend trait
// ---------------------------------------------------------------------------

/// A secret storage backend.
pub trait CredentialBackend: Send + Sync {
    /// Human-readable backend name.
    fn name(&self) -> &str;

    /// Which source type this backend represents.
    fn source(&self) -> SecretSource;

    /// Whether this backend is currently available.
    fn is_available(&self) -> bool;

    /// Retrieve a secret by key. Returns `Ok(None)` if not found.
    fn get(&self, key: &str) -> Result<Option<String>, CredentialError>;

    /// Store a secret.
    fn set(&self, key: &str, value: &str) -> Result<(), CredentialError>;

    /// Delete a secret.
    fn delete(&self, key: &str) -> Result<(), CredentialError>;

    /// List stored key names.
    fn list_keys(&self) -> Result<Vec<String>, CredentialError>;
}

// ---------------------------------------------------------------------------
// Keyring backend
// ---------------------------------------------------------------------------

pub struct KeyringBackend {
    service: String,
}

impl KeyringBackend {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(&self.service, key)
            .map_err(|e| CredentialError::Keyring(e.to_string()))
    }
}

/// Well-known secret keys that MindVault uses.
const KNOWN_SECRET_KEYS: &[&str] = &[
    "OPENAI_API_KEY",
    "MINDVAULT_EMBEDDING_API_KEY",
    "MINDVAULT_ENCRYPTION_KEY",
];

impl CredentialBackend for KeyringBackend {
    fn name(&self) -> &str {
        "OS Keyring"
    }

    fn source(&self) -> SecretSource {
        SecretSource::OsKeyring
    }

    fn is_available(&self) -> bool {
        // Try creating an entry to see if the keyring service is reachable.
        self.entry("__mindvault_probe").is_ok()
    }

    fn get(&self, key: &str) -> Result<Option<String>, CredentialError> {
        let entry = self.entry(key)?;
        match entry.get_password() {
            Ok(pw) => Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CredentialError::Keyring(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), CredentialError> {
        let entry = self.entry(key)?;
        entry
            .set_password(value)
            .map_err(|e| CredentialError::Keyring(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), CredentialError> {
        let entry = self.entry(key)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CredentialError::Keyring(e.to_string())),
        }
    }

    fn list_keys(&self) -> Result<Vec<String>, CredentialError> {
        // The keyring crate doesn't support enumeration.
        // Probe known keys to see which ones are stored.
        let mut found = Vec::new();
        for &key in KNOWN_SECRET_KEYS {
            if let Ok(Some(_)) = self.get(key) {
                found.push(key.to_string());
            }
        }
        Ok(found)
    }
}

// ---------------------------------------------------------------------------
// Environment variable backend
// ---------------------------------------------------------------------------

pub struct EnvBackend;

impl CredentialBackend for EnvBackend {
    fn name(&self) -> &str {
        "Environment"
    }

    fn source(&self) -> SecretSource {
        SecretSource::EnvironmentVariable
    }

    fn is_available(&self) -> bool {
        true
    }

    fn get(&self, key: &str) -> Result<Option<String>, CredentialError> {
        match std::env::var(key) {
            Ok(v) if !v.is_empty() => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    fn set(&self, _key: &str, _value: &str) -> Result<(), CredentialError> {
        Err(CredentialError::Other(
            "cannot persist secrets to environment variables".into(),
        ))
    }

    fn delete(&self, _key: &str) -> Result<(), CredentialError> {
        Err(CredentialError::Other(
            "cannot delete environment variables".into(),
        ))
    }

    fn list_keys(&self) -> Result<Vec<String>, CredentialError> {
        let mut found = Vec::new();
        for &key in KNOWN_SECRET_KEYS {
            if std::env::var(key).map(|v| !v.is_empty()).unwrap_or(false) {
                found.push(key.to_string());
            }
        }
        Ok(found)
    }
}

// ---------------------------------------------------------------------------
// Credential store (resolution chain)
// ---------------------------------------------------------------------------

/// Status of a single backend in the credential store.
#[derive(Debug, Clone)]
pub struct BackendStatus {
    pub name: String,
    pub available: bool,
    pub keys: Vec<String>,
}

/// The credential store resolves secrets through a prioritized backend chain.
pub struct CredentialStore {
    backends: Vec<Box<dyn CredentialBackend>>,
}

impl CredentialStore {
    /// Create a new credential store with the default backend chain:
    /// OS Keyring → Environment Variables.
    ///
    /// The encrypted file backend can be inserted via `add_backend` (Phase B).
    pub fn new(service_name: &str) -> Self {
        let mut backends: Vec<Box<dyn CredentialBackend>> = Vec::new();

        let keyring = KeyringBackend::new(service_name);
        if keyring.is_available() {
            backends.push(Box::new(keyring));
        } else {
            tracing::warn!("OS keyring not available, skipping keyring backend");
        }

        backends.push(Box::new(EnvBackend));

        Self { backends }
    }

    /// Create a store with only the environment backend (for testing).
    pub fn env_only() -> Self {
        Self {
            backends: vec![Box::new(EnvBackend)],
        }
    }

    /// Resolve a secret by walking the backend chain (highest priority first).
    pub fn get(&self, key: &str) -> Result<Option<SecretValue>, CredentialError> {
        for backend in &self.backends {
            match backend.get(key) {
                Ok(Some(value)) => {
                    tracing::info!(
                        key = key,
                        source = %backend.source(),
                        "mindvault_credential_resolved"
                    );
                    return Ok(Some(SecretValue::new(value, backend.source())));
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(
                        key = key,
                        backend = backend.name(),
                        error = %e,
                        "mindvault_credential_backend_error"
                    );
                    continue;
                }
            }
        }
        Ok(None)
    }

    /// Store a secret in the first writable backend.
    pub fn set(&self, key: &str, value: &str) -> Result<SecretSource, CredentialError> {
        for backend in &self.backends {
            match backend.set(key, value) {
                Ok(()) => return Ok(backend.source()),
                Err(_) => continue,
            }
        }
        Err(CredentialError::Other(
            "no writable backend available".into(),
        ))
    }

    /// Store a secret in a specific backend.
    pub fn set_in(
        &self,
        key: &str,
        value: &str,
        target: SecretSource,
    ) -> Result<(), CredentialError> {
        for backend in &self.backends {
            if backend.source() == target {
                return backend.set(key, value);
            }
        }
        Err(CredentialError::Other(format!(
            "backend {target} not available"
        )))
    }

    /// Delete a secret from all backends that have it.
    pub fn delete(&self, key: &str) -> Result<Vec<SecretSource>, CredentialError> {
        let mut deleted_from = Vec::new();
        for backend in &self.backends {
            if let Ok(Some(_)) = backend.get(key) {
                backend.delete(key)?;
                deleted_from.push(backend.source());
            }
        }
        Ok(deleted_from)
    }

    /// Status of all backends.
    pub fn status(&self) -> Vec<BackendStatus> {
        self.backends
            .iter()
            .map(|b| BackendStatus {
                name: b.name().to_string(),
                available: b.is_available(),
                keys: b.list_keys().unwrap_or_default(),
            })
            .collect()
    }

    /// Convenience: resolve an API key, returning just the string (for engine integration).
    pub fn get_secret_string(&self, key: &str) -> Option<String> {
        self.get(key)
            .ok()
            .flatten()
            .map(|sv| sv.expose().to_string())
    }
}

impl fmt::Debug for CredentialStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialStore")
            .field("backends", &self.backends.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_backend_reads_set_var() {
        let backend = EnvBackend;
        std::env::set_var("MV_TEST_CRED_1", "test_value_123");
        let val = backend.get("MV_TEST_CRED_1").unwrap();
        assert_eq!(val.as_deref(), Some("test_value_123"));
        std::env::remove_var("MV_TEST_CRED_1");
    }

    #[test]
    fn env_backend_returns_none_for_missing() {
        let backend = EnvBackend;
        std::env::remove_var("MV_TEST_CRED_MISSING");
        let val = backend.get("MV_TEST_CRED_MISSING").unwrap();
        assert!(val.is_none());
    }

    #[test]
    fn env_backend_returns_none_for_empty() {
        let backend = EnvBackend;
        std::env::set_var("MV_TEST_CRED_EMPTY", "");
        let val = backend.get("MV_TEST_CRED_EMPTY").unwrap();
        assert!(val.is_none());
        std::env::remove_var("MV_TEST_CRED_EMPTY");
    }

    #[test]
    fn env_backend_cannot_set() {
        let backend = EnvBackend;
        assert!(backend.set("X", "Y").is_err());
    }

    #[test]
    fn credential_store_env_only_resolves() {
        std::env::set_var("MV_TEST_CRED_STORE", "from_env");
        let store = CredentialStore::env_only();
        let sv = store.get("MV_TEST_CRED_STORE").unwrap().unwrap();
        assert_eq!(sv.expose(), "from_env");
        assert_eq!(sv.source(), SecretSource::EnvironmentVariable);
        std::env::remove_var("MV_TEST_CRED_STORE");
    }

    #[test]
    fn credential_store_returns_none_when_missing() {
        std::env::remove_var("MV_TEST_CRED_ABSENT");
        let store = CredentialStore::env_only();
        assert!(store.get("MV_TEST_CRED_ABSENT").unwrap().is_none());
    }

    #[test]
    fn secret_value_debug_redacts() {
        let sv = SecretValue::new("super_secret".into(), SecretSource::OsKeyring);
        let debug = format!("{sv:?}");
        assert!(!debug.contains("super_secret"));
        assert!(debug.contains("REDACTED"));
    }
}
