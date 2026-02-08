# Performance Guide

## SQLite Tuning

### Connection Pooling
MindVault supports two SQLite backends:
- `SqliteNodeStore` — single `Mutex<Connection>`, good for tests and low-concurrency
- `PooledSqliteNodeStore` — `deadpool-sqlite` pool with configurable size

```toml
[storage]
sqlite_pool_size = 8  # default
```

### WAL Mode
SQLite runs in WAL (Write-Ahead Logging) mode by default:
- Allows concurrent readers alongside a single writer
- `PRAGMA busy_timeout=5000` prevents immediate lock failures

### Tips
- Keep the database on an SSD for best performance
- The `PRAGMA journal_mode=WAL` is set automatically on open
- For bulk imports, consider batching inserts in transactions

## LanceDB Vector Search

### Table Caching
The vector store caches the LanceDB table handle via `OnceCell` to avoid re-opening on every query.

### Indexing
For vaults with 256+ embeddings, IVF-PQ indexing significantly speeds up search:
```rust
vector_store.ensure_index().await?;
```

Index is created automatically when row count exceeds the threshold.

### Batch Operations
Use `upsert_batch()` for bulk embedding updates (e.g., during import or re-indexing).

## Embedding Provider Selection

| Provider | Latency | Quality | Privacy |
|----------|---------|---------|---------|
| Local (ONNX) | ~10ms/embed | Good (384d) | Full |
| OpenAI | ~100ms/embed | Best (1536d) | Cloud |
| NoOp | 0ms | None | Full |

For best performance with acceptable quality, use local embeddings. For best retrieval quality, use OpenAI.

## Frontend Performance

### Virtualized Lists
Lists on tasks, notes, inbox, and search pages use `@tanstack/svelte-virtual` for windowed rendering. Only visible rows are rendered to the DOM, enabling smooth scrolling at 10K+ items.

### Dexie Offline Cache
IndexedDB via Dexie provides instant UI rendering from cache while API calls sync in the background:
- Tasks and notes are cached locally
- Optimistic updates show changes immediately
- Sync queue retries failed operations

## Watcher Agent

The watcher runs periodic cycles to detect intents and generate insights:

```toml
[watcher]
enabled = true
interval_secs = 300     # 5 minutes
lookback_hours = 24
max_nodes_per_cycle = 50
expiry_days = 7          # auto-expire stale proposals
```

Increase `interval_secs` if the watcher is consuming too many resources. Decrease `max_nodes_per_cycle` for faster cycles.

## Benchmarking

Run the benchmark suite:
```bash
cargo bench -p mv-storage   # SQLite + vector benchmarks
cargo bench -p mv-engine    # Engine pipeline benchmarks
```

Results are output as HTML reports in `target/criterion/`.
