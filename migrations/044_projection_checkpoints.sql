-- IK-016: durable local projection checkpoints for knowledge.node.created.
--
-- Canonical commit + outbox land in one transaction; FTS/vector/graph run
-- afterward. Transport `published` must not imply projections applied. This
-- table records local projection readiness so a recovery worker can reconcile
-- crash windows without rewriting outbox delivery state.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS interoperability_projection_checkpoints (
    event_id TEXT PRIMARY KEY NOT NULL,
    node_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('ready', 'failed')),
    projected_at TEXT,
    error_summary TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES interoperability_outbox(event_id)
);

CREATE INDEX IF NOT EXISTS idx_interoperability_projection_checkpoints_status
    ON interoperability_projection_checkpoints (status, updated_at);

INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (44, datetime('now'));

COMMIT;
