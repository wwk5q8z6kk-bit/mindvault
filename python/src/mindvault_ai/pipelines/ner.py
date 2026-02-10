"""NER pipeline — spaCy entity extraction with custom entity types."""

from __future__ import annotations

import json
import time
from typing import Any

import structlog

logger = structlog.get_logger()


async def run_ner(
    messages: list[dict[str, str]],
    nlp: Any,
) -> dict[str, Any]:
    """Extract named entities from the last user message using spaCy.

    Returns entities as JSON in the assistant content field, maintaining
    OpenAI chat completion response format.
    """
    text = ""
    for msg in reversed(messages):
        if msg["role"] == "user":
            text = msg["content"]
            break

    if not text:
        return _make_response([])

    doc = nlp(text)

    entities = []
    for ent in doc.ents:
        entities.append({
            "text": ent.text,
            "label": ent.label_,
            "description": _entity_description(ent.label_),
            "start": ent.start_char,
            "end": ent.end_char,
        })

    # Also extract noun chunks as potential custom entities
    noun_chunks = []
    for chunk in doc.noun_chunks:
        # Skip if already covered by a named entity
        is_entity = any(
            chunk.start_char >= e["start"] and chunk.end_char <= e["end"]
            for e in entities
        )
        if not is_entity and len(chunk.text.split()) >= 2:
            noun_chunks.append({
                "text": chunk.text,
                "label": "NOUN_PHRASE",
                "description": "Multi-word noun phrase",
                "start": chunk.start_char,
                "end": chunk.end_char,
            })

    result = {
        "entities": entities,
        "noun_phrases": noun_chunks,
        "total_entities": len(entities),
        "total_noun_phrases": len(noun_chunks),
    }

    return _make_response(result)


def _entity_description(label: str) -> str:
    """Human-readable description for spaCy entity labels."""
    descriptions = {
        "PERSON": "Person name",
        "ORG": "Organization",
        "GPE": "Geopolitical entity (country, city, state)",
        "LOC": "Non-GPE location",
        "DATE": "Date or period",
        "TIME": "Time of day",
        "MONEY": "Monetary value",
        "PERCENT": "Percentage",
        "PRODUCT": "Product name",
        "EVENT": "Named event",
        "WORK_OF_ART": "Title of creative work",
        "LAW": "Named legal document",
        "LANGUAGE": "Named language",
        "FAC": "Facility (building, airport, etc.)",
        "NORP": "Nationalities, religious or political groups",
        "QUANTITY": "Measurement",
        "ORDINAL": "Ordinal number (first, second, etc.)",
        "CARDINAL": "Cardinal number",
    }
    return descriptions.get(label, label)


def _make_response(result: Any) -> dict[str, Any]:
    content = json.dumps(result) if not isinstance(result, str) else result
    return {
        "id": f"chatcmpl-mv-ner-{int(time.time())}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": "ner:default",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": content},
                "finish_reason": "stop",
            }
        ],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }
