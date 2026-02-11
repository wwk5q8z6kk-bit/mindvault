#!/usr/bin/env bash
set -euo pipefail

# MindVault AI Service Launcher
# Reads secrets from macOS Keychain, then execs uvicorn.
# Called by LaunchAgent — no secrets in config files or plists.

KEYCHAIN_ACCOUNT="$USER"
KEYCHAIN_KIND="environment variable"

read_keychain() {
    local name="$1"
    local val
    val="$(security find-generic-password -w -a "$KEYCHAIN_ACCOUNT" -D "$KEYCHAIN_KIND" -s "$name" 2>/dev/null)" || true
    if [[ -n "$val" ]]; then
        export "$name=$val"
    fi
}

# Load secrets from Keychain (used by remote LLM fallback and RAG auth)
read_keychain OPENAI_API_KEY
read_keychain MINDVAULT_AUTH_TOKEN

# Change to the python project directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Use the venv's Python directly (avoids uv re-resolution picking incompatible versions)
VENV_PYTHON="$SCRIPT_DIR/.venv/bin/python3"

# Exec uvicorn (replaces this shell process)
exec "$VENV_PYTHON" -m uvicorn mindvault_ai.main:app \
    --host 127.0.0.1 \
    --port 8100 \
    --workers 1 \
    --log-level info
