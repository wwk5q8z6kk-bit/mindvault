# ADR-006: Embedding Provider Abstraction

## Status
Accepted

## Context
Vector search requires text embeddings. Different users have different preferences: local models (privacy), OpenAI (quality), or custom endpoints. The system must support multiple providers without coupling to any one.

## Decision
Define an `Embedder` trait in `mv-core`:
```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> MvResult<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> MvResult<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}
```

Implementations:
- **LocalEmbedder** — uses ONNX Runtime for local models (bge-small-en-v1.5, 384d)
- **OpenAIEmbedder** — calls OpenAI-compatible API (text-embedding-3-small, 1536d)
- **NoOpEmbedder** — returns zero vectors when embeddings are disabled

Provider selection is configured via `EmbeddingConfig` with runtime auto-detection.

## Consequences
- **Positive:** Users choose between privacy (local) and quality (cloud).
- **Positive:** New providers can be added by implementing one trait.
- **Positive:** Graceful degradation when embeddings are unavailable.
- **Negative:** Dimension mismatch between providers requires re-indexing.
- **Negative:** Local models require bundling ONNX Runtime (~50MB).
