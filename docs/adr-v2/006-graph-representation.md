# ADR 006: Graph Representation

Status: proposed

## Problem

Represent provenance, citations, human links, inferred relations, and traversal without making an
opaque graph store canonical.

## Current state

Relationships are normalized SQLite rows and queried by a SQLite graph store. Obsidian links are
resolved by title/stem; unresolved links are silently skipped.

## Alternatives

1. normalized SQLite edges;
2. graph database;
3. in-memory graph rebuilt at startup;
4. graph projection derived from typed provenance/assertion tables.

## Measurements required

Depth/degree traversal latency, recursive-CTE limits, high-degree memory, update throughput,
authorization filtering, rebuild time, and correctness on cyclic/adversarial graphs.

## Security implications

Traversal can cross authorization boundaries; every expansion must filter. Inferred edges must not
masquerade as human facts.

## Privacy implications

Link structure reveals sensitive associations even without content and follows the same access and
export policy.

## Migration implications

Type existing edges as human/imported/inferred/provenance, retain IDs, attach evidence/version, and
project traversal indexes from canonical facts.

## Chosen direction

Keep typed canonical relationship/provenance facts in SQLite. Use recursive queries or a
rebuildable in-process projection selected by benchmark; AI-inferred edges remain proposals or
derived candidates.

## Rejected alternatives

A graph database adds operations without a measured need. In-memory-only state loses authority.
Untyped edges cannot represent provenance or authority.

## Reversal path

Rebuild any traversal projection from canonical edge/version rows; export edges in open JSONL.

## Acceptance criteria

Exact traversal fixtures, authorization at every hop, evidence and origin for every edge,
deterministic rebuild, and S/M graph performance budgets pass.
