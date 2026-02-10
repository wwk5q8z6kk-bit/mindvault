"""Tests for the NER pipeline."""

from __future__ import annotations

import json


def test_ner_extracts_entities(client):
    """NER pipeline extracts entities from text."""
    resp = client.post("/v1/chat/completions", json={
        "model": "ner:default",
        "messages": [{"role": "user", "content": 'I met "John Smith" at the "Acme Corp" office'}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["total_entities"] >= 1
    assert any(e["text"] == "John Smith" for e in content["entities"])


def test_ner_no_entities(client):
    """NER pipeline handles text with no recognized entities."""
    resp = client.post("/v1/chat/completions", json={
        "model": "ner:default",
        "messages": [{"role": "user", "content": "just some plain text here"}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["total_entities"] == 0


def test_ner_response_format(client):
    """NER response matches OpenAI chat completion format."""
    resp = client.post("/v1/chat/completions", json={
        "model": "ner:default",
        "messages": [{"role": "user", "content": "test text"}],
    })
    assert resp.status_code == 200
    data = resp.json()
    assert data["object"] == "chat.completion"
    assert data["model"] == "ner:default"
    assert len(data["choices"]) == 1
    assert data["choices"][0]["message"]["role"] == "assistant"
    assert data["choices"][0]["finish_reason"] == "stop"
