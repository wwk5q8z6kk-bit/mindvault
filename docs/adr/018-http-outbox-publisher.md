# ADR 018: HTTP Outbox Publisher (IK-005)

- **Status:** Accepted
- **Date:** 2026-07-31
- **Owners:** MindVault
- **Constitution:** Law 4, Law 5
- **Related:** ADR 016, `ACTION_RECEIPT_MODEL.md`, IK-005

## Context

IK-004 shipped the dispatcher runtime with `LocalAckPublisher`. Operators need
an opt-in path to deliver outbox events to an authenticated HTTP destination
without bypassing receipt invariants or enabling side effects by default.

## Decision

Add `HttpOutboxPublisher` behind the existing `OutboxPublisher` trait:

1. POST the claimed `EventEnvelope` as JSON to a configured URL.
2. Map HTTP status to delivery results: 2xx published; 408/429/502/503/504 retry;
   other 4xx/5xx dead-letter.
3. Complete only through `complete_outbox_delivery` (no receipt shortcuts).
4. Keep `LocalAckPublisher` as the default. Enable HTTP only when both
   `MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED=1` and command admission is
   `observe` or `enforce`, plus a valid `MINDVAULT_OUTBOX_HTTP_DESTINATION_URL`.

## Consequences

- First live transport is available for conformance and operator pilots.
- Production defaults remain side-effect free until explicitly gated.
- Additional transports (Slack, webhooks, …) can reuse the same trait seam.
