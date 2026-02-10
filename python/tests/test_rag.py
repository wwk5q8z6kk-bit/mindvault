"""Tests for the RAG pipeline with mocked retrieval."""

from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest


@pytest.mark.asyncio
async def test_rag_with_context(async_client):
    """RAG pipeline retrieves context and generates augmented response."""
    mock_search_response = {
        "results": [
            {"title": "Note 1", "body": "MindVault is a knowledge management system."},
            {"title": "Note 2", "body": "It uses Rust for the backend server."},
        ]
    }

    with patch("mindvault_ai.pipelines.rag.httpx.AsyncClient") as MockClient:
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.raise_for_status = MagicMock()  # sync method
        mock_resp.json.return_value = mock_search_response  # sync method

        mock_instance = AsyncMock()
        mock_instance.post = AsyncMock(return_value=mock_resp)
        mock_instance.__aenter__ = AsyncMock(return_value=mock_instance)
        mock_instance.__aexit__ = AsyncMock(return_value=False)
        MockClient.return_value = mock_instance

        resp = await async_client.post("/v1/chat/completions", json={
            "model": "rag:default",
            "messages": [{"role": "user", "content": "What is MindVault?"}],
        })

    assert resp.status_code == 200
    data = resp.json()
    assert data["object"] == "chat.completion"
    # The Ollama mock returns "Test response"
    assert data["choices"][0]["message"]["content"] == "Test response"


@pytest.mark.asyncio
async def test_rag_without_context(async_client):
    """RAG pipeline handles no retrieval results gracefully."""
    with patch("mindvault_ai.pipelines.rag.httpx.AsyncClient") as MockClient:
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.raise_for_status = MagicMock()
        mock_resp.json.return_value = {"results": []}

        mock_instance = AsyncMock()
        mock_instance.post = AsyncMock(return_value=mock_resp)
        mock_instance.__aenter__ = AsyncMock(return_value=mock_instance)
        mock_instance.__aexit__ = AsyncMock(return_value=False)
        MockClient.return_value = mock_instance

        resp = await async_client.post("/v1/chat/completions", json={
            "model": "rag:default",
            "messages": [{"role": "user", "content": "Something obscure"}],
        })

    assert resp.status_code == 200
