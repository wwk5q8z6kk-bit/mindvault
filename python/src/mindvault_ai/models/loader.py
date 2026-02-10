"""Model loading utilities for sentence-transformers, spaCy, and ONNX."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import structlog

logger = structlog.get_logger()

# Estimated memory usage per model (bytes)
MODEL_MEMORY_ESTIMATES: dict[str, int] = {
    "all-MiniLM-L6-v2": 90 * 1024 * 1024,
    "en_core_web_sm": 12 * 1024 * 1024,
}


def load_sentence_transformer(model_name: str, cache_dir: str | None = None) -> Any:
    """Load a sentence-transformers model."""
    from sentence_transformers import SentenceTransformer

    kwargs: dict[str, Any] = {}
    if cache_dir:
        expanded = str(Path(cache_dir).expanduser())
        kwargs["cache_folder"] = expanded

    logger.info("loading_sentence_transformer", model=model_name)
    return SentenceTransformer(model_name, **kwargs)


def load_spacy_model(model_name: str) -> Any:
    """Load a spaCy model, downloading if needed."""
    import spacy

    try:
        return spacy.load(model_name)
    except OSError:
        logger.info("downloading_spacy_model", model=model_name)
        spacy.cli.download(model_name)  # type: ignore[attr-defined]
        return spacy.load(model_name)


def get_memory_estimate(model_name: str) -> int:
    """Return estimated memory in bytes for a known model, or a default."""
    return MODEL_MEMORY_ESTIMATES.get(model_name, 50 * 1024 * 1024)
