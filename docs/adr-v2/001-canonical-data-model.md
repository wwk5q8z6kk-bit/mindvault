# ADR 001: Canonical Data Model

Status: proposed

## Problem

The node model mixes human content, derived extraction, operational metadata, and AI enrichment,
without universal evidence, bitemporal version, or mutation authority.

## Current state

`KnowledgeNode` rows are SQLite authority with temporal fields; attachments and extracted text live
in metadata/files; proposals and relationships are separate. Multiple writers bypass one version
boundary.

## Alternatives

1. keep and extend the node table;
2. event-source every field;
3. normalize source, evidence, document/version, assertion, proposal, decision, audit, and outbox;
4. use a document database.

## Measurements required

Migration size/time, transaction latency, query joins, export size, index rebuild rate, and recovery
from interrupted writes on S/M/L fixtures.

## Security implications

Typed authority and immutable evidence reduce confused-deputy writes. More tables require strict
foreign keys and one mutation capability.

## Privacy implications

Evidence and provenance make disclosure scope visible, but increase sensitive metadata; access and
retention apply at source/evidence/version level.

## Migration implications

Add tables, preserve legacy IDs, copy attachments into evidence without moving originals, and map
nodes to initial document/assertion versions. Retain a legacy read/export adapter.

## Chosen direction

Use normalized source, evidence object, document, document version, memory assertion, proposal,
decision, audit event, outbox event, and tombstone entities. Record valid time and transaction time
where facts can change.

## Rejected alternatives

Extending only `knowledge_nodes` preserves ambiguity. Full event sourcing adds operational
complexity before it is justified. A document database weakens the SQLite transaction/portability
goal.

## Reversal path

Materialize v2 records into the versioned open export and legacy-node projection; keep original
legacy tables/data root unchanged during adoption.

## Acceptance criteria

Every canonical field has an owner and type; every mutation yields a version, provenance, audit,
and outbox record atomically; foreign-key/integrity checks pass; export/restore is lossless.
