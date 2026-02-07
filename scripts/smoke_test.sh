#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MV_BIN="$ROOT_DIR/target/debug/mv"
if [[ ! -x "$MV_BIN" ]]; then
  MV_BIN="cargo run -p mv-cli --"
fi

REST_PORT="${MV_REST_PORT:-9570}"
GRPC_PORT="${MV_GRPC_PORT:-50071}"
BIND_HOST="127.0.0.1"
TOKEN="${MINDVAULT_AUTH_TOKEN:-demo-token}"
CONFIG_PATH="${MV_CONFIG_PATH:-/tmp/mindvault-demo.toml}"
LOG_PATH="${MV_LOG_PATH:-/tmp/mindvault-demo-server.log}"

cat > "$CONFIG_PATH" <<CFG
[server]
bind_host = "$BIND_HOST"
rest_port = $REST_PORT
grpc_port = $GRPC_PORT
cors_allowed_origins = ["http://localhost:3000"]

[storage]
data_dir = "/tmp/mindvault-demo-data"

[embedding]
provider = "openai"
model = "text-embedding-3-small"
dimensions = 1536
CFG

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "==> Starting server"
export MINDVAULT_AUTH_TOKEN="$TOKEN"
if [[ "$MV_BIN" == "cargo run -p mv-cli --" ]]; then
  (cd "$ROOT_DIR" && cargo run -p mv-cli -- server start --foreground --config "$CONFIG_PATH" >"$LOG_PATH" 2>&1) &
else
  "$MV_BIN" server start --foreground --config "$CONFIG_PATH" >"$LOG_PATH" 2>&1 &
fi
SERVER_PID=$!

echo "==> Waiting for health endpoint"
for _ in {1..60}; do
  if curl -s -H "Authorization: Bearer $TOKEN" "http://$BIND_HOST:$REST_PORT/api/v1/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -s -H "Authorization: Bearer $TOKEN" "http://$BIND_HOST:$REST_PORT/api/v1/health" >/dev/null 2>&1; then
  echo "Server did not become ready. Log: $LOG_PATH"
  exit 1
fi

echo "==> Auth negative test (expect 401)"
STATUS_NO_AUTH="$(curl -s -o /dev/null -w '%{http_code}' "http://$BIND_HOST:$REST_PORT/api/v1/health")"
if [[ "$STATUS_NO_AUTH" != "401" ]]; then
  echo "Expected 401 without token, got $STATUS_NO_AUTH"
  exit 1
fi

echo "==> Auth positive test"
STATUS_AUTH="$(curl -s -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $TOKEN" "http://$BIND_HOST:$REST_PORT/api/v1/health")"
if [[ "$STATUS_AUTH" != "200" ]]; then
  echo "Expected 200 with token, got $STATUS_AUTH"
  exit 1
fi

echo "==> Store node"
STORE_RESP="$(curl -s -X POST "http://$BIND_HOST:$REST_PORT/api/v1/nodes" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"kind":"fact","title":"Demo Fact","content":"MindVault supports hybrid recall with graph boost.","namespace":"demo","tags":["demo","smoke"],"importance":0.9}')"

if command -v jq >/dev/null 2>&1; then
  NODE_ID="$(printf '%s' "$STORE_RESP" | jq -r '.id')"
else
  NODE_ID="$(printf '%s' "$STORE_RESP" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')"
fi

if [[ -z "$NODE_ID" || "$NODE_ID" == "null" ]]; then
  echo "Failed to parse node id from response: $STORE_RESP"
  exit 1
fi

echo "Stored node: $NODE_ID"

echo "==> Recall"
RECALL_RESP="$(curl -s -X POST "http://$BIND_HOST:$REST_PORT/api/v1/recall" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"text":"hybrid recall graph boost","strategy":"hybrid","limit":3,"namespace":"demo"}')"

if command -v jq >/dev/null 2>&1; then
  echo "$RECALL_RESP" | jq .
else
  echo "$RECALL_RESP"
fi

echo "==> Smoke test passed"
echo "Server log: $LOG_PATH"
