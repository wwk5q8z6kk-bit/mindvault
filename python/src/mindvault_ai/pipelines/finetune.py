"""Fine-tuning pipeline — LoRA via peft (optional dependencies).

This module is a placeholder for Phase 3f. It provides stub endpoints
that return appropriate messages when the optional dependencies are not installed.
"""

from __future__ import annotations

from typing import Any

import structlog

logger = structlog.get_logger()


def is_available() -> bool:
    """Check if fine-tuning dependencies are installed."""
    try:
        import peft  # noqa: F401
        import datasets  # noqa: F401
        import accelerate  # noqa: F401

        return True
    except ImportError:
        return False


async def start_finetune(params: dict[str, Any]) -> dict[str, Any]:
    """Start a fine-tuning job (stub)."""
    if not is_available():
        return {
            "status": "error",
            "message": "Fine-tuning dependencies not installed. Install with: uv pip install 'mindvault-ai[finetune]'",
        }

    return {
        "status": "planned",
        "message": "Fine-tuning support is planned for Phase 3f.",
    }


async def get_status(job_id: str) -> dict[str, Any]:
    """Get fine-tuning job status (stub)."""
    return {"status": "not_found", "job_id": job_id}


async def cancel_finetune(job_id: str) -> dict[str, Any]:
    """Cancel a fine-tuning job (stub)."""
    return {"status": "not_found", "job_id": job_id}
