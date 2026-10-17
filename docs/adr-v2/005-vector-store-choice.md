# ADR 005: Vector Store Choice

Status: proposed

## Problem

Semantic retrieval needs a local, rebuildable vector store with reproducible model/version
semantics.

## Current state

LanceDB is integrated behind optional availability. Embeddings can fail independently, and schema,
model, dimension, normalization, and index-version provenance are not a complete contract.

## Alternatives

1. retain LanceDB;
2. SQLite vector extension;
3. embedded HNSW library/files;
4. local Qdrant/service;
5. brute force for small vaults.

## Measurements required

Recall@k against brute force, latency, ingest/update/delete rate, disk/RSS, startup, compaction,
filter correctness, crash recovery, rebuild, model migration, and package/signing burden.

## Security implications

Native extensions/services expand supply-chain and process surface. Vectors can reveal content and
must obey namespace/sealed deletion.

## Privacy implications

Embedding generation is local by default; cloud embeddings need explicit disclosure. Store model
and source-version provenance.

## Migration implications

Add vector port/outbox, model registry, dimension/version validation, and rebuild tooling. Derived
vectors never enter canonical backup requirements.

## Chosen direction

Retain LanceDB for the first slice behind the vector port and benchmark it against brute force plus
one SQLite/embedded candidate. Use exact brute force below a measured corpus threshold if simpler.

## Rejected alternatives

A network service is unjustified for a personal local product. Immediate replacement lacks data.
Permanent multi-store support increases drift.

## Reversal path

Delete vector state and rebuild another adapter from canonical evidence with the recorded embedding
model.

## Acceptance criteria

Quality/performance budgets, deterministic model/version metadata, namespace filters, deletion and
sealed-lock tests, rebuild from empty, and no silent cloud embedding fallback.
