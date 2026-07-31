-- SPACE-001: Collaborative Workspace / Space / Membership / resource ownership.
--
-- ADR 010: Space is the authorization root; a product-level Collab Workspace is
-- only an administrative container. Document workspaces (migration 031) are a
-- different concept — tables here are explicitly named `collab_*` / `spaces`
-- so neither inherits the other's authorization semantics by name (SPACE-006).
--
-- Actors are the governed identity registry (migration 040). Memberships
-- foreign-key `principal_id` and denormalize `actor_kind` for matrix checks.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS collab_workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (
        state IN ('active', 'suspended', 'retired')
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS spaces (
    id TEXT PRIMARY KEY NOT NULL,
    collab_workspace_id TEXT NOT NULL
        REFERENCES collab_workspaces(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    display_name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (
        state IN ('active', 'archived', 'retired')
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (collab_workspace_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_spaces_collab_workspace
    ON spaces (collab_workspace_id, state);

CREATE TABLE IF NOT EXISTS space_memberships (
    id TEXT PRIMARY KEY NOT NULL,
    space_id TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    principal_id TEXT NOT NULL
        REFERENCES interoperability_identities(principal_id),
    actor_kind TEXT NOT NULL CHECK (
        actor_kind IN ('human', 'agent', 'service', 'integration')
    ),
    role TEXT NOT NULL CHECK (
        role IN ('owner', 'admin', 'member', 'viewer')
    ),
    state TEXT NOT NULL CHECK (
        state IN ('active', 'suspended', 'revoked')
    ),
    granted_at TEXT NOT NULL,
    expires_at TEXT,
    UNIQUE (space_id, principal_id)
);

CREATE INDEX IF NOT EXISTS idx_space_memberships_principal
    ON space_memberships (principal_id, state);
CREATE INDEX IF NOT EXISTS idx_space_memberships_space_active
    ON space_memberships (space_id, state, role);

CREATE TABLE IF NOT EXISTS space_resources (
    id TEXT PRIMARY KEY NOT NULL,
    space_id TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    resource_kind TEXT NOT NULL CHECK (
        resource_kind IN (
            'document',
            'artifact',
            'subscription',
            'search_index',
            'generic'
        )
    ),
    resource_key TEXT NOT NULL,
    owner_principal_id TEXT
        REFERENCES interoperability_identities(principal_id),
    created_at TEXT NOT NULL,
    UNIQUE (space_id, resource_kind, resource_key)
);

CREATE INDEX IF NOT EXISTS idx_space_resources_space
    ON space_resources (space_id, resource_kind);

INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (42, datetime('now'));

COMMIT;
