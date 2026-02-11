# MindVault Architecture Overview

## High-Level Components

```mermaid
graph TD
  UI[Frontend (Web/Tauri)]
  API[REST + gRPC Server]
  ENG[MindVault Engine]
  STORE[Unified Store]
  DB[(SQLite)]
  VEC[(LanceDB)]
  FTS[Tantivy Index]
  GRAPH[Graph Store]
  KC[Keychain + Credentials]
  AGENT[Agentic Services]
  ADAPTERS[Adapters (Email/Slack/Discord)]
  FILES[File Storage]

  UI --> API
  API --> ENG
  ENG --> STORE
  STORE --> DB
  STORE --> VEC
  ENG --> FTS
  ENG --> GRAPH
  ENG --> KC
  ENG --> AGENT
  ENG --> ADAPTERS
  ENG --> FILES
```

## Data Flow Summary

- **Ingest**: UI/Adapters submit nodes to the REST API, the Engine normalizes and stores them in SQLite, then updates the vector index and full-text index.
- **Recall/Search**: Queries hit the Engine, which blends vector search, full-text search, and graph traversal to return ranked results.
- **Agentic Loop**: The Engine emits events to the Proactive/Intent systems, which generate insights and suggestions for the UI.
- **Attachments**: Files are stored on disk, metadata stored in SQLite, and content chunks indexed for search.
- **Observability**: Metrics are recorded in-process and exposed via `/metrics` and `/api/v1/metrics/*`.

## Boundaries & Ownership

- **mv-core**: Domain models + storage traits.
- **mv-storage**: SQLite and LanceDB implementations.
- **mv-engine**: Business logic and orchestration.
- **mv-server**: REST/gRPC API surface, auth, and middleware.
- **frontend/web/tauri**: UI and desktop shell.
