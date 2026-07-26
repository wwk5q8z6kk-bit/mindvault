# ADR 002: File-First Human Documents

Status: proposed

## Problem

Human-authored Markdown must remain readable and exportable, while SQLite must manage identity,
policy, versions, and retrieval coordination.

## Current state

Obsidian/Markdown import parses files into SQLite using path-derived source and mtime. Original
bytes and rename/delete history are not preserved as immutable evidence.

## Alternatives

1. SQLite-only content;
2. managed Markdown-only repository;
3. registered external files plus SQLite metadata/versions/evidence;
4. bidirectional automatic file/database editing.

## Measurements required

Scan/import throughput, hash cost, rename detection, watcher churn, large-vault startup, round-trip
fidelity, and conflict behavior.

## Security implications

File roots require symlink/path traversal controls, security-scoped access, parser sandboxing, and
no write permission for the first slice.

## Privacy implications

Paths and frontmatter are sensitive metadata. Preserve locally and disclose only selected spans.

## Migration implications

Register source roots, hash/copy original bytes into immutable evidence, link existing imported
nodes by source, and never modify source files during migration.

## Chosen direction

Human files own human-authored bytes. SQLite owns stable identity, provenance, temporal versions,
policy, proposals, and audit. The first slice is read-only ingestion; write-back requires a later
ADR.

## Rejected alternatives

SQLite-only violates readability. Managed-only files cannot efficiently own policy/audit. Immediate
bidirectional editing adds unsafe conflict semantics before versioning is proven.

## Reversal path

Unregister the source and delete derived projections; original files are untouched and evidence can
be exported by hash.

## Acceptance criteria

Source bytes never change, every observation has a hash/time/parser version, rename/delete is
explicit, import resumes idempotently, and an open export reconstructs readable files.
