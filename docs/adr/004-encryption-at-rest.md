# ADR-004: Encryption at Rest (AES-GCM + Argon2)

## Status
Accepted

## Context
Sensitive personal knowledge must be protected at rest. The encryption system must handle key derivation, key rotation, and domain-scoped encryption.

## Decision
Use **AES-256-GCM** for encryption with **Argon2id** for key derivation from a user passphrase.

Architecture:
- **Key Epochs:** Master keys rotate via epochs. Each epoch derives encryption keys using HKDF.
- **Domain Keys:** Scoped encryption domains (e.g., "credentials", "notes") derive sub-keys from the current epoch.
- **Keychain:** Stores credentials, delegations, and audit entries with full chain verification.
- **Lockout:** Progressive lockout after failed passphrase attempts.

Parameters (configurable):
- Argon2 memory: 64 MiB
- Argon2 iterations: 3
- Argon2 parallelism: 4

## Consequences
- **Positive:** Industry-standard authenticated encryption prevents tampering.
- **Positive:** Argon2id is the recommended KDF (OWASP, NIST).
- **Positive:** Key epochs enable rotation without re-encrypting all data.
- **Negative:** Forgetting the passphrase means permanent data loss (by design).
- **Negative:** Encryption adds latency to every read/write operation.
