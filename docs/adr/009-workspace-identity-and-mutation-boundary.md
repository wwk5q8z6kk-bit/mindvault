# ADR 009: Workspace Identity and Mutation Boundary

- **Status:** Accepted
- **Date:** 2026-07-26
- **Owners:** MindVault
- **Related:** ADR 004 (encryption at rest), ADR 008 (file-first workspace)

## Context

ADR 008 makes Markdown files and directories canonical for human-authored
documents. That decision requires a stable identity system and one guarded
mutation boundary.

The current Obsidian importer uses `obsidian://<relative-path>` as its source
identity and compares filesystem modification times. A move therefore looks
like a different source, and an mtime is not a safe concurrency token. Current
note updates use the generic node endpoint and keep a bounded version history
inside node metadata. These mechanisms remain useful for database nodes but
cannot safely own canonical files.

Four identity-storage options were considered:

1. **Path as identity.** Minimal implementation cost, but every rename breaks
   identity and can duplicate projections.
2. **IDs in Markdown frontmatter.** Portable and inexpensive, but modifies
   existing vaults and makes MindVault metadata visible in user documents.
3. **A hidden database inside every workspace.** Portable with the directory,
   but modifies read-only vaults, exposes path metadata unless separately
   encrypted, and creates another database lifecycle.
4. **Workspace manifest tables in MindVault's managed SQLite store.** Reuses
   transactions, migration, and backup; can apply sealed payload envelopes;
   leaves mounted document roots untouched; requires an identity-aware backup
   when users want relationships and stable IDs to move with the files.

Option 4 has the lowest integrity and security risk while preserving ordinary
Markdown. The downside is explicit and acceptable: copying only the Markdown
directory preserves all authored content but not MindVault-only identity,
history, or structured relationships. An identity-preserving transfer must
include the managed workspace manifest.

## Decision

### Managed manifest

The workspace manifest will be stored in versioned tables in MindVault's
canonical SQLite store, not inside the user's document root.

The executable logical schema is
`docs/architecture/knowledge-workspace-manifest-v1.sql`. It includes:

- `workspaces`: stable ID, namespace, mode, lifecycle state, and a sensitive
  descriptor payload containing display name, root, rules, and scan state;
- `workspace_documents`: stable document ID, workspace ID, non-reversible path
  token, sensitive document payload, projection state, and lifecycle state;
- `workspace_events`: durable mutation/reconciliation journal with actor,
  operation, encrypted or plaintext-sensitive payload, status, and correlation
  ID;
- `workspace_document_versions`: content-addressed snapshot references and
  retention/pinning metadata;
- `workspace_conflicts`: competing hashes/versions, detection evidence, state,
  and resolution;
- `workspace_migrations` and `workspace_migration_items`:
  dry-run/apply/verify/commit/rollback checkpoints and exact source mappings.

In sealed mode, descriptors, paths, content hashes, diagnostics, journal
details, and other sensitive fields use authenticated `mvenc-v1` payload
envelopes. Stable IDs, lifecycle enums, and timestamps may remain structural
plaintext. Path uniqueness uses a keyed HMAC token in sealed mode instead of a
reversible path column.

Canonical document content is never stored in the manifest as the authoritative
copy. Content may appear in explicitly retained version snapshots, evidence
records, or encrypted proposal undo data.

SQLite and the host filesystem cannot be committed atomically as one
transaction. Every workspace mutation must therefore use a recoverable
prepare/apply/finalize protocol:

1. persist the expected old state, intended new state, and a `prepared` event;
2. apply the guarded filesystem mutation;
3. transactionally update the manifest and mark the event `completed`;
4. reconcile any `prepared` event after interruption by comparing observed,
   expected-old, and intended-new hashes.

An observed state matching neither known hash becomes a conflict. Recovery must
never guess which content should win.

No ID is injected into frontmatter by default. An opt-in interoperability mode
may expose document IDs later, but mounting or indexing a vault must not depend
on it.

### Stable identity

Every workspace and document receives a UUID. A path is mutable state, not
identity.

- MindVault-initiated moves carry the document ID through the journal
  transaction.
- External moves use platform file identity hints when trustworthy and content
  fingerprints as supporting evidence.
- A missing path plus a new path is treated as the same document only when the
  evidence is unambiguous.
- Ambiguous rename detection creates a review item; it must not silently merge
  two documents or assign one document's relationships to another.
- If the manifest is lost, MindVault can re-index the Markdown, but recovered
  IDs are not guaranteed to match. This is reported as identity recovery, not
  ordinary reconciliation.

### Mutation service

All application-initiated workspace mutations go through
`WorkspaceService` in `mv-engine`. UI, REST, MCP, plugins, automations, and AI
may request operations but may not write arbitrary workspace paths.

The dependency direction is:

```text
frontend / integrations
        ↓
workspace REST or local command adapter
        ↓
mv-engine::WorkspaceService
        ↓
path policy + canonical file operation + manifest transaction
        ↓
projection events to node/search/graph stores
```

`mv-core` owns workspace domain types and repository traits. `mv-storage` owns
manifest persistence. The generic node API remains authoritative for structured
nodes but not for canonical document mutation.

Projected document nodes are read/search representations. A generic node update
targeting a workspace document must be rejected with a conflict response that
directs the caller to the workspace API.

### Workspace API shape

The initial API is resource-oriented and document-ID based:

- `POST /api/v1/workspaces` — mount or create a workspace;
- `GET /api/v1/workspaces` — list authorized workspaces;
- `GET /api/v1/workspaces/{workspace_id}/tree` — list a bounded tree snapshot;
- `POST /api/v1/workspaces/{workspace_id}/folders` — create a directory;
- `POST /api/v1/workspaces/{workspace_id}/documents` — create a Markdown file;
- `GET /api/v1/workspaces/{workspace_id}/documents/{document_id}` — read with
  content hash and state;
- `PUT /api/v1/workspaces/{workspace_id}/documents/{document_id}` — update with
  required `expected_content_hash`;
- `POST /api/v1/workspaces/{workspace_id}/operations/move` — rename/move by
  stable IDs and destination parent;
- `POST /api/v1/workspaces/{workspace_id}/operations/trash` — recoverable trash;
- `POST /api/v1/workspaces/{workspace_id}/operations/restore` — restore;
- `POST /api/v1/workspaces/{workspace_id}/reconcile` — bounded rescan;
- `GET /api/v1/workspaces/{workspace_id}/conflicts` — review conflicts.

Raw absolute paths are accepted only when mounting a workspace through an
authorized local capability. Normal operations use workspace/document IDs and
validated relative destinations.

Mutation responses include the stable ID, normalized path, resulting content
hash, projection state, journal correlation ID, and conflict details when
applicable.

### Portable versus sealed storage

A Markdown workspace is plaintext unless the underlying filesystem provides
encryption. This is necessary for ordinary editors to read it.

MindVault sealed mode protects the managed manifest, structured records,
versions, and derived stores. It cannot truthfully claim to encrypt an external
Markdown root that must remain readable by other applications. The Library must
display this boundary and recommend operating-system or encrypted-volume
protection for portable workspaces.

A future MindVault-managed encrypted-document mode would be a separate workspace
mode and would not promise ordinary plaintext interoperability while locked.

### Backup

Two backup levels are explicit:

- **Content backup:** copy the Markdown root; preserves authored content and
  directory structure.
- **Identity-aware backup:** capture a consistent Markdown snapshot plus the
  corresponding managed manifest, structured relationships, versions, and
  audit checkpoint.

The existing database-only backup and data-directory archive are foundations,
not sufficient implementations of an identity-aware workspace backup.

## Consequences

### Positive

- Existing vaults can be mounted without creating hidden MindVault files.
- Manifest state uses existing SQLite transactions and is designed to use the
  established sealed-payload mechanism for sensitive fields.
- All write-capable callers share one path and concurrency policy.
- Document IDs survive ordinary moves without making frontmatter proprietary.
- The portability/encryption tradeoff is visible rather than hidden.

### Costs and risks

- A plain directory copy does not preserve MindVault-only IDs or relationships.
- External rename matching remains probabilistic when platform file IDs are
  unavailable and content changes during the move.
- The workspace API adds a second mutation surface that must coexist with the
  generic node API during migration.
- Consistent backup spans a filesystem snapshot and a database checkpoint.
- Existing audit and node-version implementations are insufficient for the
  durable workspace journal and version contract.

## Implementation conformance gates

The decision is accepted. A workspace writer may ship only when:

1. the logical manifest schema has migration and rollback tests;
2. projected document nodes cannot be mutated through generic node endpoints;
3. path and authorization policies are shared by every write-capable adapter;
4. expected-hash conflicts are proven to leave canonical files unchanged;
5. identity-aware backup and restore preserve IDs across a round trip;
6. the UI accurately communicates plaintext and sealed-store boundaries.
