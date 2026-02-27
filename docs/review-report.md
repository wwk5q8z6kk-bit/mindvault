# MindVault Comprehensive Code Review (Feb 12, 2026)

## Executive Summary

MindVault has a strong, modular Rust core with a full SvelteKit/Tauri UI. The system is largely feature-complete across storage, search, relay, sync, plugins, and agent interoperability. Security hardening from the audit report has been effectively completed. AI/autonomy features are well-integrated. The modular crate structure and comprehensive frontend provide a solid foundation for future growth.

This report anchors each claim to code and corrects earlier misstatements.

---

## 1. Architecture Assessment

### 1.1 Rust Crate Structure

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

**Crates reviewed:**
- `mv-core` — Core models and traits
- `mv-engine` — Business logic engine
- `mv-server` — REST/gRPC/WebSocket servers
- `mv-storage` — SQLite/LanceDB/Tantivy persistence
- `mv-graph` — Graph traversal and relationships
- `mv-mcp` — Model Context Protocol server
- `mv-plugin` — Plugin system (WASM runtime behind `wasm-runtime` feature flag)
- `mv-cli` — CLI interface

**Findings:**
- Clear crate boundaries maintained with `mv-core` as the central types hub
- `mv-engine` has grown significantly with: `ai_autotag`, `autonomy`, `proactive`, `multihop`, `secure_enclave` (macOS Keychain), `recurrence`, `daily_notes`, `backlinks`, `sync/clock`
- `mv-server` checks cleanly as of Feb 12, 2026

### 1.2 Frontend Architecture

- SvelteKit-based application at `frontend/src/`
- Comprehensive API client at `src/lib/api/` with good REST endpoint parity
- Rich editor (TipTap-based) with plugin architecture at `src/lib/editor/`
- Command palette system at `src/lib/command-palette/`
- Multiple UI routes: calendar, kanban, graph, daily notes, goals, autonomy, canvas, etc.

### 1.3 Admin Web UI

- Static Vite-based admin UI at `web/` (HTML + CSS, no framework)
- Contains UX audit artifacts

### 1.4 Plugin System

`crates/mv-plugin/src/lib.rs`, `hooks.rs`, `manifest.rs`

**Hook points:** `PreIngest`/`PostIngest`, `PreSearch`/`PostSearch`, `OnChange`, `Scheduled`, `OnIntent`

---

## 2. Integration Points

### 2.1 MCP Connectors

1. **Claude Code MCP** (`connectors/claude-code-mcp/index.ts`)
   - stdio-based MCP server
   - Tools: `mindvault_store`, `mindvault_recall`, `mindvault_search`, `mindvault_forget`, `mindvault_graph`, `mindvault_stats`
   - Resources: `mindvault://knowledge/{id}`
   - MCP server runs directly on the engine (not via gRPC): `crates/mv-mcp/src/server.rs`

2. **OpenClaw Plugin** (`connectors/openclaw-plugin/`)
   - JSON-based plugin configuration, auto-capture/recall, namespace + auth support

### 2.2 Browser Extension

`extensions/mindvault-clipper/`

- Manifest V3, context menu integration, keyboard shortcuts (Cmd/Ctrl+Shift+Y)
- API integration with enrichment endpoint, fallback to web app bookmark page

### 2.3 Protocol Buffers

`proto/mindvault/v1/mindvault.proto`

**Services:**
- `MindVaultService` — Store/Get/Update/Delete/Recall/List
- `KeychainService` — `InitVault`, `Unseal`, `Seal`, `RotateKey`, `StoreCredential`, `ReadCredential`, `ListCredentials`, `DestroyCredential`, `GenerateProof`, `VerifyProof`

Proto definitions align with gRPC server in `crates/mv-server/src/grpc.rs` (1,092 lines).

---

## 3. Security Assessment

### 3.1 Completed Security Work (Audit-Aligned)

All P0 items are **Done**:
- JWT + shared-token authentication — `crates/mv-server/src/auth.rs`
- RBAC and namespace-scoped authorization — namespace checks in REST handlers
- Input validation guards — `crates/mv-server/src/validation.rs`
- Auth-identity rate limiting + namespace quotas — `crates/mv-server/src/limits.rs`
- Structured audit logging with stats-tab viewer — `crates/mv-server/src/audit.rs`, `frontend/src/routes/settings/audit/+page.svelte`

### 3.2 Configuration Security

`config/default.toml`:
- Auth tokens via environment variables (`MINDVAULT_AUTH_TOKEN`, `MINDVAULT_JWT_SECRET`)
- OAuth credentials via environment variables
- Encryption defaults disabled with Argon2 parameters defined
- Rate limiting configurable

---

## 4. Verified Capabilities

### 4.1 Completed Features

| Feature | Evidence |
|---------|----------|
| JWT + shared-token auth | `mv-server/src/auth.rs` |
| RBAC + namespace authorization | Namespace checks in REST handlers |
| Rate limiting + quotas | `mv-server/src/limits.rs` |
| Structured audit logs + viewer | `mv-server/src/audit.rs`, `routes/settings/audit/` |
| Encryption at rest (AES-256-GCM + Argon2id) | `mv-storage/src/crypto.rs`, `docs/adr/004-encryption-at-rest.md` |
| Metrics + observability | `mv-server/src/metrics.rs` (Prometheus-style: REST/gRPC counters, latency histograms, `/metrics` endpoint) |
| Local embeddings (default) | `config/default.toml` (FastEmbed + bge-small) |
| Backup/export/import | `mv-cli/src/commands/backup.rs`, `mv-engine/src/backup.rs` |
| AI auto-tagging | `mv-engine/src/ai_autotag.rs` |
| Rich editor + WYSIWYG (TipTap) | `frontend/src/lib/editor/` |
| Daily notes templates | `mv-engine/src/daily_notes.rs` |
| Attachment pipeline | `mv-server/src/rest/attachments.rs` |
| Wiki-link backlinking | `mv-engine/src/backlinks.rs` |
| Node backlinks REST endpoint | `GET /api/v1/nodes/:id/backlinks` — paginated, filterable (auto/manual/source), `References`-only (test: `get_node_backlinks_returns_filtered_paginated_references`) |
| Templates system | REST endpoints + frontend UI |
| AI semantic suggestions | `mv-server/src/rest/assist.rs` |
| Hybrid search (FTS + vector + graph) | `mv-engine/src/engine.rs`, `mv-storage/src/vector.rs`, `mv-index` |
| MCP tools/resources | `mv-mcp/src/tools.rs`, `mv-mcp/src/resources.rs` |
| WASM plugins + hooks | `mv-plugin/src/lib.rs`, `mv-plugin/src/hooks.rs` |
| Relay adapters (Email/Slack/Discord) | `mv-server/src/email.rs`, `mv-engine/src/adapters/` |
| Public shares (create/list/revoke + viewer) | `mv-server/src/rest/shares.rs` |
| Google Calendar sync | `mv-server/src/rest/google_calendar.rs` |
| Sync (vector-clock based) | `mv-engine/src/sync/clock.rs` |
| ADRs + architecture docs | `docs/adr/README.md`, `docs/architecture/system-overview.md` |

### 4.2 In-Progress Features

| Feature | Evidence |
|---------|----------|
| JSON Canvas import/export | `routes/canvas/+page.svelte` |
| Global quick-capture | `lib/components/QuickCaptureModal.svelte` |
| AI task prioritization | FocusPlannerModal |
| Recurring tasks | `mv-engine/src/recurrence.rs` |
| Calendar workspace | `routes/calendar/+page.svelte` |

---

## 5. Cross-Component Consistency

- **API parity:** REST endpoints (`crates/mv-server/src/rest/`) generally match frontend API client (`lib/api/`), gRPC definitions align, WebSocket via `crates/mv-server/src/websocket.rs`. **Gap:** the new `GET /nodes/:id/backlinks` endpoint has no frontend API client yet — `BacklinksPanel.svelte` still uses the general-purpose `getNeighbors()`/`getNodeRelationships()` graph endpoints
- **Configuration:** `config/default.toml` maps to env vars, CLI commands align with config sections, engine config mirrors file structure
- **Data model:** KnowledgeNode consistent across proto, Rust structs, and frontend types; tags/namespaces/metadata consistent across layers

---

## 6. Known Build Constraints

- Avoid concurrent `cargo` builds on low-RAM machines — `CLAUDE.md`
- Swagger UI build step downloads from GitHub; offline builds need `SWAGGER_UI_DOWNLOAD_URL` — `crates/mv-server/src/openapi.rs`
- `aws-lc-sys` can take several minutes to compile from scratch (native C build)
- LanceDB pinned to compatible Arrow line in current lockfile; avoid ad-hoc Arrow upgrades

---

## 7. Corrections vs Prior Reports

- **Encryption at rest is implemented** (AES-256-GCM + Argon2id), not pending.
- **Local embeddings are the default** in config (FastEmbed + bge-small).
- **Google Calendar sync is implemented** (REST endpoints + scheduler).
- **Public share viewer is implemented** (HTML view + JSON via `Accept`).
- **MCP server runs directly on the engine** (not via gRPC).
- **Sync is vector-clock based**, not a full CRDT system.
- **Editor is TipTap** (not "TiTip").
- **Metrics are fully implemented** with Prometheus-style counters, latency histograms, middleware, and `/metrics` endpoint — not "partially implemented" or "pending."
- **ADRs and architecture docs already exist.**
- **`web/` admin UI directory exists** at project root (static Vite-based admin UI with UX audit artifacts).
- **`mv-server` checks cleanly** as of Feb 12, 2026 (previously reported grpc.rs partial-move errors are not reproducible).

---

## 8. Recommendations

### High Priority

1. **Keep documentation in sync with code** by updating this report after major merges.
2. **Expand developer docs** (CLAUDE + runbook) to reduce onboarding friction.
3. **Track P0/P1 execution explicitly** with a living checklist tied to owners/files — `docs/p0-p1-execution-checklist.md`

### Medium Priority

1. **Create plugin development documentation** — only manifest and hook definitions currently available.
2. **Add API documentation** via swagger/OpenAPI generation (utoipa infrastructure already present).

### Low Priority

1. **Create migration guide** for encryption at rest feature.
2. **Document Arrow version dependency constraint** for LanceDB compatibility.
3. **Add Grafana/alerting integration** on top of existing `/metrics` endpoint.
