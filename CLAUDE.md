# MindVault Development Guide

## Building

```bash
cargo check -p mv-storage          # check a single crate
cargo check --workspace            # check all crates
cargo test -p mv-engine -- test_name  # run a specific test
cargo build -p mv-cli --release    # build CLI binary
```

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
