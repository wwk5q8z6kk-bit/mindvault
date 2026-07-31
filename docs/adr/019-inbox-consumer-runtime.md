# ADR 019: Inbox Consumer Runtime (IK-006)

- **Status:** Accepted
- **Date:** 2026-07-31
- **Owners:** MindVault
- **Constitution:** Law 4, Law 14
- **Related:** `CONSUMER_INBOX_MODEL.md`, IK-006 / IK-007

## Context

Migration 037 and storage APIs already enforce consumer admission, exclusive
leases, immutable application receipts, and checkpoint advancement. Without a
runtime loop, admitted inbox events never reach domain application.

## Decision

Add an engine consumer (`consume_inbox_once`) that:

1. claims via `claim_consumer_events`
2. asks an `InboxDomainHandler` for one application result
3. completes via `complete_consumer_event` (sole receipt write path)

Ship `LocalProjectionHandler` for this milestone. Server spawn is env-gated
(`MINDVAULT_INBOX_CONSUMER_ENABLED`, default off). Admission remains explicit
via `admit_inbound_event`; no remote transport listener in this slice.

## Consequences

- Admitted events can be applied locally without a remote subscriber.
- Retry receipts do not advance checkpoints; applied outcomes do atomically.
- IK-007 adds governed dead-letter redrive without rewriting terminal state.
- Remote signature verification remains IK-014.
