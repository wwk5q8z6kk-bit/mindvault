# ADR 010: Synchronization

Status: proposed

## Problem

Synchronize devices without silent last-write-wins corruption, index drift, or loss of human source
authority.

## Current state

Snapshots carry nodes and vector clocks, but device ID/clock/status are in memory, export is capped,
and imports write SQLite directly without index maintenance. Some conflicts become proposals.

## Alternatives

1. ship current snapshots;
2. encrypted canonical event/version replication;
3. filesystem sync only;
4. hosted canonical server;
5. defer sync.

## Measurements required

convergence, conflict precision, replay/rollback resistance, throughput/bytes, long-offline peers,
clock skew, deletion, attachment transfer, and recovery under interruption.

## Security implications

Peers require durable identities, authenticated encrypted batches, replay protection, least
privilege, and compromised-device revocation.

## Privacy implications

Users select sources/namespaces/devices; relay storage sees minimum metadata and never unencrypted
content.

## Migration implications

Quarantine current sync. First establish immutable versions, tombstones, outbox/audit, and stable
device keys. Later replicate signed version events/evidence, not derived indexes.

## Chosen direction

Defer production sync beyond the first slice. Design eventual peer sync around encrypted,
authenticated canonical versions and proposal-based conflict resolution.

## Rejected alternatives

Current snapshot sync violates consistency. Filesystem sync cannot safely own SQLite state. A
hosted canonical server violates local authority.

## Reversal path

Disable peer grants and retain local canonical history; replicated events remain exportable and
auditable.

## Acceptance criteria

Deterministic convergence suite, no direct derived-index transfer, replay/rollback tests, stable
device identity, conflict proposal semantics, selective scope, and full offline operation.
