# ADR 016: Outbox Dispatcher Runtime (IK-004)

- **Status:** Accepted
- **Date:** 2026-07-27
- **Owners:** MindVault
- **Constitution:** Law 4
- **Related:** ADR 010, `ACTION_RECEIPT_MODEL.md`, IK-004 / IK-005

## Context

Migration 036 and storage APIs already enforce exclusive leases, attempt
counters, `next_attempt_at`, and immutable action receipts. Without a runtime
loop, pending outbox events accumulate forever.

## Decision

Add an engine dispatcher (`dispatch_outbox_once`) that:

1. claims via `claim_outbox_events`
2. asks an `OutboxPublisher` for one attempt result
3. completes via `complete_outbox_delivery` (sole receipt write path)

Ship `LocalAckPublisher` for this milestone. Server spawn is env-gated
(`MINDVAULT_OUTBOX_DISPATCH_ENABLED`, default off) so operators opt in.

## Consequences

- Pending events can drain without a live network publisher.
- IK-005 swaps in the first authenticated transport behind the same trait.
- Publisher errors fail closed to `RetryScheduled` with one receipt still written.
