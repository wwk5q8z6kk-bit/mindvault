-- Governed identity registry replacing the transitional local-system derivation.
--
-- Stable identity fields are indexed in plaintext. The complete portable
-- record is stored as JSON or as an encrypted mvenc-v1 payload. Every update
-- advances one revision and archives the prior record.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_identity_records (
    principal_id           TEXT PRIMARY KEY,
    revision               INTEGER NOT NULL CHECK (revision > 0),
    principal_uri          TEXT NOT NULL UNIQUE,
    governing_node_uri     TEXT NOT NULL,
    subject_binding        TEXT NOT NULL,
    subject_binding_digest TEXT NOT NULL
        CHECK (
            length(subject_binding_digest) = 64
            AND subject_binding_digest NOT GLOB '*[^0-9a-f]*'
        ),
    actor_kind             TEXT NOT NULL
        CHECK (actor_kind IN ('human', 'agent', 'service', 'integration')),
    status                 TEXT NOT NULL
        CHECK (status IN ('active', 'suspended', 'revoked')),
    record_digest          TEXT NOT NULL
        CHECK (
            length(record_digest) = 64
            AND record_digest NOT GLOB '*[^0-9a-f]*'
        ),
    record_payload         BLOB NOT NULL,
    payload_format         TEXT NOT NULL CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek    TEXT,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL,
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    ),
    UNIQUE (governing_node_uri, subject_binding_digest)
);

CREATE INDEX IF NOT EXISTS idx_interoperability_identity_records_governance
    ON interoperability_identity_records (governing_node_uri, status, updated_at);

CREATE INDEX IF NOT EXISTS idx_interoperability_identity_records_subject
    ON interoperability_identity_records (governing_node_uri, subject_binding);

CREATE TABLE IF NOT EXISTS interoperability_identity_history (
    principal_id           TEXT NOT NULL,
    revision               INTEGER NOT NULL CHECK (revision > 0),
    principal_uri          TEXT NOT NULL,
    governing_node_uri     TEXT NOT NULL,
    subject_binding        TEXT NOT NULL,
    subject_binding_digest TEXT NOT NULL,
    actor_kind             TEXT NOT NULL,
    status                 TEXT NOT NULL,
    record_digest          TEXT NOT NULL,
    record_payload         BLOB NOT NULL,
    payload_format         TEXT NOT NULL,
    payload_wrapped_dek    TEXT,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL,
    archived_at            TEXT NOT NULL,
    PRIMARY KEY (principal_id, revision)
);

CREATE TRIGGER IF NOT EXISTS validate_interoperability_identity_update
BEFORE UPDATE ON interoperability_identity_records
FOR EACH ROW
WHEN
    NEW.principal_id != OLD.principal_id
    OR NEW.principal_uri != OLD.principal_uri
    OR NEW.governing_node_uri != OLD.governing_node_uri
    OR NEW.subject_binding != OLD.subject_binding
    OR NEW.subject_binding_digest != OLD.subject_binding_digest
    OR NEW.created_at != OLD.created_at
    OR NEW.revision != OLD.revision + 1
    OR NEW.updated_at < OLD.updated_at
BEGIN
    SELECT RAISE(ABORT, 'invalid identity revision or immutable field change');
END;

CREATE TRIGGER IF NOT EXISTS archive_interoperability_identity_update
BEFORE UPDATE ON interoperability_identity_records
FOR EACH ROW
BEGIN
    INSERT INTO interoperability_identity_history (
        principal_id,
        revision,
        principal_uri,
        governing_node_uri,
        subject_binding,
        subject_binding_digest,
        actor_kind,
        status,
        record_digest,
        record_payload,
        payload_format,
        payload_wrapped_dek,
        created_at,
        updated_at,
        archived_at
    ) VALUES (
        OLD.principal_id,
        OLD.revision,
        OLD.principal_uri,
        OLD.governing_node_uri,
        OLD.subject_binding,
        OLD.subject_binding_digest,
        OLD.actor_kind,
        OLD.status,
        OLD.record_digest,
        OLD.record_payload,
        OLD.payload_format,
        OLD.payload_wrapped_dek,
        OLD.created_at,
        OLD.updated_at,
        datetime('now')
    );
END;

CREATE TRIGGER IF NOT EXISTS prevent_interoperability_identity_delete
BEFORE DELETE ON interoperability_identity_records
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'identity records cannot be deleted');
END;

INSERT OR IGNORE INTO interoperability_public_schemas (
    schema_uri, schema_version, media_type, definition_json, content_digest,
    lifecycle, owner_uri, created_at, deprecated_at
) VALUES
(
    'mindvault://schemas/identity-registered',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["principal_id","actor_kind","status","record_digest","subject_binding_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.identity.registered.v1"}',
    '4da4891e0ffe833dbab1c278b2f170cb925bd1d0e3a2b97221cfde7693ade2c1',
    'active',
    'mindvault://schemas/governance',
    '2026-07-30T00:00:00Z',
    NULL
);

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (40, datetime('now'));

COMMIT;
