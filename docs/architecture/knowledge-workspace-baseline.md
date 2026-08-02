# Knowledge Workspace Current-State Baseline

- **Observed:** 2026-07-26
- **Scope:** Notes UI/API, Obsidian import, versioning, audit, backup, and
  filesystem-adjacent engine facilities
- **Purpose:** Establish what can be reused and what must be replaced before the
  file-first Library becomes authoritative

## Executive finding

MindVault has strong database-node, authorization, search, proposal, and
portability foundations. It does not currently have a canonical document
workspace.

The existing Notes surface is database-first. Obsidian support is a one-shot
import that projects files into database nodes. The component named
`WatcherAgent` scans recent database nodes for intents and insights; it does not
observe filesystem events. Current note version history and request audit logs
do not satisfy the durability or concurrency requirements of canonical files.

Shipping a folder component over these mechanisms would create a virtual
filesystem rather than the product required by ADR 008.

## Evidence table

| Area | Current behavior | Evidence | Disposition |
| --- | --- | --- | --- |
| Notes storage | Note-like items are `KnowledgeNode` records read and mutated through generic node APIs. | `frontend/src/lib/api/notes.ts`; `frontend/src/lib/api/nodes.ts`; `crates/mv-server/src/rest.rs:10523-10697` | Preserve during compatibility period; do not extend as canonical file storage. |
| Folder hierarchy | Obsidian parent paths become `folder:<path>` tags. | `crates/mv-engine/src/import/obsidian.rs:82-95,571-595` | Retire as a hierarchy mechanism; keep tags only as legacy import metadata. |
| Obsidian identity | Source is `obsidian://<relative-path>`. Existing records are located by source. | `crates/mv-engine/src/import/obsidian.rs:100-119,193-217,227-258` | Replace with workspace/document IDs and a path mapping. |
| Change detection | Import compares recorded and current mtimes. | `crates/mv-engine/src/import/obsidian.rs:103-111,197-212,233-258` | Replace with content hashes, a journal, filesystem hints, and reconciliation. |
| Directory scan | `WalkDir` scans `.md`, does not follow links, skips hidden entries, and drops walk errors. | `crates/mv-engine/src/import/obsidian.rs:338-356` | Reuse `walkdir` only behind explicit error reporting, inclusion rules, and path policy. |
| Markdown parsing | A custom parser handles a limited subset of frontmatter and wiki links. Heading fragments are discarded. | `crates/mv-engine/src/import/obsidian.rs:358-506` | Replace or isolate behind a lossless parser contract before any writeback. |
| Link resolution | Links resolve through lowercased file stems; unresolved links are silently skipped. | `crates/mv-engine/src/import/obsidian.rs:290-328,597-604` | Replace with path/alias/heading-aware resolution and observable unresolved state. |
| Watcher | `WatcherAgent` queries recent nodes, extracts intents, and generates insights on a timer. | `crates/mv-engine/src/watcher.rs:27-33,58-208` | Preserve as agentic analysis; create a separately named filesystem observer. |
| Filesystem event library | Engine dependencies include `walkdir` but no filesystem notification crate. | `crates/mv-engine/Cargo.toml` | Add only after the filesystem capability matrix and reconciliation fallback are approved. |
| Node concurrency | Generic update accepts a full `KnowledgeNode`; no expected content hash is required. | `crates/mv-server/src/rest.rs:10596-10676` | Do not reuse for canonical document writes. |
| Node versioning | Authored-field changes push snapshots into node metadata, capped at 40 entries. | `crates/mv-server/src/rest/node_versions.rs:5-22`; `crates/mv-server/src/rest.rs:10648-10653` | Preserve for database nodes; replace with durable document versions. |
| Authorization | Node routes enforce write permission and namespace authorization. | `crates/mv-server/src/rest.rs:10523-10570,10596-10617,10679-10697` | Reuse concepts; extend policy to workspace and folder subtree scope. |
| Input validation | Node payloads pass central validation before storage. | `crates/mv-server/src/rest.rs:10536-10546,10655-10665` | Reuse the validation pattern with path/content-specific validators. |
| Audit | Request audit can write JSON lines, console, webhook, and a bounded in-memory store; it is disabled by default. | `crates/mv-server/src/audit.rs:20-24,61-102,125-223,449-480` | Preserve request telemetry; add mandatory durable workspace mutation journal. |
| Proposal undo | Approved proposals can store and apply undo snapshots. | `crates/mv-server/src/rest/exchange.rs`; `migrations/005_relay_safeguards.sql` | Reuse the proposal/approval concept; bind proposals to document base hashes. |
| Knowledge conflicts | A structured conflicts table exists for contradictory nodes. | `migrations/014_conflicts.sql` | Preserve for semantic contradictions; do not overload for file-write conflicts. |
| Sync conflicts | Vector-clock sync models concurrent node edits and can associate a proposal. | `crates/mv-engine/src/sync/snapshot.rs`; `crates/mv-engine/src/sync/mod.rs` | Reuse concepts after mapping document revisions; not a filesystem sync solution. |
| Archive path safety | Backup restore rejects absolute paths, parent traversal, and unsupported entry types. | `crates/mv-cli/src/commands/backup.rs:25-91,223-242` | Reuse tests and normalization lessons in workspace path policy. |
| Database backup | Encrypted backup/restore covers the vault database. | `crates/mv-engine/src/backup.rs`; `crates/mv-engine/src/keychain.rs:2078-2093` | Extend with a consistent file snapshot and workspace manifest checkpoint. |
| Data-directory backup | CLI archives the configured data directory and makes a pre-restore backup. | `crates/mv-cli/src/commands/backup.rs:93-261` | Reuse archive validation; it does not automatically cover user-selected workspace roots. |
| REST portability | Server exposes synchronous `GET /api/v1/export` and JSON `POST /api/v1/import`. | `crates/mv-server/src/rest.rs:336-337,7149-7291` | Reuse bundle schema selectively for structured state. |
| Frontend portability | Client expects asynchronous job-style `POST /api/v1/export`, upload import, and status routes. | `frontend/src/lib/api/portability.ts:77-218` | Existing contract mismatch must be resolved before using this UI for migration. |
| Sealed mode | Documentation states all persisted vault artifacts are encrypted in sealed mode. | `docs/security.md` | Add a clear exception/policy before plaintext Markdown workspaces ship. |

## Reusable foundations

The following should be extended rather than replaced:

- Rust domain/engine/storage layering;
- authenticated REST entry points and namespace-aware authorization;
- central payload validation;
- SQLite migrations and transactions;
- graph and hybrid-search projections;
- proposal approval/rejection and undo concepts;
- WebSocket change notifications;
- encrypted database backup primitives;
- archive path validation tests;
- existing Notes as a read-only migration source.

## Required new capabilities

The workspace needs new first-class components:

1. workspace and document domain types;
2. manifest tables and repository traits;
3. normalized, platform-aware path policy;
4. filesystem observer plus periodic reconciliation;
5. stable identity and ambiguous-rename review;
6. expected-hash file reads and writes;
7. atomic mutation and durable journal;
8. document-version storage and retention;
9. workspace-scoped authorization;
10. projection isolation and rebuild state;
11. identity-aware backup/restore;
12. staged Notes migration with verification and rollback.

## Baseline blockers

| Blocker | Why it blocks writable Library work | Unlock evidence |
| --- | --- | --- |
| No canonical ownership enforcement | Generic node and future file writes could diverge. | Projected document nodes reject generic mutation. |
| No stable document identity | Paths change on ordinary organization actions. | Manifest round-trip and external rename tests preserve IDs. |
| No safe write token | External edits could be silently overwritten. | Expected-hash conflict test leaves file unchanged. |
| Lossy Markdown parsing | Writeback could destroy unsupported syntax or frontmatter. | Lossless parse/render fixtures pass without semantic edits. |
| No filesystem reconciliation | Watcher event loss or downtime would leave stale state. | Full rescan converges after dropped-event tests. |
| Audit is optional/request-level | File changes need durable operation evidence. | Mandatory journal survives restart and correlates file/manifest state. |
| Version history is node-local and bounded | Canonical rollback cannot depend on projection metadata. | Document version restore verifies bytes and hash. |
| Backup excludes arbitrary workspace roots | DB restore alone can orphan file identity. | Identity-aware backup/restore passes a full round trip. |
| Portability API mismatch | Migration UI cannot safely rely on the current client contract. | One tested server/client schema replaces both assumptions. |
| Encryption claim conflicts with plaintext files | Product could overstate protection. | UI/docs explicitly distinguish portable plaintext from sealed managed data. |

## Baseline conclusion

The repository is suitable for an incremental implementation. It is not
suitable for treating the current Obsidian importer or Notes API as the
filesystem layer.

ADR 008 defines canonical ownership. ADR 009 defines identity and the mutation
boundary. The workspace-specific design gate is now closed by the migration
plan, document contract, manifest contract, contract fixtures, and explicit
plaintext/encryption copy. The blockers above remain runtime conformance gates;
the broader product brief still governs when implementation may begin.
