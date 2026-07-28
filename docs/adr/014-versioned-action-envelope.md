# ADR 014: Versioned Action Envelope (IK-002)

- **Status:** Accepted
- **Date:** 2026-07-27
- **Owners:** MindVault
- **Constitution:** `INTEROPERABILITY_CONSTITUTION.md` Law 8
- **Related:** ADR 010 §Action envelope, ADR 013, `ACTION_RECEIPT_MODEL.md`, IK-002 / IK-020

## Context

IK-001 wired grant admission for `POST /api/v1/nodes`. Admission answers an
authorization question (`CommandAdmissionRequest`) but does not yet produce the
versioned attribution record ADR 010 requires for every durable command.

ADR 010 lists a full envelope including Space, work-order, approval, budget,
and outcome fields. Those registries are incomplete. Claiming the name only for
a Trust Ledger-complete record would leave mutating commands without any
validated attribution surface until IK-020.

## Decision

Introduce `ActionEnvelope` (`mindvault.action-envelope/v1`) as the kernel-scoped
command-attribution record with fail-closed validation for:

- action ID and correlation ID (nil UUIDs rejected)
- principal and acting actor
- resource and operation
- grant IDs (required and non-empty when admitted; empty when denied)
- policy decision (`AdmissionDecision`)

`admit_command` returns this envelope whenever admission mode is active.
`POST` / `PUT` / `DELETE /api/v1/nodes` construct it before mutating. Create
embeds `action_envelope` metadata beside the existing `admission` block in the
event payload so `off` stays byte-identical when admission is inactive.

Space, work-order, agent-run, approval, budget, and outcome fields remain
deferred. Expanding the envelope is additive under a new version string.

## Consequences

- Observe/enforce paths carry a validated attribution record before mutation.
- Trust Ledger (IK-020) can append complete envelopes later without renaming.
- Remaining mutating REST surfaces adopt the same helper as they gain grant
  admission (tracked with IK-017 event coverage where applicable).
