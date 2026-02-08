# Contributing to MindVault

## Development Setup

### Prerequisites
- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Node.js 20+ and pnpm 8+
- SQLite 3.35+ (bundled via `rusqlite`)

### Getting Started

```bash
# Clone the repository
git clone https://github.com/yourusername/mindvault.git
cd mindvault

# Build the Rust workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Set up the frontend
cd frontend
pnpm install
pnpm run check   # Type checking
pnpm test         # Run vitest
pnpm run dev      # Start dev server
```

### Running the Server
```bash
cargo run -p mv-server
# Server starts on http://localhost:9470
# Swagger UI at http://localhost:9470/swagger-ui
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
migrations/      # SQL migration files
config/          # Default configuration
docs/            # Architecture docs, guides
```

## Code Style

### Rust
- Follow standard Rust formatting (`cargo fmt`)
- Use `cargo clippy` for lints
- Prefer `thiserror` for error types, `anyhow` for ad-hoc errors
- Async traits use `#[async_trait]` from the `async-trait` crate
- New storage features follow the trait pipeline: `mv-core` trait -> `mv-storage` impl -> `mv-engine` delegation -> `mv-server` REST

### Frontend
- Svelte 5 runes syntax (`$state`, `$derived`, `$effect`)
- Tab indentation (enforced by formatter)
- TypeScript strict mode
- pnpm only (npm has peer dependency conflicts)

## Pull Request Workflow

1. Create a feature branch: `git checkout -b feat/my-feature`
2. Make changes with tests
3. Run `cargo test --workspace` and `cd frontend && pnpm test`
4. Run `cargo clippy --workspace` and `cd frontend && pnpm run check`
5. Commit with a descriptive message (conventional commits preferred)
6. Open a PR against `main`

## Test Requirements

- New Rust features should include unit tests in the module and/or integration tests
- New frontend stores/utilities should include vitest spec files
- Existing tests must pass: `cargo test --workspace` and `pnpm test`

## Commit Conventions

Use conventional commits:
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `test:` adding tests
- `refactor:` code restructuring
- `chore:` maintenance tasks
