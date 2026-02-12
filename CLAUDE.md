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

## Workspace Layout

- `crates/` — Rust workspace crates (core, storage, engine, server, CLI, plugins, MCP).
- `frontend/` — SvelteKit UI (Tauri shell lives here).
- `web/` — Admin web UI (static, no framework).
- `extensions/` — Web clipper extension.
- `connectors/` — External connectors (MCP, OpenClaw).
- `config/` — Default config and examples.
- `docs/` — Architecture, ADRs, onboarding, security, performance.
- `migrations/` — SQLite schema migrations.

## Cargo Features (Common)

Storage / Engine / Server share these flags:

- `local-embeddings` (default): FastEmbed + local embedding models  
  `crates/mv-storage/Cargo.toml`, `crates/mv-engine/Cargo.toml`, `crates/mv-server/Cargo.toml`
- `image-embeddings`: image embedding models (engine)
- `local-llm`: local LLM backends (engine/server)
- `wip`: WIP features (engine/server)

## Configuration & Environment

Baseline config: `config/default.toml`.  
Most keys map to `MINDVAULT_*` environment variables.

Examples (see comments in `config/default.toml`):
- Auth: `MINDVAULT_AUTH_TOKEN`, `MINDVAULT_AUTH_ROLE`, `MINDVAULT_JWT_SECRET`
- Rate limits: `MINDVAULT_RATE_LIMIT_REQUESTS`, `MINDVAULT_RATE_LIMIT_WINDOW_SECS`
- Google Calendar: `MINDVAULT_GOOGLE_CALENDAR_*`
- Encryption: `MINDVAULT_ENCRYPTION_*`
- AI auto‑tagging: `MINDVAULT_AI_AUTO_TAGGING_*`
- Daily notes: `MINDVAULT_DAILY_NOTES_*`
- Recurrence: `MINDVAULT_RECURRENCE_*`
- AI sidecar (CLI override): `MINDVAULT_AI_SIDECAR_*`  
  (`crates/mv-cli/src/commands/mod.rs`)

## Testing Guidance

Always use the coordinator:

```bash
# run a single test
~/.mindvault/scripts/mv-check test -p mv-server -- public_share_lifecycle
```

Integration tests may require `-- --test-threads=1` for determinism.

## Swagger UI Build Note

`utoipa-swagger-ui` downloads a release from GitHub during build. Offline builds
need `SWAGGER_UI_DOWNLOAD_URL` set to a local file or require network access.

## Known Build Constraints

- Build coordination is required in this environment; use `~/.mindvault/scripts/mv-check` for all cargo commands.
- `aws-lc-sys` can take several minutes to compile from scratch (native C build).
- Swagger UI assets may download during build; offline environments must set `SWAGGER_UI_DOWNLOAD_URL`.
- LanceDB is pinned to the compatible Arrow line in the current lockfile; avoid ad-hoc Arrow upgrades.

## Ports

- REST: 9470
- gRPC: 50051
- UDS: ~/.mindvault/mindvault.sock
