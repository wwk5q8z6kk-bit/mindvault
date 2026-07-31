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

SERVER_PID=""
cleanup() {
    # Kill by PID, not by pattern. `cargo run -p mv-cli` execs `target/debug/mv`,
    # so the old `pkill -f "mv-cli.*"` matched NOTHING and every run leaked a
    # server still holding the port. The next run's health check then talked to
    # that orphan — whose data dir this trap had already deleted — so create
    # "succeeded" against a corpse and search legitimately found nothing.
    if [[ -n "$SERVER_PID" ]]; then
        kill "$SERVER_PID" 2>/dev/null
        # Silence bash's "Terminated: 15" job notice — an expected shutdown
        # should not look like a failure in the output.
        disown "$SERVER_PID" 2>/dev/null
        wait "$SERVER_PID" 2>/dev/null
    fi
    rm -rf "$DIR"
}
trap cleanup EXIT

# Refuse to run against someone else's server. Without this the health check is
# a gate that cannot fail: it goes green against ANY listener on the port, which
# is precisely how the orphan above stayed invisible.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "UNVERIFIED: port $PORT is already in use — refusing to smoke-test an unknown server." >&2
    echo "  offender: $(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN | tail -1)" >&2
    exit 2
fi

printf '[storage]\ndata_dir = "%s/data"\n' "$DIR" > "$DIR/config.toml"

echo "== build =="
cargo build -q -p mv-cli 2>/dev/null || { echo "FAIL: mv-cli does not build"; exit 1; }

echo "== start =="
cargo run -q -p mv-cli -- server start --foreground \
    --port "$PORT" --grpc-port "$GRPC" --config "$DIR/config.toml" >"$LOG" 2>&1 &
SERVER_PID=$!

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

# Search is NOT synchronous with write. Tantivy is near-real-time: the writer
# commits and the reader reloads on its own cadence, so a create followed
# immediately by a search can legitimately miss. The first version of this
# script searched instantly and was itself flaky — it passed standalone and
# failed on the very next run, with both search paths reporting nothing while
# the node plainly existed.
#
# So: poll with a bound, and report how long visibility actually took. A fixed
# `sleep` would hide a regression in that latency; a bounded poll surfaces it.
await_visible() { # name, url-or-empty, post-body-or-empty
    local name="$1" url="$2" body="$3" start elapsed out
    start=$(date +%s)
    for _ in $(seq 30); do
        if [[ -n "$body" ]]; then
            out=$(curl -s -m 20 -X POST "$url" -H 'content-type: application/json' -d "$body")
        else
            out=$(curl -s -m 10 "$url")
        fi
        if grep -q 'gate axiom' <<<"$out"; then
            elapsed=$(( $(date +%s) - start ))
            echo "  ok   $name (visible after ${elapsed}s)"
            return 0
        fi
        sleep 1
    done
    echo "  FAIL $name (not visible within 30s)"; fail=1; return 1
}

await_visible "keyword search" "http://127.0.0.1:$PORT/api/v1/search?q=gate" ""

# NOTE: the recall body field is `text`, not `query`. Getting this wrong returns
# a precise 422 ("missing field `text`"), which is good API behaviour and worth
# preserving.
await_visible "recall" "http://127.0.0.1:$PORT/api/v1/recall" \
    '{"text":"what must a gate be able to do","limit":3}'

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
