-- AGENT-002: retire the superseded plans / plan_steps schema (migration 024).
--
-- ADR 012 / migration 038 already replaced planning with Work Orders. Verified
-- before dropping: no Rust reader or writer touches these tables, and the plan
-- REST surface either returns 410 Gone or (create_plan) returns an ephemeral
-- suggestion that never persists. There is no data to migrate.

BEGIN IMMEDIATE;

DROP INDEX IF EXISTS idx_plan_steps_plan_id;
DROP INDEX IF EXISTS idx_plans_status;
DROP TABLE IF EXISTS plan_steps;
DROP TABLE IF EXISTS plans;

INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (41, datetime('now'));

COMMIT;
