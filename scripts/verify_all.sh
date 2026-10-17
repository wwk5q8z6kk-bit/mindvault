#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> Rust format check"
cargo fmt --all -- --check

echo "==> Rust clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> Rust tests"
cargo test --workspace --exclude mv-server --exclude mv-index
# Sealed tantivy runtime keys can race under parallel lib tests.
cargo test -p mv-index --lib -- --test-threads=1
cargo test -p mv-server --lib
# Integration tests mutate process env (auth/quota/sealed failpoints);
# serialize them to avoid cross-test leakage under cargo's default parallelism.
cargo test -p mv-server --tests -- --test-threads=1

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

echo "==> Frontend checks"
(
  cd frontend
  corepack enable
  pnpm install --frozen-lockfile --ignore-scripts
  pnpm run check
  pnpm run lint
  pnpm run test:unit
)

if [[ "${RUN_SMOKE_TEST:-0}" == "1" ]]; then
  echo "==> End-to-end smoke test"
  ./scripts/smoke_test.sh
else
  echo "==> Skipping smoke test (set RUN_SMOKE_TEST=1 to enable)"
fi

echo "==> Full verification passed"
