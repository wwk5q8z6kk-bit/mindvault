"""Tests for the /v1/embeddings endpoint."""

from __future__ import annotations


def test_embeddings_single_text(client):
    """Single text input returns one 384-dim embedding."""
    resp = client.post("/v1/embeddings", json={
        "model": "all-MiniLM-L6-v2",
        "input": "Hello world",
    })
    assert resp.status_code == 200
    data = resp.json()
    assert data["object"] == "list"
    assert data["model"] == "all-MiniLM-L6-v2"
    assert len(data["data"]) == 1
    assert data["data"][0]["object"] == "embedding"
    assert data["data"][0]["index"] == 0
    assert len(data["data"][0]["embedding"]) == 384
    assert "usage" in data


def test_embeddings_batch(client):
    """Batch input returns matching number of embeddings."""
    resp = client.post("/v1/embeddings", json={
        "model": "all-MiniLM-L6-v2",
        "input": ["Hello", "World", "Test"],
    })
    assert resp.status_code == 200
    data = resp.json()
    assert len(data["data"]) == 3
    for i, item in enumerate(data["data"]):
        assert item["index"] == i
        assert len(item["embedding"]) == 384


def test_embeddings_unknown_model(client):
    """Unknown model returns 404."""
    resp = client.post("/v1/embeddings", json={
        "model": "nonexistent-model",
        "input": "test",
    })
    assert resp.status_code == 404


def test_embeddings_empty_input(client):
    """Empty input returns 400."""
    resp = client.post("/v1/embeddings", json={
        "model": "all-MiniLM-L6-v2",
        "input": [],
    })
    assert resp.status_code == 400
