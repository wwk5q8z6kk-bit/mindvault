-- Governed identity registry for principals (ADR 010 actor kinds).
--
-- Principal URIs resolve through versioned identity records. External auth
-- subjects map into Actors; they are not Actors themselves. Updates archive the
-- prior revision into history.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_identities (
    principal_id          TEXT PRIMARY KEY,
    revision              INTEGER NOT NULL CHECK (revision > 0),
    principal_uri         TEXT NOT NULL UNIQUE,
    governing_node_uri    TEXT NOT NULL,
    actor_kind            TEXT NOT NULL
        CHECK (actor_kind IN ('human', 'agent', 'service', 'integration')),
    status                TEXT NOT NULL
        CHECK (status IN ('active', 'suspended', 'revoked', 'retired')),
    display_name          TEXT,
    external_subject      TEXT,
    record_payload        BLOB NOT NULL,
    payload_format        TEXT NOT NULL CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek   TEXT,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_interoperability_identities_subject_node
    ON interoperability_identities (governing_node_uri, external_subject)
    WHERE external_subject IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_interoperability_identities_status
    ON interoperability_identities (status, actor_kind, updated_at);

CREATE TABLE IF NOT EXISTS interoperability_identity_history (
    principal_id          TEXT NOT NULL,
    revision              INTEGER NOT NULL CHECK (revision > 0),
    principal_uri         TEXT NOT NULL,
    governing_node_uri    TEXT NOT NULL,
    actor_kind            TEXT NOT NULL,
    status                TEXT NOT NULL,
    display_name          TEXT,
    external_subject      TEXT,
    record_payload        BLOB NOT NULL,
    payload_format        TEXT NOT NULL,
    payload_wrapped_dek   TEXT,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    archived_at           TEXT NOT NULL,
    PRIMARY KEY (principal_id, revision)
);

CREATE TRIGGER IF NOT EXISTS validate_interoperability_identity_update
BEFORE UPDATE ON interoperability_identities
FOR EACH ROW
WHEN
    NEW.principal_id != OLD.principal_id
    OR NEW.principal_uri != OLD.principal_uri
    OR NEW.governing_node_uri != OLD.governing_node_uri
    OR NEW.actor_kind != OLD.actor_kind
    OR NEW.revision != OLD.revision + 1
BEGIN
    SELECT RAISE(ABORT, 'interoperability identity identity fields or revision jump invalid');
END;

CREATE TRIGGER IF NOT EXISTS archive_interoperability_identity_revision
AFTER UPDATE ON interoperability_identities
FOR EACH ROW
WHEN NEW.revision = OLD.revision + 1
BEGIN
    INSERT INTO interoperability_identity_history (
        principal_id, revision, principal_uri, governing_node_uri, actor_kind,
        status, display_name, external_subject, record_payload, payload_format,
        payload_wrapped_dek, created_at, updated_at, archived_at
    ) VALUES (
        OLD.principal_id, OLD.revision, OLD.principal_uri, OLD.governing_node_uri,
        OLD.actor_kind, OLD.status, OLD.display_name, OLD.external_subject,
        OLD.record_payload, OLD.payload_format, OLD.payload_wrapped_dek,
        OLD.created_at, OLD.updated_at, NEW.updated_at
    );
END;

CREATE TRIGGER IF NOT EXISTS forbid_interoperability_identity_delete
BEFORE DELETE ON interoperability_identities
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'interoperability identities cannot be deleted; retire instead');
END;

INSERT INTO schema_version (version, applied_at)
VALUES (40, datetime('now'))
ON CONFLICT(version) DO NOTHING;

COMMIT;
