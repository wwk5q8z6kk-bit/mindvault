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

MindVault encrypts **keychain secrets** and **blob attachments** at rest when
sealed mode is enabled. Knowledge node text content remains in plaintext to
preserve full-text search and embedding functionality.

| Data | Encrypted at rest? | Notes |
|------|--------------------|-------|
| Keychain credentials | Yes | AES-256-GCM with domain-scoped keys |
| Blob attachments | Yes | Per-file DEK wrapped with namespace KEK |
| Node text content | No | Required for FTS and vector search |
| FTS / vector indexes | Ephemeral | Rebuilt in-memory after each unseal |

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
- Returns HTTP 503 / gRPC `FAILED_PRECONDITION` on all data routes
- Allows only keychain status, init, unseal, and Shamir endpoints
- Blocks background jobs (reindex, enrichment, watchers)

After **unseal**, the server:
1. Runs `migrate_sealed_storage` — encrypts any legacy plaintext blobs
2. Runs `rebuild_runtime_indexes` — rebuilds FTS and vector indexes
3. Resumes normal operation

If either post-unseal step fails, the vault is automatically re-sealed.

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
