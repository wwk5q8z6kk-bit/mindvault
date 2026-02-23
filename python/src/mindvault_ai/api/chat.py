"""POST /v1/chat/completions — model-name routing to pipelines and Ollama."""

from __future__ import annotations

import time
from typing import Any

from fastapi import APIRouter, Request, HTTPException
from pydantic import BaseModel

from mindvault_ai.utils.metrics import REQUEST_COUNT, REQUEST_LATENCY

router = APIRouter()


class ChatMessage(BaseModel):
    role: str
    content: str


class ChatCompletionRequest(BaseModel):
    """Matches the Rust ChatCompletionRequest (llm.rs:121-128)."""

    model: str
    messages: list[ChatMessage]
    max_tokens: int | None = None
    temperature: float | None = None


@router.post("/v1/chat/completions")
async def chat_completions(body: ChatCompletionRequest, request: Request) -> dict[str, Any]:
    """Route chat completions by model name to the appropriate pipeline.

    Model routing:
      - rag:*           → RAG pipeline
      - classify:*      → Classification pipeline
      - summarize:*     → Summarization pipeline
      - ner:*           → NER pipeline
      - ollama/<model>  → Direct Ollama proxy
      - mindvault-default → Ollama with configured default model
    """
    start = time.monotonic()
    config = request.app.state.config
    model = body.model
    messages = [{"role": m.role, "content": m.content} for m in body.messages]
    max_tokens = body.max_tokens or config.llm.max_tokens
    temperature = body.temperature if body.temperature is not None else config.llm.temperature

    try:
        if model.startswith("rag:"):
            from mindvault_ai.pipelines.rag import run_rag

            variant = model.split(":", 1)[1] if ":" in model else "default"
            result = await run_rag(
                messages=messages,
                variant=variant,
                config=config,
                max_tokens=max_tokens,
                temperature=temperature,
                ollama=request.app.state.ollama,
            )
        elif model.startswith("classify:"):
            from mindvault_ai.pipelines.classify import run_classification

            variant = model.split(":", 1)[1]
            result = await run_classification(
                messages=messages,
                variant=variant,
                nlp=request.app.state.spacy_nlp,
            )
        elif model.startswith("summarize:"):
            from mindvault_ai.pipelines.summarize import run_summarization

            variant = model.split(":", 1)[1]
            result = await run_summarization(
                messages=messages,
                variant=variant,
                config=config,
                ollama=request.app.state.ollama,
            )
        elif model.startswith("ner:"):
            from mindvault_ai.pipelines.ner import run_ner

            result = await run_ner(
                messages=messages,
                nlp=request.app.state.spacy_nlp,
            )
        elif model.startswith("relations:"):
            from mindvault_ai.pipelines.relations import run_relations

            result = await run_relations(
                messages=messages,
                nlp=request.app.state.spacy_nlp,
            )
        elif model.startswith("ollama/"):
            ollama_model = model[len("ollama/"):]
            result = await request.app.state.ollama.chat_completion(
                model=ollama_model,
                messages=messages,
                max_tokens=max_tokens,
                temperature=temperature,
            )
        elif model == "mindvault-default":
            result = await request.app.state.ollama.chat_completion(
                model=config.llm.ollama_model,
                messages=messages,
                max_tokens=max_tokens,
                temperature=temperature,
            )
        else:
            raise HTTPException(
                status_code=400,
                detail=f"Unknown model: {model!r}. Use rag:*, classify:*, summarize:*, ner:*, ollama/<model>, or mindvault-default",
            )

        elapsed = time.monotonic() - start
        REQUEST_COUNT.labels(endpoint="/v1/chat/completions", status="ok").inc()
        REQUEST_LATENCY.labels(endpoint="/v1/chat/completions").observe(elapsed)
        return result

    except HTTPException:
        raise
    except Exception as e:
        elapsed = time.monotonic() - start
        REQUEST_COUNT.labels(endpoint="/v1/chat/completions", status="error").inc()
        REQUEST_LATENCY.labels(endpoint="/v1/chat/completions").observe(elapsed)
        raise HTTPException(status_code=500, detail=str(e))
