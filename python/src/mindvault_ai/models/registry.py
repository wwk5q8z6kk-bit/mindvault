"""Model registry — manages lifecycle of loaded models (load/unload/evict)."""

from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field
from typing import Any

import structlog

from mindvault_ai.utils.metrics import MODELS_LOADED, MEMORY_USAGE_BYTES

logger = structlog.get_logger()


@dataclass
class ModelEntry:
    name: str
    model: Any
    memory_bytes: int = 0
    loaded_at: float = field(default_factory=time.time)
    last_used: float = field(default_factory=time.time)


class ModelRegistry:
    """Thread-safe model registry with memory budget enforcement."""

    def __init__(self, memory_budget_mb: int = 4096, cache_dir: str | None = None):
        self._models: dict[str, ModelEntry] = {}
        self._lock = threading.Lock()
        self._memory_budget = memory_budget_mb * 1024 * 1024
        self.cache_dir = cache_dir

    def get(self, name: str) -> Any | None:
        with self._lock:
            entry = self._models.get(name)
            if entry:
                entry.last_used = time.time()
                return entry.model
        return None

    def put(self, name: str, model: Any, memory_bytes: int = 0) -> None:
        with self._lock:
            self._evict_if_needed(memory_bytes)
            self._models[name] = ModelEntry(
                name=name, model=model, memory_bytes=memory_bytes
            )
            self._update_metrics()
            logger.info("model_loaded", name=name, memory_mb=memory_bytes // (1024 * 1024))

    def remove(self, name: str) -> bool:
        with self._lock:
            if name in self._models:
                del self._models[name]
                self._update_metrics()
                logger.info("model_unloaded", name=name)
                return True
        return False

    def list_models(self) -> list[dict[str, Any]]:
        with self._lock:
            return [
                {
                    "name": e.name,
                    "memory_mb": e.memory_bytes // (1024 * 1024),
                    "loaded_at": e.loaded_at,
                    "last_used": e.last_used,
                }
                for e in self._models.values()
            ]

    @property
    def total_memory(self) -> int:
        return sum(e.memory_bytes for e in self._models.values())

    def _evict_if_needed(self, needed: int) -> None:
        """Evict least-recently-used models until there's room."""
        while self.total_memory + needed > self._memory_budget and self._models:
            lru_name = min(self._models, key=lambda k: self._models[k].last_used)
            logger.info("model_evicted", name=lru_name, reason="memory_pressure")
            del self._models[lru_name]

    def _update_metrics(self) -> None:
        MODELS_LOADED.set(len(self._models))
        MEMORY_USAGE_BYTES.set(self.total_memory)
