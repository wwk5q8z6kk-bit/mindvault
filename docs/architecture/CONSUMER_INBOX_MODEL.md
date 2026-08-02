# Consumer Inbox and Checkpoint Model

- **Status:** Implemented internal storage contract
- **Date:** 2026-07-26
- **Scope:** Durable event admission, local application attempts, and checkpoints

## Purpose

The consumer inbox turns an accepted event envelope into durable local work.
It separates destination acknowledgement from consumer application and records
independently queryable evidence for every processing attempt.

## Admission and ordering

Admission requires a valid event envelope and an active public schema whose
declared event type matches the envelope. The same consumer and event ID may be
admitted once. Exact redelivery returns the original local sequence; changed
content under the same identity fails as an idempotency conflict.

Inbox sequence numbers are assigned locally. Processing preserves admission
order within each `(consumer, source)` stream. This ordering does not prove that
a remote issuer stream was complete or gap-free.

## Application leases

An eligible event may be claimed by one processor under a lease of at most one
hour. The lease interval is `[claimed_at, lease_expires_at)`: completion at or
after expiration fails closed. Expired claims are recoverable and every new
claim advances the attempt number exactly once.

One completion records exactly one outcome:

- `applied`: the consumer reports a durable application reference;
- `retry_scheduled`: the attempt remains pending until a future retry time;
- `dead_lettered`: the consumer cannot apply the event and disposition is
  terminal.

## Receipts and checkpoints

Each application receipt binds the event and local inbox sequence to the exact
claim ID, attempt, consumer, processor, source, subject, principal, actor,
correlation ID, request digest, timestamps, policy metadata, provenance, and
outcome-specific evidence. Receipt payloads are cross-checked against their
indexed fields when read.

Applied and dead-letter outcomes atomically advance the consumer/source
checkpoint. Retry receipts do not advance it. A checkpoint records the latest
terminally dispositioned local sequence, the latest applied sequence when one
exists, and cumulative applied and dead-letter counts.

Receipts, admitted envelopes, and checkpoints cannot be deleted through the
storage contract. Receipts are immutable, and terminal inbox states cannot be
rewritten.

## Security and non-claims

Sealed vaults encrypt full envelope and receipt payloads with `mvenc-v1`; only
bounded routing and lookup fields remain indexed. The storage contract does not
verify remote signatures, run a background consumer, authorize an action, or
prove global exactly-once delivery. A trustworthy end-to-end claim still
requires authenticated transport and a policy-authorized action envelope.
