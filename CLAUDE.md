# MindVault Development Guide

## Build Coordination (CRITICAL)

**NEVER run `cargo check`, `cargo build`, or `cargo test` directly.**

This is a multi-session environment on a 16GB machine. Multiple concurrent cargo
processes cause OOM kills, lock contention, and a death spiral of processes killing
each other. Use the build coordinator instead:

```bash
# Instead of: cargo check -p mv-storage
~/.mindvault/scripts/mv-check -p mv-storage

# Instead of: cargo check --workspace
~/.mindvault/scripts/mv-check --workspace

# Instead of: cargo check -p mv-engine -p mv-server
~/.mindvault/scripts/mv-check -p mv-engine -p mv-server
```

The coordinator:
- Queues builds (only 1 cargo process runs at a time — prevents OOM)
- Caches results by source hash (skips check if code hasn't changed)
- Deduplicates identical checks (second caller waits for first's result)
- Uses a renamed binary (immune to `pkill -f cargo` from other sessions)

**NEVER run `pkill -f cargo` or `kill` on cargo processes.** Other sessions depend on them.
Use `~/.mindvault/scripts/mv-check-cleanup --status` to see what's running.

For `cargo test`, run tests directly (the test binary doesn't need the coordinator):
```bash
~/.mindvault/scripts/mv-check -p mv-engine  # check first
cargo test -p mv-engine -- test_name         # then test
```

## Project Structure

Rust workspace with 7+ crates:
- `mv-core` — Core types, credential store, model definitions
- `mv-storage` — SQLite + LanceDB persistence layer
- `mv-index` — Tantivy full-text search index
- `mv-graph` — petgraph-based knowledge graph
- `mv-engine` — Orchestration engine combining all backends
- `mv-server` — axum REST + tonic gRPC server
- `mv-cli` — CLI binary (`mv` command)

## Known Build Issues

- `mv-core/src/model/` directory may conflict with `model.rs` (sovereign keychain WIP)
- `mv-server/src/grpc.rs` has partial-move errors (unrelated to current work)
- `aws-lc-sys` takes several minutes to compile from scratch (C code)
- LanceDB requires arrow v52 (not v53)

## Ports

- REST: 9470
- gRPC: 50051
- UDS: ~/.mindvault/mindvault.sock
