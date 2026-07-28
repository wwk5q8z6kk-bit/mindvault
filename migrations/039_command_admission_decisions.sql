-- Durable, append-only command-admission decisions (IK-001c).
--
-- Denied commands produce no mutation and therefore no outbox event; Law 15
-- still requires the denial be recorded. This table is that record. It is not
-- an action receipt (see ACTION_RECEIPT_MODEL.md) and must never share the
-- outbox/receipt write path.
--
-- Idempotent on (principal_uri, idempotency_key). Rows are immutable.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_command_admission_decisions (
    decision_id                 TEXT PRIMARY KEY,
    principal_uri               TEXT NOT NULL,
    actor_uri                   TEXT NOT NULL,
    idempotency_key             TEXT NOT NULL,
    correlation_id              TEXT NOT NULL,
    request_id                  TEXT NOT NULL,
    resource_uri                TEXT NOT NULL,
    subject_uri                 TEXT NOT NULL,
    operation                   TEXT NOT NULL,
    required_grant_kind         TEXT NOT NULL
        CHECK (required_grant_kind IN ('context', 'tool')),
    decision                    TEXT NOT NULL
        CHECK (decision IN ('admitted', 'denied')),
    denial_reason               TEXT,
    grant_id                    TEXT,
    grant_uri                   TEXT,
    grant_kind                  TEXT
        CHECK (grant_kind IS NULL OR grant_kind IN ('context', 'tool')),
    capability                  TEXT,
    delegation_depth_remaining  INTEGER
        CHECK (delegation_depth_remaining IS NULL OR delegation_depth_remaining >= 0),
    admission_digest            TEXT NOT NULL,
    decided_at                  TEXT NOT NULL,
    created_at                  TEXT NOT NULL,
    UNIQUE (principal_uri, idempotency_key),
    CHECK (
        (
            decision = 'admitted'
            AND denial_reason IS NULL
            AND grant_id IS NOT NULL
            AND grant_uri IS NOT NULL
            AND grant_kind IS NOT NULL
            AND capability IS NOT NULL
            AND delegation_depth_remaining IS NOT NULL
        )
        OR
        (
            decision = 'denied'
            AND denial_reason IS NOT NULL
            AND grant_id IS NULL
            AND grant_uri IS NULL
            AND grant_kind IS NULL
            AND capability IS NULL
            AND delegation_depth_remaining IS NULL
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_command_admission_decisions_principal_decided
    ON interoperability_command_admission_decisions (principal_uri, decided_at);

CREATE INDEX IF NOT EXISTS idx_command_admission_decisions_decision_decided
    ON interoperability_command_admission_decisions (decision, decided_at);

CREATE TRIGGER IF NOT EXISTS forbid_command_admission_decision_update
BEFORE UPDATE ON interoperability_command_admission_decisions
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'command admission decisions are immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_command_admission_decision_delete
BEFORE DELETE ON interoperability_command_admission_decisions
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'command admission decisions are immutable');
END;

COMMIT;
