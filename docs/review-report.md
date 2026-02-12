# MindVault Codebase Re‑Review (Feb 12, 2026)

## Executive Summary
MindVault has a strong, modular Rust core with a full SvelteKit/Tauri UI. The system is now largely feature‑complete across storage, search, relay, sync, plugins, and agent interoperability. The biggest remaining work is documentation polish and maintaining an accurate, source‑linked view of what is actually implemented.

This report corrects earlier misstatements and anchors each claim to code.

---

## Corrections vs the Prior Report

- **Encryption at rest is implemented** (AES‑256‑GCM + Argon2id), not pending.  
  `crates/mv-storage/src/crypto.rs`, `docs/adr/004-encryption-at-rest.md`
- **Local embeddings are the default** in config (FastEmbed + bge‑small).  
  `config/default.toml`
- **Google Calendar sync is implemented** (REST endpoints + scheduler).  
  `crates/mv-server/src/rest/google_calendar.rs`, `crates/mv-server/src/lib.rs`
- **Public share viewer is implemented** (HTML view + JSON via `Accept`).  
  `crates/mv-server/src/rest/shares.rs`
- **MCP server runs directly on the engine** (not via gRPC).  
  `crates/mv-mcp/src/server.rs`
- **Sync is vector‑clock based**, not a full CRDT system.  
  `crates/mv-engine/src/sync/clock.rs`
- **Editor is TipTap** (not “TiTip”).  
  `frontend/package.json`, `frontend/src/lib/components/editor/`

---

## Architecture (Current)

```mermaid
graph TB
  subgraph UI["Frontend Layer"]
    FE[SvelteKit Frontend]
    Tauri[Tauri Desktop Shell]
  end

  subgraph API["Server Layer"]
    REST[Axum REST API]
    GRPC[gRPC API]
    WS[WebSocket Streams]
  end

  subgraph ENG["Engine Layer"]
    MV[MindVault Engine]
    Relay[Relay + Adapters]
    Auto[Autonomy + Reflection]
    Sync[Sync + Conflicts]
  end

  subgraph STORE["Storage Layer"]
    SQLite[(SQLite + FTS5)]
    Lance[(LanceDB Vectors)]
    Tantivy[(Tantivy Index)]
    Graph[(Graph Store)]
    Files[(File Store)]
    Keychain[(Keychain)]
  end

  subgraph INT["Integration Layer"]
    MCP[MCP Server (stdio)]
    Plugins[WASM Plugin Runtime]
    Clipper[Web Clipper Extension]
  end

  FE --> REST
  FE --> WS
  Tauri --> REST
  REST --> MV
  GRPC --> MV
  WS --> MV
  MV --> SQLite
  MV --> Lance
  MV --> Tantivy
  MV --> Graph
  MV --> Files
  MV --> Keychain
  MCP --> MV
  Plugins --> REST
  Clipper --> REST
```

---

## Verified Capabilities (Selected)

- **Auth + rate limiting + namespace scoping**  
  `crates/mv-server/src/auth.rs`, `crates/mv-server/src/limits.rs`
- **Hybrid search (FTS + vector + graph)**  
  `crates/mv-engine/src/engine.rs`, `crates/mv-storage/src/vector.rs`, `crates/mv-index`
- **MCP tools/resources**  
  `crates/mv-mcp/src/tools.rs`, `crates/mv-mcp/src/resources.rs`
- **WASM plugins + hooks**  
  `crates/mv-plugin/src/lib.rs`, `crates/mv-plugin/src/hooks.rs`
- **Relay adapters (Email/Slack/Discord)**  
  `crates/mv-server/src/email.rs`, `crates/mv-engine/src/adapters/`
- **Public shares (create/list/revoke + viewer)**  
  `crates/mv-server/src/rest/shares.rs`

---

## Known Build Constraints

- Use the build coordinator (`~/.mindvault/scripts/mv-check`) for any `cargo` operation.  
  `CLAUDE.md`
- Swagger UI build step downloads from GitHub; offline builds must set a local
  `SWAGGER_UI_DOWNLOAD_URL` or enable network access.  
  `crates/mv-server/src/openapi.rs`, `CLAUDE.md`

---

## Recommended Next Steps

1. **Keep documentation in sync with code** by updating this report after major merges.  
2. **Expand developer docs** (CLAUDE + runbook) to reduce onboarding friction.  
3. **Add a short ADR index** (if not already present) to anchor key decisions.  

