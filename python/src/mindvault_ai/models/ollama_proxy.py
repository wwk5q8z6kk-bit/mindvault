"""Proxy requests to a local Ollama instance, reformatting as OpenAI-compatible."""

from __future__ import annotations

from typing import Any

import httpx
import structlog

logger = structlog.get_logger()


class OllamaProxy:
    """Forwards chat completion requests to Ollama's OpenAI-compatible API."""

    def __init__(self, base_url: str = "http://localhost:11434"):
        self.base_url = base_url.rstrip("/")
        self._client: httpx.AsyncClient | None = None

    async def _get_client(self) -> httpx.AsyncClient:
        if self._client is None or self._client.is_closed:
            self._client = httpx.AsyncClient(base_url=self.base_url, timeout=60.0)
        return self._client

    async def chat_completion(
        self,
        model: str,
        messages: list[dict[str, str]],
        max_tokens: int = 512,
        temperature: float = 0.3,
        stream: bool = False,
    ) -> dict[str, Any]:
        """Send a chat completion request to Ollama and return OpenAI-formatted response."""
        client = await self._get_client()

        payload = {
            "model": model,
            "messages": messages,
            "options": {
                "num_predict": max_tokens,
                "temperature": temperature,
            },
            "stream": stream,
        }

        logger.debug("ollama_request", model=model, message_count=len(messages))

        try:
            resp = await client.post("/api/chat", json=payload)
            resp.raise_for_status()
            data = resp.json()
        except httpx.ConnectError:
            raise RuntimeError(
                f"Cannot connect to Ollama at {self.base_url}. "
                "Ensure Ollama is running: `ollama serve`"
            )
        except httpx.HTTPStatusError as e:
            raise RuntimeError(f"Ollama returned {e.response.status_code}: {e.response.text}")

        # Convert Ollama response to OpenAI format
        content = data.get("message", {}).get("content", "")
        return _to_openai_response(model, content)

    async def list_models(self) -> list[dict[str, Any]]:
        """List models available in Ollama."""
        try:
            client = await self._get_client()
            resp = await client.get("/api/tags")
            resp.raise_for_status()
            data = resp.json()
            return [
                {"id": f"ollama/{m['name']}", "object": "model", "owned_by": "ollama"}
                for m in data.get("models", [])
            ]
        except httpx.ConnectError:
            logger.warning("ollama_unavailable", base_url=self.base_url)
            return []

    async def is_available(self) -> bool:
        """Check if Ollama is reachable."""
        try:
            client = await self._get_client()
            resp = await client.get("/")
            return resp.status_code == 200
        except (httpx.ConnectError, httpx.TimeoutException):
            return False

    async def close(self) -> None:
        if self._client and not self._client.is_closed:
            await self._client.aclose()


def _to_openai_response(model: str, content: str) -> dict[str, Any]:
    """Format a completion as an OpenAI chat completion response."""
    import time

    return {
        "id": f"chatcmpl-mv-{int(time.time())}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": model,
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": content},
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        },
    }
