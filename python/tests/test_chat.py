"""Tests for the /v1/chat/completions endpoint — model routing."""

from __future__ import annotations

import json


def test_chat_mindvault_default(client):
    """mindvault-default routes to Ollama with configured model."""
    resp = client.post("/v1/chat/completions", json={
        "model": "mindvault-default",
        "messages": [{"role": "user", "content": "Hello"}],
    })
    assert resp.status_code == 200
    data = resp.json()
    assert data["object"] == "chat.completion"
    assert len(data["choices"]) == 1
    assert data["choices"][0]["message"]["role"] == "assistant"
    assert data["choices"][0]["message"]["content"] == "Test response"
    assert data["choices"][0]["finish_reason"] == "stop"


def test_chat_ollama_proxy(client):
    """ollama/<model> routes to Ollama with specified model."""
    resp = client.post("/v1/chat/completions", json={
        "model": "ollama/llama3.2:3b",
        "messages": [{"role": "user", "content": "Hello"}],
    })
    assert resp.status_code == 200


def test_chat_classify_sentiment(client):
    """classify:sentiment returns JSON with sentiment data."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:sentiment",
        "messages": [{"role": "user", "content": "This is a great and amazing product"}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["type"] == "sentiment"
    assert content["label"] in ("positive", "negative", "neutral", "mixed")
    assert "score" in content


def test_chat_classify_topic(client):
    """classify:topic returns JSON with topic data."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:topic",
        "messages": [{"role": "user", "content": "The software API uses a database algorithm"}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["type"] == "topic"
    assert "primary_topic" in content


def test_chat_classify_intent(client):
    """classify:intent returns JSON with intent data."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:intent",
        "messages": [{"role": "user", "content": "What is the weather?"}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["type"] == "intent"
    assert content["intent"] == "question"


def test_chat_ner(client):
    """ner:default returns JSON with entities."""
    resp = client.post("/v1/chat/completions", json={
        "model": "ner:default",
        "messages": [{"role": "user", "content": 'Meeting with "John Smith" tomorrow'}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert "entities" in content
    assert "total_entities" in content


def test_chat_unknown_model(client):
    """Unknown model returns 400."""
    resp = client.post("/v1/chat/completions", json={
        "model": "unknown-model",
        "messages": [{"role": "user", "content": "Hello"}],
    })
    assert resp.status_code == 400


def test_chat_with_optional_params(client):
    """max_tokens and temperature are forwarded."""
    resp = client.post("/v1/chat/completions", json={
        "model": "mindvault-default",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 100,
        "temperature": 0.5,
    })
    assert resp.status_code == 200
