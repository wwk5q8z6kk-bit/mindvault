-- Track re-encryption completion per key epoch (idempotent ALTER TABLE).
ALTER TABLE key_epochs ADD COLUMN re_encryption_completed_at TEXT;
