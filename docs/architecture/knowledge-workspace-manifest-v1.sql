-- Knowledge Workspace managed-manifest contract, version 1.
--
-- This is the readable contract source for active production migration
-- `migrations/031_knowledge_workspace_manifest.sql`. Changes must remain
-- forward-compatible and be proven through upgrade/restore tests.

PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE workspace_manifest_versions (
    version INTEGER PRIMARY KEY,
    contract_name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    namespace TEXT NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('mounted', 'managed_plaintext')),
    state TEXT NOT NULL CHECK (
        state IN ('indexing', 'ready', 'degraded', 'offline', 'error')
    ),
    -- JSON when unsealed mode is configured; an authenticated encrypted
    -- envelope when sealed mode is configured. Contains display name, root
    -- locator, inclusion rules, platform observations, and scan cursor.
    descriptor_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_reconciled_at TEXT
);

CREATE INDEX idx_workspaces_namespace ON workspaces(namespace);
CREATE INDEX idx_workspaces_state ON workspaces(state);

CREATE TABLE workspace_documents (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Lowercase hex sha256(normalized path) outside sealed mode; lowercase hex
    -- HMAC-SHA256 with a protected workspace key in sealed mode. This supports
    -- uniqueness without persisting a reversible path in a plaintext column.
    path_token TEXT NOT NULL CHECK (
        length(path_token) = 64
        AND path_token NOT GLOB '*[^0-9a-f]*'
    ),
    -- Contains relative path, exact content hash, observed filesystem identity
    -- hints, byte size, file timestamps, and diagnostics.
    document_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    lifecycle_state TEXT NOT NULL CHECK (
        lifecycle_state IN (
            'active',
            'missing',
            'trashed',
            'conflict',
            'unsupported'
        )
    ),
    projection_state TEXT NOT NULL CHECK (
        projection_state IN ('pending', 'ready', 'failed', 'stale')
    ),
    projected_node_id TEXT REFERENCES knowledge_nodes(id) ON DELETE SET NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (workspace_id, path_token)
);

CREATE INDEX idx_workspace_documents_lifecycle
    ON workspace_documents(workspace_id, lifecycle_state);
CREATE INDEX idx_workspace_documents_projection
    ON workspace_documents(workspace_id, projection_state);
CREATE INDEX idx_workspace_documents_projected_node
    ON workspace_documents(projected_node_id);

CREATE TABLE workspace_events (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    document_id TEXT REFERENCES workspace_documents(id) ON DELETE SET NULL,
    event_seq INTEGER NOT NULL CHECK (event_seq > 0),
    correlation_id TEXT NOT NULL,
    actor_kind TEXT NOT NULL CHECK (
        actor_kind IN (
            'user',
            'system',
            'migration',
            'external',
            'plugin',
            'mcp',
            'automation',
            'ai_proposal'
        )
    ),
    actor_id TEXT,
    operation TEXT NOT NULL CHECK (
        operation IN (
            'mount',
            'scan',
            'create',
            'update',
            'move',
            'trash',
            'restore',
            'external_create',
            'external_update',
            'external_move',
            'external_delete',
            'conflict_resolve',
            'migration_stage',
            'migration_commit',
            'migration_rollback'
        )
    ),
    status TEXT NOT NULL CHECK (
        status IN ('prepared', 'completed', 'aborted', 'conflict')
    ),
    -- Contains expected-old and intended-new paths/hashes, authorization scope,
    -- operation parameters, result, and error detail.
    event_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    prepared_at TEXT NOT NULL,
    completed_at TEXT,
    UNIQUE (workspace_id, event_seq)
);

CREATE INDEX idx_workspace_events_document
    ON workspace_events(document_id, event_seq);
CREATE INDEX idx_workspace_events_status
    ON workspace_events(workspace_id, status);
CREATE INDEX idx_workspace_events_correlation
    ON workspace_events(correlation_id);

CREATE TABLE workspace_document_versions (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    document_id TEXT NOT NULL REFERENCES workspace_documents(id) ON DELETE CASCADE,
    source_event_id TEXT REFERENCES workspace_events(id) ON DELETE SET NULL,
    -- Opaque content-addressed reference into managed blob storage.
    blob_ref TEXT NOT NULL,
    -- Contains the exact content hash, source path at capture time, reason,
    -- pinning references, and optional retention decision.
    version_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    created_at TEXT NOT NULL,
    UNIQUE (document_id, blob_ref)
);

CREATE INDEX idx_workspace_versions_document
    ON workspace_document_versions(document_id, created_at);
CREATE INDEX idx_workspace_versions_pinned
    ON workspace_document_versions(workspace_id, pinned);

CREATE TABLE workspace_conflicts (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    document_id TEXT REFERENCES workspace_documents(id) ON DELETE SET NULL,
    source_event_id TEXT REFERENCES workspace_events(id) ON DELETE SET NULL,
    conflict_kind TEXT NOT NULL CHECK (
        conflict_kind IN (
            'stale_write',
            'path_collision',
            'ambiguous_rename',
            'external_divergence',
            'unsafe_path',
            'restore_collision'
        )
    ),
    state TEXT NOT NULL CHECK (
        state IN ('open', 'resolved', 'dismissed')
    ),
    -- Contains observed, expected-old, and intended-new paths/hashes plus the
    -- selected resolution. Canonical content is referenced, not embedded.
    conflict_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX idx_workspace_conflicts_open
    ON workspace_conflicts(workspace_id, state, created_at);
CREATE INDEX idx_workspace_conflicts_document
    ON workspace_conflicts(document_id);

CREATE TABLE workspace_migrations (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
    source_namespace TEXT NOT NULL,
    plan_hash TEXT NOT NULL,
    state TEXT NOT NULL CHECK (
        state IN (
            'planned',
            'staged',
            'verified',
            'committed',
            'rolled_back',
            'failed'
        )
    ),
    -- Contains the stage-root locator, policy version, counts, report location,
    -- checkpoints, and failure detail.
    migration_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    committed_at TEXT,
    rolled_back_at TEXT,
    UNIQUE (source_namespace, plan_hash)
);

CREATE INDEX idx_workspace_migrations_state
    ON workspace_migrations(state, updated_at);

CREATE TABLE workspace_migration_items (
    migration_id TEXT NOT NULL
        REFERENCES workspace_migrations(id) ON DELETE CASCADE,
    source_node_id TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE RESTRICT,
    source_node_version INTEGER NOT NULL CHECK (source_node_version > 0),
    target_path_token TEXT NOT NULL CHECK (
        length(target_path_token) = 64
        AND target_path_token NOT GLOB '*[^0-9a-f]*'
    ),
    item_state TEXT NOT NULL CHECK (
        item_state IN (
            'planned',
            'staged',
            'verified',
            'committed',
            'rolled_back',
            'failed',
            'skipped'
        )
    ),
    -- Contains original title, selected relative path, expected/staged hash,
    -- diagnostics, and rollback result.
    item_payload BLOB NOT NULL,
    payload_format TEXT NOT NULL CHECK (
        payload_format IN ('json-v1', 'mvenc-v1')
    ),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (migration_id, source_node_id),
    UNIQUE (migration_id, target_path_token)
);

CREATE INDEX idx_workspace_migration_items_state
    ON workspace_migration_items(migration_id, item_state);

INSERT INTO workspace_manifest_versions(version, contract_name, applied_at)
VALUES (1, 'knowledge-workspace-manifest-v1', datetime('now'));

COMMIT;

-- Schema rollback policy:
--
-- * Before a release has written workspace data, a failed test migration may
--   drop these tables in reverse dependency order inside one transaction.
-- * After activation, production migrations are forward-only. Product-level
--   rollback disables workspace writers, restores the pre-migration Notes
--   authority checkpoint, preserves journal/version evidence, and leaves these
--   tables available for diagnosis. It must not DROP user history.
