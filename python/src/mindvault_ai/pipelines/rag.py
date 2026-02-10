"""RAG pipeline — retrieve from Rust server, augment prompt, generate via Ollama."""

from __future__ import annotations

import json
from typing import Any

import httpx
import structlog

from mindvault_ai.config import AppConfig
from mindvault_ai.models.ollama_proxy import OllamaProxy

logger = structlog.get_logger()

# Retry config for Rust server calls
_MAX_RETRIES = 3
_RETRY_BACKOFF = [0.5, 1.0, 2.0]


async def run_rag(
    messages: list[dict[str, str]],
    variant: str,
    config: AppConfig,
    max_tokens: int,
    temperature: float,
    ollama: OllamaProxy,
) -> dict[str, Any]:
    """Execute RAG pipeline: retrieve context from MindVault, then generate."""
    # Extract the user query from the last user message
    query = ""
    for msg in reversed(messages):
        if msg["role"] == "user":
            query = msg["content"]
            break

    if not query:
        return _make_response(config.llm.ollama_model, "No user message found for RAG query.")

    # Retrieve relevant documents from the Rust server
    context_docs = await _retrieve(query, config)

    # Build augmented prompt
    if context_docs:
        context_text = "\n\n---\n\n".join(
            f"[{doc.get('title', 'Untitled')}]\n{doc.get('body', doc.get('content', ''))}"
            for doc in context_docs
        )
        system_prompt = (
            "You are a helpful assistant with access to the user's knowledge base. "
            "Use the following retrieved context to answer the question. "
            "If the context doesn't contain relevant information, say so.\n\n"
            f"## Retrieved Context\n\n{context_text}"
        )
    else:
        system_prompt = (
            "You are a helpful assistant. No relevant context was found in the knowledge base. "
            "Answer based on your general knowledge and note that no stored documents matched."
        )

    augmented_messages = [{"role": "system", "content": system_prompt}]
    # Include conversation history (skip any existing system messages)
    for msg in messages:
        if msg["role"] != "system":
            augmented_messages.append(msg)

    # Generate via Ollama
    result = await ollama.chat_completion(
        model=config.llm.ollama_model,
        messages=augmented_messages,
        max_tokens=max_tokens,
        temperature=temperature,
    )

    # Annotate with retrieval metadata
    if result.get("choices"):
        result["choices"][0]["metadata"] = {
            "pipeline": "rag",
            "variant": variant,
            "documents_retrieved": len(context_docs),
        }

    return result


async def _retrieve(query: str, config: AppConfig) -> list[dict[str, Any]]:
    """Call the Rust server's search API with retry backoff."""
    url = f"{config.rag.mindvault_base_url}/api/v1/search"
    payload = {
        "query": query,
        "strategy": config.rag.strategy,
        "limit": config.rag.top_k,
    }

    for attempt in range(_MAX_RETRIES):
        try:
            async with httpx.AsyncClient(timeout=10.0) as client:
                resp = await client.post(url, json=payload)
                resp.raise_for_status()
                data = resp.json()
                return data.get("results", data.get("nodes", []))
        except (httpx.ConnectError, httpx.TimeoutException) as e:
            if attempt < _MAX_RETRIES - 1:
                import asyncio

                await asyncio.sleep(_RETRY_BACKOFF[attempt])
                logger.warning(
                    "rag_retrieve_retry",
                    attempt=attempt + 1,
                    error=str(e),
                )
            else:
                logger.error("rag_retrieve_failed", error=str(e))
                return []
        except httpx.HTTPStatusError as e:
            logger.error("rag_retrieve_http_error", status=e.response.status_code)
            return []

    return []


def _make_response(model: str, content: str) -> dict[str, Any]:
    import time

    return {
        "id": f"chatcmpl-mv-rag-{int(time.time())}",
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
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }
