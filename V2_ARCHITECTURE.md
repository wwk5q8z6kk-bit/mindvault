# V2 Architecture

## Chosen direction

Build v2 as a strangler inside the existing Rust product, not a clean-room rewrite. SQLite and
registered human-readable files are co-canonical by responsibility: files own human-authored
document bytes; SQLite owns identity, versions, provenance, policy, proposals, decisions, and
audit. Immutable content-addressed evidence preserves every observed source version. FTS, vector,
and graph query structures are derived projections driven by a durable SQLite outbox.

## Logical components

```text
Source Registry
  path/bookmark, source identity, scan cursor, policy
        |
Evidence Ingestor (untrusted-input boundary)
  hash, immutable bytes, MIME, observed_at, parser metadata
        |
Canonical Transaction Service
  documents, versions, evidence refs, provenance, policies,
  proposals, approvals, audit, rollback, outbox
        |
Projectors
  exact/FTS | vector | graph
        |
Retrieval Service
  authorization -> candidates -> hybrid rank -> citation ledger
        |
Model Gateway
  local/cloud policy, minimization, disclosure audit
        |
Proposal Service
  contradiction -> typed diff -> approval/reject -> CAS apply
```

Adapters are the Tauri/Svelte UI, CLI, MCP, optional modules, connector workers, and eventual sync.

## Canonical model

- `source`: stable registered source and access policy;
- `evidence_object`: content hash, bytes or managed file reference, media type, size, observed time;
- `document`: stable logical identity and human-file ownership;
- `document_version`: immutable parsed/user version with source/evidence hash and validity interval;
- `memory_assertion`: typed statement linked to supporting/refuting evidence and temporal scope;
- `proposal`: typed candidate mutation with base version, evidence, policy, and diff;
- `decision`: approval/rejection actor, time, reason, and policy;
- `audit_event`: append-only mutation/disclosure/security event;
- `outbox_event`: committed projector work with canonical version watermark;
- `tombstone`: deletion intent and retention state.

All identifiers are stable UUIDv7 or content hashes. Timestamps distinguish source time,
observation time, valid time, transaction time, and model-generation time.

## Consistency model

Canonical changes commit atomically in SQLite with audit and outbox events. Derived projections are
eventually consistent but expose their applied watermark. Reads that require freshness either wait
for the watermark or use exact canonical fallback. Projectors are idempotent, version-aware, and
fully rebuildable.

Delete writes a tombstone first. Evidence is retained or purged only by explicit policy after
references and backups are evaluated. Derived deletion is asynchronous and repairable.

## Retrieval and citations

Authorization filters source/document/evidence scope before candidate generation and after graph or
semantic expansion. A retrieval run persists candidate IDs, canonical versions, scores, index/model
versions, and evidence spans. Answers cite immutable evidence/version hashes. Stale citations are
visible and reproducible.

## Security/privacy architecture

- authenticated local session or restricted local IPC for the desktop;
- capability tokens for agents/connectors;
- keychain broker owns secrets and returns use, not raw value, when possible;
- isolated extraction/plugin/connector workers with no ambient vault access;
- model gateway is the only cloud egress path;
- append-only hash-chained audit linked transactionally to mutations;
- full backup manifest, staged restore, and recurring recovery test.

## Optional modules

Tasks, goals, habits, calendar, relay, public sharing, federation, messaging, comments, and
marketplace depend on core ports. They have separate migrations and feature gates. Disabling an
optional module cannot make core documents/evidence unreadable.

## Client and transport

Keep Tauri/Svelte for the first slice. Retain one versioned local API used by UI, CLI adapter, and
MCP gateway. Quarantine gRPC, UDS, and duplicate WebSocket semantics until a measured consumer and
parity requirement justifies them. Streaming may use one authenticated event channel.

## Quality attributes

- no canonical data loss under injected failures;
- deterministic export and index rebuild;
- offline core;
- explicit cloud disclosures;
- accessible approval/evidence flow;
- migrations resume safely;
- performance budgets from `PERFORMANCE_BASELINE.md`;
- no C++ without the ADR benchmark gate.
