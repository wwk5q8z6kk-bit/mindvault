-- SPACE-003: durable adapter bindings and unique provider delivery IDs.
--
-- Adapter configs were process-memory only; poll cursors already persist in
-- adapter_poll_state (022). This migration adds binding persistence and a
-- uniqueness boundary for inbound provider delivery IDs so duplicate ingest
-- is rejected and poll cursors can stay behind partial failure.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS adapter_bindings (
    id TEXT PRIMARY KEY NOT NULL,
    adapter_type TEXT NOT NULL CHECK (
        adapter_type IN ('slack', 'discord', 'email')
    ),
    name TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    settings_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_adapter_bindings_type
    ON adapter_bindings (adapter_type, enabled);

CREATE TABLE IF NOT EXISTS adapter_provider_deliveries (
    adapter_id TEXT NOT NULL,
    provider_delivery_id TEXT NOT NULL,
    received_at TEXT NOT NULL,
    PRIMARY KEY (adapter_id, provider_delivery_id)
);

CREATE INDEX IF NOT EXISTS idx_adapter_provider_deliveries_received
    ON adapter_provider_deliveries (adapter_id, received_at);

INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (43, datetime('now'));

COMMIT;
