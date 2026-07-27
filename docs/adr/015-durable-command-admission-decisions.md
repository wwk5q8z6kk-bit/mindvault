# ADR 015: Durable Command-Admission Decisions (IK-001c)

- **Status:** Accepted
- **Date:** 2026-07-27
- **Owners:** MindVault
- **Constitution:** Law 15
- **Related:** ADR 010, ADR 014, `ACTION_RECEIPT_MODEL.md`, IK-001 / IK-001c / IK-020

## Context

Denied commands produce no mutation and therefore no outbox event. Law 15 still
requires the denial be recorded. Writing denials into `interoperability_action_receipts`
is forbidden: that table is publication-attempt evidence only.

## Decision

Add append-only `interoperability_command_admission_decisions` (migration 039)
with immutability triggers. `MindVaultEngine::resolve_command_admission`
persists every decision fail-closed before returning. Rows are idempotent on
`(principal_uri, idempotency_key)`; a digest or semantic decision mismatch is
`IdempotencyConflict`.

Admitted decisions may also be stored here as the first Trust Ledger brick;
committed mutations continue to carry admission metadata in the event envelope.

## Consequences

- Observe/enforce denials survive restart and are queryable by principal+key.
- Action receipts remain publication-only.
- Trust Ledger (IK-020) can later unify this surface with complete action envelopes.
