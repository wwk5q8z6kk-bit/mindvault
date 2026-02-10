"""Tests for the summarization pipeline."""

from __future__ import annotations

import json


def test_extractive_summarize(client):
    """Extractive summarization returns summary text."""
    long_text = (
        "Machine learning is a subset of artificial intelligence. "
        "It involves training algorithms on data. "
        "The algorithms learn patterns from the data. "
        "These patterns can be used for prediction. "
        "Deep learning uses neural networks with many layers. "
        "Convolutional networks are used for images. "
        "Recurrent networks handle sequential data. "
        "Transformers have revolutionized natural language processing. "
        "They use self-attention mechanisms. "
        "Large language models are based on transformers."
    )
    resp = client.post("/v1/chat/completions", json={
        "model": "summarize:extractive",
        "messages": [{"role": "user", "content": long_text}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["type"] == "extractive"
    assert len(content["summary"]) > 0
    assert content["method"] == "textrank"


def test_abstractive_summarize(client):
    """Abstractive summarization calls the LLM."""
    resp = client.post("/v1/chat/completions", json={
        "model": "summarize:abstractive",
        "messages": [{"role": "user", "content": "A long text that needs summarizing."}],
    })
    assert resp.status_code == 200
    data = resp.json()
    content = json.loads(data["choices"][0]["message"]["content"])
    assert content["type"] == "abstractive"
