# ADR 013: Admin Authority Grant APIs

- **Status:** Accepted
- **Date:** 2026-07-27
- **Owners:** MindVault
- **Constitution:** `INTEROPERABILITY_CONSTITUTION.md` Law 2, Law 7, Law 8
- **Related:** ADR 010, ADR 011, `AUTHORITY_GRANT_MODEL.md`, IK-001 / IK-001a / IK-001b

## Context

IK-001 wired grant admission to `POST /api/v1/nodes` behind
`MINDVAULT_COMMAND_ADMISSION_MODE`. IK-001a exposed local Context Node
registration so a fresh vault can satisfy the storage prerequisite for
issuance. Public grant APIs were deliberately deferred in
`AUTHORITY_GRANT_MODEL.md` ("remain later integration gates") so that the
storage and admission contracts could ship without inventing an incomplete
operator surface.

Without an issuance path, `enforce` is only usable from test fixtures. That
blocks the constitution's feature-completeness requirement that command and
query surfaces ship together and that grants be obtainable without hand-rolled
storage commits.

## Decision

Lift the deferral for **admin-only** grant issuance and lifecycle:

- `POST /api/v1/authority-grants` issues a Context or Tool Grant
- `GET /api/v1/authority-grants` and `GET /api/v1/authority-grants/:id` read them
- `POST .../suspend`, `.../revoke`, and `.../resume` apply the existing
  lifecycle transitions

The grantor on issued records is always the vault's local owner principal
(`local-context-owner`). Admin authentication authorizes the *command*; it does
not become the grantor. That keeps `grantor != grantee` satisfiable when an
operator grants their own acting principal (including the default
`local-system` subject).

Defaults chosen for the common enforce path: kind `tool`, capability
`command`, target = local node URI, retention ceiling `durable`. Callers that
need Context Grants or narrower targets pass them explicitly.

Delegation issuance, signatures, and non-admin public grant APIs remain gated.

## Consequences

- `enforce` becomes reachable on a real vault after IK-001a registration plus
  one Tool Grant issuance.
- Operators can suspend and revoke without direct storage access.
- The model document's "later integration gates" sentence for public grant APIs
  is narrowed: admin APIs are now in, non-admin and signed grants remain later.
