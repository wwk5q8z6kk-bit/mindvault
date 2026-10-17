-- Governed agent execution graph: Work Orders, node contracts, typed edges,
-- Agent Runs, exclusive write leases, gate evidence, and artifacts.
--
-- Declared write scope is the AuthorityGrant target set; admission rejects any
-- node whose scope exceeds its effective Tool Grant. Write leases carry the
-- bounded-lease semantics of migration 036 without variation. Approval is
-- unbounded and holds no leases. Budgets are monotonically decreasing ceilings.
--
-- Target URIs are stored as SHA-256 digests, never as plaintext. Exact stable
-- URI equality is the only matching mode grants permit, so digest equality is
-- exactly equivalent, and a sealed vault does not expose scope detail through
-- the lease or conflict indexes.
--
-- Supersedes migrations/024_plans.sql. Verified before writing this migration:
-- no code reads or writes `plans` or `plan_steps`, so no data migration is
-- required. The superseded tables were left in place for this revision and
-- are dropped by migration 041 (AGENT-002).

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- Work Orders
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS work_orders (
    work_order_id            TEXT PRIMARY KEY,
    revision                 INTEGER NOT NULL CHECK (revision > 0),
    work_order_uri           TEXT NOT NULL UNIQUE,
    principal_uri            TEXT NOT NULL,
    actor_uri                TEXT NOT NULL,
    governing_node_uri       TEXT NOT NULL,
    status                   TEXT NOT NULL CHECK (status IN (
                                 'draft', 'admitted', 'running', 'completed',
                                 'failed', 'cancelled', 'rejected',
                                 'budget_exhausted')),
    status_reason            TEXT,
    sensitivity              TEXT NOT NULL,
    retention                TEXT NOT NULL,
    correlation_id           TEXT NOT NULL,
    causation_id             TEXT,
    idempotency_key          TEXT NOT NULL,

    -- Budget ceilings are immutable; remaining balances only decrease.
    budget_wall_clock_secs   INTEGER NOT NULL CHECK (budget_wall_clock_secs >= 0),
    budget_run_attempts      INTEGER NOT NULL CHECK (budget_run_attempts >= 0),
    budget_model_tokens      INTEGER NOT NULL CHECK (budget_model_tokens >= 0),
    budget_effect_actions    INTEGER NOT NULL CHECK (budget_effect_actions >= 0),
    remaining_wall_clock_secs INTEGER NOT NULL CHECK (remaining_wall_clock_secs >= 0),
    remaining_run_attempts   INTEGER NOT NULL CHECK (remaining_run_attempts >= 0),
    remaining_model_tokens   INTEGER NOT NULL CHECK (remaining_model_tokens >= 0),
    remaining_effect_actions INTEGER NOT NULL CHECK (remaining_effect_actions >= 0),

    record_payload           BLOB NOT NULL,
    payload_format           TEXT NOT NULL
        CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek      TEXT,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL,
    CHECK (remaining_wall_clock_secs <= budget_wall_clock_secs),
    CHECK (remaining_run_attempts <= budget_run_attempts),
    CHECK (remaining_model_tokens <= budget_model_tokens),
    CHECK (remaining_effect_actions <= budget_effect_actions),
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_work_orders_principal
    ON work_orders (principal_uri, status, updated_at);

CREATE INDEX IF NOT EXISTS idx_work_orders_governance
    ON work_orders (governing_node_uri, status, updated_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_work_orders_idempotency
    ON work_orders (principal_uri, idempotency_key);

CREATE TABLE IF NOT EXISTS work_order_history (
    work_order_id            TEXT NOT NULL,
    revision                 INTEGER NOT NULL CHECK (revision > 0),
    work_order_uri           TEXT NOT NULL,
    principal_uri            TEXT NOT NULL,
    actor_uri                TEXT NOT NULL,
    governing_node_uri       TEXT NOT NULL,
    status                   TEXT NOT NULL,
    status_reason            TEXT,
    sensitivity              TEXT NOT NULL,
    retention                TEXT NOT NULL,
    correlation_id           TEXT NOT NULL,
    causation_id             TEXT,
    idempotency_key          TEXT NOT NULL,
    budget_wall_clock_secs   INTEGER NOT NULL,
    budget_run_attempts      INTEGER NOT NULL,
    budget_model_tokens      INTEGER NOT NULL,
    budget_effect_actions    INTEGER NOT NULL,
    remaining_wall_clock_secs INTEGER NOT NULL,
    remaining_run_attempts   INTEGER NOT NULL,
    remaining_model_tokens   INTEGER NOT NULL,
    remaining_effect_actions INTEGER NOT NULL,
    record_payload           BLOB NOT NULL,
    payload_format           TEXT NOT NULL,
    payload_wrapped_dek      TEXT,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL,
    archived_at              TEXT NOT NULL,
    PRIMARY KEY (work_order_id, revision)
);

-- Identity, budget ceilings, and idempotency are immutable. Remaining balances
-- never increase: a refill would be a silent budget expansion.
CREATE TRIGGER IF NOT EXISTS validate_work_order_update
BEFORE UPDATE ON work_orders
FOR EACH ROW
WHEN
    NEW.work_order_id != OLD.work_order_id
    OR NEW.work_order_uri != OLD.work_order_uri
    OR NEW.principal_uri != OLD.principal_uri
    OR NEW.actor_uri != OLD.actor_uri
    OR NEW.governing_node_uri != OLD.governing_node_uri
    OR NEW.correlation_id != OLD.correlation_id
    OR NEW.causation_id IS NOT OLD.causation_id
    OR NEW.idempotency_key != OLD.idempotency_key
    OR NEW.created_at != OLD.created_at
    OR NEW.budget_wall_clock_secs != OLD.budget_wall_clock_secs
    OR NEW.budget_run_attempts != OLD.budget_run_attempts
    OR NEW.budget_model_tokens != OLD.budget_model_tokens
    OR NEW.budget_effect_actions != OLD.budget_effect_actions
    OR NEW.remaining_wall_clock_secs > OLD.remaining_wall_clock_secs
    OR NEW.remaining_run_attempts > OLD.remaining_run_attempts
    OR NEW.remaining_model_tokens > OLD.remaining_model_tokens
    OR NEW.remaining_effect_actions > OLD.remaining_effect_actions
    OR NEW.revision != OLD.revision + 1
    OR NEW.updated_at < OLD.updated_at
    OR (
        NEW.status != OLD.status
        AND NOT (
            (OLD.status = 'draft' AND NEW.status IN ('admitted', 'rejected', 'cancelled'))
            OR (OLD.status = 'admitted' AND NEW.status IN ('running', 'cancelled'))
            OR (OLD.status = 'running' AND NEW.status IN (
                    'completed', 'failed', 'cancelled', 'budget_exhausted'))
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'invalid work-order update');
END;

CREATE TRIGGER IF NOT EXISTS forbid_work_order_delete
BEFORE DELETE ON work_orders
BEGIN
    SELECT RAISE(ABORT, 'work orders are immutable history');
END;

-- ---------------------------------------------------------------------------
-- Node contracts
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS work_order_nodes (
    node_id            TEXT PRIMARY KEY,
    work_order_id      TEXT NOT NULL REFERENCES work_orders(work_order_id),
    node_uri           TEXT NOT NULL UNIQUE,
    executor_kind      TEXT NOT NULL CHECK (executor_kind IN (
                           'engine', 'local_model', 'owner', 'external_agent')),
    risk_tier          TEXT NOT NULL CHECK (risk_tier IN ('low', 'standard', 'high')),
    status             TEXT NOT NULL CHECK (status IN (
                           'pending', 'ready', 'active', 'completed',
                           'failed', 'cancelled')),
    timeout_secs       INTEGER NOT NULL CHECK (timeout_secs > 0),
    max_attempts       INTEGER NOT NULL CHECK (max_attempts > 0),
    authorizing_grant_id TEXT REFERENCES interoperability_authority_grants(grant_id),
    record_payload     BLOB NOT NULL,
    payload_format     TEXT NOT NULL
        CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_work_order_nodes_order
    ON work_order_nodes (work_order_id, status);

CREATE TRIGGER IF NOT EXISTS forbid_work_order_node_delete
BEFORE DELETE ON work_order_nodes
BEGIN
    SELECT RAISE(ABORT, 'work-order nodes are immutable history');
END;

-- Declared write targets, stored as digests so conflict derivation and lease
-- pre-checks work on sealed vaults without exposing scope.
CREATE TABLE IF NOT EXISTS work_order_node_write_targets (
    node_id        TEXT NOT NULL REFERENCES work_order_nodes(node_id),
    work_order_id  TEXT NOT NULL REFERENCES work_orders(work_order_id),
    target_digest  TEXT NOT NULL
        CHECK (length(target_digest) = 64 AND target_digest NOT GLOB '*[^0-9a-f]*'),
    created_at     TEXT NOT NULL,
    PRIMARY KEY (node_id, target_digest)
);

CREATE INDEX IF NOT EXISTS idx_work_order_node_write_targets_digest
    ON work_order_node_write_targets (target_digest, work_order_id);

CREATE TRIGGER IF NOT EXISTS forbid_work_order_node_write_target_update
BEFORE UPDATE ON work_order_node_write_targets
BEGIN
    SELECT RAISE(ABORT, 'declared write scope is immutable after admission');
END;

-- ---------------------------------------------------------------------------
-- Typed edges
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS work_order_edges (
    edge_id        TEXT PRIMARY KEY,
    work_order_id  TEXT NOT NULL REFERENCES work_orders(work_order_id),
    from_node_id   TEXT NOT NULL REFERENCES work_order_nodes(node_id),
    to_node_id     TEXT NOT NULL REFERENCES work_order_nodes(node_id),
    edge_kind      TEXT NOT NULL CHECK (edge_kind IN (
                       'data', 'artifact', 'state', 'conflict',
                       'resource', 'policy', 'verification', 'temporal')),
    -- Conflict edges are derived at admission from intersecting write scopes,
    -- not authored. Derivation is recorded so audits can distinguish them.
    derived        INTEGER NOT NULL CHECK (derived IN (0, 1)),
    detail         TEXT,
    created_at     TEXT NOT NULL,
    UNIQUE (work_order_id, from_node_id, to_node_id, edge_kind),
    CHECK (from_node_id != to_node_id),
    CHECK (edge_kind != 'conflict' OR derived = 1)
);

CREATE INDEX IF NOT EXISTS idx_work_order_edges_inbound
    ON work_order_edges (to_node_id, edge_kind);

CREATE INDEX IF NOT EXISTS idx_work_order_edges_outbound
    ON work_order_edges (from_node_id, edge_kind);

CREATE TRIGGER IF NOT EXISTS forbid_work_order_edge_update
BEFORE UPDATE ON work_order_edges
BEGIN
    SELECT RAISE(ABORT, 'admitted work-order edges are immutable');
END;

-- ---------------------------------------------------------------------------
-- Agent runs
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS agent_runs (
    run_id             TEXT PRIMARY KEY,
    run_uri            TEXT NOT NULL UNIQUE,
    work_order_id      TEXT NOT NULL REFERENCES work_orders(work_order_id),
    node_id            TEXT NOT NULL REFERENCES work_order_nodes(node_id),
    attempt_no         INTEGER NOT NULL CHECK (attempt_no > 0),
    status             TEXT NOT NULL CHECK (status IN (
                           'ready', 'leased', 'running', 'awaiting_approval',
                           'gated', 'completed', 'failed', 'cancelled',
                           'budget_exhausted')),
    failure_class      TEXT CHECK (failure_class IS NULL OR failure_class IN (
                           'transient', 'deterministic', 'specification',
                           'authorization', 'budget', 'conflict')),
    principal_uri      TEXT NOT NULL,
    actor_uri          TEXT NOT NULL,
    correlation_id     TEXT NOT NULL,
    causation_id       TEXT,
    started_at         TEXT,
    ended_at           TEXT,
    record_payload     BLOB NOT NULL,
    payload_format     TEXT NOT NULL
        CHECK (payload_format IN ('json-v1', 'mvenc-v1')),
    payload_wrapped_dek TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    UNIQUE (node_id, attempt_no),
    CHECK (
        (payload_format = 'json-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_work_order
    ON agent_runs (work_order_id, status, updated_at);

CREATE INDEX IF NOT EXISTS idx_agent_runs_correlation
    ON agent_runs (correlation_id, created_at);

CREATE TRIGGER IF NOT EXISTS enforce_agent_run_insert
BEFORE INSERT ON agent_runs
BEGIN
    SELECT CASE WHEN
        NEW.status != 'ready'
        OR NEW.failure_class IS NOT NULL
        OR NEW.started_at IS NOT NULL
        OR NEW.ended_at IS NOT NULL
        OR NEW.updated_at != NEW.created_at
    THEN RAISE(ABORT, 'invalid initial agent-run state') END;

    -- Attempt numbers advance exactly once per node; no skips.
    SELECT CASE WHEN NEW.attempt_no != 1 + COALESCE(
        (SELECT MAX(attempt_no) FROM agent_runs WHERE node_id = NEW.node_id), 0)
    THEN RAISE(ABORT, 'agent-run attempts must advance exactly once') END;
END;

CREATE TRIGGER IF NOT EXISTS enforce_agent_run_transition
BEFORE UPDATE ON agent_runs
BEGIN
    SELECT CASE WHEN
        NEW.run_id != OLD.run_id
        OR NEW.run_uri != OLD.run_uri
        OR NEW.work_order_id != OLD.work_order_id
        OR NEW.node_id != OLD.node_id
        OR NEW.attempt_no != OLD.attempt_no
        OR NEW.principal_uri != OLD.principal_uri
        OR NEW.actor_uri != OLD.actor_uri
        OR NEW.correlation_id != OLD.correlation_id
        OR NEW.created_at != OLD.created_at
        OR NEW.updated_at < OLD.updated_at
    THEN RAISE(ABORT, 'agent-run identity and attribution are immutable') END;

    SELECT CASE WHEN OLD.status IN (
        'completed', 'failed', 'cancelled', 'budget_exhausted')
    THEN RAISE(ABORT, 'terminal agent-run state is immutable') END;

    SELECT CASE WHEN NEW.status != OLD.status AND NOT (
        -- `awaiting_approval` is reachable from `ready` because the autonomy
        -- gate is consulted when a run starts, not only mid-execution. A run
        -- the owner has not authorized parks before it takes any write lease,
        -- so an unbounded wait never holds a target URI.
        (OLD.status = 'ready' AND NEW.status IN (
                'leased', 'awaiting_approval', 'cancelled', 'budget_exhausted'))
        OR (OLD.status = 'leased' AND NEW.status IN ('running', 'ready', 'cancelled', 'budget_exhausted'))
        OR (OLD.status = 'running' AND NEW.status IN (
                'awaiting_approval', 'gated', 'failed', 'cancelled', 'budget_exhausted'))
        -- Approval is unbounded and returns to the ready set, where the run
        -- must re-acquire its write leases under a fresh conflict check.
        OR (OLD.status = 'awaiting_approval' AND NEW.status IN (
                'ready', 'cancelled', 'budget_exhausted'))
        OR (OLD.status = 'gated' AND NEW.status IN (
                'completed', 'failed', 'ready', 'cancelled', 'budget_exhausted'))
    ) THEN RAISE(ABORT, 'invalid agent-run lifecycle transition') END;

    -- A run parked on approval holds no write leases.
    SELECT CASE WHEN NEW.status = 'awaiting_approval' AND EXISTS (
        SELECT 1 FROM agent_run_write_leases
        WHERE run_id = NEW.run_id AND released_at IS NULL
    ) THEN RAISE(ABORT, 'a run awaiting approval cannot hold write leases') END;

    SELECT CASE WHEN NEW.status IN ('failed', 'budget_exhausted')
        AND NEW.failure_class IS NULL
    THEN RAISE(ABORT, 'a failed run must record its failure class') END;

    SELECT CASE WHEN NEW.status IN (
        'completed', 'failed', 'cancelled', 'budget_exhausted')
        AND NEW.ended_at IS NULL
    THEN RAISE(ABORT, 'a terminal run must record an end time') END;
END;

CREATE TRIGGER IF NOT EXISTS forbid_agent_run_delete
BEFORE DELETE ON agent_runs
BEGIN
    SELECT RAISE(ABORT, 'agent runs are immutable history');
END;

-- ---------------------------------------------------------------------------
-- Exclusive write leases (migration 036 semantics)
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS agent_run_write_leases (
    lease_id            TEXT PRIMARY KEY,
    run_id              TEXT NOT NULL REFERENCES agent_runs(run_id),
    work_order_id       TEXT NOT NULL REFERENCES work_orders(work_order_id),
    target_digest       TEXT NOT NULL
        CHECK (length(target_digest) = 64 AND target_digest NOT GLOB '*[^0-9a-f]*'),
    governing_node_uri  TEXT NOT NULL,
    attempt_no          INTEGER NOT NULL CHECK (attempt_no > 0),
    claimed_at          TEXT NOT NULL,
    lease_expires_at    TEXT NOT NULL,
    released_at         TEXT,
    release_reason      TEXT CHECK (release_reason IS NULL OR release_reason IN (
                            'completed', 'awaiting_approval', 'cancelled',
                            'failed', 'expired_replaced')),
    created_at          TEXT NOT NULL,
    -- Half-open interval, bounded to at most one hour.
    --
    -- Integer epoch seconds, not julianday: julianday returns a float, and
    -- with sub-second timestamps its rounding error rejects a legitimate
    -- exactly-3600s lease. strftime('%s') truncates fractional seconds, so
    -- this bound admits at most ~1s of slop; the Rust-side WriteLease
    -- validation is nanosecond-exact and is the primary enforcement.
    CHECK (lease_expires_at > claimed_at),
    CHECK (
        CAST(strftime('%s', lease_expires_at) AS INTEGER)
        - CAST(strftime('%s', claimed_at) AS INTEGER) <= 3600
    ),
    CHECK ((released_at IS NULL) = (release_reason IS NULL)),
    CHECK (released_at IS NULL OR released_at >= claimed_at)
);

-- At most one unreleased lease per target within a governing node.
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_run_write_leases_active
    ON agent_run_write_leases (target_digest, governing_node_uri)
    WHERE released_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_agent_run_write_leases_run
    ON agent_run_write_leases (run_id, released_at);

CREATE TRIGGER IF NOT EXISTS enforce_write_lease_insert
BEFORE INSERT ON agent_run_write_leases
BEGIN
    SELECT CASE WHEN
        NEW.released_at IS NOT NULL
        OR NEW.release_reason IS NOT NULL
    THEN RAISE(ABORT, 'a write lease is claimed unreleased') END;

    -- The target must be within the declared write scope of the run's node
    -- contract. This is independent of the run's own attempt number: a lease
    -- replacement advances the claim counter, not the run attempt.
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM agent_runs r
        JOIN work_order_node_write_targets t ON t.node_id = r.node_id
        WHERE r.run_id = NEW.run_id
          AND t.target_digest = NEW.target_digest
    ) THEN RAISE(ABORT, 'write lease target is outside the declared write scope') END;

    -- An unexpired lease held by another run blocks the claim. An expired one
    -- must be released in the same transaction before replacement.
    SELECT CASE WHEN EXISTS (
        SELECT 1 FROM agent_run_write_leases
        WHERE target_digest = NEW.target_digest
          AND governing_node_uri = NEW.governing_node_uri
          AND released_at IS NULL
    ) THEN RAISE(ABORT, 'write lease target is already claimed') END;

    -- The claim counter advances exactly once per target, as migration 036
    -- advances delivery attempts exactly once per claimed event. Skips would
    -- make replacement history unauditable.
    SELECT CASE WHEN NEW.attempt_no != 1 + COALESCE((
        SELECT MAX(attempt_no) FROM agent_run_write_leases
        WHERE target_digest = NEW.target_digest
          AND governing_node_uri = NEW.governing_node_uri
    ), 0) THEN RAISE(ABORT, 'write lease claims must advance exactly once per target') END;
END;

-- Only release is permitted, only once, and only from an unreleased lease.
CREATE TRIGGER IF NOT EXISTS enforce_write_lease_release
BEFORE UPDATE ON agent_run_write_leases
BEGIN
    SELECT CASE WHEN
        NEW.lease_id != OLD.lease_id
        OR NEW.run_id != OLD.run_id
        OR NEW.work_order_id != OLD.work_order_id
        OR NEW.target_digest != OLD.target_digest
        OR NEW.governing_node_uri != OLD.governing_node_uri
        OR NEW.attempt_no != OLD.attempt_no
        OR NEW.claimed_at != OLD.claimed_at
        OR NEW.lease_expires_at != OLD.lease_expires_at
        OR NEW.created_at != OLD.created_at
    THEN RAISE(ABORT, 'write lease terms are immutable') END;

    SELECT CASE WHEN OLD.released_at IS NOT NULL
    THEN RAISE(ABORT, 'a released write lease cannot be modified') END;

    SELECT CASE WHEN NEW.released_at IS NULL
    THEN RAISE(ABORT, 'the only permitted write lease update is release') END;
END;

CREATE TRIGGER IF NOT EXISTS forbid_write_lease_delete
BEFORE DELETE ON agent_run_write_leases
BEGIN
    SELECT RAISE(ABORT, 'write lease history is immutable');
END;

-- ---------------------------------------------------------------------------
-- Gate evidence
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS agent_run_gate_results (
    result_id            TEXT PRIMARY KEY,
    run_id               TEXT NOT NULL REFERENCES agent_runs(run_id),
    work_order_id        TEXT NOT NULL REFERENCES work_orders(work_order_id),
    gate                 TEXT NOT NULL CHECK (gate IN (
                             'g0', 'g1', 'g2', 'g3', 'g4', 'g5', 'g6', 'g7')),
    outcome              TEXT NOT NULL CHECK (outcome IN ('pass', 'fail', 'blocked')),
    evaluator_actor_uri  TEXT NOT NULL,
    evidence_digest      TEXT NOT NULL
        CHECK (length(evidence_digest) = 64
               AND evidence_digest NOT GLOB '*[^0-9a-f]*'),
    detail               TEXT,
    evaluated_at         TEXT NOT NULL,
    created_at           TEXT NOT NULL,
    UNIQUE (run_id, gate)
);

CREATE INDEX IF NOT EXISTS idx_agent_run_gate_results_order
    ON agent_run_gate_results (work_order_id, gate, outcome);

-- A run can never satisfy its own review gate.
CREATE TRIGGER IF NOT EXISTS enforce_gate_result_independence
BEFORE INSERT ON agent_run_gate_results
BEGIN
    SELECT CASE WHEN NEW.gate = 'g5' AND EXISTS (
        SELECT 1 FROM agent_runs
        WHERE run_id = NEW.run_id
          AND actor_uri = NEW.evaluator_actor_uri
    ) THEN RAISE(ABORT, 'a run cannot satisfy its own G5') END;
END;

CREATE TRIGGER IF NOT EXISTS forbid_gate_result_update
BEFORE UPDATE ON agent_run_gate_results
BEGIN
    SELECT RAISE(ABORT, 'gate results are immutable evidence');
END;

CREATE TRIGGER IF NOT EXISTS forbid_gate_result_delete
BEFORE DELETE ON agent_run_gate_results
BEGIN
    SELECT RAISE(ABORT, 'gate results are immutable evidence');
END;

-- ---------------------------------------------------------------------------
-- Artifacts
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS agent_run_artifacts (
    artifact_id        TEXT PRIMARY KEY,
    artifact_uri       TEXT NOT NULL UNIQUE,
    run_id             TEXT NOT NULL REFERENCES agent_runs(run_id),
    work_order_id      TEXT NOT NULL REFERENCES work_orders(work_order_id),
    artifact_kind      TEXT NOT NULL,
    content_digest     TEXT NOT NULL
        CHECK (length(content_digest) = 64
               AND content_digest NOT GLOB '*[^0-9a-f]*'),
    schema_uri         TEXT,
    schema_version     TEXT,
    sensitivity        TEXT NOT NULL,
    retention          TEXT NOT NULL,
    -- Artifact content is opaque bytes, not a governed JSON record, so the
    -- plaintext format is `bytes-v1` rather than the `json-v1` used by every
    -- other governed table here. Routing content through serde_json would
    -- encode it as a JSON array of integers — about four bytes stored per byte
    -- of content, and a lie about what the column holds.
    payload            BLOB NOT NULL,
    payload_format     TEXT NOT NULL
        CHECK (payload_format IN ('bytes-v1', 'mvenc-v1')),
    payload_wrapped_dek TEXT,
    created_at         TEXT NOT NULL,
    CHECK ((schema_uri IS NULL) = (schema_version IS NULL)),
    CHECK (
        (payload_format = 'bytes-v1' AND payload_wrapped_dek IS NULL)
        OR
        (payload_format = 'mvenc-v1' AND payload_wrapped_dek IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_agent_run_artifacts_run
    ON agent_run_artifacts (run_id, created_at);

CREATE INDEX IF NOT EXISTS idx_agent_run_artifacts_order
    ON agent_run_artifacts (work_order_id, artifact_kind, created_at);

-- Provenance references from an artifact to the anchors and inputs it derives
-- from. Derived knowledge never erases the authority of its evidence.
CREATE TABLE IF NOT EXISTS agent_run_artifact_provenance (
    artifact_id    TEXT NOT NULL REFERENCES agent_run_artifacts(artifact_id),
    relation       TEXT NOT NULL,
    reference_uri  TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    PRIMARY KEY (artifact_id, relation, reference_uri)
);

CREATE INDEX IF NOT EXISTS idx_agent_run_artifact_provenance_reference
    ON agent_run_artifact_provenance (reference_uri);

CREATE TRIGGER IF NOT EXISTS forbid_artifact_update
BEFORE UPDATE ON agent_run_artifacts
BEGIN
    SELECT RAISE(ABORT, 'artifacts are immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_artifact_delete
BEFORE DELETE ON agent_run_artifacts
BEGIN
    SELECT RAISE(ABORT, 'artifacts are immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_artifact_provenance_update
BEFORE UPDATE ON agent_run_artifact_provenance
BEGIN
    SELECT RAISE(ABORT, 'artifact provenance is immutable');
END;

CREATE TRIGGER IF NOT EXISTS forbid_artifact_provenance_delete
BEFORE DELETE ON agent_run_artifact_provenance
BEGIN
    SELECT RAISE(ABORT, 'artifact provenance is immutable');
END;

-- ---------------------------------------------------------------------------
-- Built-in event schemas
--
-- Event admission fails closed against this registry, which is gate G1.
-- ---------------------------------------------------------------------------

INSERT OR IGNORE INTO interoperability_public_schemas (
    schema_uri, schema_version, media_type, definition_json, content_digest,
    lifecycle, owner_uri, created_at, deprecated_at
) VALUES
(
    'mindvault://schemas/work-order-admitted',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["work_order_id","node_count","edge_count","governing_node_uri","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.work-order.admitted.v1"}',
    'f541cd04ee6dc23a798c9a161d1fa3af416e5b3ce22bf8f855f3d1b42d7f2f05',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/work-order-lifecycle-transitioned',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["work_order_id","revision","from_status","to_status","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.work-order.lifecycle.transitioned.v1"}',
    '75b770b3e11c431ef4fc3c4a8537efe5668a15e4aa6ce696023bd849d56609f3',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/agent-run-started',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["run_id","work_order_id","node_id","attempt_no","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.agent-run.started.v1"}',
    '496e7e671968e13eed4bc82f79507e0ce847caf845e1023e4433808b3f083a03',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
),
(
    'mindvault://schemas/agent-run-lifecycle-transitioned',
    '1.0.0',
    'application/schema+json',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","required":["run_id","from_status","to_status","record_digest"],"type":"object","x-mindvault-event-type":"dev.mindvault.agent-run.lifecycle.transitioned.v1"}',
    'f74391a44abc00756170042b18a1b61087fc491dabc3dcda66494444fa161886',
    'active',
    'mindvault://schemas/governance',
    '2026-07-26T00:00:00Z',
    NULL
);

INSERT OR IGNORE INTO schema_version (version, applied_at)
VALUES (38, datetime('now'));

COMMIT;
