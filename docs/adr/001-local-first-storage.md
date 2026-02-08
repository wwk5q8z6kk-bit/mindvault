# ADR-001: Local-First SQLite + LanceDB Storage

## Status
Accepted

## Context
MindVault needs a storage architecture that supports:
- Offline-first operation (Tauri desktop app)
- Structured metadata queries (filters, sorting, pagination)
- High-dimensional vector similarity search for semantic recall
- Single-user sovereignty (no shared database servers)

## Decision
Use **SQLite** for structured data (nodes, relationships, intents, proposals, keychain) and **LanceDB** for vector embeddings.

- SQLite runs in WAL mode for concurrent read access with `Mutex<Connection>`.
- LanceDB stores embeddings in a local Lance format directory alongside the SQLite file.
- Both are file-based, requiring no external services.
- A `PooledSqliteNodeStore` variant uses `deadpool-sqlite` for concurrent async access.

## Consequences
- **Positive:** Zero-config deployment, works offline, single data directory backup.
- **Positive:** SQLite is battle-tested for single-writer workloads.
- **Positive:** LanceDB provides native vector search without an external service.
- **Negative:** Single-writer SQLite limits concurrent write throughput (mitigated by WAL + pool).
- **Negative:** LanceDB's Rust SDK is newer and has fewer ecosystem integrations.
