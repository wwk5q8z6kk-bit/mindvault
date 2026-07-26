# Current State

## Executive finding

MindVault is a broad, ambitious local-first Rust/Svelte/Tauri codebase with real implementations
for SQLite storage, Tantivy search, LanceDB vectors, graph persistence, proposals, multiple
transports, encryption/keychain concepts, and many optional product modules. It is not a verified
production baseline. The shipped paths do not consistently uphold the new invariants, the exact
public commit is failing CI, and several “complete” capabilities are isolated implementations or
default-off modules rather than end-to-end product behavior.

The correct next move is a constrained v2 strangler architecture around canonical files + SQLite,
immutable evidence, an outbox-driven rebuildable index layer, proposal-only AI writes, and one
verified local transport. It is not a rewrite or a C++ introduction.

## Repository facts

- Public baseline: `dcd7dbdb8c87ca01e22777f79e8f196b993fca23` on `main`.
- Workspace: nine Rust crates, SvelteKit frontend, thin Tauri v2 shell, Python AI sidecar, connector
  packages, migrations, and examples.
- The captured user work was dirty and this audit is isolated in a separate worktree. During final
  validation, the original checkout independently moved to a later feature branch; that lineage is
  not covered by this report and must be reconciled under B-021.
- 726 tracked files; especially large modules (`rest.rs`, `sqlite.rs`) concentrate unrelated
  responsibilities.
- No repository tags, submodules, or toolchain pins.

## What is real

- SQLite uses WAL and foreign-key enforcement through a connection pool.
- Tantivy full-text and LanceDB vector implementations are called by the main ingest/search engine.
- Relationships use SQLite-backed storage and are included in hybrid recall.
- The proposal store, manual approve/reject flow, undo snapshots, and keychain audit chain are
  substantive implementations.
- REST, WebSocket, gRPC, UDS, CLI, Tauri/Svelte, MCP, imports, attachments, and encrypted/sealed
  paths exist in source.
- The server refuses non-loopback bind without auth unless explicitly overridden.
- Rust source contains extensive unit tests, but coverage and integration gates are incomplete.

## What is not yet trustworthy

### Canonical consistency

`crates/mv-engine/src/ingest.rs` writes SQLite, commits Tantivy, then writes vectors. There is no
transaction or durable outbox across those systems. Delete removes derived indexes, graph edges,
and blobs before deleting SQLite. Failure can therefore leave either missing indexes or lost
evidence while the canonical record survives.

### File-first behavior

Obsidian ingestion reads Markdown, strips/parses it, and creates or updates SQLite nodes keyed by a
source URI and mtime (`crates/mv-engine/src/import/obsidian.rs`). It does not preserve the original
bytes as immutable evidence, maintain a content hash/version ledger, or treat the source file as a
first-class canonical document.

### Proposal-only writes

MCP tools generally propose, but AI auto-tagging can mutate a node before storage when enabled.
Sync imports directly insert/update SQLite, enrichment paths update nodes directly, and the agent
intent apply path can execute canonical writes outside the proposal executor. “AI never writes
canonical knowledge directly” is not currently enforceable at the storage boundary.

### Backup/restore

The encrypted backup reads and overwrites a raw SQLite file and omits WAL coherence, blobs,
keychain, configuration, source files, and derived-rebuild metadata. A separate CLI archive covers
more files but has no uniform quiesce/integrity/manifest protocol. Neither has a proved disaster
recovery objective.

### Authentication and desktop integration

Auth disabled means system administrator, intentionally suitable only for loopback single-user
operation. The Svelte API client sends no bearer token, and browser WebSocket construction has no
auth-token path. Enabling backend auth breaks normal desktop HTTP/WebSocket use. Consumer tokens
resolve to unscoped write contexts; policy enforcement is not a uniform route-level capability
boundary.

### Privacy and audit

The settings UI persists an AI API key in localStorage. General REST audit is disabled by default,
bounded in process memory unless sinks are configured, and not tamper evident. Cloud/provider calls
do not share one disclosure and data-classification ledger.

### Packaging and verification

Tauri is a webview shell pointed at a separately running localhost service; it does not own server
lifecycle or a bundled sidecar contract. The public CI does not verify the complete frontend,
Tauri, Python, migration, backup/restore, benchmark, or release paths. The exact commit fails its
existing CI.

## Current decision

Freeze feature expansion. Preserve the legacy implementation as migration input, establish the
contracts in the Phase 1 documents and ADR set, then build only the first vertical slice after all
acceptance gates in [V2_MIGRATION_PLAN.md](V2_MIGRATION_PLAN.md) are met.
