# Knowledge Workspace Migration and Safety Plan

## Scope

This plan moves current database-backed Notes into a file-first Library without
destroying source data, changing authority prematurely, or hiding unsupported
metadata.

It also defines the safety boundary for the future workspace service. It is a
pre-implementation contract, not authorization to migrate a real vault.

ADR 008 and ADR 009 are accepted. Portable naming and byte-preservation rules
come from `knowledge-workspace-document-contract.md`; manifest state uses the
executable design in `knowledge-workspace-manifest-v1.sql`; accepted fixtures
live under `fixtures/knowledge-workspace/`.

## Migration invariants

1. The current database remains authoritative until a verified commit.
2. Dry run and staging never mutate source notes.
3. Generated files are written to a new, empty staging root.
4. Every source node has one terminal result: staged, intentionally excluded, or
   failed with a reason.
5. Counts alone are insufficient; content hashes and mapping records are
   required.
6. No source note is deleted by migration.
7. A failed apply or verification can be retried idempotently.
8. Commit is explicit and rollback remains available.
9. Structured system data is not dumped into frontmatter merely to claim
   completeness.
10. AI does not choose filenames, merge collisions, or rewrite content during
    migration.

## Source set

The initial source set matches the current Notes surface:

- `fact`;
- `decision`;
- `procedure`;
- `observation`;
- `preference`;
- `concept`.

Nodes of other kinds remain structured records unless a later mapping is
approved. A migration preview must report all excluded kinds and counts.

## Field mapping

| Source field | File-first target | Rule |
| --- | --- | --- |
| Node ID | Manifest document ID | Preserve the UUID when valid and unique. |
| Namespace | Top-level migration folder or workspace mapping | Use an explicit user-selected rule; never infer cross-namespace access. |
| Title | Filename plus optional `title` property | Sanitize for portability; preserve the exact title when filename differs. |
| Content | Markdown body | Preserve bytes after line-ending normalization is explicitly chosen; default is no semantic rewrite. |
| Tags | `tags` frontmatter | Preserve order only if source semantics require it; deduplicate exact values. |
| Source | Provenance record | Do not expose internal source URIs as visible frontmatter by default. |
| Importance | Structured document property | Include in frontmatter only through a user-approved property schema. |
| User metadata | Frontmatter or structured property | Map only recognized user-owned keys; report all others. |
| System metadata | Manifest/version/audit stores | Never serialize version snapshots or internal control state into the document. |
| Relationships | Structured graph plus safe Markdown link rewrites | Rewrite only when both endpoints map unambiguously. |
| Attachments | Workspace attachment location | Copy with hashes; preserve source until verification. |
| Version history | Document version store | Convert through a version-specific migrator; do not embed snapshots in Markdown. |

## Deterministic path plan

The dry run produces the complete path map before writing.

Default layout:

```text
<staging-root>/
  Imported from MindVault/
    <namespace>/
      <sanitized-title>.md
```

Rules:

- normalize the namespace and title through one portable filename policy;
- replace prohibited characters without dropping the original title;
- use `Untitled` only when no meaningful title exists;
- compare proposed paths under case-folded and Unicode-normalized forms;
- resolve collisions deterministically with ` (<short-document-id>)`;
- reserve all MindVault-managed names and the staging report directory;
- enforce the approved path length and depth budget before apply;
- never calculate a destination outside the staging root.

The final report includes every source ID, proposed relative path, collision
decision, and warning.

## Migration states

```text
planned
  → staged
  → verified
  → committed
  → compatibility

planned/staged/verified
  → failed
  → retry

committed
  → rolled_back
```

Each transition is durable and idempotent. A migration ID and plan hash bind dry
run output to the apply request; changing rules requires a new dry run.

## Phase A — dry run

The migrator:

1. freezes a logical source snapshot without blocking ordinary reads;
2. enumerates eligible notes, attachments, relationships, and versions;
3. validates IDs and namespace authorization;
4. computes deterministic paths and collision outcomes;
5. classifies metadata as user-owned, structured, system, or unknown;
6. parses links and reports ambiguous or unresolved targets;
7. computes estimated file count and bytes;
8. emits a content-addressed plan and human-readable report.

Dry run fails the gate when it finds:

- duplicate source IDs;
- an unrepresentable path without a deterministic fallback;
- missing attachment bytes;
- metadata that would be silently dropped;
- a relationship rewrite that changes target meaning;
- a source read error;
- insufficient target capacity.

Warnings may be accepted only through an explicit plan revision or waiver.

## Phase B — stage

Apply requires the exact dry-run plan hash.

For each document:

1. create parent directories through the path policy;
2. render recognized frontmatter and the original body;
3. write atomically to a new file;
4. hash and re-read the resulting bytes;
5. record source ID, destination path, source/content hashes, and status;
6. copy and verify attachments;
7. build manifest entries and derived projections only after file verification.

Staging does not update current Notes nodes. Partial output remains isolated and
can be resumed or discarded.

## Phase C — verify

Verification compares:

- eligible source count against terminal migration results;
- source body against rendered document body;
- attachment source and destination hashes;
- title, tags, recognized properties, and provenance;
- relationship endpoints and unresolved-link reports;
- version counts and restore samples;
- exact and semantic retrieval coverage;
- projected document IDs and paths;
- full projection rebuild results.

At least one deterministic fixture must cover duplicate titles, Unicode,
reserved names, empty titles, frontmatter-like bodies, unsupported metadata,
missing attachments, dangling links, cycles, and maximum-size content.

A verification report is immutable and references the migration plan hash and
staged workspace hash.

## Phase D — commit

Commit is an explicit authority switch:

1. require a successful verification report with no unwaived errors;
2. record a database checkpoint and identity-aware backup;
3. mark migrated source nodes read-only compatibility records;
4. activate the staged workspace as the Library authority;
5. route document reads/writes through `WorkspaceService`;
6. retain source-to-document mappings;
7. start reconciliation and verify a clean projection state.

The source records remain present. Generic update/delete endpoints reject
mutations to migrated compatibility records.

## Phase E — rollback

Rollback:

1. stops workspace writes and completes or cancels in-flight journal entries;
2. checks whether files changed after commit;
3. exports post-commit file changes instead of discarding them;
4. restores database Notes authority from the checkpoint;
5. returns source nodes to their previous mutable state;
6. preserves the staged workspace, mappings, and audit history for inspection.

Rollback is automatic only when no post-commit file change exists. Otherwise it
requires an explicit reconciliation choice so new user writing is never lost.

## Workspace safety boundary

### Trusted components

- the authenticated user operating the local app;
- `WorkspaceService` after authorization and path-policy checks;
- the managed SQLite manifest and journal while unsealed;
- verified application binaries and migrations.

### Untrusted or conditionally trusted inputs

- all relative paths and names from UI/API/integrations;
- external filesystem events;
- Markdown, frontmatter, HTML, links, embeds, and attachments;
- Obsidian plugins or other editors modifying the same root;
- MCP, plugins, automations, and AI proposals;
- imported archives and migration plans;
- platform file identity and timestamps as hints.

### Required controls

| Risk | Required control |
| --- | --- |
| Root escape through traversal | Reject absolute paths, `..`, nulls, invalid components, and post-resolution paths outside the canonical root. |
| Symlink/junction escape | Do not follow links during normal traversal; verify parent and target immediately before mutation using platform-safe handles where available. |
| Case/Unicode aliasing | Detect collisions under portable normalized comparison before writing. |
| Time-of-check/time-of-use race | Bind operations to stable parent handles where supported and revalidate before atomic replacement. |
| External overwrite | Require expected content hash; preserve both versions on mismatch. |
| Event loss/reordering | Treat events as hints and reconcile against a scan plus journal. |
| Parser or index denial of service | Enforce file, frontmatter, link, nesting, batch, and processing-time budgets; isolate per-document failures. |
| Active Markdown/HTML content | Render untrusted content in a restricted surface; do not execute scripts or unsafe URLs. |
| Unauthorized subtree access | Authorize workspace and applicable subtree for every operation, not only at tree load. |
| Plugin/AI path abuse | Expose typed operations with enumerated targets; never an arbitrary filesystem-write tool. |
| Audit tampering or loss | Store mandatory mutation events durably with correlation IDs and before/after hashes. |
| Plaintext disclosure | State that portable Markdown is plaintext; rely on OS/volume encryption or a separate non-portable encrypted mode. |
| Destructive migration | Stage in a new root, verify, checkpoint, require commit, retain sources, and prove rollback. |

## API and authorization migration

The workspace API is introduced alongside generic node APIs.

- Existing Notes keep using node endpoints before commit.
- Read-only workspace projections may be returned through search and graph
  endpoints.
- Projected documents carry a marker that makes generic mutation fail closed.
- Workspace permissions begin by mapping existing namespace access to explicit
  workspace access; folder-subtree grants are added only with deny/allow tests.
- MCP and plugins receive read-only workspace capabilities first.
- Write capabilities are added only after expected-hash, audit, and rollback
  tests pass for that adapter.

## Implementation unlock checklist

The workspace-specific design gate is complete:

- [x] ADR 008 and ADR 009 are accepted.
- [x] The current-state baseline is reviewed against the current branch.
- [x] Portable filename/path limits are selected for supported platforms.
- [x] A lossless Markdown preservation fixture is approved.
- [x] The managed manifest contract and production rollback policy are
  specified.
- [x] The plaintext/sealed-mode boundary is approved and reflected in security
  and future UI copy.
- [x] The migration fixture and pass/fail report schema are approved.
- [x] The full first-v2 vertical-slice acceptance contract remains unchanged.

The read-only workspace domain and managed-manifest foundation is implemented
by `mv-core`, `mv-storage`, and migration 031. It does not mount or mutate a
user vault. The next slice remains path policy and read-only reconciliation—not
the folder-tree UI or writable behavior.

Writable behavior may be enabled for an adapter only when executable tests
prove:

- [ ] generic node mutation fails closed for projected documents;
- [ ] all path-capable adapters share path and authorization policy;
- [ ] stale expected hashes leave canonical bytes unchanged;
- [ ] prepare/apply/finalize recovery converges after interruption;
- [ ] identity-aware backup and restore preserve document IDs;
- [ ] the adapter preserves the approved Markdown and migration fixtures.
