"""GET /v1/models — list available models and pipelines."""

from __future__ import annotations

import time
from typing import Any

from fastapi import APIRouter, Request

router = APIRouter()

# Static pipeline "models" always available
PIPELINE_MODELS = [
    {"id": "rag:default", "object": "model", "owned_by": "mindvault", "description": "RAG pipeline"},
    {"id": "classify:topic", "object": "model", "owned_by": "mindvault", "description": "Topic classification"},
    {"id": "classify:sentiment", "object": "model", "owned_by": "mindvault", "description": "Sentiment analysis"},
    {"id": "classify:intent", "object": "model", "owned_by": "mindvault", "description": "Intent classification"},
    {"id": "summarize:extractive", "object": "model", "owned_by": "mindvault", "description": "Extractive summarization"},
    {"id": "summarize:abstractive", "object": "model", "owned_by": "mindvault", "description": "Abstractive summarization"},
    {"id": "ner:default", "object": "model", "owned_by": "mindvault", "description": "Named entity recognition"},
    {"id": "mindvault-default", "object": "model", "owned_by": "mindvault", "description": "Default LLM (Ollama)"},
]


@router.get("/v1/models")
async def list_models(request: Request) -> dict[str, Any]:
    """List all available models/pipelines in OpenAI format."""
    app_state = request.app.state
    ollama = app_state.ollama
    config = app_state.config

    models: list[dict[str, Any]] = []

    # Embedding models from registry
    registry = app_state.registry
    for m in registry.list_models():
        models.append({
            "id": m["name"],
            "object": "model",
            "created": int(m["loaded_at"]),
            "owned_by": "local",
        })

    # Pipeline models
    for pm in PIPELINE_MODELS:
        models.append({**pm, "created": int(time.time())})

    # Ollama models
    ollama_models = await ollama.list_models()
    models.extend(ollama_models)

    return {"object": "list", "data": models}


@router.post("/api/v1/models/load")
async def load_model(request: Request) -> dict[str, str]:
    """Manually load a model into the registry."""
    body = await request.json()
    model_name = body.get("model", "")
    if not model_name:
        return {"error": "model name required"}

    from mindvault_ai.models.loader import load_sentence_transformer, get_memory_estimate

    registry = request.app.state.registry
    if registry.get(model_name) is not None:
        return {"status": "already_loaded", "model": model_name}

    model = load_sentence_transformer(model_name, cache_dir=registry.cache_dir)
    registry.put(model_name, model, get_memory_estimate(model_name))
    return {"status": "loaded", "model": model_name}


@router.post("/api/v1/models/unload")
async def unload_model(request: Request) -> dict[str, str]:
    """Manually unload a model from the registry."""
    body = await request.json()
    model_name = body.get("model", "")
    if not model_name:
        return {"error": "model name required"}

    registry = request.app.state.registry
    if registry.remove(model_name):
        return {"status": "unloaded", "model": model_name}
    return {"status": "not_found", "model": model_name}
