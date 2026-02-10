"""Prometheus metrics for the AI service."""

from __future__ import annotations

from prometheus_client import Counter, Histogram, Gauge

REQUEST_COUNT = Counter(
    "mindvault_ai_requests_total",
    "Total API requests",
    ["endpoint", "status"],
)

REQUEST_LATENCY = Histogram(
    "mindvault_ai_request_duration_seconds",
    "Request latency in seconds",
    ["endpoint"],
    buckets=(0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0),
)

MODELS_LOADED = Gauge(
    "mindvault_ai_models_loaded",
    "Number of models currently loaded",
)

EMBEDDING_BATCH_SIZE = Histogram(
    "mindvault_ai_embedding_batch_size",
    "Number of texts per embedding request",
    buckets=(1, 2, 4, 8, 16, 32, 64, 128),
)

MEMORY_USAGE_BYTES = Gauge(
    "mindvault_ai_memory_usage_bytes",
    "Estimated memory usage of loaded models",
)
