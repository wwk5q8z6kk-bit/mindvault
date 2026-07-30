#!/usr/bin/env bash
# mindvault first-run smoke — does the product actually START and WORK?
#
# Exit: 0 pass · 1 fail · 2 unverified (could not run the check at all)
#
# WHY THIS EXISTS, SEPARATE FROM scripts/release-gate.sh
#
#   On 2026-07-29 the release gate reported PASS — cargo check clean, clippy
#   clean, 529 tests green — while the product could not start at all. Two
#   blockers, neither visible to compile-or-test:
#
#     1. `utoipa-swagger-ui`'s build output had an absolute path baked in from a
#        different checkout (`Projects/GitHub/mindvault`); 546 stale artifacts
#        pointed at a directory that no longer existed, so `mv-cli` would not
#        build. Fix was `cargo clean -p utoipa-swagger-ui` (120 MB, not 88 GB —
#        `target` is a symlink to /Volumes/Express4M2/Builds/mindvault/target).
#
#     2. `ort` (ONNX Runtime binding behind fastembed, pulled in by
#        mv-engine's default `local-embeddings` feature) PANICS instead of
#        returning Err when it cannot dlopen libonnxruntime. That bypassed the
#        engine's own `..._falling_back_to_noop` path and killed the server.
#        Fixed by catching the unwind in crates/mv-storage/src/vector.rs.
#
#   A green suite is not a running product. This script closes that gap: it
#   starts the real binary on throwaway ports against a temp data dir and drives
#   the core user flow over HTTP.
#
# It deliberately runs WITHOUT libonnxruntime installed, so the degraded path is
# what gets exercised. If you install ONNX Runtime, semantic recall improves —
# this script must still pass either way.

set -uo pipefail
cd "$(dirname "$0")/.."

PORT="${MV_SMOKE_PORT:-19480}"
GRPC="${MV_SMOKE_GRPC:-15060}"
DIR="$(mktemp -d "${TMPDIR:-/tmp}/mv-smoke.XXXXXX")"
LOG="$DIR/server.log"
fail=0

cleanup() { pkill -f "mv-cli.*--port $PORT" 2>/dev/null; rm -rf "$DIR"; }
trap cleanup EXIT

printf '[storage]\ndata_dir = "%s/data"\n' "$DIR" > "$DIR/config.toml"

echo "== build =="
cargo build -q -p mv-cli 2>/dev/null || { echo "FAIL: mv-cli does not build"; exit 1; }

echo "== start =="
(cargo run -q -p mv-cli -- server start --foreground \
    --port "$PORT" --grpc-port "$GRPC" --config "$DIR/config.toml" >"$LOG" 2>&1 &)

up=0
for _ in $(seq 60); do
    sleep 1
    if curl -sf -m 2 -o /dev/null "http://127.0.0.1:$PORT/api/v1/health"; then up=1; break; fi
done
if [[ $up -eq 0 ]]; then
    echo "FAIL: server never became healthy. Last log lines:"; tail -15 "$LOG"; exit 1
fi
echo "  healthy"

check() { # name, condition-output, needle
    if grep -q "$3" <<<"$2"; then echo "  ok   $1"; else echo "  FAIL $1"; fail=1; fi
}

echo "== core flow =="
check "health"        "$(curl -s -m 5 http://127.0.0.1:$PORT/api/v1/health)" '"status":"ok"'

created=$(curl -s -m 10 -X POST "http://127.0.0.1:$PORT/api/v1/nodes" \
    -H 'content-type: application/json' \
    -d '{"kind":"fact","content":"The release gate must be able to fail, or it is not a gate.","title":"gate axiom","tags":["verification"]}')
check "create node"   "$created" '"title":"gate axiom"'

check "keyword search" "$(curl -s -m 10 "http://127.0.0.1:$PORT/api/v1/search?q=gate")" 'gate axiom'

# NOTE: the recall body field is `text`, not `query`. Getting this wrong returns
# a precise 422 ("missing field `text`"), which is good API behaviour and worth
# preserving.
check "recall"        "$(curl -s -m 20 -X POST "http://127.0.0.1:$PORT/api/v1/recall" \
    -H 'content-type: application/json' -d '{"text":"what must a gate be able to do","limit":3}')" 'gate axiom'

check "openapi"       "$(curl -s -m 5 -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/api/openapi.json)" '200'
check "swagger ui"    "$(curl -s -m 5 -L -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/api/docs)" '200'

# Degradation must be reported, not silent. If libonnxruntime IS present this
# line is absent and that is fine — the check is that we never panicked.
if grep -q "panicked" "$LOG" && ! grep -q "falling_back_to_noop" "$LOG"; then
    echo "  FAIL panic without fallback"; fail=1
else
    echo "  ok   no unhandled panic"
fi

echo
[[ $fail -eq 0 ]] && { echo "FIRST-RUN SMOKE: PASS"; exit 0; } || { echo "FIRST-RUN SMOKE: FAIL"; exit 1; }
