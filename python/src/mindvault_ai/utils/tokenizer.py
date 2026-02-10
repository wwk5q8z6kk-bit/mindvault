"""Token counting and text chunking utilities."""

from __future__ import annotations


def estimate_tokens(text: str) -> int:
    """Rough token estimate: ~4 chars per token for English text."""
    return max(1, len(text) // 4)


def chunk_text(text: str, max_tokens: int = 512, overlap_tokens: int = 64) -> list[str]:
    """Split text into chunks respecting approximate token limits.

    Uses sentence boundaries where possible, falls back to word boundaries.
    """
    if estimate_tokens(text) <= max_tokens:
        return [text]

    # Approximate char limits
    max_chars = max_tokens * 4
    overlap_chars = overlap_tokens * 4
    sentences = _split_sentences(text)

    chunks: list[str] = []
    current: list[str] = []
    current_len = 0

    for sentence in sentences:
        slen = len(sentence)
        if current_len + slen > max_chars and current:
            chunks.append(" ".join(current))
            # Keep overlap from end of current chunk
            overlap: list[str] = []
            overlap_len = 0
            for s in reversed(current):
                if overlap_len + len(s) > overlap_chars:
                    break
                overlap.insert(0, s)
                overlap_len += len(s)
            current = overlap
            current_len = overlap_len

        current.append(sentence)
        current_len += slen

    if current:
        chunks.append(" ".join(current))

    return chunks


def _split_sentences(text: str) -> list[str]:
    """Basic sentence splitting on period/question/exclamation followed by space."""
    import re

    parts = re.split(r"(?<=[.!?])\s+", text)
    return [p.strip() for p in parts if p.strip()]
