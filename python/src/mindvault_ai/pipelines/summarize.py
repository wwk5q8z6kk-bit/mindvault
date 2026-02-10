"""Summarization pipeline — extractive (sumy) + abstractive (LLM)."""

from __future__ import annotations

import json
import time
from typing import Any

import structlog

from mindvault_ai.config import AppConfig
from mindvault_ai.models.ollama_proxy import OllamaProxy

logger = structlog.get_logger()


async def run_summarization(
    messages: list[dict[str, str]],
    variant: str,
    config: AppConfig,
    ollama: OllamaProxy,
) -> dict[str, Any]:
    """Run summarization on the last user message.

    Variants:
      - extractive: uses sumy (TextRank/LSA) — no LLM needed
      - abstractive: uses Ollama LLM to generate a summary
    """
    text = ""
    for msg in reversed(messages):
        if msg["role"] == "user":
            text = msg["content"]
            break

    if not text:
        return _make_response(f"summarize:{variant}", "No text provided for summarization.")

    if variant == "extractive":
        summary = _extractive_summarize(
            text,
            method=config.summarization.extractive_method,
            sentence_count=config.summarization.extractive_sentences,
        )
        result = {"type": "extractive", "summary": summary, "method": config.summarization.extractive_method}
        return _make_response(f"summarize:{variant}", json.dumps(result))

    elif variant == "abstractive":
        summary = await _abstractive_summarize(text, config, ollama)
        result = {"type": "abstractive", "summary": summary}
        return _make_response(f"summarize:{variant}", json.dumps(result))

    else:
        return _make_response(f"summarize:{variant}", json.dumps({"error": f"Unknown variant: {variant}"}))


def _extractive_summarize(text: str, method: str = "textrank", sentence_count: int = 5) -> str:
    """Extract key sentences using sumy."""
    from sumy.parsers.plaintext import PlaintextParser
    from sumy.nlp.tokenizers import Tokenizer
    from sumy.nlp.stemmers import Stemmer

    language = "english"
    stemmer = Stemmer(language)

    if method == "lsa":
        from sumy.summarizers.lsa import LsaSummarizer as Summarizer
    else:
        from sumy.summarizers.text_rank import TextRankSummarizer as Summarizer

    parser = PlaintextParser.from_string(text, Tokenizer(language))
    summarizer = Summarizer(stemmer)

    sentences = summarizer(parser.document, sentence_count)
    return " ".join(str(s) for s in sentences)


async def _abstractive_summarize(text: str, config: AppConfig, ollama: OllamaProxy) -> str:
    """Generate an abstractive summary using the LLM."""
    messages = [
        {
            "role": "system",
            "content": "You are a concise summarizer. Provide a clear, accurate summary of the given text. "
            "Focus on the key points and main ideas. Keep the summary to 2-4 sentences.",
        },
        {
            "role": "user",
            "content": f"Please summarize the following text:\n\n{text}",
        },
    ]

    result = await ollama.chat_completion(
        model=config.llm.ollama_model,
        messages=messages,
        max_tokens=config.llm.max_tokens,
        temperature=0.2,  # Low temperature for factual summaries
    )

    # Extract the summary text from the response
    choices = result.get("choices", [])
    if choices:
        return choices[0].get("message", {}).get("content", "")
    return ""


def _make_response(model: str, content: str) -> dict[str, Any]:
    return {
        "id": f"chatcmpl-mv-{model.replace(':', '-')}-{int(time.time())}",
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
