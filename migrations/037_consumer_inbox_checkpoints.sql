-- Durable consumer inbox, local disposition checkpoints, and immutable
-- application receipts.
--
-- Inbox sequence numbers are assigned on local admission. They preserve local
-- processing order within one (consumer, source) stream, but do not claim to
-- detect gaps or reordering in a remote issuer stream.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_consumer_inbox (
    inbox_sequence      INTEGER PRIMARY KEY AUTOINCREMENT,
    consumer_uri        TEXT NOT NULL,
    event_id            TEXT NOT NULL,
    event_digest        TEXT NOT NULL
        CHECK (
            length(event_digest) = 64
            AND event_digest NOT GLOB '*[^0-9a-f]*'
        ),
    source_uri          TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    principal_uri       TEXT NOT NULL,
    actor_uri           TEXT NOT NULL,
    event_type          TEXT NOT NULL,
    schema_uri          TEXT NOT NULL,
    schema_version      TEXT NOT NULL,
    correlation_id      TEXT NOT NULL,
    occurred_at         TEXT NOT NULL,
    sensitivity         TEXT NOT NULL,
    retention           TEXT NOT NULL,
    envelope_payload    BLOB NOT NULL,
    payload_format      TEXT NOT NULL
        CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek TEXT,
    state               TEXT NOT NULL
        CHECK (state IN ('pending', 'applied', 'dead_letter')),
    attempts            INTEGER NOT NULL CHECK (attempts >= 0),
    next_attempt_at     TEXT NOT NULL,
    lease_id            TEXT,
    lease_processor_uri TEXT,
    lease_expires_at    TEXT,
    last_attempt_at     TEXT,
    applied_at          TEXT,
    last_error_code     TEXT,
    received_at         TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE (consumer_uri, event_id),
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_interoperability_consumer_inbox_dispatchable
    ON interoperability_consumer_inbox
       (consumer_uri, state, next_attempt_at, lease_expires_at, inbox_sequence);

CREATE INDEX IF NOT EXISTS idx_interoperability_consumer_inbox_stream
    ON interoperability_consumer_inbox
       (consumer_uri, source_uri, inbox_sequence);

CREATE INDEX IF NOT EXISTS idx_interoperability_consumer_inbox_correlation
    ON interoperability_consumer_inbox (correlation_id, received_at);

CREATE TABLE IF NOT EXISTS interoperability_consumer_application_receipts (
    receipt_id          TEXT PRIMARY KEY,
    receipt_version     TEXT NOT NULL
        CHECK (
            receipt_version = 'mindvault.consumer-application-receipt/v1'
        ),
    inbox_sequence      INTEGER NOT NULL,
    event_id            TEXT NOT NULL,
    claim_id            TEXT NOT NULL,
    attempt_no          INTEGER NOT NULL CHECK (attempt_no > 0),
    outcome             TEXT NOT NULL
        CHECK (outcome IN ('applied', 'retry_scheduled', 'dead_lettered')),
    consumer_uri        TEXT NOT NULL,
    processor_uri       TEXT NOT NULL,
    source_uri          TEXT NOT NULL,
    subject_uri         TEXT NOT NULL,
    principal_uri       TEXT NOT NULL,
    actor_uri           TEXT NOT NULL,
    correlation_id      TEXT NOT NULL,
    request_digest      TEXT NOT NULL
        CHECK (
            length(request_digest) = 64
            AND request_digest NOT GLOB '*[^0-9a-f]*'
        ),
    started_at          TEXT NOT NULL,
    completed_at        TEXT NOT NULL,
    sensitivity         TEXT NOT NULL,
    retention           TEXT NOT NULL,
    payload             BLOB NOT NULL,
    payload_format      TEXT NOT NULL
        CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek TEXT,
    created_at          TEXT NOT NULL,
    FOREIGN KEY (inbox_sequence)
        REFERENCES interoperability_consumer_inbox(inbox_sequence),
    UNIQUE (inbox_sequence, attempt_no),
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_interoperability_consumer_receipts_event
    ON interoperability_consumer_application_receipts
       (consumer_uri, event_id, attempt_no);

CREATE INDEX IF NOT EXISTS idx_interoperability_consumer_receipts_correlation
    ON interoperability_consumer_application_receipts
       (correlation_id, completed_at);

CREATE TABLE IF NOT EXISTS interoperability_consumer_checkpoints (
    consumer_uri                    TEXT NOT NULL,
    source_uri                      TEXT NOT NULL,
    last_dispositioned_sequence     INTEGER NOT NULL
        CHECK (last_dispositioned_sequence > 0),
    last_dispositioned_event_id     TEXT NOT NULL,
    last_applied_sequence           INTEGER,
    last_applied_event_id           TEXT,
    applied_count                   INTEGER NOT NULL CHECK (applied_count >= 0),
    dead_letter_count               INTEGER NOT NULL
        CHECK (dead_letter_count >= 0),
    updated_at                      TEXT NOT NULL,
    PRIMARY KEY (consumer_uri, source_uri),
    FOREIGN KEY (last_dispositioned_sequence)
        REFERENCES interoperability_consumer_inbox(inbox_sequence),
    FOREIGN KEY (last_applied_sequence)
        REFERENCES interoperability_consumer_inbox(inbox_sequence),
    CHECK (
        (last_applied_sequence IS NULL AND last_applied_event_id IS NULL
            AND applied_count = 0)
        OR
        (last_applied_sequence IS NOT NULL
            AND last_applied_event_id IS NOT NULL
            AND applied_count > 0
            AND last_applied_sequence <= last_dispositioned_sequence)
    ),
    CHECK (applied_count + dead_letter_count > 0)
);

CREATE TRIGGER IF NOT EXISTS enforce_consumer_inbox_schema_admission
BEFORE INSERT ON interoperability_consumer_inbox
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM interoperability_public_schemas
        WHERE schema_uri = NEW.schema_uri
          AND schema_version = NEW.schema_version
          AND lifecycle = 'active'
          AND json_extract(
                definition_json,
                '$."x-mindvault-event-type"'
              ) = NEW.event_type
    ) THEN RAISE(ABORT, 'consumer inbox event schema is not active or does not match its type') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_consumer_inbox_insert
BEFORE INSERT ON interoperability_consumer_inbox
BEGIN
    SELECT CASE WHEN
        NEW.state != 'pending'
        OR NEW.attempts != 0
        OR NEW.next_attempt_at != NEW.received_at
        OR NEW.updated_at != NEW.received_at
        OR NEW.lease_id IS NOT NULL
        OR NEW.lease_processor_uri IS NOT NULL
        OR NEW.lease_expires_at IS NOT NULL
        OR NEW.last_attempt_at IS NOT NULL
        OR NEW.applied_at IS NOT NULL
        OR NEW.last_error_code IS NOT NULL
    THEN RAISE(ABORT, 'invalid initial consumer inbox state') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_consumer_inbox_update
BEFORE UPDATE ON interoperability_consumer_inbox
BEGIN
    SELECT CASE WHEN
        NEW.inbox_sequence IS NOT OLD.inbox_sequence
        OR NEW.consumer_uri IS NOT OLD.consumer_uri
        OR NEW.event_id IS NOT OLD.event_id
        OR NEW.event_digest IS NOT OLD.event_digest
        OR NEW.source_uri IS NOT OLD.source_uri
        OR NEW.subject_uri IS NOT OLD.subject_uri
        OR NEW.principal_uri IS NOT OLD.principal_uri
        OR NEW.actor_uri IS NOT OLD.actor_uri
        OR NEW.event_type IS NOT OLD.event_type
        OR NEW.schema_uri IS NOT OLD.schema_uri
        OR NEW.schema_version IS NOT OLD.schema_version
        OR NEW.correlation_id IS NOT OLD.correlation_id
        OR NEW.occurred_at IS NOT OLD.occurred_at
        OR NEW.sensitivity IS NOT OLD.sensitivity
        OR NEW.retention IS NOT OLD.retention
        OR NEW.envelope_payload IS NOT OLD.envelope_payload
        OR NEW.payload_format IS NOT OLD.payload_format
        OR NEW.payload_wrapped_dek IS NOT OLD.payload_wrapped_dek
        OR NEW.received_at IS NOT OLD.received_at
    THEN RAISE(ABORT, 'consumer inbox event identity and envelope are immutable') END;

    SELECT CASE WHEN OLD.state != 'pending'
        THEN RAISE(ABORT, 'terminal consumer inbox state is immutable') END;

    SELECT CASE WHEN NEW.attempts = OLD.attempts + 1
        AND NOT (
            NEW.state = 'pending'
            AND NEW.lease_id IS NOT NULL
            AND NEW.lease_processor_uri IS NOT NULL
            AND NEW.lease_expires_at IS NOT NULL
            AND NEW.last_attempt_at IS NOT NULL
            AND NEW.updated_at = NEW.last_attempt_at
            AND NEW.applied_at IS NULL
            AND NEW.next_attempt_at IS OLD.next_attempt_at
            AND NEW.last_error_code IS OLD.last_error_code
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
            AND NOT EXISTS (
                SELECT 1
                FROM interoperability_consumer_inbox predecessor
                WHERE predecessor.consumer_uri = OLD.consumer_uri
                  AND predecessor.source_uri = OLD.source_uri
                  AND predecessor.inbox_sequence < OLD.inbox_sequence
                  AND predecessor.state = 'pending'
            )
        )
        THEN RAISE(ABORT, 'invalid consumer inbox claim') END;

    SELECT CASE WHEN NEW.attempts = OLD.attempts
        AND NOT (
            OLD.lease_id IS NOT NULL
            AND NEW.lease_id IS NULL
            AND NEW.lease_processor_uri IS NULL
            AND NEW.lease_expires_at IS NULL
            AND NEW.last_attempt_at IS OLD.last_attempt_at
            AND NEW.updated_at >= OLD.updated_at
            AND (
                (
                    NEW.state = 'pending'
                    AND NEW.next_attempt_at > NEW.updated_at
                    AND NEW.last_error_code IS NOT NULL
                    AND NEW.applied_at IS NULL
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_consumer_application_receipts
                        WHERE inbox_sequence = OLD.inbox_sequence
                          AND attempt_no = OLD.attempts
                          AND claim_id = OLD.lease_id
                          AND outcome = 'retry_scheduled'
                          AND completed_at = NEW.updated_at
                    )
                )
                OR
                (
                    NEW.state = 'applied'
                    AND NEW.next_attempt_at IS OLD.next_attempt_at
                    AND NEW.last_error_code IS NULL
                    AND NEW.applied_at = NEW.updated_at
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_consumer_application_receipts
                        WHERE inbox_sequence = OLD.inbox_sequence
                          AND attempt_no = OLD.attempts
                          AND claim_id = OLD.lease_id
                          AND outcome = 'applied'
                          AND completed_at = NEW.updated_at
                    )
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_consumer_checkpoints
                        WHERE consumer_uri = OLD.consumer_uri
                          AND source_uri = OLD.source_uri
                          AND last_dispositioned_sequence = OLD.inbox_sequence
                          AND last_dispositioned_event_id = OLD.event_id
                          AND last_applied_sequence = OLD.inbox_sequence
                          AND last_applied_event_id = OLD.event_id
                          AND updated_at = NEW.updated_at
                    )
                )
                OR
                (
                    NEW.state = 'dead_letter'
                    AND NEW.next_attempt_at IS OLD.next_attempt_at
                    AND NEW.last_error_code IS NOT NULL
                    AND NEW.applied_at IS NULL
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_consumer_application_receipts
                        WHERE inbox_sequence = OLD.inbox_sequence
                          AND attempt_no = OLD.attempts
                          AND claim_id = OLD.lease_id
                          AND outcome = 'dead_lettered'
                          AND completed_at = NEW.updated_at
                    )
                    AND EXISTS (
                        SELECT 1
                        FROM interoperability_consumer_checkpoints
                        WHERE consumer_uri = OLD.consumer_uri
                          AND source_uri = OLD.source_uri
                          AND last_dispositioned_sequence = OLD.inbox_sequence
                          AND last_dispositioned_event_id = OLD.event_id
                          AND updated_at = NEW.updated_at
                    )
                )
            )
        )
        THEN RAISE(ABORT, 'invalid consumer inbox completion') END;

    SELECT CASE WHEN NEW.attempts < OLD.attempts
        OR NEW.attempts > OLD.attempts + 1
        THEN RAISE(ABORT, 'consumer application attempts must advance exactly once per claim') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_consumer_application_receipt_claim
BEFORE INSERT ON interoperability_consumer_application_receipts
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM interoperability_consumer_inbox
        WHERE inbox_sequence = NEW.inbox_sequence
          AND event_id = NEW.event_id
          AND consumer_uri = NEW.consumer_uri
          AND source_uri = NEW.source_uri
          AND subject_uri = NEW.subject_uri
          AND principal_uri = NEW.principal_uri
          AND actor_uri = NEW.actor_uri
          AND correlation_id = NEW.correlation_id
          AND event_digest = NEW.request_digest
          AND state = 'pending'
          AND attempts = NEW.attempt_no
          AND lease_id = NEW.claim_id
          AND lease_processor_uri = NEW.processor_uri
          AND last_attempt_at = NEW.started_at
          AND NEW.completed_at >= last_attempt_at
          AND NEW.completed_at < lease_expires_at
    ) THEN RAISE(ABORT, 'consumer receipt does not match an active inbox claim') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_consumer_checkpoint_insert
BEFORE INSERT ON interoperability_consumer_checkpoints
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM interoperability_consumer_inbox inbox
        JOIN interoperability_consumer_application_receipts receipt
          ON receipt.inbox_sequence = inbox.inbox_sequence
         AND receipt.attempt_no = inbox.attempts
         AND receipt.claim_id = inbox.lease_id
        WHERE inbox.inbox_sequence = NEW.last_dispositioned_sequence
          AND inbox.event_id = NEW.last_dispositioned_event_id
          AND inbox.consumer_uri = NEW.consumer_uri
          AND inbox.source_uri = NEW.source_uri
          AND inbox.state = 'pending'
          AND receipt.completed_at = NEW.updated_at
          AND (
              (
                  receipt.outcome = 'applied'
                  AND NEW.applied_count = 1
                  AND NEW.dead_letter_count = 0
                  AND NEW.last_applied_sequence = inbox.inbox_sequence
                  AND NEW.last_applied_event_id = inbox.event_id
              )
              OR
              (
                  receipt.outcome = 'dead_lettered'
                  AND NEW.applied_count = 0
                  AND NEW.dead_letter_count = 1
                  AND NEW.last_applied_sequence IS NULL
                  AND NEW.last_applied_event_id IS NULL
              )
          )
          AND NOT EXISTS (
              SELECT 1
              FROM interoperability_consumer_inbox predecessor
              WHERE predecessor.consumer_uri = inbox.consumer_uri
                AND predecessor.source_uri = inbox.source_uri
                AND predecessor.inbox_sequence < inbox.inbox_sequence
                AND predecessor.state = 'pending'
          )
    ) THEN RAISE(ABORT, 'invalid initial consumer checkpoint') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_consumer_checkpoint_update
BEFORE UPDATE ON interoperability_consumer_checkpoints
BEGIN
    SELECT CASE WHEN
        NEW.consumer_uri IS NOT OLD.consumer_uri
        OR NEW.source_uri IS NOT OLD.source_uri
        OR NEW.last_dispositioned_sequence <= OLD.last_dispositioned_sequence
        OR NOT EXISTS (
            SELECT 1
            FROM interoperability_consumer_inbox inbox
            JOIN interoperability_consumer_application_receipts receipt
              ON receipt.inbox_sequence = inbox.inbox_sequence
             AND receipt.attempt_no = inbox.attempts
             AND receipt.claim_id = inbox.lease_id
            WHERE inbox.inbox_sequence = NEW.last_dispositioned_sequence
              AND inbox.event_id = NEW.last_dispositioned_event_id
              AND inbox.consumer_uri = NEW.consumer_uri
              AND inbox.source_uri = NEW.source_uri
              AND inbox.state = 'pending'
              AND receipt.completed_at = NEW.updated_at
              AND (
                  (
                      receipt.outcome = 'applied'
                      AND NEW.applied_count = OLD.applied_count + 1
                      AND NEW.dead_letter_count = OLD.dead_letter_count
                      AND NEW.last_applied_sequence = inbox.inbox_sequence
                      AND NEW.last_applied_event_id = inbox.event_id
                  )
                  OR
                  (
                      receipt.outcome = 'dead_lettered'
                      AND NEW.applied_count = OLD.applied_count
                      AND NEW.dead_letter_count = OLD.dead_letter_count + 1
                      AND NEW.last_applied_sequence IS OLD.last_applied_sequence
                      AND NEW.last_applied_event_id IS OLD.last_applied_event_id
                  )
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM interoperability_consumer_inbox predecessor
                  WHERE predecessor.consumer_uri = inbox.consumer_uri
                    AND predecessor.source_uri = inbox.source_uri
                    AND predecessor.inbox_sequence < inbox.inbox_sequence
                    AND predecessor.state = 'pending'
              )
        )
    THEN RAISE(ABORT, 'invalid consumer checkpoint advance') END;
END;

CREATE TRIGGER IF NOT EXISTS forbid_consumer_inbox_delete
BEFORE DELETE ON interoperability_consumer_inbox
BEGIN
    SELECT RAISE(ABORT, 'consumer inbox events are immutable history');
END;

CREATE TRIGGER IF NOT EXISTS forbid_consumer_application_receipt_update
BEFORE UPDATE ON interoperability_consumer_application_receipts
BEGIN
    SELECT RAISE(ABORT, 'consumer application receipts are immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_consumer_application_receipt_delete
BEFORE DELETE ON interoperability_consumer_application_receipts
BEGIN
    SELECT RAISE(ABORT, 'consumer application receipts are immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_consumer_checkpoint_delete
BEFORE DELETE ON interoperability_consumer_checkpoints
BEGIN
    SELECT RAISE(ABORT, 'consumer checkpoints are durable history');
END;

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (37, datetime('now'));

COMMIT;
