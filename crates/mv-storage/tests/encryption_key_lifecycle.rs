//! At-rest encryption key lifecycle matrix.
//!
//! Covers boot (env / unseal), rotate (re-key + grace period), restore
//! (re-derive / unwrap grace key), and failure paths (wrong password,
//! corrupted ciphertext, truncated wrap material).
//!
//! Run with: cargo test -p mv-storage --test encryption_key_lifecycle

use std::sync::{Mutex, OnceLock};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use mv_storage::crypto::{
    is_encrypted, unwrap_encrypted, wrap_encrypted, CryptoError, EncryptedData, EncryptionConfig,
    KeyManager, ENCRYPTED_PREFIX,
};
use mv_storage::vault_crypto::VaultCrypto;
use zeroize::Zeroizing;

fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct ScopedEnvVars {
    originals: Vec<(&'static str, Option<String>)>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl ScopedEnvVars {
    fn set(pairs: &[(&'static str, &str)]) -> Self {
        let guard = test_env_lock().lock().expect("env lock");
        let mut originals = Vec::with_capacity(pairs.len());
        for &(key, value) in pairs {
            originals.push((key, std::env::var(key).ok()));
            std::env::set_var(key, value);
        }
        Self {
            originals,
            _guard: guard,
        }
    }
}

impl Drop for ScopedEnvVars {
    fn drop(&mut self) {
        for (key, original) in self.originals.drain(..).rev() {
            match original {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn light_config() -> EncryptionConfig {
    EncryptionConfig {
        enabled: true,
        argon2_memory_kib: 1024,
        argon2_iterations: 1,
        argon2_parallelism: 1,
    }
}

fn key_manager(password: &str, salt: &[u8]) -> KeyManager {
    let mut manager = KeyManager::new(light_config());
    manager.derive_master_key(password, salt).expect("derive");
    manager
}

fn vault_unsealed(password: &str, salt: &[u8]) -> VaultCrypto {
    let mut vc = VaultCrypto::new();
    vc.unseal(password, salt, &light_config()).expect("unseal");
    vc
}

// ---------------------------------------------------------------------------
// KeyManager (content encryption) lifecycle
// ---------------------------------------------------------------------------

#[test]
fn boot_from_env_requires_key_when_enabled() {
    let _env = ScopedEnvVars::set(&[
        ("MINDVAULT_ENCRYPTION_ENABLED", "true"),
        // Empty key is treated as unset by KeyManager::from_env.
        ("MINDVAULT_ENCRYPTION_KEY", ""),
        ("MINDVAULT_ENCRYPTION_SALT", ""),
        ("MINDVAULT_ENCRYPTION_ARGON2_MEMORY_KIB", "1024"),
        ("MINDVAULT_ENCRYPTION_ARGON2_ITERATIONS", "1"),
        ("MINDVAULT_ENCRYPTION_ARGON2_PARALLELISM", "1"),
    ]);

    match KeyManager::from_env() {
        Err(CryptoError::KeyNotSet) => {}
        Ok(_) => panic!("missing key must fail boot"),
        Err(other) => panic!("unexpected error: {other}"),
    }
}

#[test]
fn boot_from_env_derives_ready_manager() {
    let _env = ScopedEnvVars::set(&[
        ("MINDVAULT_ENCRYPTION_ENABLED", "true"),
        ("MINDVAULT_ENCRYPTION_KEY", "lifecycle-boot-password"),
        ("MINDVAULT_ENCRYPTION_SALT", "lifecycle-boot-salt"),
        ("MINDVAULT_ENCRYPTION_ARGON2_MEMORY_KIB", "1024"),
        ("MINDVAULT_ENCRYPTION_ARGON2_ITERATIONS", "1"),
        ("MINDVAULT_ENCRYPTION_ARGON2_PARALLELISM", "1"),
    ]);

    let manager = KeyManager::from_env().expect("boot");
    assert!(manager.is_ready());
    let cipher = manager.encrypt_string("boot-marker").expect("encrypt");
    assert_ne!(cipher, "boot-marker");
    assert_eq!(
        manager.decrypt_string(&cipher).expect("decrypt"),
        "boot-marker"
    );
}

#[test]
fn restore_same_password_and_salt_recovers_ciphertext() {
    let salt = b"restore-salt-16b";
    let manager = key_manager("original-pass", salt);
    let encoded = manager
        .encrypt_string("restore-me")
        .expect("encrypt under original");

    // Fresh process simulation: re-derive from the same material.
    let restored = key_manager("original-pass", salt);
    assert_eq!(
        restored.decrypt_string(&encoded).expect("restore decrypt"),
        "restore-me"
    );
}

#[test]
fn rotate_password_invalidates_old_ciphertext_until_reencrypt() {
    let salt = b"rotate-salt-16by";
    let old = key_manager("password-v1", salt);
    let old_cipher = old.encrypt_string("rotate-payload").expect("encrypt v1");

    let new = key_manager("password-v2", salt);
    assert!(
        new.decrypt_string(&old_cipher).is_err(),
        "rotated key must not decrypt pre-rotation ciphertext"
    );

    let reencrypted = new
        .encrypt_string("rotate-payload")
        .expect("re-encrypt under v2");
    assert_eq!(
        new.decrypt_string(&reencrypted).expect("decrypt v2"),
        "rotate-payload"
    );

    // Old key still recovers the original ciphertext (backup / dual-read path).
    assert_eq!(
        old.decrypt_string(&old_cipher)
            .expect("old key still works"),
        "rotate-payload"
    );
}

#[test]
fn failure_wrong_password_cannot_decrypt() {
    let salt = b"wrong-pass-salt16";
    let good = key_manager("correct", salt);
    let cipher = good.encrypt_string("secret").expect("encrypt");

    let bad = key_manager("incorrect", salt);
    assert!(bad.decrypt_string(&cipher).is_err());
}

#[test]
fn failure_corrupted_and_truncated_ciphertext() {
    let manager = key_manager("corrupt-pass", b"corrupt-salt-16b");
    let good = manager.encrypt_string("payload").expect("encrypt");

    // Flip a character in the middle of the base64 payload.
    let mut chars: Vec<char> = good.chars().collect();
    let mid = chars.len() / 2;
    chars[mid] = if chars[mid] == 'A' { 'B' } else { 'A' };
    let flipped: String = chars.into_iter().collect();
    assert!(manager.decrypt_string(&flipped).is_err());

    assert!(EncryptedData::from_base64("!!!not-base64!!!").is_err());
    assert!(EncryptedData::from_base64("YWJj").is_err()); // too short

    let not_ready = KeyManager::new(light_config());
    match not_ready.encrypt(b"x") {
        Err(CryptoError::KeyNotSet) => {}
        Ok(_) => panic!("encrypt without key must fail"),
        Err(other) => panic!("unexpected error: {other}"),
    }
}

#[test]
fn encrypted_prefix_wrap_roundtrip() {
    let manager = key_manager("prefix-pass", b"prefix-salt-16by");
    let encoded = manager.encrypt_string("prefixed").expect("encrypt");
    let wrapped = wrap_encrypted(&encoded);
    assert!(is_encrypted(&wrapped));
    assert!(wrapped.starts_with(ENCRYPTED_PREFIX));
    let inner = unwrap_encrypted(&wrapped).expect("unwrap");
    assert_eq!(manager.decrypt_string(inner).expect("decrypt"), "prefixed");
}

// ---------------------------------------------------------------------------
// VaultCrypto (master key / grace epoch) lifecycle
// ---------------------------------------------------------------------------

#[test]
fn vault_boot_seal_unseal_and_verify() {
    let salt = b"vault-boot-salt16";
    let mut vc = vault_unsealed("vault-pass", salt);
    assert!(vc.is_unsealed());
    let blob = vc.generate_verification_blob().expect("blob");
    assert!(vc.verify_password(&blob).expect("verify"));

    vc.seal();
    assert!(!vc.is_unsealed());

    // Restore by re-deriving the same password + salt.
    vc.unseal("vault-pass", salt, &light_config())
        .expect("restore unseal");
    assert!(vc.verify_password(&blob).expect("verify after restore"));
}

#[test]
fn vault_rotate_grace_reencrypt_and_restore_wrapped_key() {
    let domain = "domain:lifecycle";
    let cred = "cred:api-token";
    let plaintext = b"sk-lifecycle-secret";

    // Boot under epoch 0.
    let vc = vault_unsealed("pass-epoch-0", b"vault-rot-salt16");
    let epoch0_cipher = vc
        .encrypt_credential(plaintext, domain, cred)
        .expect("encrypt epoch 0");
    let old_master = vc.extract_master_key().expect("extract old master");

    // Rotate: new master key, keep old as grace key, wrap old under new.
    let mut rotated = VaultCrypto::new();
    rotated
        .unseal("pass-epoch-1", b"vault-new-salt16", &light_config())
        .expect("unseal new epoch");
    let new_master = rotated.extract_master_key().expect("extract new master");
    let wrapped_old = BASE64.encode(
        VaultCrypto::aes_gcm_encrypt_pub(&new_master, old_master.as_ref())
            .expect("wrap old master"),
    );
    rotated.add_grace_key(0, old_master);

    // Pre-rotation ciphertext is readable via grace, not via new master alone.
    let via_grace = rotated
        .decrypt_credential_with_epoch(&epoch0_cipher, domain, cred, 0)
        .expect("decrypt via grace");
    assert_eq!(&*via_grace, plaintext);
    assert!(
        rotated
            .decrypt_credential(&epoch0_cipher, domain, cred)
            .is_err(),
        "new master must not decrypt epoch-0 ciphertext without grace"
    );

    // Re-encrypt under the new epoch master.
    let epoch1_cipher = rotated
        .encrypt_credential(plaintext, domain, cred)
        .expect("re-encrypt");
    assert_eq!(
        &*rotated
            .decrypt_credential(&epoch1_cipher, domain, cred)
            .expect("decrypt new"),
        plaintext
    );

    // After grace eviction, epoch-0 ciphertext is unreachable.
    rotated.remove_grace_key(0);
    assert!(rotated
        .decrypt_credential_with_epoch(&epoch0_cipher, domain, cred, 0)
        .is_err());

    // Restore path: boot with new password and unwrap the persisted grace key.
    let mut restored = VaultCrypto::new();
    restored
        .unseal("pass-epoch-1", b"vault-new-salt16", &light_config())
        .expect("restore boot");
    restored
        .unwrap_grace_key(0, &wrapped_old)
        .expect("restore wrapped grace key");
    assert_eq!(restored.grace_key_count(), 1);
    assert_eq!(
        &*restored
            .decrypt_credential_with_epoch(&epoch0_cipher, domain, cred, 0)
            .expect("decrypt after grace restore"),
        plaintext
    );
}

#[test]
fn vault_failure_wrong_password_and_corrupted_material() {
    let mut vc = vault_unsealed("right-password", b"vault-fail-salt1");
    let blob = vc.generate_verification_blob().expect("blob");
    let cipher = vc
        .encrypt_credential(b"token", "domain:x", "cred:y")
        .expect("encrypt");

    // Wrong password boot fails verification against the stored blob.
    let mut wrong = VaultCrypto::new();
    wrong
        .unseal("wrong-password", b"vault-fail-salt1", &light_config())
        .expect("derive still succeeds");
    assert!(!wrong.verify_password(&blob).expect("verify wrong"));

    // Corrupted ciphertext / wrap material.
    assert!(vc
        .decrypt_credential("!!!not-base64!!!", "domain:x", "cred:y")
        .is_err());
    let mut flipped = BASE64.decode(&cipher).expect("decode");
    let idx = flipped.len() - 1;
    flipped[idx] ^= 0x5a;
    let flipped_b64 = BASE64.encode(&flipped);
    assert!(vc
        .decrypt_credential(&flipped_b64, "domain:x", "cred:y")
        .is_err());

    let new_master = vc.extract_master_key().expect("master");
    assert!(vc.unwrap_grace_key(9, "not-valid-base64!!!").is_err());
    // Valid AES wrap of non-32-byte payload must be rejected.
    let short_wrap = BASE64.encode(
        VaultCrypto::aes_gcm_encrypt_pub(&new_master, b"too-short").expect("encrypt short"),
    );
    assert!(vc.unwrap_grace_key(9, &short_wrap).is_err());

    // Sealed vault rejects crypto ops.
    vc.seal();
    assert!(vc.encrypt_credential(b"x", "d", "c").is_err());
}

#[test]
fn vault_grace_key_cap_evicts_oldest_epochs() {
    let mut vc = vault_unsealed("grace-cap", b"vault-grace-salt");
    for epoch in 0..7u64 {
        let key = Zeroizing::new([epoch as u8; 32]);
        vc.add_grace_key(epoch, key);
    }
    // MAX_GRACE_KEYS is 5 — oldest epochs (0, 1) must be pruned.
    assert_eq!(vc.grace_key_count(), 5);

    // Missing grace key → KeyDerivation error; present grace + bad b64 → Decryption.
    let missing = vc
        .decrypt_credential_with_epoch("!!!", "domain:a", "cred:a", 0)
        .unwrap_err()
        .to_string();
    assert!(
        missing.contains("no grace key") || missing.contains("KeyDerivation"),
        "epoch 0 should be pruned: {missing}"
    );

    let present = vc
        .decrypt_credential_with_epoch("!!!", "domain:a", "cred:a", 6)
        .unwrap_err()
        .to_string();
    assert!(
        present.contains("base64") || present.to_lowercase().contains("decrypt"),
        "epoch 6 should still be present: {present}"
    );
}
