# First V2 Vertical Slice Contract

## User outcome

A user registers a disposable Obsidian/filesystem fixture, preserves the exact source evidence,
retrieves it by exact and semantic search, receives a citation-backed answer, sees a possible
contradiction, approves or rejects a typed memory proposal, and can audit and roll back the approved
change.

## In scope

1. read-only source registration using a security-scoped/bookmarked or explicit path;
2. deterministic Markdown ingestion without following symlinks outside the root;
3. immutable evidence bytes, SHA-256, source identity, observed time, and parser version;
4. file-owned human document plus SQLite identity/version/provenance;
5. exact and one benchmark-selected semantic retrieval adapter;
6. authorization-filtered citation spans tied to evidence/document version;
7. deterministic contradiction candidate plus optional local-model explanation;
8. typed memory proposal with structured diff and base-version hash;
9. accessible approve/reject;
10. atomic apply + audit + rollback version + outbox;
11. rollback and deterministic index rebuild;
12. open export and full backup/restore of the slice.

## Out of scope

File write-back, continuous watcher, bidirectional sync, tasks/goals/habits/calendar, relay,
messaging, public sharing, federation, general plugin marketplace, multiple cloud providers,
native-client rewrite, and C++.

## Functional contract

### Ingestion

- never modify the source vault;
- preserve original bytes before parsing;
- identify content by hash and logical document by registered source identity;
- model create/change/rename/delete explicitly;
- reject or safely record unreadable/oversized/unsupported input;
- import is idempotent and resumable.

### Retrieval and answer

- exact lookup is canonical and independent of derived index availability;
- semantic results expose model/index versions and projection watermark;
- every answer claim links to an immutable evidence span;
- insufficient or conflicting evidence produces uncertainty, not invention;
- local model is default; cloud requires an explicit per-run policy approval.

### Contradiction and proposal

- contradiction detection creates a derived candidate, never a canonical fact;
- proposal includes supporting/refuting evidence, confidence, structured diff, and base version;
- rejection changes no canonical knowledge;
- approval uses compare-and-swap and fails visibly if the base changed;
- execution, audit, new version, rollback pointer, and outbox commit atomically.

## Acceptance criteria

- source bytes before/after are identical;
- 100% evidence objects match recorded hashes;
- interrupted ingest resumes without duplicates;
- empty derived stores rebuild to identical query fixtures;
- exact lookup corpus passes 100%;
- semantic quality meets the predeclared Recall@10/nDCG threshold;
- all citations resolve to the correct version/span;
- contradiction fixture precision/recall threshold is met;
- unapproved and rejected proposals cannot change canonical state;
- approval replay is idempotent; stale approval fails;
- rollback restores the prior export and generates a new auditable version;
- P0 threat tests pass, including prompt injection and parser limits;
- keyboard and VoiceOver completion succeeds;
- backup restores on a fresh data root;
- performance and memory remain within declared S/M dataset budgets.

## Evidence package

The slice release candidate includes fixture hashes, commands, benchmark JSON, accessibility
recording/checklist, threat-test results, SBOM, migration/restore report, before/after exports, and
the complete audit chain.
