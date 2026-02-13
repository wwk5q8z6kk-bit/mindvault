# Security Guide

## Authentication

MindVault uses a token-based authentication system with permission templates.

### Access Keys
- Created via `POST /api/v1/permissions/keys`
- Each key is tied to a `PermissionTemplate` that defines scope (namespace, kinds, actions)
- Keys are hashed with SHA-256 before storage; the raw key is shown only once at creation
- Pass the key via `Authorization: Bearer <key>` header

### Permission Tiers
| Tier | Capabilities |
|------|-------------|
| View | Read nodes, search, list resources |
| Edit | View + create/update/delete nodes |
| Action | Edit + submit proposals, trigger actions |
| Admin | Full access including keychain, plugins, federation |

### OAuth2 Client Credentials
For machine-to-machine access, MindVault supports OAuth2 client credentials flow:
- Register clients via `POST /api/v1/oauth/clients`
- Exchange credentials at `POST /api/v1/oauth/token`

### Public Shares
Public share links provide read-only access to a single node:
- Tokens are high-entropy and stored as SHA-256 hashes
- Links can be created with optional expiry timestamps
- Revoke instantly via `DELETE /api/v1/shares/{id}`
- Access is via `GET /public/shares/{token}` and bypasses auth, so treat links as secrets
- The browser view is read-only and noindexed; request JSON with `Accept: application/json`

## Encryption at Rest (Sealed Mode)

### What is encrypted

MindVault encrypts all persisted vault artifacts at rest in sealed mode.
Plaintext exists only transiently in process memory while the vault is unsealed.

| Data | Encrypted at rest? | Notes |
|------|--------------------|-------|
| SQLite node payloads + metadata | Yes | Per-node DEK (AES-256-GCM), DEK wrapped per namespace |
| Tantivy index segments/files | Yes | Transparent encrypted directory wrapper |
| LanceDB table files | Yes | Transparent encrypted storage wrapper |
| Blob attachments + derived extraction text | Yes | Envelope format with wrapped DEK (`MVB1` prefix) |
| Keychain credentials | Yes | Domain-scoped encryption + audit controls |
| API responses | Runtime only | Decrypted only while process is unsealed |

### Sealed mode lifecycle

```
UNINITIALIZED ──> mv keychain init ──> UNSEALED
                                         │
                              seal / idle timeout
                                         ▼
                                       SEALED  ◄── server start (sealed_mode=true)
                                         │
                                  mv keychain unseal
                                         ▼
                                       UNSEALED
```

While **sealed**, the server:
- Returns HTTP 503 / gRPC `UNAVAILABLE` on all data routes
- Allows only keychain status, init, unseal, and Shamir endpoints
- Blocks background jobs (reindex, enrichment, watchers)

After **unseal**, the server:
1. Runs `migrate_sealed_storage` — encrypts any legacy plaintext blobs
2. Runs `rebuild_runtime_indexes` — rebuilds FTS and vector indexes
3. Resumes normal operation

If either post-unseal step fails, the vault is automatically re-sealed.

### Legacy plaintext migration command

Use the keychain migration command when enabling sealed mode for an existing vault:

```bash
mv keychain migrate-sealed [--passphrase <pw> | --from-env | --from-macos-keychain | --from-secure-enclave] [--keep-unsealed]
```

Behavior:
- Unseals (or initializes + unseals) the vault
- Migrates legacy plaintext artifacts into encrypted sealed storage
- Rebuilds runtime indexes
- Re-seals by default (or stays unsealed with `--keep-unsealed`)

Compatibility alias:

```bash
mv server migrate-sealed --passphrase <pw>
```

### Configuration

```toml
# Enable sealed mode (keychain + blob encryption)
sealed_mode = true

# Argon2 parameters for passphrase-derived master key
[encryption]
argon2_memory_kib = 65536  # 64 MiB
argon2_iterations = 3
argon2_parallelism = 4
```

### Startup preflight

When `sealed_mode = true`, the server scans the data directory before engine
initialization and **fails fast** if it finds:
- Legacy `tantivy/` or `lancedb/` index directories (should be removed by migration)
- Plaintext blob files without the `MVB1` encryption prefix

Run `mv server preflight` to check without starting the server.

### Unseal rate limiting

Failed unseal attempts are rate-limited per subject (default: 5 failures per
300 seconds). Configurable via:
- `MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT` (max failures)
- `MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW` (window in seconds)

### Audit trail

All unseal attempts (success and failure) are recorded in the chronicle log with
subject, method, outcome, and failure reason. Query via:
```
GET /api/v1/agent/chronicle?limit=50
```

### Sealed-mode telemetry counters

Prometheus `/metrics` includes sealed lifecycle counters:
- `mindvault_vault_sealed_http_requests_blocked_total`
- `mindvault_vault_sealed_grpc_requests_blocked_total`
- `mindvault_vault_unseal_failures_total`
- `mindvault_vault_unseal_rate_limited_total`
- `mindvault_vault_sealed_migration_success_total`
- `mindvault_vault_sealed_migration_failures_total`
- `mindvault_vault_runtime_rebuild_success_total`
- `mindvault_vault_runtime_rebuild_failures_total`

## Operator Runbook — Sealed Mode

### Enabling sealed mode on an existing vault

1. **Back up your data directory** before proceeding.
2. Run preflight to verify the current state:
   ```bash
   mv server preflight
   ```
3. Run the migration command to encrypt all legacy plaintext artifacts:
   ```bash
   mv keychain migrate-sealed --passphrase <pw>
   # or use OS-backed key sources:
   mv keychain migrate-sealed --from-macos-keychain
   mv keychain migrate-sealed --from-secure-enclave
   ```
   This unseals the vault (or initializes it if needed), encrypts all legacy
   blobs, removes legacy index directories, rebuilds runtime indexes, and
   re-seals. Use `--keep-unsealed` to leave the vault open after migration.
4. Enable sealed mode in your config:
   ```toml
   sealed_mode = true
   ```
5. Restart the server. It will start in sealed state; unseal via CLI or API.

### Upgrading a sealed vault

When upgrading MindVault with sealed mode enabled:

1. Seal the vault before stopping the server (`POST /api/v1/keychain/seal`).
2. Back up the data directory.
3. Upgrade the binary.
4. Start the server — it will start sealed and run preflight automatically.
5. Unseal. Post-unseal maintenance (migration + index rebuild) runs automatically.
6. Verify via `GET /api/v1/keychain/status` that the vault reports `"sealed": false`.

### Recovery from failed migration

If `migrate_sealed_storage` or `rebuild_runtime_indexes` fails after unseal,
the vault automatically re-seals to protect data. To recover:

1. Check logs for the failure reason (search for `post-unseal maintenance`).
2. If the failure is transient (e.g. disk full), free resources and unseal again.
   The migration is idempotent — it skips already-encrypted blobs.
3. If the failure persists, restore from backup and retry with verbose logging:
   ```bash
   RUST_LOG=mv_engine=debug mv keychain migrate-sealed --passphrase <pw>
   ```
4. Monitor the telemetry counters:
   - `mindvault_vault_sealed_migration_failures_total` — migration failures
   - `mindvault_vault_runtime_rebuild_failures_total` — index rebuild failures
5. Check the chronicle audit log for detailed failure context:
   ```
   GET /api/v1/agent/chronicle?limit=50
   ```

### Strict hardware mode

Set `MINDVAULT_REQUIRE_HARDWARE=true` to require OS-backed secure storage
(macOS Keychain or Secure Enclave) for the master key. When enabled:

- The server **refuses to start** if OS secure storage is unavailable.
- Passphrase-only unseal is rejected at the keychain layer.
- Suitable for production deployments on macOS where the master key must never
  exist as a passphrase-derived value in process memory.

### Auto-seal timeout

After unseal, the vault starts an auto-seal timer (default: 900 seconds / 15 minutes).
If no activity occurs within that window, the vault re-seals automatically.
Configure via the CLI `--auto-seal-timeout` flag or the `auto_seal_timeout_secs`
config key.

### Shamir secret sharing

For multi-party unseal, enable Shamir splitting of the vault encryption key:

1. Enable: `POST /api/v1/keychain/shamir/enable` with `threshold` and `total`.
2. Distribute shares to key holders.
3. To unseal, each holder submits their share via `POST /api/v1/keychain/shamir/submit`.
4. When the threshold is met, the vault unseals automatically.
5. Rotate shares: `POST /api/v1/keychain/shamir/rotate`.
6. Check status: `GET /api/v1/keychain/shamir/status`.

### Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Server returns 503 on all routes | Vault is sealed | Unseal via `mv keychain unseal` or API |
| Preflight fails: "legacy plaintext artifacts" | Sealed mode enabled before migration | Run `mv keychain migrate-sealed` |
| Unseal returns 429 | Rate limit hit (5 failures / 300s) | Wait for the window to expire, or check passphrase |
| Post-unseal migration fails repeatedly | Corrupt blob or disk issue | Check logs, restore from backup, retry |
| `MINDVAULT_REQUIRE_HARDWARE` fails at startup | No OS secure storage | Deploy on macOS with Keychain access, or disable the flag |
| Metrics show high `sealed_http_requests_blocked` | Clients hitting sealed vault | Automate unseal in your deployment scripts |
| Auto-seal fires unexpectedly | Idle timeout too short | Increase `auto_seal_timeout_secs` |

### Environment variable reference (sealed mode)

| Variable | Default | Description |
|----------|---------|-------------|
| `MINDVAULT_REQUIRE_HARDWARE` | `false` | Require OS secure storage for master key |
| `MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT` | `5` | Max failed unseal attempts per window |
| `MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW` | `300` | Rate limit window in seconds |
| `MINDVAULT_ENCRYPTION_ARGON2_MEMORY_KIB` | `65536` | Argon2 memory parameter (KiB) |
| `MINDVAULT_ENCRYPTION_ARGON2_ITERATIONS` | `3` | Argon2 time cost |
| `MINDVAULT_ENCRYPTION_ARGON2_PARALLELISM` | `4` | Argon2 lane count |

## Keychain

The keychain securely stores third-party credentials:
- Credentials are encrypted with domain-scoped keys
- Access patterns are tracked for breach detection
- Delegations allow time-limited access sharing
- Audit chain with hash verification ensures tamper detection
- Progressive lockout after failed attempts

## CORS

No CORS headers are sent by default. Set `cors_allowed_origins` in server config
or use a reverse proxy to control cross-origin access.

## Rate Limiting

### REST API
Built-in sliding window rate limiter controlled by environment variables:
- `MINDVAULT_RATE_LIMIT_REQUESTS` — max requests per window (default: 120)
- `MINDVAULT_RATE_LIMIT_WINDOW_SECS` — window duration in seconds (default: 60)

A reverse proxy (nginx/Caddy) is still recommended for production deployments.

### Keychain
- Read rate limit: `MINDVAULT_KEYCHAIN_READ_RATE_LIMIT` / `_WINDOW`
- Unseal failure backoff: `MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT` / `_WINDOW`

### MCP Server
Built-in sliding window rate limiter: 60 calls/minute per access key.

## Plugin Sandboxing

WASM plugins run in a sandboxed Wasmtime runtime:
- No filesystem access unless `Filesystem` permission is granted
- No network access unless `Network` permission is granted
- Host functions (`mv_read_node`, `mv_write_node`, `mv_search`) are gated by `PermissionGate`
- CPU time is limited via Wasmtime fuel budgeting

## Exchange Inbox

External agents submit proposals (not direct modifications):
- Blocked senders are rejected at submission time (403)
- Auto-approve rules can fast-track trusted proposals
- Quiet hours pause all autonomous actions
- Undo snapshots allow reversal within a time window
