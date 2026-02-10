"""Health and metrics endpoints."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Request
from prometheus_client import generate_latest, CONTENT_TYPE_LATEST
from starlette.responses import Response

router = APIRouter()


@router.get("/health")
async def health(request: Request) -> dict[str, Any]:
    """Service health check including loaded models and Ollama availability."""
    app_state = request.app.state
    registry = app_state.registry
    ollama = app_state.ollama

    ollama_ok = await ollama.is_available()

    return {
        "status": "healthy",
        "version": "0.1.0",
        "models_loaded": [m["name"] for m in registry.list_models()],
        "ollama_available": ollama_ok,
        "memory_used_mb": registry.total_memory // (1024 * 1024),
    }


@router.get("/metrics")
async def metrics() -> Response:
    """Prometheus metrics endpoint."""
    return Response(
        content=generate_latest(),
        media_type=CONTENT_TYPE_LATEST,
    )
