"""Classification pipeline — topic, sentiment, and intent classification."""

from __future__ import annotations

import json
import time
from typing import Any

import structlog

logger = structlog.get_logger()

# Simple lexicon-based sentiment
_POSITIVE_WORDS = frozenset([
    "good", "great", "excellent", "amazing", "wonderful", "fantastic", "love",
    "happy", "joy", "positive", "best", "perfect", "nice", "awesome", "brilliant",
    "pleased", "glad", "delighted", "superb", "outstanding", "beautiful",
])
_NEGATIVE_WORDS = frozenset([
    "bad", "terrible", "awful", "horrible", "hate", "sad", "angry", "worst",
    "poor", "negative", "ugly", "disappointing", "frustrated", "annoyed",
    "disgusting", "dreadful", "miserable", "pathetic", "useless", "broken",
])

# Topic keywords mapping
_TOPIC_KEYWORDS: dict[str, list[str]] = {
    "technology": ["software", "code", "programming", "computer", "api", "server", "database", "algorithm"],
    "science": ["research", "experiment", "hypothesis", "data", "analysis", "study", "theory"],
    "business": ["market", "revenue", "profit", "company", "strategy", "customer", "sales"],
    "health": ["medical", "health", "doctor", "treatment", "symptom", "disease", "wellness"],
    "education": ["learning", "school", "student", "teacher", "course", "study", "curriculum"],
    "general": [],
}


async def run_classification(
    messages: list[dict[str, str]],
    variant: str,
    nlp: Any,
) -> dict[str, Any]:
    """Run classification pipeline on the last user message.

    Variants: topic, sentiment, intent
    """
    text = _extract_user_text(messages)

    if variant == "sentiment":
        result = _classify_sentiment(text)
    elif variant == "topic":
        result = _classify_topic(text, nlp)
    elif variant == "intent":
        result = _classify_intent(text, nlp)
    else:
        result = {"error": f"Unknown classification variant: {variant}"}

    return _make_response(f"classify:{variant}", json.dumps(result))


def _classify_sentiment(text: str) -> dict[str, Any]:
    """Lexicon-based sentiment analysis."""
    words = set(text.lower().split())
    pos_count = len(words & _POSITIVE_WORDS)
    neg_count = len(words & _NEGATIVE_WORDS)
    total = pos_count + neg_count

    if total == 0:
        label, score = "neutral", 0.5
    elif pos_count > neg_count:
        label = "positive"
        score = pos_count / total
    elif neg_count > pos_count:
        label = "negative"
        score = neg_count / total
    else:
        label, score = "mixed", 0.5

    return {
        "type": "sentiment",
        "label": label,
        "score": round(score, 3),
        "positive_count": pos_count,
        "negative_count": neg_count,
    }


def _classify_topic(text: str, nlp: Any) -> dict[str, Any]:
    """Keyword + NER-based topic classification."""
    text_lower = text.lower()
    scores: dict[str, float] = {}

    for topic, keywords in _TOPIC_KEYWORDS.items():
        if not keywords:
            continue
        count = sum(1 for kw in keywords if kw in text_lower)
        if count > 0:
            scores[topic] = count / len(keywords)

    if not scores:
        # Use spaCy NER as fallback signal
        doc = nlp(text)
        entity_labels = {ent.label_ for ent in doc.ents}
        if entity_labels & {"ORG", "PRODUCT"}:
            scores["business"] = 0.4
        if entity_labels & {"PERSON", "GPE", "LOC"}:
            scores["general"] = 0.3
        if not scores:
            scores["general"] = 0.5

    # Sort by score descending
    sorted_topics = sorted(scores.items(), key=lambda x: x[1], reverse=True)
    return {
        "type": "topic",
        "primary_topic": sorted_topics[0][0],
        "topics": [{"topic": t, "score": round(s, 3)} for t, s in sorted_topics],
    }


def _classify_intent(text: str, nlp: Any) -> dict[str, Any]:
    """Simple intent classification based on sentence structure."""
    text_stripped = text.strip()
    doc = nlp(text_stripped)

    # Basic heuristics
    if text_stripped.endswith("?"):
        intent = "question"
    elif any(token.pos_ == "VERB" and token.dep_ == "ROOT" and token.i == 0 for token in doc):
        intent = "command"
    elif len(text_stripped.split()) < 5:
        intent = "keyword_search"
    else:
        intent = "statement"

    return {
        "type": "intent",
        "intent": intent,
        "confidence": 0.7,
    }


def _extract_user_text(messages: list[dict[str, str]]) -> str:
    for msg in reversed(messages):
        if msg["role"] == "user":
            return msg["content"]
    return ""


def _make_response(model: str, content: str) -> dict[str, Any]:
    return {
        "id": f"chatcmpl-mv-{model}-{int(time.time())}",
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
