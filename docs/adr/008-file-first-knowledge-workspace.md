# ADR 008: File-First Knowledge Workspace

- **Status:** Accepted
- **Date:** 2026-07-26
- **Owners:** MindVault
- **Related:** ADR 001 (local-first storage), ADR 003 (hybrid search)

## Context

MindVault's current Notes experience stores note-like knowledge nodes in SQLite.
It can import Markdown from Obsidian, but the imported directory hierarchy is
represented indirectly through source metadata and tags. That is not a true
folder and file system: users cannot treat their notes as ordinary files,
reorganize them outside MindVault, or move their knowledge base without a
MindVault-specific export.

The product direction requires Notes to become a complete knowledge workspace:
human-authored documents must remain portable and user-owned while MindVault
adds search, graph relationships, structured context, provenance, governed AI,
and automation.

Two approaches were considered:

1. **Database-first folders.** Add `parent_id` and path metadata to existing note
   records, then render a folder tree.
2. **File-first documents.** Make Markdown files and directories canonical for
   human-authored documents, with SQLite and search stores acting as projections
   and as the canonical home only for structured system data.

The database-first option is cheaper in the short term, but it creates a
MindVault-only virtual filesystem and makes later migration harder. The
file-first option costs more because it needs path safety, filesystem watching,
conflict handling, and migration, but it directly satisfies portability and
interoperability. It is also the more reversible product choice: users can
always open or move their Markdown files without MindVault.

## Decision

MindVault will use a **hybrid canonical storage model**:

- Markdown files and directories are canonical for user-authored documents and
  their folder hierarchy.
- SQLite is canonical for structured entities, claims, relationships, tasks,
  permissions, automation state, audit records, and document identity mappings.
- Full-text, vector, and graph search representations are derived projections
  that can be rebuilt from canonical sources.
- Attachments remain ordinary files referenced by documents or structured
  records.

The product surface currently called Notes will evolve into the **Library**
workspace. Folders are real directories, notes are real `.md` files, and
operations such as create, rename, move, and trash are filesystem operations
performed through a guarded workspace service.

MindVault will not silently replace an externally modified file or apply an AI
rewrite directly to canonical content. Writes require optimistic concurrency
checks; conflicts and AI changes are presented as explicit, reviewable changes.

Stable document identity will not depend on the current path. A workspace
manifest stored in MindVault-managed metadata maps a stable document ID to the
current relative path and content fingerprint. Renames and moves update the
mapping while preserving links, history, and graph identity.

The detailed contract and staged delivery plan are defined in
`docs/architecture/knowledge-workspace.md`.

## Consequences

### Positive

- A workspace can be backed up or moved with ordinary filesystem tools.
- Existing Markdown collections can be mounted without flattening their
  hierarchy into tags.
- External editors and version-control systems can operate on canonical files.
- MindVault's semantic search, graph, evidence, and automation layers remain
  rebuildable rather than becoming proprietary document containers.
- Folder organization and cross-cutting organization can coexist: paths express
  location; links, tags, properties, and saved views express relationships.

### Costs and risks

- Filesystem behavior differs across platforms, particularly case sensitivity,
  reserved names, Unicode normalization, symlinks, and maximum path lengths.
- External edits create concurrency and conflict scenarios that do not exist in
  a database-only editor.
- The existing database-backed notes require a guarded, reversible migration.
- Search and graph projections must tolerate partial failure and converge after
  reconciliation.
- Multi-device sync must eventually reconcile both file content and structured
  metadata; this ADR does not choose a sync provider or protocol.

## Implementation constraint

This accepted decision does not imply that the runtime currently implements the
workspace. The existing Notes implementation remains authoritative until a
staged migration proves file parity and rollback. A metadata-only folder tree
must not be shipped or described as the target filesystem.

Implementation must conform to the workspace architecture, document contract,
manifest contract, migration plan, security boundary, and first vertical-slice
acceptance tests before authority can change.
