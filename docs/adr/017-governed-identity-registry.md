# ADR 017: Governed Identity Registry

- **Status:** Accepted
- **Date:** 2026-07-30
- **Owners:** MindVault
- **Constitution:** `INTEROPERABILITY_CONSTITUTION.md` Law 5
- **Related:** ADR 010 §Actors, ADR 013, IK-003

## Context

Command admission and grant issuance previously derived principal URIs
inline via `Uuid::new_v5(local_node_id, subject)`. That transitional
`local-system` identity was explicitly labelled temporary in
`interoperability-kernel-v1.md`.

## Decision

Lift the deferral with a versioned identity registry:

- `IdentityRecord` stores actor kind, subject binding, and lifecycle status
- migration `040_identity_registry.sql` mirrors Context Node / Grant registry patterns
- bootstrap registers `local-system` and `local-context-owner` with unchanged v5 URIs
- REST resolves principals through `resolve_command_identity`
- legacy v5 derivation remains opt-in via `MINDVAULT_IDENTITY_LEGACY_FALLBACK=1`

## Consequences

- Principal URIs stay byte-identical; replay indexes and grants are not orphaned
- Unknown auth subjects fail closed in production configuration
- Admin `POST/GET /api/v1/identities` exposes governed registration and listing
