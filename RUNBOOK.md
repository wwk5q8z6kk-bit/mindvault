# Demo Runbook

## Objective
Show that MindVault is secure by default, searchable, and stable for live demo conditions.

## 60-second Validation

```bash
cd /Users/x/Projects/mindvault
./scripts/smoke_test.sh
```

This verifies:

1. Server starts with local bind (`127.0.0.1`) and auth token enabled.
2. Unauthenticated request is rejected (`401`).
3. Authenticated health passes (`200`).
4. Role/namespace-aware auth path is active.
5. Node storage works.
6. Hybrid recall returns results.
7. Request limits and namespace quota controls are available.
8. Optional AI auto-tagging can be toggled and tuned.

## Full Validation Gate

```bash
./scripts/verify_all.sh
```

This runs format, clippy, tests, connector checks, and the smoke test.

## Manual Demo Sequence

1. Start server:

```bash
MINDVAULT_AUTH_TOKEN=demo-token \
  cargo run -p mv-cli -- server start --foreground --config /tmp/mindvault-demo.toml
```

2. Store memory:

```bash
curl -X POST http://127.0.0.1:9570/api/v1/nodes \
  -H 'Authorization: Bearer demo-token' \
  -H 'Content-Type: application/json' \
  -d '{"kind":"fact","content":"MindVault demo memory","namespace":"demo"}'
```

3. Recall memory:

```bash
curl -X POST http://127.0.0.1:9570/api/v1/recall \
  -H 'Authorization: Bearer demo-token' \
  -H 'Content-Type: application/json' \
  -d '{"text":"demo memory","strategy":"hybrid","limit":5,"namespace":"demo"}'
```

4. Optional role-scoped demo (shared token):

```bash
MINDVAULT_AUTH_TOKEN=demo-token \
MINDVAULT_AUTH_ROLE=read \
MINDVAULT_AUTH_NAMESPACE=demo \
  cargo run -p mv-cli -- server start --foreground --config /tmp/mindvault-demo.toml
```

This token can read from `demo`, but write requests return `403`.

5. Optional limit demo:

```bash
MINDVAULT_AUTH_TOKEN=demo-token \
MINDVAULT_RATE_LIMIT_REQUESTS=3 \
MINDVAULT_RATE_LIMIT_WINDOW_SECS=60 \
MINDVAULT_NAMESPACE_NODE_QUOTA=100 \
  cargo run -p mv-cli -- server start --foreground --config /tmp/mindvault-demo.toml
```

After 3 authenticated requests in the 60-second window for the same identity, REST returns `429`.

6. Optional AI auto-tagging demo:

```bash
MINDVAULT_AUTH_TOKEN=demo-token \
MINDVAULT_AI_AUTO_TAGGING_ENABLED=true \
MINDVAULT_AI_AUTO_TAGGING_MAX_GENERATED_TAGS=6 \
  cargo run -p mv-cli -- server start --foreground --config /tmp/mindvault-demo.toml
```

Store a node without tags and verify tags are auto-enriched on create/update.

## Pre-Submit Checklist

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Vector Index Rebuild

When embedding settings change or namespace-aware vector filtering is enabled, rebuild LanceDB:

```bash
cargo run -p mv-cli -- db rebuild-vectors --dry-run
cargo run -p mv-cli -- db rebuild-vectors --batch-size 64
cargo run -p mv-cli -- db rebuild-vectors --batch-size 64 --apply --confirm
```

Stop the server before running `--apply` to avoid file locks.

## Key Improvements

1. Security defaults: local bind, auth on REST/gRPC/WebSocket, non-permissive CORS.
2. API correctness: strict kind parsing, stable `match_source`, better error mapping.
3. Data correctness: filter/count parity, SQL-level tag filtering, strict decode handling.
4. Reliability: connector retries/timeouts, privacy-safer auto-capture defaults.
