# ADR 014: Proposal-Only Non-Human Writes

Status: accepted invariant

## Problem

AI, connectors, agents, sync peers, and derived jobs must not silently change canonical knowledge.

## Current state

Proposal/approval/undo implementations exist, but auto-tagging, sync, intent apply, enrichment, and
some direct store calls bypass them. Approval lacks a universal atomic compare-and-swap boundary.

## Alternatives

1. trusted direct writers;
2. confidence-based auto-approval;
3. universal typed proposals with human or explicit deterministic policy decision;
4. append-only suggestions never applicable.

## Measurements required

proposal latency/volume, stale-base rate, review time, false approvals, rollback success, and
idempotency under retries/races.

## Security implications

The proposal gateway limits compromised AI/connectors. Approval authentication, base-version CAS,
and transaction-linked audit prevent replay/confused-deputy attacks.

## Privacy implications

Proposal evidence and diffs can expose content; authorize and redact them like canonical knowledge.
Model provenance/disclosure remains attached.

## Migration implications

Inventory and revoke direct store capabilities, define typed operations, convert existing automated
paths, and quarantine any path that cannot propose.

## Chosen direction

All non-human canonical changes use typed proposals. Human approval is default; narrowly scoped,
visible, reversible deterministic policy may approve low-risk types. Model confidence alone never
approves.

## Rejected alternatives

Trusted direct writers and confidence thresholds violate the constitution. Never-applicable
suggestions prevent useful controlled automation.

## Reversal path

Disable automatic policy decisions and require human approval for all proposals; canonical data and
proposal history remain valid.

## Acceptance criteria

One canonical mutation capability, compile/runtime bypass tests, typed evidence/diff/base hash,
atomic decision+mutation+audit+rollback+outbox, idempotency, stale-base rejection, and successful
rollback.
