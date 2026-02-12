# MindVault Architecture Overview

## High-Level Components

```mermaid
graph TD
  subgraph UI["Frontend Layer"]
    UIWeb[SvelteKit Web]
    UITauri[Tauri Desktop]
  end

  subgraph API["Server Layer"]
    REST[REST API]
    GRPC[gRPC API]
    WS[WebSocket Streams]
  end

  subgraph ENG["Engine Layer"]
    ENGCore[MindVault Engine]
    Relay[Relay + Adapters]
    Auto[Autonomy + Reflection]
    Sync[Sync + Conflicts]
  end

  subgraph STORE["Storage Layer"]
    SQLite[(SQLite + FTS5)]
    Lance[(LanceDB Vectors)]
    Tantivy[(Tantivy Index)]
    Graph[(Graph Store)]
    Files[(File Storage)]
    Keychain[(Keychain)]
  end

  subgraph INT["Integration Layer"]
    MCP[MCP Server]
    Plugins[WASM Plugin Runtime]
    Clipper[Web Clipper Extension]
  end

  UIWeb --> REST
  UIWeb --> WS
  UITauri --> REST
  REST --> ENGCore
  GRPC --> ENGCore
  WS --> ENGCore
  ENGCore --> SQLite
  ENGCore --> Lance
  ENGCore --> Tantivy
  ENGCore --> Graph
  ENGCore --> Files
  ENGCore --> Keychain
  MCP --> ENGCore
  Plugins --> REST
  Clipper --> REST
```

## Data Flow Summary

- **Ingest**: UI/Adapters submit nodes to the REST API, the Engine normalizes and stores them in SQLite, then updates the vector index and full‑text index.
- **Recall/Search**: Queries hit the Engine, which blends vector search, full-text search, and graph traversal to return ranked results.
- **Agentic Loop**: The Engine emits events to the Proactive/Intent systems, which generate insights and suggestions for the UI.
- **Attachments**: Files are stored on disk, metadata stored in SQLite, and content chunks indexed for search.
- **Observability**: Metrics are recorded in-process and exposed via `/metrics` and `/api/v1/metrics/*`.

## Boundaries & Ownership

- **mv-core**: Domain models + storage traits.
- **mv-storage**: SQLite and LanceDB implementations.
- **mv-engine**: Business logic and orchestration.
- **mv-server**: REST/gRPC API surface, auth, and middleware.
- **mv-mcp**: MCP stdio server for agent interoperability.
- **mv-plugin**: WASM plugin runtime + hooks.
- **frontend/web/tauri**: UI and desktop shell.
