# Recovery: governed agent execution graph (ADR 012)

Written 2026-07-27. Delete this file once the work is either reintegrated or
deliberately abandoned.

## What happened

The `feat/product-evolution-session` branch was reset to
`origin/feat/product-evolution-session` (`37b39ef`) partway through a session
that had built the `WorkOrder` / `AgentRun` / `Artifact` layer. A
`backup/product-evolution-local-20260727` branch was created immediately before
the reset, so the reset reads as deliberate history cleanup rather than an
accident.

The reset discarded far more than the execution graph. `37b39ef` does **not**
contain the interoperability kernel that landed earlier on the same branch:

- `migrations/035_authority_grants.sql`
- `migrations/036_outbox_dispatch_and_action_receipts.sql`
- `migrations/037_consumer_inbox_checkpoints.sql`
- `crates/mv-core/src/model/interoperability.rs`

## Why the execution graph cannot be cherry-picked alone

It is built directly on that kernel and does not compile without it:

- declared write scope resolves against an **AuthorityGrant** target set
  (migration 035) — this is the translation that makes the design safe rather
  than decorative, because a prompt-level scope list means nothing to a hostile
  agent;
- write leases reuse the bounded-lease/attempt-counter pattern from migration
  036 verbatim;
- `StableUri`, `EventEnvelope`, `ProvenanceReference`, and the governed
  string-enum macro all come from `interoperability.rs`.

Cherry-picking the three execution-graph commits onto `37b39ef` produces a tree
referencing types that do not exist there.

## Where the work is

Three local branches, created after the reset. They are **local only** and
survive until deleted.

| Branch | Commit | Contents |
| --- | --- | --- |
| `rescue/wo-run-driver` | `642d151` | run driver (`start_run`) |
| `rescue/wo-artifacts` | `f0f634d` | + artifact recording over HTTP (linear on the above) |
| `rescue/wo-live-observation` | `3a5f9c1` | **most complete tree** — also live run observation |

`rescue/wo-live-observation` sits on a *different* commit lineage (the reset to
the backup branch happened mid-session), so `642d151` and `f0f634d` are not its
ancestors. Its **tree** nonetheless contains all of the work — verified by
probing for `start_run`, `record_artifact`, `deliverable_to`, and
`agentRunObservations`. Recover from this branch, not by replaying the other
two.

```bash
git branch --list 'rescue/*'          # confirm the refs still exist
git diff --stat 37b39ef rescue/wo-live-observation
```

## What the layer contains

Contracts (`docs/adr/012-governed-agent-execution-graph.md`,
`WORK_ORDER_MODEL.md`, `EXECUTION_ISOLATION_MODEL.md`,
`AGENT_EXECUTION_GRAPH_ASSESSMENT.md`), `migrations/038_work_orders_and_agent_runs.sql`
(9 trigger-enforced tables), core types, storage, engine admission, the
non-bypassable autonomy gate, REST command+query surfaces with OpenAPI, portable
export/restore, read-only extension access, the `/work-orders` operator surface,
and a conformance suite covering all ten items of the Feature Completeness
Contract.

Last verified green before the reset:

```
mv-core 82 · mv-storage 98 · mv-engine 385 · mv-plugin 21
api_integration 33 · work_order_conformance 14 · frontend 522
cargo fmt --check clean; clippy -D warnings clean on the workspace
and on mv-plugin --all-features
```

## Decisions worth preserving on reintegration

- **Restore never carries authority.** A restored Work Order's grant reference is
  stripped from the record payload, not merely the column, because the read
  model reconstructs contracts from the sealed payload. An export must not be a
  way to move authority between vaults.
- **Restore refuses to merge.** Two vaults' histories of one identifier cannot be
  reconciled by overwriting; picking a winner would destroy gate evidence.
- **Gate G2 is computed server-side.** A caller's asserted outcome and evidence
  digest are discarded. Artifact digests are derived from submitted bytes.
- **A run parks before taking leases.** Approval is unbounded, so a parked run
  holds nothing and cannot deadlock the owner's own vault.
- **Scoped sessions receive no execution-graph stream events** and must poll; the
  graph is not namespace-partitioned. Pinned by
  `execution_graph_events_are_withheld_from_namespace_scoped_sessions`.

## Known gaps at the time of the reset

- No external dispatcher, provider call, or third-party agent execution. ADR 012
  governs internal runs only; public grant admission and enforcement are
  prerequisites, not parallel work.
- The `/work-orders` page has no UI for recording an artifact (the API client
  function exists).
