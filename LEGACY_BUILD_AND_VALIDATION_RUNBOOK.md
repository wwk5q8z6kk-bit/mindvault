# Legacy Build and Validation Runbook

This runbook reproduces the public baseline without claiming it is currently green.

## Preconditions

- macOS or Linux build host
- Rust/Cargo 1.95.0 (observed baseline; not pinned by repository)
- Node.js 26.0.0 and pnpm 10.28.2 (observed)
- Python 3.14.5 (observed)
- protoc 34.1 (observed)
- SQLite 3.51.0
- enough free disk for Rust, frontend, Tauri, connector, and Python dependency trees

Use a fresh clone and an isolated runtime directory. Do not run against user data.

## Source

```bash
git clone https://github.com/wwk5q8z6kk-bit/mindvault.git
cd mindvault
git checkout dcd7dbdb8c87ca01e22777f79e8f196b993fca23
git status --short
```

Expected: clean checkout at the exact commit.

## Dependency installation

Respect committed lockfiles:

```bash
cargo fetch --locked
(cd frontend && pnpm install --frozen-lockfile)
(cd python && uv sync --frozen)
```

Connector/example packages have separate manifests and lockfiles; install them only when their
test lane is being exercised. Record hashes for any downloaded model or binary tool.

## Static validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
(cd frontend && pnpm check && pnpm test -- --run && pnpm build)
(cd frontend/src-tauri && cargo check --locked)
(cd python && uv run pytest)
```

At the recorded baseline, `cargo fmt --all -- --check` fails. Continue other lanes only to collect
diagnostic evidence, and report each independently.

## Runtime smoke

Use a disposable home/data directory:

```bash
export MINDVAULT_TEST_ROOT="$(mktemp -d)"
export MINDVAULT_DATA_DIR="$MINDVAULT_TEST_ROOT/data"
export MINDVAULT_AUTH_TOKEN="$(openssl rand -hex 32)"
cargo run --locked -p mv-cli -- server
```

In another terminal, use the token via a protected environment or header file—not the process
title—to exercise health, node CRUD, exact lookup, FTS, vector availability, graph traversal,
proposal approve/reject, audit, export, and shutdown. Then run the benchmark adapters described in
`benchmarks/phase0/README.md`.

## Release and packaging

The current GitHub release workflow builds `mv-cli`; it does not establish a complete signed,
notarized Tauri release. Validate separately:

```bash
cargo build --release --locked -p mv-cli
(cd frontend && pnpm tauri build)
```

Record binary hashes, linked-library inventory, package contents, signatures, notarization state,
and license notices.

## Evidence to retain

- host/tool versions;
- exact commit and lockfile hashes;
- command, exit code, wall time, peak RSS, stdout/stderr;
- generated SBOM and vulnerability database timestamp;
- test result files;
- disposable fixture seed;
- package/binary hashes;
- every deviation from this runbook.
