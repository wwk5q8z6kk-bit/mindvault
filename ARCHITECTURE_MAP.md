# Architecture Map

## Deployed shape

```text
SvelteKit UI in Tauri WebView or browser
        |
        | REST :9470 / WebSocket
        v
mv-server ------------------------------------------------ gRPC :50051
        |                                                    UDS socket
        v
mv-engine
  | ingest / recall / proposals / sync / imports / AI routing
  |
  +--> mv-storage --> SQLite authority + LanceDB vectors
  +--> mv-index ----> Tantivy full-text
  +--> mv-graph ----> relationships in the same SQLite domain
  +--> mv-plugin ---> manifest registry; WASM execution only with optional feature
  +--> provider APIs / local models / Python AI service :8100

mv-cli --> engine/server/MCP
mv-mcp --> proposal-oriented tools + read tools
filesystem/Obsidian/connectors --> import/sync paths
```

The Tauri crate has no substantive Rust commands beyond startup/plugin wiring. It does not embed or
supervise `mv-server`.

## Rust workspace dependency direction

| Crate | Direct workspace dependencies | Intended role | Audit finding |
|---|---|---|---|
| `mv-core` | none | Types/contracts | Appropriate leaf, but broad domain surface |
| `mv-storage` | core | SQLite, keychain, vectors | Canonical and derived storage mixed |
| `mv-index` | core, storage | Tantivy search | Derived index coupled to storage |
| `mv-graph` | core | SQLite graph store | Graph is a canonical table but also treated like an index |
| `mv-engine` | core, graph, index, storage | Orchestration/business logic | Main integration layer; direct-store bypasses exist |
| `mv-plugin` | core | Plugin manifests/runtime | Runtime feature not enabled by server |
| `mv-server` | core, engine, plugin, storage | REST/WS/gRPC/UDS | Very large route module and duplicated transport policy |
| `mv-mcp` | core, engine | MCP adapter | Proposal-oriented mutation is a useful pattern |
| `mv-cli` | core, engine, mcp, server, storage | Operator surface | Broad integration and alternate backup/import paths |

## Production call paths

### Desktop

`frontend/src/lib/api/client.ts` → localhost REST → `mv-server::rest` → `AppState` →
`MindVaultEngine`. WebSocket clients connect separately for changes, reminders, agent events, and
collaboration. Tauri CSP permits the localhost server and unsafe-inline scripts/styles.

### Node mutation

REST/gRPC/CLI/import → `MindVaultEngine::store_node/update_node/delete_node` →
`IngestPipeline` → SQLite → Tantivy commit → embed/LanceDB → conflict/graph side effects.

Not all callers use this path. Sync, enrichment, relay, and several operational stores call the
SQLite node store directly.

### Retrieval

REST/gRPC/MCP/CLI → `RecallEngine` → exact/FTS/vector/graph candidates → reciprocal-rank fusion and
optional graph boost/query rewrite/HyDE/rerank → results. No benchmark-backed default weighting or
retrieval quality baseline exists.

### AI

Provider abstraction can select local or remote embeddings/chat. A separate Python FastAPI service
offers models, embeddings, chat, and fine-tuning. Local/cloud policy and disclosure are not
enforced at one choke point.

## Architectural pressure points

1. `mv-engine` is not the only canonical write boundary.
2. canonical commit and derived indexing have no durable consistency protocol;
3. files are import sources, not canonical documents/evidence;
4. four transports duplicate security and compatibility burden;
5. server route and SQLite modules are monoliths;
6. optional modules share tables and route surface with core;
7. Tauri packaging does not define server lifecycle or authenticated IPC;
8. plugin/connector capabilities are not uniformly sandboxed.

## Target boundaries

```text
Core domain (no UI/provider imports)
  document | evidence | memory | version | proposal | policy | audit
        |
Canonical transaction service
  SQLite + registered human files + content-addressed evidence
        |
Durable outbox / rebuild coordinators
  FTS5 or Tantivy | vector | graph projection
        |
Ports
  local API | model gateway | connector workers | export/backup
        |
Adapters
  Tauri/Svelte | CLI | MCP | optional modules
```

Optional modules depend inward through public ports. Core does not depend on tasks, habits,
calendar, relay, public sharing, federation, or messaging.
