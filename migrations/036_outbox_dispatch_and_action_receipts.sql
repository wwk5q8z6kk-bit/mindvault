-- Durable, transport-neutral outbox dispatch and immutable action receipts.
--
-- A pending event is claimed through a bounded lease. Completion and its
-- receipt share one transaction. "published" proves acknowledgement by the
-- declared destination; it does not claim application by every consumer.

BEGIN IMMEDIATE;

ALTER TABLE interoperability_outbox ADD COLUMN next_attempt_at TEXT;
ALTER TABLE interoperability_outbox ADD COLUMN lease_id TEXT;
ALTER TABLE interoperability_outbox ADD COLUMN lease_owner_uri TEXT;
ALTER TABLE interoperability_outbox ADD COLUMN lease_destination_uri TEXT;
ALTER TABLE interoperability_outbox ADD COLUMN lease_expires_at TEXT;
ALTER TABLE interoperability_outbox ADD COLUMN last_attempt_at TEXT;
ALTER TABLE interoperability_outbox ADD COLUMN updated_at TEXT;

UPDATE interoperability_outbox
SET next_attempt_at = created_at,
    updated_at = created_at
WHERE next_attempt_at IS NULL OR updated_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_interoperability_outbox_dispatchable
    ON interoperability_outbox
       (delivery_state, next_attempt_at, lease_expires_at, created_at, event_id);

CREATE TABLE IF NOT EXISTS interoperability_action_receipts (
    receipt_id       TEXT PRIMARY KEY,
    receipt_version  TEXT NOT NULL
        CHECK (receipt_version = 'mindvault.action-receipt/v1'),
    event_id         TEXT NOT NULL,
    claim_id         TEXT NOT NULL,
    attempt_no       INTEGER NOT NULL CHECK (attempt_no > 0),
    outcome          TEXT NOT NULL
        CHECK (outcome IN ('published', 'retry_scheduled', 'dead_lettered')),
    executor_uri     TEXT NOT NULL,
    destination_uri  TEXT NOT NULL,
    subject_uri      TEXT NOT NULL,
    principal_uri    TEXT NOT NULL,
    actor_uri        TEXT NOT NULL,
    correlation_id   TEXT NOT NULL,
    request_digest   TEXT NOT NULL
        CHECK (
            length(request_digest) = 64
            AND request_digest NOT GLOB '*[^0-9a-f]*'
        ),
    started_at       TEXT NOT NULL,
    completed_at     TEXT NOT NULL,
    sensitivity      TEXT NOT NULL,
    retention        TEXT NOT NULL,
    payload          BLOB NOT NULL,
    payload_format   TEXT NOT NULL
        CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek TEXT,
    created_at       TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES interoperability_outbox(event_id),
    UNIQUE (event_id, attempt_no),
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_interoperability_action_receipts_event
    ON interoperability_action_receipts (event_id, attempt_no);

CREATE INDEX IF NOT EXISTS idx_interoperability_action_receipts_correlation
    ON interoperability_action_receipts (correlation_id, completed_at);

CREATE TRIGGER IF NOT EXISTS enforce_interoperability_outbox_dispatch_insert
BEFORE INSERT ON interoperability_outbox
BEGIN
    SELECT CASE WHEN
        NEW.delivery_state != 'pending'
        OR NEW.delivery_attempts != 0
        OR NEW.next_attempt_at IS NULL
        OR NEW.next_attempt_at != NEW.created_at
        OR NEW.updated_at IS NULL
        OR NEW.updated_at != NEW.created_at
        OR NEW.lease_id IS NOT NULL
        OR NEW.lease_owner_uri IS NOT NULL
        OR NEW.lease_destination_uri IS NOT NULL
        OR NEW.lease_expires_at IS NOT NULL
        OR NEW.last_attempt_at IS NOT NULL
        OR NEW.published_at IS NOT NULL
        OR NEW.last_error IS NOT NULL
    THEN RAISE(ABORT, 'invalid initial outbox delivery state') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_interoperability_outbox_delivery_update
BEFORE UPDATE ON interoperability_outbox
BEGIN
    SELECT CASE WHEN
        NEW.event_id IS NOT OLD.event_id
        OR NEW.source_uri IS NOT OLD.source_uri
        OR NEW.principal_uri IS NOT OLD.principal_uri
        OR NEW.event_type IS NOT OLD.event_type
        OR NEW.subject_uri IS NOT OLD.subject_uri
        OR NEW.schema_uri IS NOT OLD.schema_uri
        OR NEW.schema_version IS NOT OLD.schema_version
        OR NEW.correlation_id IS NOT OLD.correlation_id
        OR NEW.causation_id IS NOT OLD.causation_id
        OR NEW.idempotency_key IS NOT OLD.idempotency_key
        OR NEW.payload_digest IS NOT OLD.payload_digest
        OR NEW.envelope_json IS NOT OLD.envelope_json
        OR NEW.created_at IS NOT OLD.created_at
    THEN RAISE(ABORT, 'outbox event identity and envelope are immutable') END;

    SELECT CASE WHEN OLD.delivery_state != 'pending'
        THEN RAISE(ABORT, 'terminal outbox delivery state is immutable') END;

    SELECT CASE WHEN NEW.delivery_attempts = OLD.delivery_attempts + 1
        AND NOT (
            NEW.delivery_state = 'pending'
            AND NEW.lease_id IS NOT NULL
            AND NEW.lease_owner_uri IS NOT NULL
            AND NEW.lease_destination_uri IS NOT NULL
            AND NEW.lease_expires_at IS NOT NULL
            AND NEW.last_attempt_at IS NOT NULL
            AND NEW.updated_at = NEW.last_attempt_at
            AND NEW.published_at IS NULL
            AND NEW.next_attempt_at IS OLD.next_attempt_at
            AND NEW.last_error IS OLD.last_error
            AND NEW.last_attempt_at >= OLD.next_attempt_at
            AND NEW.last_attempt_at >= OLD.updated_at
            AND NEW.lease_expires_at > NEW.last_attempt_at
            AND (
                OLD.lease_id IS NULL
                OR (
                    OLD.lease_expires_at <= NEW.last_attempt_at
                    AND NEW.lease_id != OLD.lease_id
                )
            )
        )
        THEN RAISE(ABORT, 'invalid outbox delivery claim') END;

    SELECT CASE WHEN NEW.delivery_attempts = OLD.delivery_attempts
        AND NOT (
            OLD.lease_id IS NOT NULL
            AND NEW.lease_id IS NULL
            AND NEW.lease_owner_uri IS NULL
            AND NEW.lease_destination_uri IS NULL
            AND NEW.lease_expires_at IS NULL
            AND NEW.last_attempt_at IS OLD.last_attempt_at
            AND NEW.updated_at >= OLD.updated_at
            AND (
                (
                    NEW.delivery_state = 'pending'
                    AND NEW.next_attempt_at > NEW.updated_at
                    AND NEW.last_error IS NOT NULL
                    AND NEW.published_at IS NULL
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_action_receipts
                        WHERE event_id = OLD.event_id
                          AND attempt_no = OLD.delivery_attempts
                          AND claim_id = OLD.lease_id
                          AND outcome = 'retry_scheduled'
                          AND completed_at = NEW.updated_at
                    )
                )
                OR
                (
                    NEW.delivery_state = 'published'
                    AND NEW.next_attempt_at IS OLD.next_attempt_at
                    AND NEW.last_error IS NULL
                    AND NEW.published_at = NEW.updated_at
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_action_receipts
                        WHERE event_id = OLD.event_id
                          AND attempt_no = OLD.delivery_attempts
                          AND claim_id = OLD.lease_id
                          AND outcome = 'published'
                          AND completed_at = NEW.updated_at
                    )
                )
                OR
                (
                    NEW.delivery_state = 'dead_letter'
                    AND NEW.next_attempt_at IS OLD.next_attempt_at
                    AND NEW.last_error IS NOT NULL
                    AND NEW.published_at IS NULL
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_action_receipts
                        WHERE event_id = OLD.event_id
                          AND attempt_no = OLD.delivery_attempts
                          AND claim_id = OLD.lease_id
                          AND outcome = 'dead_lettered'
                          AND completed_at = NEW.updated_at
                    )
                )
            )
        )
        THEN RAISE(ABORT, 'invalid outbox delivery completion') END;

    SELECT CASE WHEN NEW.delivery_attempts < OLD.delivery_attempts
        OR NEW.delivery_attempts > OLD.delivery_attempts + 1
        THEN RAISE(ABORT, 'outbox delivery attempts must advance exactly once per claim') END;
END;

CREATE TRIGGER IF NOT EXISTS forbid_interoperability_outbox_delete
BEFORE DELETE ON interoperability_outbox
BEGIN
    SELECT RAISE(ABORT, 'outbox events are immutable history');
END;

CREATE TRIGGER IF NOT EXISTS enforce_action_receipt_claim_binding
BEFORE INSERT ON interoperability_action_receipts
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM interoperability_outbox
        WHERE event_id = NEW.event_id
          AND delivery_state = 'pending'
          AND delivery_attempts = NEW.attempt_no
          AND lease_id = NEW.claim_id
          AND lease_owner_uri = NEW.executor_uri
          AND lease_destination_uri = NEW.destination_uri
          AND last_attempt_at = NEW.started_at
          AND NEW.completed_at >= last_attempt_at
          AND NEW.completed_at < lease_expires_at
    ) THEN RAISE(ABORT, 'action receipt does not match an active outbox claim') END;
END;

CREATE TRIGGER IF NOT EXISTS forbid_action_receipt_update
BEFORE UPDATE ON interoperability_action_receipts
BEGIN
    SELECT RAISE(ABORT, 'action receipts are immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_action_receipt_delete
BEFORE DELETE ON interoperability_action_receipts
BEGIN
    SELECT RAISE(ABORT, 'action receipts are immutable');
END;

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (36, datetime('now'));

COMMIT;
