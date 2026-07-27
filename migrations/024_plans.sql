-- Phase 3: Agent planning framework.
-- Plans and steps with status state machine.
--
-- SUPERSEDED by migration 038 (Work Orders, Agent Runs, Artifacts) per
-- docs/adr/012-governed-agent-execution-graph.md.
--
-- These tables are retained but unused. Verified before superseding: no code
-- reads or writes `plans` or `plan_steps`, and no store trait exposes them, so
-- no data migration is required. `plan_steps` models a linear `step_order`
-- sequence with no typed edges, declared scope, budgets, or gate evidence, and
-- its `action` vocabulary does not compose with `IntentType`.
--
-- Do not add readers or writers here. New orchestration belongs in the
-- governed execution graph, where declared write scope resolves against an
-- AuthorityGrant before anything becomes schedulable.

CREATE TABLE IF NOT EXISTS plans (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    goal TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',   -- draft, approved, running, completed, failed, cancelled
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    completed_at TEXT,
    metadata TEXT DEFAULT '{}'              -- JSON for extensibility
);

CREATE TABLE IF NOT EXISTS plan_steps (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    step_order INTEGER NOT NULL,
    action TEXT NOT NULL,           -- recall, store, link, tag, summarize, ask_user
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, skipped
    input TEXT DEFAULT '{}',        -- JSON: parameters for the action
    output TEXT,                    -- JSON: result of the action
    error TEXT,                     -- error message if failed
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_plan_steps_plan_id
    ON plan_steps(plan_id, step_order);

CREATE INDEX IF NOT EXISTS idx_plans_status
    ON plans(status);

INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (24, datetime('now'));
