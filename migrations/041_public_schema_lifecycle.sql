-- Governed public-schema lifecycle transitions.
--
-- Schema definitions remain immutable. A version may advance only from active
-- to deprecated and then to withdrawn. The transition must be paired with its
-- durable governance event, and withdrawal requires an active replacement and
-- no pending outbox event that still depends on the retiring version.

BEGIN IMMEDIATE;

ALTER TABLE interoperability_public_schemas ADD COLUMN withdrawn_at TEXT;
ALTER TABLE interoperability_public_schemas ADD COLUMN replacement_schema_uri TEXT;
ALTER TABLE interoperability_public_schemas ADD COLUMN replacement_schema_version TEXT;

INSERT OR IGNORE INTO interoperability_public_schemas (
    schema_uri, schema_version, media_type, definition_json, content_digest,
    lifecycle, owner_uri, created_at, deprecated_at,
    withdrawn_at, replacement_schema_uri, replacement_schema_version
) VALUES (
    'mindvault://schemas/public-schema-lifecycle-transitioned',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["schema_uri","schema_version","from_lifecycle","to_lifecycle","reason"],"type":"object","x-mindvault-event-type":"dev.mindvault.schema.lifecycle.transitioned.v1"}',
    'b32ba5f8f4435cfce6c985199d5afd67084a4b6bd6f2d42efda82aba50d1e9f5',
    'active',
    'mindvault://schemas/governance',
    '2026-08-01T00:00:00Z',
    NULL, NULL, NULL, NULL
);

DROP TRIGGER IF EXISTS enforce_interoperability_public_schema_immutability;

CREATE TRIGGER enforce_interoperability_public_schema_governed_lifecycle
BEFORE UPDATE ON interoperability_public_schemas
FOR EACH ROW
BEGIN
    SELECT CASE WHEN
        NEW.schema_uri IS NOT OLD.schema_uri
        OR NEW.schema_version IS NOT OLD.schema_version
        OR NEW.media_type IS NOT OLD.media_type
        OR NEW.definition_json IS NOT OLD.definition_json
        OR NEW.content_digest IS NOT OLD.content_digest
        OR NEW.owner_uri IS NOT OLD.owner_uri
        OR NEW.created_at IS NOT OLD.created_at
    THEN RAISE(ABORT, 'public schema definitions are immutable') END;

    SELECT CASE WHEN NOT (
        (OLD.lifecycle = 'active' AND NEW.lifecycle = 'deprecated')
        OR (OLD.lifecycle = 'deprecated' AND NEW.lifecycle = 'withdrawn')
    ) THEN RAISE(ABORT, 'invalid public schema lifecycle transition') END;

    SELECT CASE WHEN
        NEW.deprecated_at IS NULL
        OR julianday(NEW.deprecated_at) IS NULL
        OR (OLD.lifecycle = 'deprecated' AND NEW.deprecated_at IS NOT OLD.deprecated_at)
        OR (NEW.lifecycle = 'deprecated' AND NEW.withdrawn_at IS NOT NULL)
        OR (NEW.lifecycle = 'withdrawn' AND NEW.withdrawn_at IS NULL)
        OR (NEW.lifecycle = 'withdrawn' AND julianday(NEW.withdrawn_at) IS NULL)
        OR (NEW.lifecycle = 'withdrawn' AND
            julianday(NEW.withdrawn_at) < julianday(NEW.deprecated_at))
        OR ((NEW.replacement_schema_uri IS NULL) !=
            (NEW.replacement_schema_version IS NULL))
        OR (NEW.replacement_schema_uri = NEW.schema_uri
            AND NEW.replacement_schema_version = NEW.schema_version)
        OR (OLD.replacement_schema_uri IS NOT NULL AND (
            NEW.replacement_schema_uri IS NOT OLD.replacement_schema_uri
            OR NEW.replacement_schema_version IS NOT OLD.replacement_schema_version
        ))
        OR (NEW.lifecycle = 'withdrawn' AND NEW.replacement_schema_uri IS NULL)
    THEN RAISE(ABORT, 'invalid public schema lifecycle metadata') END;

    SELECT CASE WHEN NEW.replacement_schema_uri IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM interoperability_public_schemas AS replacement
        WHERE replacement.schema_uri = NEW.replacement_schema_uri
          AND replacement.schema_version = NEW.replacement_schema_version
          AND replacement.lifecycle = 'active'
    ) THEN RAISE(ABORT, 'public schema replacement is not active') END;

    SELECT CASE WHEN NEW.lifecycle = 'withdrawn' AND EXISTS (
        SELECT 1
        FROM interoperability_outbox AS live_event
        WHERE live_event.schema_uri = OLD.schema_uri
          AND live_event.schema_version = OLD.schema_version
          AND live_event.delivery_state = 'pending'
    ) THEN RAISE(ABORT, 'public schema has live outbox references') END;

    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM interoperability_outbox AS governance_event
        WHERE governance_event.event_type =
              'dev.mindvault.schema.lifecycle.transitioned.v1'
          AND governance_event.subject_uri =
              OLD.schema_uri || '/version/' || OLD.schema_version
          AND json_extract(governance_event.envelope_json, '$.data.schema_uri') =
              OLD.schema_uri
          AND json_extract(governance_event.envelope_json, '$.data.schema_version') =
              OLD.schema_version
          AND json_extract(governance_event.envelope_json, '$.data.from_lifecycle') =
              OLD.lifecycle
          AND json_extract(governance_event.envelope_json, '$.data.to_lifecycle') =
              NEW.lifecycle
          AND json_extract(governance_event.envelope_json, '$.data.replacement_schema_uri')
              IS NEW.replacement_schema_uri
          AND json_extract(governance_event.envelope_json, '$.data.replacement_schema_version')
              IS NEW.replacement_schema_version
    ) THEN RAISE(ABORT, 'public schema lifecycle transition requires its governance event') END;
END;

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (41, datetime('now'));

COMMIT;
