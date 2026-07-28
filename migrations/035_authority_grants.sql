-- Governed Context and Tool Grants.
--
-- Grant terms are immutable after issuance. Lifecycle transitions archive the
-- prior revision, and parent-grant effectiveness is evaluated at authorization
-- time so suspension, revocation, or expiry fail closed for all descendants.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_authority_grants (
    grant_id                       TEXT PRIMARY KEY,
    revision                       INTEGER NOT NULL CHECK (revision > 0),
    grant_uri                      TEXT NOT NULL UNIQUE,
    grant_kind                     TEXT NOT NULL CHECK (grant_kind IN ('context', 'tool')),
    grantor_uri                    TEXT NOT NULL,
    grantee_uri                    TEXT NOT NULL,
    governing_node_uri             TEXT NOT NULL,
    status                         TEXT NOT NULL
        CHECK (status IN ('active', 'suspended', 'revoked', 'expired')),
    parent_grant_id                TEXT
        REFERENCES interoperability_authority_grants(grant_id),
    not_before                     TEXT NOT NULL,
    expires_at                     TEXT NOT NULL,
    record_payload                 BLOB NOT NULL,
    payload_format                 TEXT NOT NULL CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek            TEXT,
    created_at                     TEXT NOT NULL,
    updated_at                     TEXT NOT NULL,
    CHECK (grantor_uri != grantee_uri),
    CHECK (expires_at > not_before),
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_interoperability_authority_grants_grantee
    ON interoperability_authority_grants
       (grantee_uri, grant_kind, status, not_before, expires_at);

CREATE INDEX IF NOT EXISTS idx_interoperability_authority_grants_governance
    ON interoperability_authority_grants (governing_node_uri, status, updated_at);

CREATE INDEX IF NOT EXISTS idx_interoperability_authority_grants_parent
    ON interoperability_authority_grants (parent_grant_id);

CREATE TABLE IF NOT EXISTS interoperability_authority_grant_history (
    grant_id                       TEXT NOT NULL,
    revision                       INTEGER NOT NULL CHECK (revision > 0),
    grant_uri                      TEXT NOT NULL,
    grant_kind                     TEXT NOT NULL,
    grantor_uri                    TEXT NOT NULL,
    grantee_uri                    TEXT NOT NULL,
    governing_node_uri             TEXT NOT NULL,
    status                         TEXT NOT NULL,
    parent_grant_id                TEXT,
    not_before                     TEXT NOT NULL,
    expires_at                     TEXT NOT NULL,
    record_payload                 BLOB NOT NULL,
    payload_format                 TEXT NOT NULL,
    payload_wrapped_dek            TEXT,
    created_at                     TEXT NOT NULL,
    updated_at                     TEXT NOT NULL,
    archived_at                    TEXT NOT NULL,
    PRIMARY KEY (grant_id, revision)
);

CREATE TRIGGER IF NOT EXISTS validate_interoperability_authority_grant_update
BEFORE UPDATE ON interoperability_authority_grants
FOR EACH ROW
WHEN
    NEW.grant_id != OLD.grant_id
    OR NEW.grant_uri != OLD.grant_uri
    OR NEW.grant_kind != OLD.grant_kind
    OR NEW.grantor_uri != OLD.grantor_uri
    OR NEW.grantee_uri != OLD.grantee_uri
    OR NEW.governing_node_uri != OLD.governing_node_uri
    OR NEW.parent_grant_id IS NOT OLD.parent_grant_id
    OR NEW.not_before != OLD.not_before
    OR NEW.expires_at != OLD.expires_at
    OR NEW.created_at != OLD.created_at
    OR NEW.revision != OLD.revision + 1
    OR NEW.updated_at < OLD.updated_at
    OR (
        NEW.status != OLD.status
        AND NOT (
            (OLD.status = 'active' AND NEW.status IN ('suspended', 'revoked', 'expired'))
            OR (OLD.status = 'suspended' AND NEW.status IN ('active', 'revoked', 'expired'))
        )
    )
    OR NEW.status = OLD.status
BEGIN
    SELECT RAISE(ABORT, 'invalid authority-grant revision or lifecycle transition');
END;

CREATE TRIGGER IF NOT EXISTS archive_interoperability_authority_grant_update
BEFORE UPDATE ON interoperability_authority_grants
FOR EACH ROW
BEGIN
    INSERT INTO interoperability_authority_grant_history (
        grant_id,
        revision,
        grant_uri,
        grant_kind,
        grantor_uri,
        grantee_uri,
        governing_node_uri,
        status,
        parent_grant_id,
        not_before,
        expires_at,
        record_payload,
        payload_format,
        payload_wrapped_dek,
        created_at,
        updated_at,
        archived_at
    ) VALUES (
        OLD.grant_id,
        OLD.revision,
        OLD.grant_uri,
        OLD.grant_kind,
        OLD.grantor_uri,
        OLD.grantee_uri,
        OLD.governing_node_uri,
        OLD.status,
        OLD.parent_grant_id,
        OLD.not_before,
        OLD.expires_at,
        OLD.record_payload,
        OLD.payload_format,
        OLD.payload_wrapped_dek,
        OLD.created_at,
        OLD.updated_at,
        datetime('now')
    );
END;

CREATE TRIGGER IF NOT EXISTS prevent_interoperability_authority_grant_delete
BEFORE DELETE ON interoperability_authority_grants
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'authority-grant records cannot be deleted');
END;

INSERT OR IGNORE INTO interoperability_public_schemas (
    schema_uri, schema_version, media_type, definition_json, content_digest,
    lifecycle, owner_uri, created_at, deprecated_at
) VALUES
(
    'mindvault://schemas/authority-grant-issued',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["grant_id","grant_kind","grantee_uri","governing_node_uri","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.authority-grant.issued.v1"}',
    '1ed973b0556d674b7264404b7ac7753bfc2ee184a90cd355a7a4c8deb21588b2',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/authority-grant-lifecycle-transitioned',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["grant_id","revision","from_status","to_status","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.authority-grant.lifecycle.transitioned.v1"}',
    '28c1e9cecc96c905b5170b37eda12ee3cb8c92a8e8145e353e2e5cb4c30f1a75',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
);

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (35, datetime('now'));

COMMIT;
