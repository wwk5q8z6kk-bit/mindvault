-- Governed public-schema and source-authority registries.
--
-- Public schema versions are immutable. Source bindings retain only lookup
-- digests in indexed columns (keyed HMACs in sealed mode); the complete
-- portable record is stored as JSON or as an encrypted mvenc-v1 payload.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_public_schemas (
    schema_uri       TEXT NOT NULL,
    schema_version   TEXT NOT NULL,
    media_type       TEXT NOT NULL CHECK (media_type = 'application/schema+json'),
    definition_json  TEXT NOT NULL
        CHECK (
            json_valid(definition_json)
            AND length(definition_json) <= 1048576
            AND json_extract(definition_json, '$."$schema"')
                = 'https://json-schema.org/draft/2020-12/schema'
        ),
    content_digest   TEXT NOT NULL
        CHECK (
            length(content_digest) = 64
            AND content_digest NOT GLOB '*[^0-9a-f]*'
        ),
    lifecycle        TEXT NOT NULL
        CHECK (lifecycle IN ('active', 'deprecated', 'withdrawn')),
    owner_uri        TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    deprecated_at    TEXT,
    CHECK (
        (lifecycle = 'active' AND deprecated_at IS NULL)
        OR
        (lifecycle IN ('deprecated', 'withdrawn') AND deprecated_at IS NOT NULL)
    ),
    PRIMARY KEY (schema_uri, schema_version)
);

CREATE INDEX IF NOT EXISTS idx_interoperability_public_schemas_lifecycle
    ON interoperability_public_schemas (schema_uri, lifecycle, created_at);

CREATE TRIGGER IF NOT EXISTS enforce_interoperability_public_schema_immutability
BEFORE UPDATE ON interoperability_public_schemas
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'public schema versions are immutable');
END;

CREATE TRIGGER IF NOT EXISTS prevent_interoperability_public_schema_delete
BEFORE DELETE ON interoperability_public_schemas
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'public schema versions cannot be deleted');
END;

CREATE TABLE IF NOT EXISTS interoperability_source_bindings (
    binding_id             TEXT PRIMARY KEY,
    revision               INTEGER NOT NULL CHECK (revision > 0),
    resource_uri           TEXT NOT NULL,
    context_node_uri       TEXT NOT NULL,
    external_system        TEXT NOT NULL,
    external_account_key   TEXT NOT NULL
        CHECK (
            length(external_account_key) = 64
            AND external_account_key NOT GLOB '*[^0-9a-f]*'
        ),
    external_object_key    TEXT NOT NULL
        CHECK (
            length(external_object_key) = 64
            AND external_object_key NOT GLOB '*[^0-9a-f]*'
        ),
    status                 TEXT NOT NULL
        CHECK (status IN ('active', 'suspended', 'revoked', 'migrated')),
    supersedes_binding_id  TEXT REFERENCES interoperability_source_bindings(binding_id),
    record_payload         BLOB NOT NULL,
    payload_format         TEXT NOT NULL CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek    TEXT,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL,
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_interoperability_active_source_binding
    ON interoperability_source_bindings (
        context_node_uri,
        external_account_key,
        external_object_key
    )
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_interoperability_source_binding_resource
    ON interoperability_source_bindings (resource_uri, status);

CREATE INDEX IF NOT EXISTS idx_interoperability_source_binding_predecessor
    ON interoperability_source_bindings (supersedes_binding_id);

CREATE TABLE IF NOT EXISTS interoperability_source_binding_history (
    binding_id             TEXT NOT NULL,
    revision               INTEGER NOT NULL CHECK (revision > 0),
    resource_uri           TEXT NOT NULL,
    context_node_uri       TEXT NOT NULL,
    external_system        TEXT NOT NULL,
    external_account_key   TEXT NOT NULL,
    external_object_key    TEXT NOT NULL,
    status                 TEXT NOT NULL,
    supersedes_binding_id  TEXT,
    record_payload         BLOB NOT NULL,
    payload_format         TEXT NOT NULL,
    payload_wrapped_dek    TEXT,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL,
    archived_at            TEXT NOT NULL,
    PRIMARY KEY (binding_id, revision)
);

CREATE TRIGGER IF NOT EXISTS archive_interoperability_source_binding_update
BEFORE UPDATE ON interoperability_source_bindings
FOR EACH ROW
BEGIN
    INSERT INTO interoperability_source_binding_history (
        binding_id,
        revision,
        resource_uri,
        context_node_uri,
        external_system,
        external_account_key,
        external_object_key,
        status,
        supersedes_binding_id,
        record_payload,
        payload_format,
        payload_wrapped_dek,
        created_at,
        updated_at,
        archived_at
    ) VALUES (
        OLD.binding_id,
        OLD.revision,
        OLD.resource_uri,
        OLD.context_node_uri,
        OLD.external_system,
        OLD.external_account_key,
        OLD.external_object_key,
        OLD.status,
        OLD.supersedes_binding_id,
        OLD.record_payload,
        OLD.payload_format,
        OLD.payload_wrapped_dek,
        OLD.created_at,
        OLD.updated_at,
        datetime('now')
    );
END;

-- Bootstrap schemas make schema enforcement available before the first
-- registry command. These definitions are deliberately small v1 event
-- profiles; richer compatibility metadata can be introduced as new versions.
INSERT OR IGNORE INTO interoperability_public_schemas (
    schema_uri, schema_version, media_type, definition_json, content_digest,
    lifecycle, owner_uri, created_at, deprecated_at
) VALUES
(
    'mindvault://schemas/knowledge-node-created',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["resource_kind","node_kind","namespace"],"type":"object","x-mindvault-event-type":"dev.mindvault.knowledge.node.created.v1"}',
    'ebba5d71a7c88ba47a2b3641ad249b4f064ee0af561f88183c87e47702461a12',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/public-schema-registered',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["schema_uri","schema_version","content_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.schema.registered.v1"}',
    '22fd3d49951cfb644441262eb8697fe94e4626777196342cb13da3c9b00ce56e',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/source-binding-registered',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["binding_id","resource_uri","external_system","materialization_mode"],"type":"object","x-mindvault-event-type":"dev.mindvault.source-binding.registered.v1"}',
    '5b6416d719fdde7602772fe8e869c75eb60317e3bf4dcc028acab63cb1dbf068',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/source-binding-rebound',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["binding_id","supersedes_binding_id"],"type":"object","x-mindvault-event-type":"dev.mindvault.source-binding.rebound.v1"}',
    'd78d466068bc22d4ced96abf1f69f3f3ab945ef485431665000334e1d04f5f05',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
);

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (33, datetime('now'));

-- Schema admission is enforced independently of Rust callers. Every durable
-- event must name an active registered schema whose event-type extension
-- matches the envelope event type.
CREATE TRIGGER IF NOT EXISTS enforce_interoperability_outbox_schema_admission
BEFORE INSERT ON interoperability_outbox
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1
    FROM interoperability_public_schemas AS schema_registry
    WHERE schema_registry.schema_uri = NEW.schema_uri
      AND schema_registry.schema_version = NEW.schema_version
      AND schema_registry.lifecycle = 'active'
      AND json_extract(
            schema_registry.definition_json,
            '$."x-mindvault-event-type"'
          ) = NEW.event_type
)
BEGIN
    SELECT RAISE(ABORT, 'event schema is not admitted by the active registry');
END;

COMMIT;
