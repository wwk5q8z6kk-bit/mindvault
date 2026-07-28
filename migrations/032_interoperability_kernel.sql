-- Phase 1 interoperability kernel.
--
-- A node mutation and its event envelope are committed in one SQLite
-- transaction. The `(source_uri, principal_uri, idempotency_key)` uniqueness
-- constraint is the durable replay boundary; payload digests distinguish a
-- valid retry from an accidental key collision with different semantics.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_local_identity (
    singleton  INTEGER PRIMARY KEY CHECK (singleton = 1),
    node_id    TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS interoperability_outbox (
    event_id         TEXT PRIMARY KEY,
    source_uri       TEXT NOT NULL,
    principal_uri    TEXT NOT NULL,
    event_type       TEXT NOT NULL,
    subject_uri      TEXT NOT NULL,
    schema_uri       TEXT NOT NULL,
    schema_version   TEXT NOT NULL,
    correlation_id   TEXT NOT NULL,
    causation_id     TEXT,
    idempotency_key  TEXT NOT NULL,
    payload_digest   TEXT NOT NULL
        CHECK (
            length(payload_digest) = 64
            AND payload_digest NOT GLOB '*[^0-9a-f]*'
        ),
    envelope_json    TEXT NOT NULL,
    delivery_state   TEXT NOT NULL DEFAULT 'pending'
        CHECK (delivery_state IN ('pending', 'published', 'dead_letter')),
    delivery_attempts INTEGER NOT NULL DEFAULT 0 CHECK (delivery_attempts >= 0),
    created_at       TEXT NOT NULL,
    published_at     TEXT,
    last_error       TEXT,
    UNIQUE (source_uri, principal_uri, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_interoperability_outbox_delivery
    ON interoperability_outbox (delivery_state, created_at);

CREATE INDEX IF NOT EXISTS idx_interoperability_outbox_subject
    ON interoperability_outbox (subject_uri, created_at);

CREATE INDEX IF NOT EXISTS idx_interoperability_outbox_correlation
    ON interoperability_outbox (correlation_id);

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (32, datetime('now'));

COMMIT;
