"""Configuration loading: defaults from python/config.toml, user overrides from ~/.mindvault/ai-config.toml."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

from pydantic import BaseModel


class ServerConfig(BaseModel):
    host: str = "127.0.0.1"
    port: int = 8100


class ModelsConfig(BaseModel):
    memory_budget_mb: int = 4096
    cache_dir: str = "~/.mindvault/models"


class EmbeddingConfig(BaseModel):
    model: str = "all-MiniLM-L6-v2"
    dimensions: int = 384


class LlmConfig(BaseModel):
    backend: str = "ollama"
    ollama_base_url: str = "http://localhost:11434"
    ollama_model: str = "llama3.2:3b"
    remote_base_url: str = ""
    remote_model: str = "gpt-4o-mini"
    max_tokens: int = 512
    temperature: float = 0.3


class RagConfig(BaseModel):
    mindvault_base_url: str = "http://127.0.0.1:9470"
    strategy: str = "hybrid"
    top_k: int = 6


class ClassificationConfig(BaseModel):
    spacy_model: str = "en_core_web_sm"


class SummarizationConfig(BaseModel):
    extractive_method: str = "textrank"
    extractive_sentences: int = 5


class AppConfig(BaseModel):
    server: ServerConfig = ServerConfig()
    models: ModelsConfig = ModelsConfig()
    embedding: EmbeddingConfig = EmbeddingConfig()
    llm: LlmConfig = LlmConfig()
    rag: RagConfig = RagConfig()
    classification: ClassificationConfig = ClassificationConfig()
    summarization: SummarizationConfig = SummarizationConfig()


def _deep_merge(base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
    """Recursively merge override into base, returning a new dict."""
    result = dict(base)
    for key, val in override.items():
        if key in result and isinstance(result[key], dict) and isinstance(val, dict):
            result[key] = _deep_merge(result[key], val)
        else:
            result[key] = val
    return result


def load_config() -> AppConfig:
    """Load config from defaults + user overrides."""
    # Default config bundled with the package
    default_path = Path(__file__).parent.parent.parent / "config.toml"
    data: dict[str, Any] = {}
    if default_path.exists():
        with open(default_path, "rb") as f:
            data = tomllib.load(f)

    # User overrides
    user_path = Path.home() / ".mindvault" / "ai-config.toml"
    if user_path.exists():
        with open(user_path, "rb") as f:
            user_data = tomllib.load(f)
        data = _deep_merge(data, user_data)

    return AppConfig(**data)
