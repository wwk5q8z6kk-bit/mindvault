# Knowledge Workspace Document Contract

- **Status:** Accepted design contract
- **Date:** 2026-07-26
- **Applies to:** Mounted and MindVault-managed plaintext Markdown workspaces
- **Related:** ADR 008, ADR 009, `knowledge-workspace.md`

## Purpose

This contract closes the document-format, portability, history, link, trash,
and external-conflict decisions required before workspace foundation work.
It is intentionally conservative: MindVault may read a wider range of existing
vaults, but it creates names and writes content only within the portable profile
below.

## Compared policies

| Policy | Expected outcome | Downside risk | Reversibility | Execution cost |
|---|---|---|---|---|
| Host-native limits | Maximum freedom on the current device | Vaults can fail on another OS or sync provider | Low after nonportable names spread | Low |
| Common portable profile | Predictable macOS, Linux, Windows, Git, and editor behavior | Some valid host-native names cannot be created in MindVault | High; limits can be relaxed later | Moderate |
| ASCII-only names | Broad compatibility | Damages international and human-readable organization | Low; original names are lost | Low |

MindVault selects the **common portable profile**. It preserves Unicode and
ordinary spaces while reserving enough margin for cross-platform tooling.
Its stable policy identifier is `portable-v1`.

## Portable creation profile

These are product limits, not claims about every host filesystem's maximum.

- Paths are relative to one configured workspace root.
- New document extensions are `.md`; existing `.markdown` files are readable.
- New names are UTF-8 and normalized to NFC before creation.
- Each path component is at most 120 UTF-8 bytes.
- The normalized relative path is at most 240 UTF-8 bytes.
- Directory depth is at most 32 components.
- Empty components, `.` and `..` are rejected.
- ASCII control characters and `< > : " / \ | ? *` are rejected.
- Components ending in a space or period are rejected.
- `CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, and `LPT1` through `LPT9`
  are reserved case-insensitively, including when followed by an extension.
- `.mindvault` and names beginning `.mindvault-` are reserved for policy
  compatibility even though the initial design writes no database into a
  mounted workspace.
- Collisions are evaluated with exact, case-folded, and Unicode-normalized path
  keys. A collision under any supported target profile blocks creation.
- The resolved absolute path must also satisfy the active host filesystem.
- Symlinks may be observed but are never traversed outside the configured root.

An existing nonportable path is not silently renamed. It is indexed read-only
when safe and shown with a portability warning. Unsafe paths are quarantined
from content access and reported.

### Deterministic migration naming

Legacy-note migration derives a filename from the exact title:

1. trim leading and trailing whitespace;
2. replace each forbidden character with `-`;
3. remove trailing spaces and periods;
4. prefix `_` when the result is a reserved device name;
5. use `Untitled` when nothing remains;
6. append `.md`;
7. on collision or length overflow, append `--<source-id-prefix>`, where the
   prefix is the first eight lowercase hexadecimal UUID characters, truncating
   the human-readable portion at a Unicode scalar boundary when required.

The migration plan records both the original title and chosen path. It never
changes a path between dry-run and apply unless a new plan hash is approved.

## Byte-preserving Markdown

The canonical artifact is the file's byte sequence, not a parser's syntax tree.

- A no-op ingest/project/render cycle must produce a byte-identical file.
- Indexing and preview are read-only projections and never rewrite source.
- Unknown frontmatter keys, key order, comments, scalar style, aliases, line
  endings, whitespace, HTML, fenced blocks, math, wikilinks, embeds, callouts,
  block IDs, and unsupported extension syntax are preserved.
- The content hash covers the exact bytes, including BOM and line endings.
- Existing UTF-8 BOM and CRLF are preserved. New files default to UTF-8 without
  BOM and LF line endings.
- Invalid UTF-8 is reported as unsupported and is not rewritten.
- A user editing raw Markdown may intentionally replace the full byte sequence;
  expected-hash concurrency still applies.
- Structured property editing remains disabled until a concrete YAML
  representation-preserving implementation passes the preservation fixture.

The application may parse a recognized subset of properties for projections,
but parsed values never become the writeback source of truth.

## Frontmatter projection

The first recognized property set is:

- `title`
- `aliases`
- `tags`
- `type`
- `created`
- `updated`

Recognition is case-sensitive. Unknown keys remain queryable as untyped
properties when safely parsed, but MindVault does not normalize or reorder them.
Malformed frontmatter leaves the entire file canonical and produces a
diagnostic; it does not discard the document body.

MindVault does not inject a document ID into frontmatter by default.

## Links, blocks, and citations

- Standard Markdown links and Obsidian wikilinks are indexed.
- Wikilink aliases and embeds remain source syntax; target resolution is a
  projection.
- Explicit Obsidian block IDs such as `^decision-anchor` are stable anchors.
- Heading fragments are convenience locators, not permanent identity, because
  headings can be renamed or duplicated.
- MindVault does not inject block IDs during initial indexing.
- Citation identity is `(workspace_id, document_id, content_hash, byte_range)`.
  A human-readable path and heading may accompany it but are not authoritative.
- A citation against an older revision resolves through immutable evidence or a
  retained version snapshot, never by pretending the current bytes are equal.
- Unresolved and ambiguous links are visible diagnostics.

## Versions and retention

Version snapshots are content-addressed blobs in MindVault-managed storage.

- Every successful MindVault write records the before-image.
- External changes record the first observed new revision and retain the prior
  known revision when available.
- Identical content is deduplicated by cryptographic content hash.
- Initial releases perform no silent version-count or age pruning.
- Pruning is a separate, explicit, auditable operation with a dry-run report.
- Evidence referenced by a citation, accepted proposal, unresolved conflict, or
  migration checkpoint is pinned and cannot be pruned.
- Sealed mode encrypts snapshot content and sensitive version metadata.

This replaces the current node-local 40-version cap for canonical documents.

## Recoverable trash

Trash is managed outside the mounted document root:

1. record a prepared delete event and a verified content snapshot;
2. copy the file into managed version storage when it is not already present;
3. verify the stored bytes and hash;
4. delete the canonical path;
5. mark the document `trashed` and complete the event.

Restore writes through the same expected-hash and path policy as creation. If
the original path is occupied, restore requires a new destination or explicit
conflict resolution.

Initial releases do not purge trash automatically. Operating-system trash
integration can be added later but cannot replace the managed recovery proof.

## External and multi-device conflicts

MindVault does not implement a sync provider in the first workspace slice.
External sync tools may modify files while MindVault is stopped or running.

- Watcher events trigger reconciliation; they are not authoritative.
- A write based on a stale content hash fails closed.
- Two different contents claiming the same normalized path create a conflict.
- Ambiguous rename evidence creates a review item.
- MindVault never applies silent last-writer-wins to canonical documents.
- The initial conflict resolution choices are keep current, restore known
  revision, save the competing revision to a new portable path, or merge through
  an explicitly reviewed proposal.

Provider-specific vector clocks or file IDs may improve diagnosis later, but
the hash-based safety rule remains provider-independent.

## Contract fixtures

`docs/architecture/fixtures/knowledge-workspace/` contains:

- the byte-preservation Markdown fixture;
- representative legacy nodes;
- deterministic expected migrated files and plan;
- the migration report JSON Schema and an accepted example report.

Any parser, migration, or write-path implementation must run these fixtures
without modifying their expected artifacts.
