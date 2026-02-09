# MindVault Development Guide

## Build Coordination (CRITICAL)

**NEVER run `cargo check`, `cargo build`, `cargo test`, or `cargo clippy` directly.**

This is a multi-session environment on a 16GB machine. Running raw `cargo` causes
OOM kills (SIGKILL/exit 137), lock contention (21+ minute waits), and a death
spiral where sessions kill each other's builds. Use the build coordinator for ALL
cargo operations:

```bash
# check (default subcommand)
~/.mindvault/scripts/mv-check -p mv-storage
~/.mindvault/scripts/mv-check --workspace

# test (also coordinated — prevents OOM during test compilation)
~/.mindvault/scripts/mv-check test -p mv-engine -- test_name

# build
~/.mindvault/scripts/mv-check build -p mv-cli --release
```

The coordinator provides:
- **Global lock**: only 1 cargo process at a time (prevents OOM on 16GB RAM)
- **Source-mtime cache**: returns instantly (~150ms) if no .rs files changed
- **Deduplication**: identical concurrent requests share one build's result
- **pkill immunity**: uses a renamed binary that survives `pkill -f cargo`

**NEVER run `pkill -f cargo` or `kill` on cargo processes.** Other sessions depend
on them. Use `~/.mindvault/scripts/mv-check-cleanup --status` to see what's running.

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
