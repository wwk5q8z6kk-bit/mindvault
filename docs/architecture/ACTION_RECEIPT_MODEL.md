# Action Receipt Model

- **Status:** Implemented internal storage contract
- **Date:** 2026-07-26
- **Scope:** Transactional outbox delivery attempts and their durable evidence

## Purpose

An action receipt records what happened during one claimed publication attempt.
It is evidence of an attempt and its declared destination outcome, not the
authority to perform the action and not proof that every downstream consumer
applied it.

## Delivery lifecycle

Each pending outbox event may be claimed by one executor for one destination
under a lease of at most one hour. The lease interval is
`[claimed_at, lease_expires_at)`: completion at or after expiration fails
closed. An expired claim can be replaced atomically, advancing the attempt
number and invalidating completion by the prior claim.

One valid completion produces exactly one outcome:

- `published`: the declared destination acknowledged delivery;
- `retry_scheduled`: the attempt failed transiently and has a future retry time;
- `dead_lettered`: the failure is terminal for this event.

Published and dead-letter events cannot be claimed again. Retry scheduling
clears the lease and prevents a new claim until `next_attempt_at`.

## Receipt identity and attribution

Every receipt contains:

- receipt version and ID;
- event ID, exact claim ID, and monotonically increasing attempt number;
- executor and destination;
- subject, principal, actor, and correlation ID;
- canonical request digest and optional response digest;
- start and completion timestamps;
- sensitivity, retention, and provenance;
- outcome-specific provider reference or bounded failure details.

The receipt payload and its indexed routing fields are cross-checked on read.
Completion replay with the same claim and result returns the original receipt;
a different result for that attempt is an idempotency conflict.

## Persistence invariants

Receipt insertion and outbox state transition share one immediate SQLite
transaction. Database triggers require an active matching claim before receipt
insertion, require the receipt before delivery completion, prevent attempt
counter skips, and prohibit updates or deletion of outbox history and receipts.

Sealed vaults encrypt the full receipt payload using `mvenc-v1`. Provider
references and error summaries are not copied into plaintext status columns;
the status keeps only the bounded error code needed for operations.

## Explicit non-claims

This slice does not run a dispatcher, contact a provider, verify a remote
signature, or itself prove consumer application. Public action-envelope
admission must resolve an effective Tool Grant and policy decision before any
live publisher is enabled. Consumer application is represented separately by
the durable inbox, application receipt, and checkpoint contract.

### Command admission does not write an action receipt

`interoperability_action_receipts` is evidence of **publication attempts only**,
and it is closed to anything else by construction: the
`enforce_action_receipt_claim_binding` trigger requires a matching active outbox
claim, and `ActionReceipt::from_outbox_delivery` is the sole constructor. The
only legal write path is `claim_outbox_events` → `complete_outbox_delivery`.

A command-admission decision therefore **must not** be recorded here, and must
not be forced in by self-claiming the event with executor and destination both
set to the local node. Doing so would burn attempt #1 of the real delivery
budget, set `published_at` and make the event terminal so a future publisher
could never deliver it, and assert a destination acknowledgement that never
happened.

An admitted command records its decision in the durable event envelope instead —
committed in the same transaction as the mutation, so laws 4 and 5 hold without
a new table — plus the audit trail and metrics. A denied command produces no
mutation and therefore, correctly, no event; law 4 covers *committed* mutations,
and the denial is recorded in the audit trail.

Making denials durable requires a separate append-only table with its own
immutability triggers. That is deferred because it needs a migration, the one
change class that is not trivially reversible on a running vault.
