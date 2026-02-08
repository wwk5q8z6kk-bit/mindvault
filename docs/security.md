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

## Encryption at Rest

When enabled (`encryption.enabled = true` in config):
- Node content is encrypted with AES-256-GCM
- Master key is derived from passphrase via Argon2id
- Key epochs enable rotation without re-encrypting all data
- Domain keys scope encryption (e.g., "credentials" domain)

### Configuration
```toml
[encryption]
enabled = true
argon2_memory_kib = 65536  # 64 MiB
argon2_iterations = 3
argon2_parallelism = 4
```

## Keychain

The keychain securely stores third-party credentials:
- Credentials are encrypted with domain-scoped keys
- Access patterns are tracked for breach detection
- Delegations allow time-limited access sharing
- Audit chain with hash verification ensures tamper detection
- Progressive lockout after failed attempts

## CORS

Configured via `tower-http` CORS layer. Default allows localhost origins for development.

## Rate Limiting

### REST API
Rate limiting is handled at the reverse proxy level (recommended: nginx/Caddy).

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
