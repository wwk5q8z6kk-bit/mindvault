"""Contract tests verifying OpenAI-compatible response schemas.

These verify that responses match the structures expected by the Rust server:
- ChatCompletionResponse (llm.rs:130-143)
- EmbedResponse (vector.rs:572-580)
"""

from __future__ import annotations


class TestChatCompletionContract:
    """Verify chat completion responses match Rust's ChatCompletionResponse."""

    def test_response_has_choices(self, client):
        resp = client.post("/v1/chat/completions", json={
            "model": "mindvault-default",
            "messages": [{"role": "user", "content": "test"}],
        })
        data = resp.json()
        assert "choices" in data
        assert isinstance(data["choices"], list)
        assert len(data["choices"]) >= 1

    def test_choice_has_message(self, client):
        resp = client.post("/v1/chat/completions", json={
            "model": "mindvault-default",
            "messages": [{"role": "user", "content": "test"}],
        })
        choice = resp.json()["choices"][0]
        assert "message" in choice
        msg = choice["message"]
        assert "content" in msg
        assert isinstance(msg["content"], str) or msg["content"] is None

    def test_response_has_standard_fields(self, client):
        """Verify id, object, created, model fields."""
        resp = client.post("/v1/chat/completions", json={
            "model": "mindvault-default",
            "messages": [{"role": "user", "content": "test"}],
        })
        data = resp.json()
        assert "id" in data
        assert data["object"] == "chat.completion"
        assert "created" in data
        assert "model" in data


class TestEmbeddingContract:
    """Verify embedding responses match Rust's EmbedResponse."""

    def test_response_has_data(self, client):
        resp = client.post("/v1/embeddings", json={
            "model": "all-MiniLM-L6-v2",
            "input": "test",
        })
        data = resp.json()
        assert "data" in data
        assert isinstance(data["data"], list)

    def test_data_item_has_embedding(self, client):
        resp = client.post("/v1/embeddings", json={
            "model": "all-MiniLM-L6-v2",
            "input": "test",
        })
        item = resp.json()["data"][0]
        assert "embedding" in item
        assert isinstance(item["embedding"], list)
        assert all(isinstance(v, (int, float)) for v in item["embedding"])

    def test_embedding_dimensions(self, client):
        """Verify embeddings are 384-dimensional (all-MiniLM-L6-v2)."""
        resp = client.post("/v1/embeddings", json={
            "model": "all-MiniLM-L6-v2",
            "input": "test",
        })
        embedding = resp.json()["data"][0]["embedding"]
        assert len(embedding) == 384

    def test_batch_preserves_order(self, client):
        """Verify batch embeddings maintain input order via index field."""
        resp = client.post("/v1/embeddings", json={
            "model": "all-MiniLM-L6-v2",
            "input": ["first", "second", "third"],
        })
        data = resp.json()["data"]
        assert [d["index"] for d in data] == [0, 1, 2]


class TestModelsContract:
    """Verify /v1/models response matches OpenAI list format."""

    def test_models_list_format(self, client):
        resp = client.get("/v1/models")
        data = resp.json()
        assert data["object"] == "list"
        assert isinstance(data["data"], list)
        assert len(data["data"]) > 0

    def test_model_item_has_id(self, client):
        resp = client.get("/v1/models")
        for item in resp.json()["data"]:
            assert "id" in item
            assert "object" in item
