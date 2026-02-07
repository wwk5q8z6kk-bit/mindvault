#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> Rust format check"
cargo fmt --all -- --check

echo "==> Rust clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> Rust tests"
cargo test --workspace

echo "==> Connector checks"
(
  cd connectors/claude-code-mcp
  npm ci
  npx tsc --noEmit
)
(
  cd connectors/openclaw-plugin
  node --check index.ts
  node --check client.ts
)

echo "==> End-to-end smoke test"
./scripts/smoke_test.sh

echo "==> Full verification passed"
