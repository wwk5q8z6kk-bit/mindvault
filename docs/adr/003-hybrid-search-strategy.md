# ADR-003: Hybrid Search Strategy

## Status
Accepted

## Context
Knowledge retrieval requires both semantic understanding (finding conceptually related content) and exact matching (finding specific terms, names, identifiers).

## Decision
Implement a **hybrid search** combining three strategies:

1. **Vector search** (LanceDB) — semantic similarity via embeddings
2. **Full-text search** (Tantivy) — BM25 keyword matching
3. **Graph traversal** (petgraph) — relationship-based discovery

Results are merged using **Reciprocal Rank Fusion (RRF)** with configurable weights:
- `vector_weight`: 0.6 (default)
- `fulltext_weight`: 0.4 (default)
- `rrf_k`: 60.0

The `SearchStrategy` enum allows callers to select: `Hybrid`, `Vector`, `FullText`, or `Graph`.

## Consequences
- **Positive:** Handles both vague semantic queries and precise keyword lookups.
- **Positive:** Graph traversal discovers connections not captured by content similarity.
- **Positive:** RRF is simple, robust, and doesn't require score normalization.
- **Negative:** Three search backends increase storage and indexing overhead.
- **Negative:** Embedding quality depends on the chosen provider/model.
