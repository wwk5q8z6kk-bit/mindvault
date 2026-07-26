# Data and Migration Audit

## Canonical stores

The legacy system treats `mindvault.sqlite`, `keychain.sqlite`, and attachment/blob files as
canonical. Tantivy and LanceDB are derived in concept but lack a complete rebuild/version protocol.
Obsidian/filesystem sources are imports, not registered canonical documents.

## Schema findings

- Migrations 001–029 are committed; the user's dirty worktree has an untracked migration 030.
- Main-database registration excludes keychain-only migrations.
- Migrations 025, 026, and 027 do not advance `schema_version`; migration 028 later advances to 28.
  A failure or partial application can be hidden by a later max-version marker.
- Keychain migrations run imperatively and tolerate duplicate-column errors by message matching;
  there is no durable per-migration ledger with checksum.
- `node_comments.node_id` has no foreign key, allowing orphan comments.
- Several domain invariants live only in Rust, not schema constraints.
- Migration execution lacks a demonstrated single transaction, checksum validation, preflight
  backup, post-migration foreign-key check, and downgrade path.

## Storage consistency

The ingest order is SQLite → Tantivy commit → LanceDB. Update is equivalent. Delete is Tantivy →
LanceDB best effort → graph → blobs → SQLite. These orders can produce:

- canonical rows without FTS/vector entries;
- stale vector entries;
- removed attachments while the node survives;
- graph/index state from a different node version;
- an API failure after a canonical commit, encouraging unsafe retries.

Sync calls the SQLite node store directly, so imported nodes are not indexed. Attachment upload
writes a blob before its metadata/node update. Proposal approval, execution, audit, and undo
snapshot are not one atomic compare-and-swap transaction.

## Encryption and key data

Sealed node payloads move selected fields into encrypted payload storage, while derived indexes,
attachment extraction text, metadata, logs, backups, and existing rows require independent
verification. `keychain.sqlite` is a separate authority and its loss can make data unrecoverable.
Recovery testing must cover key rotation epochs and partial rotation.

## Backup formats

Two backup concepts diverge:

- database-only `MVBK` encryption reads/writes one SQLite file;
- CLI archive copies a wider data directory.

Neither is a complete, coordinated manifest of files, SQLite snapshots, keys, source documents,
configuration, checksums, schema versions, and rebuild recipes.

## V2 migration invariants

1. Make every migration uniquely identified, ordered, checksummed, transactional where SQLite
   permits, and recorded only after validation.
2. Create a verified full backup before migration and never delete legacy data in the first pass.
3. Add stable document/evidence/version IDs without changing existing IDs.
4. Store original evidence content-addressably; reference it from canonical SQLite rows.
5. Introduce a durable outbox in the same SQLite transaction as canonical changes.
6. Rebuild derived indexes from a snapshot/version watermark.
7. Compare row counts, hashes, foreign keys, evidence reachability, and representative queries.
8. Support dual-read comparison before switching reads; do not dual-write canonical systems.
9. Keep a documented reversal to the untouched legacy data directory.

## Acceptance suite

- migration from every supported schema fixture;
- interrupted migration at each step;
- idempotent resume;
- `integrity_check` and `foreign_key_check`;
- no orphan evidence or documents;
- identical export before/after modulo declared normalization;
- deterministic index rebuild and query smoke;
- full backup/restore on a new machine profile;
- sealed/key-rotation recovery;
- measured time and peak disk/RSS against declared limits.
