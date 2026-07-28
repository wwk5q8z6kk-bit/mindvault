# Knowledge Workspace Architecture

## Purpose

This document turns the goal of a Notion- and Obsidian-class MindVault Library
into an implementation contract. It covers canonical storage, identity,
filesystem behavior, projection into the existing knowledge engine, user
experience, migration, and the first safe delivery slices.

It is subordinate to accepted ADR 008 and ADR 009. Those decisions establish
the target; this document defines the evidence required before the target
becomes authoritative.

## Product outcome

Users can keep all human-authored knowledge in a familiar hierarchy of folders
and Markdown files without giving up MindVault's higher-level capabilities:

- semantic, full-text, and graph retrieval;
- links, backlinks, tags, properties, and saved views;
- structured entities, claims, tasks, evidence, and provenance;
- temporal context and governed memory;
- reviewable AI assistance and automation;
- offline access and an open exit path.

MindVault must treat the filesystem as a real product surface, not as a folder
metaphor layered over opaque records.

## Alignment

### Task objective

Evolve Notes into a scalable knowledge workspace in which users can structure,
connect, find, and manage information through both ordinary folders/files and
MindVault's graph and context systems.

### Non-goals for the first vertical slice

- real-time multi-user document collaboration;
- selecting a cloud sync provider or replacing the existing sync architecture;
- editing arbitrary binary formats;
- a full block-database or spreadsheet engine;
- public site publishing;
- automatic AI modification of canonical files;
- deletion of existing database notes before migration verification.

### Success signals

The architecture is successful when:

1. Every native document is readable as a normal Markdown file with MindVault
   stopped.
2. Directory changes made inside or outside MindVault converge to the same tree
   without flattening folders into tags.
3. Moving or renaming a file preserves its stable identity, backlinks, history,
   and structured relationships.
4. Concurrent or external edits never produce a silent overwrite.
5. Full-text, vector, and graph projections can be deleted and rebuilt from
   canonical data.
6. Copying the workspace and its managed metadata is a complete local backup of
   human-authored content and document identity.
7. Current database notes can be migrated through a dry run, verified, and
   rolled back without data loss.

## Canonical data model

MindVault uses different canonical stores for different kinds of information.
This avoids forcing structured system state into Markdown while preserving
human control over authored documents.

| Information | Canonical store | Derived representations |
| --- | --- | --- |
| Document title and body | Markdown file | SQLite projection, FTS, vectors, graph |
| Folder hierarchy | Directory tree | Cached tree projection |
| Stable document identity | Workspace manifest | Path lookup cache |
| Links and backlinks | Markdown links plus structured relationships | Graph index |
| Tags and document properties | Markdown frontmatter when user-authored; SQLite when system-owned | Search facets |
| Entities, claims, tasks, evidence | SQLite | Views, graph, search |
| Audit and version metadata | SQLite/version store | Activity views |
| Attachments | Files | Metadata and extracted-content indexes |
| Search indexes | Rebuildable stores | None |

### Document

A native document has:

- `document_id`: stable UUID independent of path;
- `workspace_id`: owning workspace;
- `relative_path`: normalized path within the workspace root;
- `content_hash`: hash of the canonical file bytes;
- `observed_mtime`: filesystem hint, never the sole change detector;
- `encoding`: UTF-8 for native Markdown;
- `projection_version`: version of the last successful engine projection;
- `sync_state`: `clean`, `pending`, `conflict`, `missing`, or `error`;
- optional user-authored frontmatter;
- audit and version references stored outside the Markdown body.

The stable ID-to-path mapping is maintained in MindVault-managed metadata.
Paths are mutable display and organization attributes, not record identity.
MindVault must not inject a hidden ID into frontmatter by default. A future
interoperability option may expose IDs in frontmatter, but it cannot be required
to read the workspace.

### Workspace

A workspace has:

- a stable workspace ID;
- a user-selected canonical root directory;
- a managed manifest and journal;
- inclusion and exclusion rules;
- platform and filesystem capability observations;
- reconciliation state and last successful scan.

The manifest must be transactional, recoverable, versioned, and included in an
identity-aware workspace backup. ADR 009 places it in MindVault's managed
SQLite store so existing vault roots can remain untouched. A plain directory
copy preserves authored content; preserving MindVault IDs, relationships, and
history additionally requires the managed manifest.

### Folders, tags, and collections

These mechanisms are complementary:

- **Folders** are real directories and answer “where does this live?”
- **Tags** are many-to-many labels and answer “what is this about?”
- **Links** connect individual documents or headings.
- **Properties** hold typed, user-controlled metadata.
- **Collections/saved views** query files and structured objects without moving
  them.

A folder must never be encoded only as a tag. Moving a document changes its
path; applying a tag does not.

## Workspace service boundary

All application-initiated file mutations pass through a workspace service in
the engine. The frontend and integrations do not write workspace paths
directly.

The service provides these initial operations:

- list and reconcile the workspace tree;
- create a folder;
- create and read a Markdown document;
- update a document using an expected base hash;
- rename or move a file or folder;
- move an item to a recoverable trash location;
- restore a trashed item;
- resolve a conflict by choosing or merging versions;
- inspect operation status and audit history.

Permanent deletion, bulk rewriting, and unattended automation require separate
authorization and are not part of the first writable slice.

### Path safety

Every operation must:

- resolve a relative path against one configured workspace root;
- reject absolute paths, parent traversal, null bytes, and path components that
  escape the root;
- prevent symlink traversal outside the root unless an explicit future policy
  permits it;
- detect collisions under case-insensitive and Unicode-normalized comparison;
- handle Windows reserved names and portable filename restrictions;
- enforce bounded path length and directory depth;
- reject writes to excluded or MindVault-managed internal locations.

The same checks apply to UI actions, API calls, plugins, automations, and AI
tools.

### Atomic writes and concurrency

A canonical update uses this sequence:

1. Read the current file and compute its content hash.
2. Compare it with the caller's expected base hash.
3. If hashes differ, create a conflict and leave the canonical file unchanged.
4. Persist the current version plus a durable `prepared` journal entry.
5. Write the new bytes to a temporary file in the same directory.
6. Flush and atomically replace the target where the filesystem supports it.
7. In one database transaction, update the manifest and mark the journal entry
   `completed`.
8. Emit an event that schedules search and graph projection.

The filesystem and SQLite cannot participate in one atomic transaction.
Recovery therefore treats the journal as a small state machine:

- `prepared` with the old hash still present means the file mutation did not
  land;
- `prepared` with the new hash present means the manifest transaction must be
  completed;
- any other observed hash is an external conflict and requires reconciliation.

If the filesystem cannot provide the required atomicity, the workspace is
marked with a degraded capability and writes require a tested fallback. Failed
projection must not roll back a successful canonical file write; it leaves the
document `pending` and reconciliation retries the derived work.

### External edits

Filesystem events are hints, not truth. Watcher events are debounced and
followed by a bounded rescan:

1. Observe create, update, move, or delete events.
2. Normalize the affected path and evaluate inclusion rules.
3. Compare the file hash and manifest state.
4. Detect renames using journal evidence and identity/fingerprint heuristics.
5. Update the manifest and SQLite projection in a transaction.
6. Schedule FTS, vector, and graph updates.
7. Surface ambiguous renames or unsaved-editor conflicts for review.

A periodic reconciliation scan repairs missed watcher events.

The implemented Stage 1 coordinator owns native recursive watches for every
mounted root. It routes only mutating Markdown or directory events, excludes
`.git` and reserved `.mindvault*` paths, coalesces bursts without allowing a
continuous stream to starve reconciliation, and uses a bounded event queue. A
queue overflow or watcher error schedules every known workspace immediately.
Independent periodic full scans remain active even when the platform watcher
cannot initialize, and newly mounted or temporarily unavailable roots are
rediscovered. Reconciliation never writes canonical workspace files.

## Projection into the knowledge engine

The existing node and search systems become projections for native documents,
not competing canonical stores.

Each projected document node includes:

- stable document and workspace IDs;
- normalized relative path and parent path;
- title, Markdown content, tags, and user properties;
- source URI using a workspace-aware scheme;
- content hash and projection timestamp;
- links, headings, block anchors, and attachment references;
- provenance describing whether the latest change came from MindVault, an
  external editor, import, migration, integration, or approved AI proposal.

Projection processing must be:

- idempotent by document ID plus content hash;
- restartable after partial failure;
- able to rebuild an individual document or entire workspace;
- observable through pending/error counts and last-success timestamps;
- isolated so one malformed document cannot block the workspace.

## Library experience

The current Notes route evolves into a Library workspace with three coordinated
levels:

1. **Navigation:** workspace switcher, real folder tree, favorites, recent
   documents, tags, and saved views.
2. **Content:** sortable file list or document editor with breadcrumbs,
   multi-select, drag-and-drop move, keyboard navigation, and fast search.
3. **Context:** backlinks, outgoing links, properties, evidence, history,
   related entities, tasks, and sync/conflict state.

Core behavior:

- Creating a note creates a Markdown file in the selected folder.
- Creating a folder creates a directory immediately through the workspace
  service.
- Rename and move operations show path conflicts before committing.
- Deletion defaults to recoverable trash.
- The editor displays unsaved, externally changed, projection-pending, and
  conflict states distinctly.
- Search can scope by workspace, folder subtree, tag, property, type, date,
  entity, or evidence state.
- Links can target a document, heading, or stable block anchor and remain valid
  after path changes.

Capture remains the low-friction entry point, but every captured document has an
explicit inbox path and can be filed later. Structured captures such as tasks or
claims may be represented in Library views without pretending they are files.

## Governed intelligence

AI and automation operate through the same workspace service and authorization
boundaries as humans.

- Read operations can be scoped to workspaces or folder subtrees.
- Proposed edits include the base content hash and a visible diff.
- Applying a proposal creates an audit record and a recoverable version.
- Stale proposals fail safely when the underlying file changed.
- Bulk operations require an enumerated target set and preview.
- Destructive actions require explicit approval.
- Provenance records the model/tool, initiator, evidence, and accepted change.

AI output is advisory until a user or an authorized policy accepts it. Search
summaries and derived insights never silently become canonical documents.

## Existing Notes migration

Migration is export-first and reversible.

The authoritative phase, field mapping, safety, verification, and rollback
contract is defined in `knowledge-workspace-migration.md`.

Portable paths, byte-preserving Markdown, links, citations, versions, trash,
and external-conflict semantics are defined in
`knowledge-workspace-document-contract.md`. The executable managed-manifest
design is `knowledge-workspace-manifest-v1.sql`.

### Dry run

For every current note-like node, the migrator:

- chooses a destination folder from an explicit rule;
- creates a portable, sanitized filename with deterministic collision handling;
- renders Markdown and user-owned metadata without changing the source node;
- assigns or preserves a stable document ID;
- reports filename collisions, unsupported metadata, link rewrites, and
  attachment issues;
- estimates the resulting file and projection counts.

### Apply

The migrator writes to a new staging workspace, verifies byte counts and hashes,
builds projections, and generates a reconciliation report. Existing database
notes remain available in read-only compatibility mode until the user confirms
the migrated workspace.

### Commit and rollback

Commit switches the Library to the verified workspace and records a migration
checkpoint. Rollback restores the previous authoritative Notes view and leaves
the generated workspace available for inspection. Source notes are not deleted
by the migration.

## Obsidian interoperability

The current Obsidian importer is a useful parser but not the target ownership
model. Folder tags and `obsidian://` path metadata are insufficient for a native
filesystem.

Delivery progresses from:

1. read-only mounting and faithful indexing of an existing vault;
2. stable identity and rename tracking without modifying user frontmatter;
3. guarded bidirectional editing with conflict handling;
4. a native MindVault workspace that remains compatible with ordinary Markdown
   tools.

Obsidian-specific syntax that MindVault does not understand must be preserved
losslessly even if it cannot yet be rendered.

## Staged delivery

### Implementation status — 2026-07-26

The Stage 1 foundation is implemented in the Rust core, engine, and SQLite
storage layers:

- the `portable-v1` path policy distinguishes unsafe paths from existing
  safe-but-nonportable names and emits deterministic collision keys;
- the scanner performs bounded, read-only Markdown discovery without following
  symlinks or creating metadata inside the mounted root;
- exact source-byte hashes, UTF-8/size status, diagnostics, and conservative
  filesystem identity hints feed a pure reconciliation plan;
- one optimistic SQLite transaction applies workspace and document manifest
  changes atomically;
- unambiguous external renames preserve document UUIDs, while ambiguous
  identities and path collisions fail closed;
- an administrator-only REST boundary mounts roots beneath the explicit
  `MINDVAULT_WORKSPACE_ALLOWED_ROOTS` capability allowlist, while sealed mode
  rejects mounting until descriptor encryption is available;
- namespace-scoped APIs list workspaces, return a flat virtualizable tree
  (including empty directories), reconcile external changes, and read current
  UTF-8 document content without exposing full root locators in list results;
- the Notes route now includes a Files view that renders mounted workspace
  folders, preserves empty directories, surfaces changed and untracked state,
  triggers reconciliation, and reads the canonical Markdown source;
- the Files view now mounts a selected folder through the same administrator
  boundary, using a least-privilege native desktop directory chooser with an
  explicit absolute-path fallback for browser deployments. The server
  allowlist remains authoritative and rejected roots receive actionable setup
  guidance.
- a bounded native filesystem watcher now debounces external Markdown and
  directory changes, reconciles the optimistic manifest, emits a scoped
  WebSocket refresh event, rediscovers mounted roots, and retains an
  independent periodic full-scan fallback when native events fail or overflow.
- active UTF-8 Markdown documents now project into the existing SQLite,
  full-text, vector, and Markdown-link graph systems using their stable manifest
  document UUID, while canonical files remain untouched;
- projection metadata records the workspace, canonical path, content hash,
  user-authored frontmatter, and projection timestamp. Ordinary node mutation
  APIs reject these read-only derived nodes;
- mount and reconciliation retry stale or failed documents independently,
  remove projections for missing or unsupported documents, and expose
  per-document projection state plus aggregate success/failure counts;
- an explicit authorized-write rebuild endpoint and Files-view action
  regenerate a workspace's disposable search and graph representations from
  canonical files and the managed manifest.

Stage 1 is now complete for the supported read-only Markdown workspace
contract. Stage 2 editor and guarded file-mutation operations remain
deliberately out of scope for this foundation.

### Stage 0 — contracts and safety baseline

- approve ADR 008 and the manifest format;
- document workspace threat boundaries;
- define filesystem compatibility fixtures;
- record current Notes and Obsidian-import behavior;
- approve migration and rollback acceptance tests.

**Exit:** canonical ownership, identity, path safety, and rollback decisions are
internally consistent.

### Stage 1 — read-only workspace

- select and mount one local root;
- render a faithful folder/file tree;
- parse Markdown without rewriting it;
- track external changes and stable identity;
- build observable, rebuildable search projections.

**Exit:** external creates, edits, moves, and deletes converge correctly across
the supported filesystem matrix.

### Stage 2 — guarded native writes

- create folders and Markdown files;
- edit with base-hash concurrency;
- rename, move, trash, restore, and version;
- expose conflict resolution and audit history.

**Exit:** no tested failure mode causes a silent overwrite or a workspace-root
escape.

### Stage 3 — Notes migration

- dry-run current database notes;
- stage, verify, and reconcile a generated workspace;
- switch authority only after explicit confirmation;
- prove rollback.

**Exit:** content, attachments, links, tags, IDs, and counts meet the migration
acceptance report.

### Stage 4 — connected knowledge

- stable heading/block links and backlink repair;
- folder-, property-, and evidence-aware saved views;
- structured objects embedded or referenced from documents;
- temporal and provenance exploration.

**Exit:** users can organize the same knowledge by location, relationship,
meaning, and time without duplicating canonical content.

### Stage 5 — governed automation

- diff-based AI editing;
- scoped batch organization proposals;
- user-approved rules and workflows;
- policy and audit views.

**Exit:** automation is reversible, attributable, scope-bounded, and unable to
bypass workspace safety checks.

## First v2 vertical-slice acceptance contract

The first product slice is the complete governed-context chain defined by the
product brief:

**Obsidian/filesystem ingestion → immutable evidence → exact and semantic
retrieval → citation-backed answer → contradiction detection → proposed memory
update → approval or rejection → audit and rollback.**

A real folder tree is a required foundation inside this slice, not a replacement
for the end-to-end outcome.

### Filesystem foundation gate

Given a fixture workspace containing nested Markdown files, Unicode names,
links, unsupported syntax, attachments, ignored paths, a case collision, and a
symlink escape attempt, MindVault must:

1. display the real nested tree without altering any file;
2. assign stable identities and preserve them across an external rename;
3. index supported content and report unsupported or excluded items;
4. reject or quarantine unsafe paths;
5. update results after external create, edit, move, and delete operations;
6. recover correct state after watcher interruption through reconciliation;
7. rebuild the SQLite/search projection from canonical files and the manifest;
8. expose errors and pending work instead of silently dropping documents.

### Governed-context completion gate

Using evidence from that mounted workspace, MindVault must:

1. retrieve an exact document by name, path, alias, or stable ID;
2. retrieve semantically related documents without losing source identity;
3. answer a question with citations that resolve to the exact source passage;
4. preserve immutable evidence for the cited source revision;
5. surface a conflicting claim with the evidence for both sides;
6. create a proposed memory update without changing the canonical file or
   accepted memory;
7. allow an authorized user to approve or reject the proposal;
8. record the ingest, query, citation, proposal, decision, and resulting state;
9. roll back an accepted update while preserving its audit trail;
10. rebuild all derived indexes from canonical files and structured records.

Passing the filesystem foundation gate unlocks guarded workspace writes.
Passing both gates completes the first v2 vertical slice. Neither gate
authorizes migration or deletion of current Notes data.

## Required validation

Implementation must include:

- unit tests for normalization, collision, identity, and state transitions;
- property tests for path containment and rename/move invariants;
- integration tests against real temporary directories;
- platform fixtures for macOS, Linux, and Windows path behavior;
- crash/interruption tests around canonical writes and manifest updates;
- watcher-loss and full-reconciliation tests;
- migration round-trip, collision, and rollback tests;
- security tests for traversal, symlink escape, excluded paths, and unauthorized
  automation;
- performance baselines for initial scan, incremental reconciliation, tree
  navigation, and search at agreed workspace sizes.

Performance targets must be recorded from measured baselines before numeric
budgets are committed.

## Deferred post-foundation decisions

The document contract resolves the version, path, frontmatter-preservation,
link-anchor, trash, and baseline external-conflict policies needed for the first
slice. These later choices remain outside the initial foundation:

- selection or implementation of a native multi-device sync provider;
- a non-portable MindVault-managed encrypted-document mode;
- automatic history/trash pruning policies;
- collaborative block-level editing or CRDT semantics.

ADR 009 resolves the managed manifest location, stable identity policy,
workspace API boundary, and the honest plaintext/sealed-mode distinction.
