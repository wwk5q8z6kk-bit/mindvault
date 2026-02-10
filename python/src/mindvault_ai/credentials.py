"""Credential resolution — reads API keys from environment variables.

start-ai.sh populates env vars from macOS Keychain before exec'ing uvicorn.
This module provides a single lookup function used by pipelines that need API keys.
"""

from __future__ import annotations

import os


def get_api_key(name: str) -> str | None:
    """Resolve an API key by environment variable name."""
    return os.environ.get(name)


def require_api_key(name: str) -> str:
    """Resolve an API key, raising if not set."""
    key = get_api_key(name)
    if not key:
        raise RuntimeError(
            f"Required API key {name!r} not found in environment. "
            f"Ensure start-ai.sh loads it from Keychain."
        )
    return key
