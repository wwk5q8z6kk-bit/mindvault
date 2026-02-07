//! Vault cryptographic operations: HKDF key hierarchy, AES-256-GCM, HMAC, audit hashing.

use std::collections::HashMap;

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::crypto::EncryptionConfig;

const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;
const VERIFICATION_SENTINEL: &[u8] = b"MINDVAULT_VAULT_SENTINEL_V1";

/// Vault cryptographic engine. Holds the master key and grace-period keys.
pub struct VaultCrypto {
    master_key: Option<Zeroizing<[u8; KEY_SIZE]>>,
    grace_keys: HashMap<u64, Zeroizing<[u8; KEY_SIZE]>>,
}

impl VaultCrypto {
    pub fn new() -> Self {
        Self {
            master_key: None,
            grace_keys: HashMap::new(),
        }
    }

    /// Derive the master key from a password and salt using Argon2id.
    pub fn unseal(&mut self, password: &str, salt: &[u8], config: &EncryptionConfig) -> Result<(), VaultCryptoError> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                config.argon2_memory_kib,
                config.argon2_iterations,
                config.argon2_parallelism,
                Some(KEY_SIZE),
            )
            .map_err(|e| VaultCryptoError::KeyDerivation(e.to_string()))?,
        );

        let mut key = Zeroizing::new([0u8; KEY_SIZE]);
        argon2
            .hash_password_into(password.as_bytes(), salt, key.as_mut())
            .map_err(|e| VaultCryptoError::KeyDerivation(e.to_string()))?;

        self.master_key = Some(key);
        Ok(())
    }

    /// Zeroize all keys (seal the vault).
    pub fn seal(&mut self) {
        self.master_key = None;
        self.grace_keys.clear();
    }

    pub fn is_unsealed(&self) -> bool {
        self.master_key.is_some()
    }

    /// Add a grace-period key for an old epoch (used during key rotation).
    pub fn add_grace_key(&mut self, epoch: u64, key: Zeroizing<[u8; KEY_SIZE]>) {
        self.grace_keys.insert(epoch, key);
    }

    /// Remove a grace-period key.
    pub fn remove_grace_key(&mut self, epoch: u64) {
        self.grace_keys.remove(&epoch);
    }

    fn master_key(&self) -> Result<&[u8; KEY_SIZE], VaultCryptoError> {
        self.master_key
            .as_ref()
            .map(|k| &**k)
            .ok_or(VaultCryptoError::Sealed)
    }

    /// Derive a domain-level key: HKDF-SHA256(master_key, info=derivation_info).
    pub fn derive_domain_key(&self, derivation_info: &str) -> Result<Zeroizing<[u8; KEY_SIZE]>, VaultCryptoError> {
        let master = self.master_key()?;
        let hk = Hkdf::<Sha256>::new(None, master);
        let mut okm = Zeroizing::new([0u8; KEY_SIZE]);
        hk.expand(derivation_info.as_bytes(), okm.as_mut())
            .map_err(|e| VaultCryptoError::KeyDerivation(e.to_string()))?;
        Ok(okm)
    }

    /// Derive a credential-level key: two-level HKDF derivation.
    pub fn derive_credential_key(
        &self,
        domain_info: &str,
        cred_info: &str,
    ) -> Result<Zeroizing<[u8; KEY_SIZE]>, VaultCryptoError> {
        let domain_key = self.derive_domain_key(domain_info)?;
        let hk = Hkdf::<Sha256>::new(None, domain_key.as_ref());
        let mut okm = Zeroizing::new([0u8; KEY_SIZE]);
        hk.expand(cred_info.as_bytes(), okm.as_mut())
            .map_err(|e| VaultCryptoError::KeyDerivation(e.to_string()))?;
        Ok(okm)
    }

    /// Derive a credential key using a specific epoch's grace key.
    fn derive_credential_key_with_epoch(
        &self,
        domain_info: &str,
        cred_info: &str,
        epoch: u64,
    ) -> Result<Zeroizing<[u8; KEY_SIZE]>, VaultCryptoError> {
        let base_key = self
            .grace_keys
            .get(&epoch)
            .ok_or_else(|| VaultCryptoError::KeyDerivation(format!("no grace key for epoch {epoch}")))?;

        let hk_domain = Hkdf::<Sha256>::new(None, base_key.as_ref());
        let mut domain_key = Zeroizing::new([0u8; KEY_SIZE]);
        hk_domain
            .expand(domain_info.as_bytes(), domain_key.as_mut())
            .map_err(|e| VaultCryptoError::KeyDerivation(e.to_string()))?;

        let hk_cred = Hkdf::<Sha256>::new(None, domain_key.as_ref());
        let mut cred_key = Zeroizing::new([0u8; KEY_SIZE]);
        hk_cred
            .expand(cred_info.as_bytes(), cred_key.as_mut())
            .map_err(|e| VaultCryptoError::KeyDerivation(e.to_string()))?;

        Ok(cred_key)
    }

    /// Encrypt a credential value using the HKDF-derived credential key.
    pub fn encrypt_credential(
        &self,
        plaintext: &[u8],
        domain_info: &str,
        cred_info: &str,
    ) -> Result<String, VaultCryptoError> {
        let key = self.derive_credential_key(domain_info, cred_info)?;
        let encrypted = aes_gcm_encrypt(&*key, plaintext)?;
        Ok(BASE64.encode(encrypted))
    }

    /// Decrypt a credential value.
    pub fn decrypt_credential(
        &self,
        encoded: &str,
        domain_info: &str,
        cred_info: &str,
    ) -> Result<Vec<u8>, VaultCryptoError> {
        let key = self.derive_credential_key(domain_info, cred_info)?;
        let data = BASE64
            .decode(encoded)
            .map_err(|e| VaultCryptoError::Decryption(format!("base64: {e}")))?;
        aes_gcm_decrypt(&*key, &data)
    }

    /// Decrypt using a specific epoch's grace key (for rotation grace period).
    pub fn decrypt_credential_with_epoch(
        &self,
        encoded: &str,
        domain_info: &str,
        cred_info: &str,
        epoch: u64,
    ) -> Result<Vec<u8>, VaultCryptoError> {
        let key = self.derive_credential_key_with_epoch(domain_info, cred_info, epoch)?;
        let data = BASE64
            .decode(encoded)
            .map_err(|e| VaultCryptoError::Decryption(format!("base64: {e}")))?;
        aes_gcm_decrypt(&*key, &data)
    }

    /// Generate a verification blob by encrypting a known sentinel.
    pub fn generate_verification_blob(&self) -> Result<String, VaultCryptoError> {
        let master = self.master_key()?;
        let encrypted = aes_gcm_encrypt(master, VERIFICATION_SENTINEL)?;
        Ok(BASE64.encode(encrypted))
    }

    /// Verify the password by decrypting the verification blob and checking the sentinel.
    pub fn verify_password(&self, blob: &str) -> Result<bool, VaultCryptoError> {
        let master = self.master_key()?;
        let data = BASE64
            .decode(blob)
            .map_err(|e| VaultCryptoError::Decryption(format!("base64: {e}")))?;
        match aes_gcm_decrypt(master, &data) {
            Ok(plaintext) => Ok(plaintext == VERIFICATION_SENTINEL),
            Err(_) => Ok(false),
        }
    }

    /// Compute an HMAC-SHA256 chain hash for delegation verification.
    pub fn compute_chain_hash(
        &self,
        domain_info: &str,
        prev_hash: Option<&str>,
        cred_id: &str,
        delegatee: &str,
        perms: &str,
        expires_at: Option<&str>,
    ) -> Result<String, VaultCryptoError> {
        let master = self.master_key()?;
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(master)
            .map_err(|e| VaultCryptoError::Encryption(e.to_string()))?;
        mac.update(domain_info.as_bytes());
        mac.update(prev_hash.unwrap_or("genesis").as_bytes());
        mac.update(cred_id.as_bytes());
        mac.update(delegatee.as_bytes());
        mac.update(perms.as_bytes());
        mac.update(expires_at.unwrap_or("none").as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    /// Generate a ZK access proof: HMAC-SHA256(credential_value, challenge_nonce).
    pub fn generate_zk_proof(
        &self,
        credential_value: &[u8],
        challenge_nonce: &str,
    ) -> Result<String, VaultCryptoError> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(credential_value)
            .map_err(|e| VaultCryptoError::Encryption(e.to_string()))?;
        mac.update(challenge_nonce.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    /// Verify a ZK access proof.
    pub fn verify_zk_proof(
        &self,
        credential_value: &[u8],
        challenge_nonce: &str,
        proof: &str,
    ) -> Result<bool, VaultCryptoError> {
        let expected = self.generate_zk_proof(credential_value, challenge_nonce)?;
        Ok(constant_time_eq(expected.as_bytes(), proof.as_bytes()))
    }

    /// Compute a SHA-256 audit chain hash.
    pub fn compute_audit_hash(
        previous_hash: Option<&str>,
        sequence: i64,
        action: &str,
        subject: &str,
        resource_id: Option<&str>,
        timestamp: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.unwrap_or("genesis").as_bytes());
        hasher.update(sequence.to_string().as_bytes());
        hasher.update(action.as_bytes());
        hasher.update(subject.as_bytes());
        hasher.update(resource_id.unwrap_or("").as_bytes());
        hasher.update(timestamp.as_bytes());
        hex::encode(hasher.finalize())
    }
}

impl Default for VaultCrypto {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AES-256-GCM helpers
// ---------------------------------------------------------------------------

fn aes_gcm_encrypt(key: &[u8; KEY_SIZE], plaintext: &[u8]) -> Result<Vec<u8>, VaultCryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| VaultCryptoError::Encryption(e.to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| VaultCryptoError::Encryption(e.to_string()))?;

    let mut out = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn aes_gcm_decrypt(key: &[u8; KEY_SIZE], data: &[u8]) -> Result<Vec<u8>, VaultCryptoError> {
    if data.len() < NONCE_SIZE + 16 {
        return Err(VaultCryptoError::Decryption("data too short".into()));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| VaultCryptoError::Decryption(e.to_string()))?;

    let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
    let ciphertext = &data[NONCE_SIZE..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| VaultCryptoError::Decryption(e.to_string()))
}

/// Constant-time comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Hex encoding (no extra dependency needed).
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum VaultCryptoError {
    Sealed,
    KeyDerivation(String),
    Encryption(String),
    Decryption(String),
}

impl std::fmt::Display for VaultCryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sealed => write!(f, "vault is sealed"),
            Self::KeyDerivation(msg) => write!(f, "key derivation failed: {msg}"),
            Self::Encryption(msg) => write!(f, "encryption failed: {msg}"),
            Self::Decryption(msg) => write!(f, "decryption failed: {msg}"),
        }
    }
}

impl std::error::Error for VaultCryptoError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::EncryptionConfig;

    fn test_crypto() -> VaultCrypto {
        let mut vc = VaultCrypto::new();
        let config = EncryptionConfig {
            enabled: true,
            argon2_memory_kib: 1024, // small for tests
            argon2_iterations: 1,
            argon2_parallelism: 1,
        };
        vc.unseal("test-password", b"test-salt-16byt", &config)
            .unwrap();
        vc
    }

    #[test]
    fn unseal_and_seal() {
        let mut vc = test_crypto();
        assert!(vc.is_unsealed());
        vc.seal();
        assert!(!vc.is_unsealed());
    }

    #[test]
    fn derive_domain_key_deterministic() {
        let vc = test_crypto();
        let k1 = vc.derive_domain_key("domain:api-keys").unwrap();
        let k2 = vc.derive_domain_key("domain:api-keys").unwrap();
        assert_eq!(k1.as_ref(), k2.as_ref());
    }

    #[test]
    fn derive_different_domains_different_keys() {
        let vc = test_crypto();
        let k1 = vc.derive_domain_key("domain:api-keys").unwrap();
        let k2 = vc.derive_domain_key("domain:ssh-keys").unwrap();
        assert_ne!(k1.as_ref(), k2.as_ref());
    }

    #[test]
    fn encrypt_decrypt_credential_roundtrip() {
        let vc = test_crypto();
        let plaintext = b"super-secret-api-key-12345";
        let encrypted = vc
            .encrypt_credential(plaintext, "domain:api", "cred:my-key")
            .unwrap();
        let decrypted = vc
            .decrypt_credential(&encrypted, "domain:api", "cred:my-key")
            .unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_info_cannot_decrypt() {
        let vc = test_crypto();
        let plaintext = b"secret";
        let encrypted = vc
            .encrypt_credential(plaintext, "domain:a", "cred:x")
            .unwrap();
        let result = vc.decrypt_credential(&encrypted, "domain:b", "cred:x");
        assert!(result.is_err());
    }

    #[test]
    fn verification_blob_roundtrip() {
        let vc = test_crypto();
        let blob = vc.generate_verification_blob().unwrap();
        assert!(vc.verify_password(&blob).unwrap());
    }

    #[test]
    fn chain_hash_deterministic() {
        let vc = test_crypto();
        let h1 = vc
            .compute_chain_hash("d:api", None, "cred-1", "alice", "r", None)
            .unwrap();
        let h2 = vc
            .compute_chain_hash("d:api", None, "cred-1", "alice", "r", None)
            .unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn zk_proof_roundtrip() {
        let vc = test_crypto();
        let value = b"my-api-key";
        let nonce = "random-nonce-123";
        let proof = vc.generate_zk_proof(value, nonce).unwrap();
        assert!(vc.verify_zk_proof(value, nonce, &proof).unwrap());
        assert!(!vc.verify_zk_proof(value, "wrong-nonce", &proof).unwrap());
    }

    #[test]
    fn audit_hash_chain() {
        let h1 = VaultCrypto::compute_audit_hash(None, 1, "vault_initialized", "admin", None, "2025-01-01T00:00:00Z");
        let h2 = VaultCrypto::compute_audit_hash(Some(&h1), 2, "credential_stored", "admin", Some("cred-1"), "2025-01-01T00:01:00Z");
        // h2 should incorporate h1
        assert_ne!(h1, h2);
        // recomputing with same inputs should be deterministic
        let h2b = VaultCrypto::compute_audit_hash(Some(&h1), 2, "credential_stored", "admin", Some("cred-1"), "2025-01-01T00:01:00Z");
        assert_eq!(h2, h2b);
    }

    #[test]
    fn sealed_vault_rejects_operations() {
        let vc = VaultCrypto::new();
        assert!(vc.derive_domain_key("domain:x").is_err());
        assert!(vc.encrypt_credential(b"test", "d", "c").is_err());
    }
}
