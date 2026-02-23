"""Tests for the NER pipeline."""

from __future__ import annotations

import json


def test_relations_extraction(client):
    """Test extracting Subject-Verb-Object triplets."""
    resp = client.post(
        "/v1/chat/completions",
        json={
            "model": "relations:default",
            "messages": [
                {
                    "role": "user",
                    "content": "The engine depends on the vector database. The storage module is part of the core system.",
                }
            ],
        },
    )

    assert resp.status_code == 200
    data = resp.json()
    assert data["object"] == "chat.completion"
    
    content = json.loads(data["choices"][0]["message"]["content"])
    assert "relations" in content
    
    relations = content["relations"]
    
    # We expect 2 relations based on the text
    assert len(relations) >= 2
    
    # Search for the "depends on" relation
    depends_rel = next((r for r in relations if r["kind"] == "depends_on"), None)
    assert depends_rel is not None
    assert "engine" in depends_rel["subject"]
    assert "database" in depends_rel["object"]
    
    # Search for the "part of" relation
    part_rel = next((r for r in relations if r["kind"] == "part_of"), None)
    assert part_rel is not None
    assert "storage" in part_rel["subject"]
    assert "system" in part_rel["object"]


def test_relations_empty_text(client):
    """Test relations with empty text handles gracefully."""
    resp = client.post(
        "/v1/chat/completions",
        json={
            "model": "relations:default",
            "messages": [{"role": "user", "content": ""}],
        },
    )

    assert resp.status_code == 200
    data = resp.json()
    content = data["choices"][0]["message"]["content"]
    assert content == "[]"
