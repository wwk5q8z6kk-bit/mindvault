-- Governed Context Node and capability-descriptor registry.
--
-- Stable identity fields are indexed in plaintext. The complete portable
-- descriptor, including endpoints and public keys, is stored as JSON or as an
-- encrypted mvenc-v1 payload. Every update advances one revision and archives
-- the prior descriptor.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_context_nodes (
    node_id               TEXT PRIMARY KEY,
    revision              INTEGER NOT NULL CHECK (revision > 0),
    node_uri              TEXT NOT NULL UNIQUE,
    node_type             TEXT NOT NULL
        CHECK (
            node_type IN (
                'personal', 'device', 'space', 'organization', 'application',
                'agent', 'storage', 'execution', 'relay', 'index'
            )
        ),
    owner_actor_uri       TEXT NOT NULL,
    governing_node_uri    TEXT NOT NULL,
    trust_class           TEXT NOT NULL,
    status                TEXT NOT NULL
        CHECK (
            status IN (
                'discovered', 'pending_trust', 'active', 'suspended',
                'revoked', 'retired'
            )
        ),
    capability_digest     TEXT NOT NULL
        CHECK (
            length(capability_digest) = 64
            AND capability_digest NOT GLOB '*[^0-9a-f]*'
        ),
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

CREATE INDEX IF NOT EXISTS idx_interoperability_context_nodes_status
    ON interoperability_context_nodes (status, node_type, updated_at);

CREATE INDEX IF NOT EXISTS idx_interoperability_context_nodes_governance
    ON interoperability_context_nodes (governing_node_uri, status);

CREATE TABLE IF NOT EXISTS interoperability_context_node_history (
    node_id               TEXT NOT NULL,
    revision              INTEGER NOT NULL CHECK (revision > 0),
    node_uri              TEXT NOT NULL,
    node_type             TEXT NOT NULL,
    owner_actor_uri       TEXT NOT NULL,
    governing_node_uri    TEXT NOT NULL,
    trust_class           TEXT NOT NULL,
    status                TEXT NOT NULL,
    capability_digest     TEXT NOT NULL,
    record_payload        BLOB NOT NULL,
    payload_format        TEXT NOT NULL,
    payload_wrapped_dek   TEXT,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    archived_at           TEXT NOT NULL,
    PRIMARY KEY (node_id, revision)
);

CREATE TRIGGER IF NOT EXISTS validate_interoperability_context_node_update
BEFORE UPDATE ON interoperability_context_nodes
FOR EACH ROW
WHEN
    NEW.node_id != OLD.node_id
    OR NEW.node_uri != OLD.node_uri
    OR NEW.node_type != OLD.node_type
    OR NEW.owner_actor_uri != OLD.owner_actor_uri
    OR NEW.governing_node_uri != OLD.governing_node_uri
    OR NEW.created_at != OLD.created_at
    OR NEW.revision != OLD.revision + 1
    OR NEW.updated_at < OLD.updated_at
    OR (
        NEW.status != OLD.status
        AND NOT (
            (OLD.status = 'discovered' AND NEW.status = 'pending_trust')
            OR (OLD.status = 'pending_trust' AND NEW.status IN ('active', 'revoked'))
            OR (OLD.status = 'active' AND NEW.status = 'suspended')
            OR (OLD.status = 'suspended' AND NEW.status IN ('active', 'revoked'))
            OR (OLD.status = 'revoked' AND NEW.status = 'retired')
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'invalid context-node revision or lifecycle transition');
END;

CREATE TRIGGER IF NOT EXISTS archive_interoperability_context_node_update
BEFORE UPDATE ON interoperability_context_nodes
FOR EACH ROW
BEGIN
    INSERT INTO interoperability_context_node_history (
        node_id,
        revision,
        node_uri,
        node_type,
        owner_actor_uri,
        governing_node_uri,
        trust_class,
        status,
        capability_digest,
        record_payload,
        payload_format,
        payload_wrapped_dek,
        created_at,
        updated_at,
        archived_at
    ) VALUES (
        OLD.node_id,
        OLD.revision,
        OLD.node_uri,
        OLD.node_type,
        OLD.owner_actor_uri,
        OLD.governing_node_uri,
        OLD.trust_class,
        OLD.status,
        OLD.capability_digest,
        OLD.record_payload,
        OLD.payload_format,
        OLD.payload_wrapped_dek,
        OLD.created_at,
        OLD.updated_at,
        datetime('now')
    );
END;

CREATE TRIGGER IF NOT EXISTS prevent_interoperability_context_node_delete
BEFORE DELETE ON interoperability_context_nodes
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'context-node records cannot be deleted');
END;

INSERT OR IGNORE INTO interoperability_public_schemas (
    schema_uri, schema_version, media_type, definition_json, content_digest,
    lifecycle, owner_uri, created_at, deprecated_at
) VALUES
(
    'mindvault://schemas/context-node-registered',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["node_id","node_type","status","record_digest","capability_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.context-node.registered.v1"}',
    'cc2d8b2bc0e3bf376ea8ee0661308fbb639735af2f4dad069f84138a4bff8127',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/context-node-descriptor-updated',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["node_id","revision","record_digest","capability_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.context-node.descriptor.updated.v1"}',
    '9cb18240d4ab1e0101fe14cbb6923a38b4eb6f62d5cc8d42945e3dd777bff49e',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/context-node-lifecycle-transitioned',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["node_id","revision","from_status","to_status","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.context-node.lifecycle.transitioned.v1"}',
    '11e3e6c3fa327f652cac723392a187562489453cd19f45d8ef8927fd744462a7',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
);

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (34, datetime('now'));

COMMIT;
