"""Shared test fixtures for MindVault AI service tests."""

from __future__ import annotations

from typing import AsyncIterator
from unittest.mock import AsyncMock, MagicMock

import pytest
from fastapi.testclient import TestClient
from httpx import AsyncClient, ASGITransport

from mindvault_ai.config import AppConfig
from mindvault_ai.models.registry import ModelRegistry


class FakeEmbeddingModel:
    """Fake sentence-transformers model for testing."""

    def encode(self, texts: list[str], normalize_embeddings: bool = True) -> list[list[float]]:
        """Return deterministic fake embeddings (384 dims)."""
        import numpy as np

        return [
            (np.random.RandomState(hash(t) % 2**32).rand(384).astype(float)).tolist()
            for t in texts
        ]


class FakeSpacyNlp:
    """Fake spaCy NLP pipeline for testing."""

    def __call__(self, text: str) -> FakeDoc:
        return FakeDoc(text)


class FakeDoc:
    def __init__(self, text: str) -> None:
        self.text = text
        self.ents = []
        self.noun_chunks = []

        # Create fake entities from quoted strings
        import re

        for match in re.finditer(r'"([^"]+)"', text):
            self.ents.append(FakeEntity(match.group(1), "PERSON", match.start(), match.end()))

    def __iter__(self):
        return iter([FakeToken(w, i) for i, w in enumerate(self.text.split())])


class FakeEntity:
    def __init__(self, text: str, label: str, start: int, end: int) -> None:
        self.text = text
        self.label_ = label
        self.start_char = start
        self.end_char = end


class FakeToken:
    def __init__(self, text: str, i: int) -> None:
        self.text = text
        self.i = i
        self.pos_ = "NOUN"
        self.dep_ = "nsubj"


def _create_test_app() -> "FastAPI":
    """Create a FastAPI app with mocked dependencies (no real model loading)."""
    from fastapi import FastAPI
    from mindvault_ai.api import chat, embeddings, models, health

    app = FastAPI()
    app.include_router(health.router)
    app.include_router(models.router)
    app.include_router(embeddings.router)
    app.include_router(chat.router)

    config = AppConfig()
    app.state.config = config

    registry = ModelRegistry(memory_budget_mb=1024)
    fake_model = FakeEmbeddingModel()
    registry.put("all-MiniLM-L6-v2", fake_model, 90 * 1024 * 1024)
    app.state.registry = registry

    app.state.spacy_nlp = FakeSpacyNlp()

    ollama = AsyncMock()
    ollama.is_available = AsyncMock(return_value=True)
    ollama.list_models = AsyncMock(return_value=[])
    ollama.chat_completion = AsyncMock(return_value={
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "llama3.2:3b",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": "Test response"},
                "finish_reason": "stop",
            }
        ],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
    })
    ollama.close = AsyncMock()
    app.state.ollama = ollama

    return app


@pytest.fixture
def app():
    """FastAPI app with mocked dependencies."""
    return _create_test_app()


@pytest.fixture
def client(app) -> TestClient:
    """Synchronous test client."""
    return TestClient(app)


@pytest.fixture
async def async_client(app) -> AsyncIterator[AsyncClient]:
    """Async test client for testing async endpoints."""
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        yield ac
