"""POST /v1/embeddings — local sentence-transformers inference."""

from __future__ import annotations

import time
from typing import Any

from fastapi import APIRouter, Request, HTTPException
from pydantic import BaseModel

from mindvault_ai.utils.metrics import REQUEST_COUNT, REQUEST_LATENCY, EMBEDDING_BATCH_SIZE

router = APIRouter()


class EmbeddingRequest(BaseModel):
    model: str
    input: str | list[str]
    encoding_format: str = "float"


@router.post("/v1/embeddings")
async def create_embeddings(body: EmbeddingRequest, request: Request) -> dict[str, Any]:
    """Generate embeddings using local sentence-transformers model.

    Response matches the OpenAI embeddings API format expected by
    the Rust server's OpenAiEmbedder (vector.rs:566-580).
    """
    start = time.monotonic()
    registry = request.app.state.registry

    # Normalize input to list
    texts = [body.input] if isinstance(body.input, str) else body.input
    if not texts:
        raise HTTPException(status_code=400, detail="input must not be empty")

    EMBEDDING_BATCH_SIZE.observe(len(texts))

    # Get model from registry
    model = registry.get(body.model)
    if model is None:
        raise HTTPException(
            status_code=404,
            detail=f"Model {body.model!r} not loaded. Available: {[m['name'] for m in registry.list_models()]}",
        )

    # Run inference
    try:
        import numpy as np

        embeddings = model.encode(texts, normalize_embeddings=True)
        if isinstance(embeddings, np.ndarray):
            embeddings = embeddings.tolist()
    except Exception as e:
        REQUEST_COUNT.labels(endpoint="/v1/embeddings", status="error").inc()
        raise HTTPException(status_code=500, detail=f"Embedding failed: {e}")

    # Build OpenAI-compatible response
    data = [
        {
            "object": "embedding",
            "embedding": emb,
            "index": i,
        }
        for i, emb in enumerate(embeddings)
    ]

    elapsed = time.monotonic() - start
    REQUEST_COUNT.labels(endpoint="/v1/embeddings", status="ok").inc()
    REQUEST_LATENCY.labels(endpoint="/v1/embeddings").observe(elapsed)

    return {
        "object": "list",
        "data": data,
        "model": body.model,
        "usage": {
            "prompt_tokens": sum(len(t.split()) for t in texts),
            "total_tokens": sum(len(t.split()) for t in texts),
        },
    }
