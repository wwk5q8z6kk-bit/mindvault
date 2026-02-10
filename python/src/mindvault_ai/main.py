"""FastAPI application with lifespan management for model loading."""

from __future__ import annotations

from contextlib import asynccontextmanager
from typing import AsyncIterator

import structlog
from fastapi import FastAPI

from mindvault_ai.config import load_config
from mindvault_ai.models.registry import ModelRegistry
from mindvault_ai.models.loader import (
    load_sentence_transformer,
    load_spacy_model,
    get_memory_estimate,
)
from mindvault_ai.models.ollama_proxy import OllamaProxy
from mindvault_ai.api import chat, embeddings, models, health

structlog.configure(
    processors=[
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.add_log_level,
        structlog.dev.ConsoleRenderer(),
    ],
    wrapper_class=structlog.make_filtering_bound_logger(20),  # INFO
)

logger = structlog.get_logger()


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    """Load models on startup, clean up on shutdown."""
    config = load_config()
    app.state.config = config

    # Initialize model registry
    registry = ModelRegistry(
        memory_budget_mb=config.models.memory_budget_mb,
        cache_dir=config.models.cache_dir,
    )
    app.state.registry = registry

    # Load embedding model
    logger.info("loading_embedding_model", model=config.embedding.model)
    embed_model = load_sentence_transformer(
        config.embedding.model, cache_dir=config.models.cache_dir
    )
    registry.put(
        config.embedding.model,
        embed_model,
        get_memory_estimate(config.embedding.model),
    )

    # Load spaCy model
    logger.info("loading_spacy_model", model=config.classification.spacy_model)
    nlp = load_spacy_model(config.classification.spacy_model)
    app.state.spacy_nlp = nlp

    # Initialize Ollama proxy
    ollama = OllamaProxy(base_url=config.llm.ollama_base_url)
    app.state.ollama = ollama

    ollama_ok = await ollama.is_available()
    logger.info(
        "startup_complete",
        embedding_model=config.embedding.model,
        spacy_model=config.classification.spacy_model,
        ollama_available=ollama_ok,
        port=config.server.port,
    )

    yield

    # Shutdown
    logger.info("shutting_down")
    await ollama.close()


def create_app() -> FastAPI:
    """Create and configure the FastAPI application."""
    app = FastAPI(
        title="MindVault AI Service",
        version="0.1.0",
        lifespan=lifespan,
    )

    app.include_router(health.router)
    app.include_router(models.router)
    app.include_router(embeddings.router)
    app.include_router(chat.router)

    return app


app = create_app()


if __name__ == "__main__":
    import uvicorn

    config = load_config()
    uvicorn.run(
        "mindvault_ai.main:app",
        host=config.server.host,
        port=config.server.port,
        reload=False,
    )
