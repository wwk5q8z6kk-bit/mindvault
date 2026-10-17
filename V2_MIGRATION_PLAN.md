# V2 Migration Plan

## Strategy

Use expand → verify → switch → retain → contract. Never dual-write two canonical systems and never
delete legacy data during the first migration release.

## Stage 0: freeze and protect

- preserve exact public commit and dirty worktree backup;
- make legacy CI lanes report independently;
- rotate exposed credentials;
- complete licensing/vulnerability blockers;
- produce and rehearse a complete legacy backup.

Exit: two verified backups, clean isolated migration fixture, no unknown canonical locations.

## Stage 1: contracts and fixtures

- ratify Phase 1 documents and ADRs;
- define v2 schema, proposal envelope, citation ledger, export manifest, and source registry;
- create synthetic legacy fixtures for each schema version and failure state;
- establish benchmark and accessibility/security test corpora.

Exit: schema/API contracts reviewed; no implementation ambiguity on authority or timestamps.

## Stage 2: additive canonical tables

- add migration ledger with IDs/checksums;
- add source, evidence, document/version, audit, outbox, and projection-watermark tables;
- preserve existing node IDs and rows;
- import legacy attachments into content-addressed evidence by copy, never move;
- register legacy SQLite nodes as migration source versions.

Exit: integrity/foreign-key checks pass; every new row traces to a legacy row/file/hash.

## Stage 3: projection and read comparison

- build exact/FTS/vector/graph adapters behind common ports;
- replay outbox or full rebuild from canonical snapshot;
- run legacy and v2 reads against fixed queries;
- record discrepancies without changing user-visible results.

Exit: correctness/relevance thresholds met; rebuild from empty derived stores passes.

## Stage 4: first vertical slice

Implement only [FIRST_V2_VERTICAL_SLICE.md](FIRST_V2_VERTICAL_SLICE.md) behind an opt-in flag using
a copied fixture vault. Execute failure injection, threat, privacy, accessibility, performance,
migration, backup, and rollback tests.

Exit: every slice acceptance criterion passes and a user can reverse to the untouched legacy data.

## Stage 5: read switch

- select v2 read path for the slice;
- retain legacy tables/read adapter;
- expose projection watermark and repair status;
- monitor correctness, performance, and privacy events.

Exit: declared soak period with no blocker and verified export/restore.

## Stage 6: write switch

- route human edits/imports through canonical transaction service;
- revoke direct-store capabilities from AI, sync, connectors, and intents;
- enforce proposal-only AI writes at compile-time/module boundaries and runtime policy.

Exit: negative tests prove bypasses fail; audit/outbox/rollback are atomic.

## Stage 7: optional module migration

Migrate modules individually by value and evidence. Tasks/goals/habits/calendar/relay/public
sharing/federation/messaging are not prerequisites for core v2.

## Stage 8: contract legacy paths

Only after a supported retention window and explicit owner approval:

- archive legacy tables/formats;
- keep a read/export tool;
- remove duplicate transports/direct writers;
- never remove evidence or backups solely because v2 is active.

## Failure and reversal

Every stage writes to a new data root or additive schema, records a migration checkpoint, and can
stop without deleting legacy data. Reversal disables the v2 feature flag, restores the prior
application binary/config, and points to the untouched legacy root. If v2 accepted new canonical
human changes, export them in the open migration format before reversal; never silently discard
them.

## Stop gate

Do not start v2 implementation until:

- all required audit files and 15 ADRs are internally consistent;
- P0 blockers are owned with acceptance tests;
- backup/restore rehearsal succeeds;
- benchmark fixtures and commands are ready;
- threat-model assumptions are confirmed;
- the vertical-slice contract is approved.
