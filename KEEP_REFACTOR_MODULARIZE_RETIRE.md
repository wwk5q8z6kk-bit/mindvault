# Keep, Refactor, Modularize, Retire

## Keep

- Rust core types and provider abstractions that can be made dependency-clean.
- SQLite as canonical transactional authority.
- proposal model, approval/rejection concepts, and undo/version intent.
- keychain encryption/audit concepts, after recovery verification.
- MCP’s proposal-oriented mutation pattern.
- Tauri/Svelte as the current client direction, subject to lifecycle/accessibility gates.
- Tantivy and LanceDB implementations as benchmark candidates, not predetermined winners.
- existing IDs and migration readers needed to preserve user data.

## Refactor in place

- canonical node mutation into one compare-and-swap transaction service;
- ingest/update/delete into outbox-based projection and tombstone flows;
- evidence/attachment handling into content-addressed immutable storage;
- migration registry into ordered, checksummed, resumable migrations;
- proposal execution, audit, and rollback into one atomic workflow;
- auth into explicit local session/capability tokens usable by desktop and WebSockets;
- backup into one complete manifest and staged restore protocol;
- model calls into a classified local/cloud gateway.

## Modularize

- split `mv-server/src/rest.rs` by bounded context and generated route contract;
- split `mv-storage/src/sqlite.rs` into schema repositories behind transaction ports;
- move tasks, habits, calendar, relay, public sharing, federation, messaging, comments, and
  marketplace code behind optional-module interfaces;
- run attachment extractors, connectors, and plugins in isolated workers;
- separate canonical graph facts from derived traversal projections;
- version export/import and transport DTOs independently from internal models.

## Retire or quarantine

- direct canonical writes by AI/enrichment/sync/intent paths;
- default-admin semantics anywhere other than strictly authenticated per-user local IPC;
- API keys in localStorage, argv, logs, or raw config;
- raw live-SQLite backup and destructive in-place restore;
- undocumented alternate transports; retain one local API first;
- mtime-only ingestion identity and silent unresolved-link behavior;
- default-off plugin execution advertised as production;
- documentation-only “complete” claims;
- optional-module expansion until the first core slice passes.

## Do not rewrite yet

Do not replace Rust with C++, replace Tauri with native clients, replace Tantivy with FTS5, or
replace LanceDB based on preference. The ADR measurements and first vertical slice decide those
boundaries with reversible adapters.
