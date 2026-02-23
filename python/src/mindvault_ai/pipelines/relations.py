"""Relation extraction pipeline — spaCy dependency parsing for SVO triplets."""

from __future__ import annotations

import json
import time
from typing import Any

import structlog

logger = structlog.get_logger()

# Map common verbs to mv-core RelationKind
VERB_TO_KIND = {
    "depend": "depends_on",
    "depends": "depends_on",
    "require": "depends_on",
    "requires": "depends_on",
    "need": "depends_on",
    "needs": "depends_on",
    
    "part": "part_of",
    "comprise": "part_of",
    "comprises": "part_of",
    "include": "part_of",
    "includes": "part_of",
    
    "derive": "derived_from",
    "derives": "derived_from",
    "come": "derived_from",
    "comes": "derived_from",
    
    "supersede": "supersedes",
    "supersedes": "supersedes",
    "replace": "supersedes",
    "replaces": "supersedes",
    
    "contradict": "contradicts",
    "contradicts": "contradicts",
    "oppose": "contradicts",
    "opposes": "contradicts",

    "contain": "contains",
    "contains": "contains",
    "hold": "contains",
    "holds": "contains",

    "reference": "references",
    "references": "references",
    "cite": "references",
    "cites": "references",
    "mention": "references",
    "mentions": "references",

    "similar": "similar_to",
    "resemble": "similar_to",
    "resembles": "similar_to",
    "like": "similar_to",

    "follow": "follows_from",
    "follows": "follows_from",
    "after": "follows_from",
}

async def run_relations(
    messages: list[dict[str, str]],
    nlp: Any,
) -> dict[str, Any]:
    """Extract Subject-Verb-Object (SVO) triplets from the last user message using spaCy.

    Returns relations as JSON in the assistant content field, maintaining
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
    triplets = []

    for sent in doc.sents:
        for token in sent:
            # Find verbs
            if token.pos_ == "VERB":
                subj = None
                obj = None
                
                # Check children of the verb
                for child in token.children:
                    if child.dep_ in ("nsubj", "nsubjpass"):
                        # Get full noun phrase if available
                        subj = _get_noun_chunk(child, doc) or child.text
                    elif child.dep_ in ("dobj", "pobj", "attr"):
                        obj = _get_noun_chunk(child, doc) or child.text

                # If no direct object, check for prepositional objects
                if not obj:
                    for child in token.children:
                        if child.dep_ == "prep":
                            for subchild in child.children:
                                if subchild.dep_ == "pobj":
                                    obj = _get_noun_chunk(subchild, doc) or subchild.text
                                    break

                if subj and obj:
                    # Clean up the texts
                    subj = subj.strip().lower()
                    obj = obj.strip().lower()
                    
                    # Lemmatize the verb to check our map
                    verb_lemma = token.lemma_.lower()
                    
                    # If it's a phrase like "is part of", handle the noun predicate
                    if verb_lemma == "be":
                        for child in token.children:
                            if child.dep_ == "attr":
                                verb_lemma = child.lemma_.lower()
                                break
                    
                    kind = VERB_TO_KIND.get(verb_lemma, "relates_to")
                    
                    triplets.append({
                        "subject": subj,
                        "verb": token.text.lower(),
                        "object": obj,
                        "kind": kind,
                    })

    result = {
        "relations": triplets,
        "total": len(triplets),
    }

    return _make_response(result)

def _get_noun_chunk(token: Any, doc: Any) -> str | None:
    """Find the noun chunk containing the given token."""
    for chunk in doc.noun_chunks:
        if chunk.start <= token.i < chunk.end:
            return chunk.text
    return None

def _make_response(result: Any) -> dict[str, Any]:
    content = json.dumps(result) if not isinstance(result, str) else result
    return {
        "id": f"chatcmpl-mv-rel-{int(time.time())}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": "relations:default",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": content},
                "finish_reason": "stop",
            }
        ],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }
