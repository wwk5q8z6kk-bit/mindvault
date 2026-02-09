-- Adapter poll state for cursor persistence
CREATE TABLE IF NOT EXISTS adapter_poll_state (
    adapter_name TEXT PRIMARY KEY,
    cursor TEXT NOT NULL DEFAULT '',
    last_poll_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    messages_received INTEGER NOT NULL DEFAULT 0
);
