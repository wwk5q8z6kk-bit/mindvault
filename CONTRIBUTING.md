# Contributing to MindVault

Thank you for your interest in contributing to MindVault.

## Development Setup

### Prerequisites

- Rust (stable) — install via [rustup](https://rustup.rs/)
- `protoc` — `brew install protobuf` (macOS) or `apt install protobuf-compiler` (Linux)
- Node.js 20+ and pnpm 8+ (for frontend/connectors)

### Getting Started

```bash
# Clone the repository
git clone https://github.com/wwk5q8z6kk-bit/mindvault.git
cd mindvault

# Build the Rust workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Start the server
cargo run -p mv-cli -- server start --foreground
# REST: http://127.0.0.1:9470
# Swagger UI: http://127.0.0.1:9470/api/docs
```

### Frontend (optional)

```bash
cd frontend
pnpm install
pnpm run check   # Type checking
pnpm test         # Run vitest
pnpm run dev      # Start dev server
```

## Project Structure

```
crates/
  mv-core/       # Models, traits, error types (no dependencies)
  mv-storage/    # SQLite + LanceDB implementations
  mv-index/      # Tantivy full-text search
  mv-graph/      # Petgraph knowledge graph
  mv-engine/     # Business logic, search orchestration, watcher
  mv-server/     # Axum REST/WebSocket/gRPC server
  mv-mcp/        # Model Context Protocol server
  mv-cli/        # Command-line interface
  mv-plugin/     # Plugin system (WASM runtime)
frontend/        # SvelteKit + Tauri 2 app
connectors/      # MCP/OpenClaw connectors
migrations/      # SQL migration files
config/          # Default configuration
docs/            # Architecture docs, guides
```

## Code Style

### Rust
- Follow standard Rust formatting (`cargo fmt`)
- Use `cargo clippy` with `-D warnings`
- Prefer `thiserror` for error types, `anyhow` for ad-hoc errors
- New features follow the trait pipeline: `mv-core` trait -> `mv-storage` impl -> `mv-engine` delegation -> `mv-server` REST

### Frontend
- Svelte 5 runes syntax (`$state`, `$derived`, `$effect`)
- Tab indentation (enforced by formatter)
- TypeScript strict mode
- pnpm only (npm has peer dependency conflicts)

## Submitting Changes

1. Create a feature branch: `git checkout -b feat/my-feature`
2. Make your changes with clear, focused commits.
3. Ensure all checks pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

4. Open a pull request against `main`.

Integration tests may need `-- --test-threads=1` for determinism.

## Build Notes

- `protoc` is required (tonic gRPC code generation).
- `utoipa-swagger-ui` downloads assets during build; offline builds need `SWAGGER_UI_DOWNLOAD_URL` set.
- `aws-lc-sys` can take several minutes on first compile (native C build).
- On low-RAM machines (16 GB), avoid concurrent `cargo` builds.

## Commit Conventions

Use conventional commits:
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `test:` adding tests
- `refactor:` code restructuring
- `chore:` maintenance tasks

## Reporting Issues

Open a GitHub issue with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Environment details (OS, Rust version)

## License

MindVault is proprietary software (see [LICENSE](LICENSE)), not an open source project. By contributing, you agree that your contributions become the property of the copyright holder and are licensed to the project under the same terms as the rest of the codebase.
