# ADR 004: FTS5 versus Tantivy

Status: proposed

## Problem

Choose a rebuildable full-text engine that meets retrieval quality and local operational goals.

## Current state

Tantivy is production-integrated but non-transactional with SQLite. No relevance/performance
baseline exists. SQLite FTS5 is not the main path.

## Alternatives

1. retain Tantivy;
2. use FTS5 in canonical SQLite;
3. support both indefinitely;
4. external search service.

## Measurements required

Exact same corpus/judgments: indexing throughput, p50/p95/p99, MRR/nDCG/Recall@k, disk/RSS,
Unicode/tokenizer behavior, crash recovery, incremental updates, rebuild, backup, and operational
complexity.

## Security implications

FTS5 reduces files/process surface but co-locates searchable plaintext. Tantivy adds a separate
derived store whose permissions, lifecycle, and sealed-data deletion must be verified.

## Privacy implications

Both can leak indexed plaintext. Sealed/private scope policy and complete wipe/rebuild tests are
mandatory.

## Migration implications

Introduce a full-text port and canonical outbox. Build an FTS5 comparator without changing
user-visible reads.

## Chosen direction

Retain Tantivy for the first slice behind the port because it is already integrated; run the FTS5
comparison before endorsing it for v2. Switch to FTS5 only if it meets quality budgets and has lower
operational cost without unacceptable canonical-database growth/content exposure.

## Rejected alternatives

An immediate switch lacks evidence. Permanent dual engines double correctness/security burden.
External search violates local simplicity.

## Reversal path

Drop either derived index and rebuild the other from canonical versions/outbox.

## Acceptance criteria

Published benchmark JSON, deterministic rebuild, projection watermark, failure repair, sealed-data
tests, and a recorded selection threshold with no unmeasured claims.
