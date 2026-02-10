"""Tests for the classification pipeline."""

from __future__ import annotations

import json


def test_sentiment_positive(client):
    """Positive sentiment detection."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:sentiment",
        "messages": [{"role": "user", "content": "This is amazing and wonderful"}],
    })
    assert resp.status_code == 200
    content = json.loads(resp.json()["choices"][0]["message"]["content"])
    assert content["label"] == "positive"
    assert content["score"] > 0.5


def test_sentiment_negative(client):
    """Negative sentiment detection."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:sentiment",
        "messages": [{"role": "user", "content": "This is terrible and awful"}],
    })
    assert resp.status_code == 200
    content = json.loads(resp.json()["choices"][0]["message"]["content"])
    assert content["label"] == "negative"


def test_sentiment_neutral(client):
    """Neutral text returns neutral sentiment."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:sentiment",
        "messages": [{"role": "user", "content": "The meeting is at 3pm"}],
    })
    assert resp.status_code == 200
    content = json.loads(resp.json()["choices"][0]["message"]["content"])
    assert content["label"] == "neutral"


def test_topic_technology(client):
    """Technology topic classification."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:topic",
        "messages": [{"role": "user", "content": "The software uses a database and API for the algorithm"}],
    })
    assert resp.status_code == 200
    content = json.loads(resp.json()["choices"][0]["message"]["content"])
    assert content["primary_topic"] == "technology"


def test_intent_question(client):
    """Question intent detection."""
    resp = client.post("/v1/chat/completions", json={
        "model": "classify:intent",
        "messages": [{"role": "user", "content": "What is the capital of France?"}],
    })
    assert resp.status_code == 200
    content = json.loads(resp.json()["choices"][0]["message"]["content"])
    assert content["intent"] == "question"
