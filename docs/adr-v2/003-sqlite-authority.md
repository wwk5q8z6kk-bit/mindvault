# ADR 003: SQLite Authority

Status: proposed

## Problem

MindVault needs portable local transactions and recovery while supporting multiple derived search
systems.

## Current state

SQLite stores most canonical data; Tantivy and LanceDB are updated imperatively after SQLite.
Keychain uses a second SQLite database and migrations are inconsistent.

## Alternatives

1. keep SQLite;
2. PostgreSQL;
3. embedded document database;
4. append-only files as sole authority.

## Measurements required

Write contention, p99 canonical reads/writes, migration/backup duration, database size, corruption
recovery, and M-dataset load with WAL.

## Security implications

SQLite enables a narrow transaction boundary; file permissions, SQLCipher/sealed payload design,
backup keys, and extension loading must be controlled.

## Privacy implications

Local storage minimizes disclosure. Backups, WAL, temp files, and metadata remain sensitive.

## Migration implications

Add checksummed migrations, canonical/outbox tables, online backup, staged restore, and foreign-key
validation. Do not merge keychain secrets into general content tables.

## Chosen direction

SQLite remains the canonical structured authority. Human files and immutable evidence bytes are
co-canonical by responsibility; indexes are not.

## Rejected alternatives

PostgreSQL harms zero-admin local operation. A new embedded database has no measured benefit.
File-only authority cannot provide the required atomic policy/proposal/audit transaction.

## Reversal path

The versioned open export and repository ports permit a future database adapter; retain no
SQLite-specific type in public domain contracts.

## Acceptance criteria

Atomic canonical mutation+audit+outbox, consistent online backup, integrity/foreign-key tests,
resume-safe migrations, and S/M benchmark budgets pass.
