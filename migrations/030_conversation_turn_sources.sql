-- Persist grounded chat citation sources on conversation turns.
ALTER TABLE conversation_turns ADD COLUMN sources_json TEXT;

INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (30, datetime('now'));
