# ADR 012: Governed Agent Execution Graph

- **Status:** Accepted
- **Date:** 2026-07-26
- **Owners:** MindVault
- **Constitution:** `INTEROPERABILITY_CONSTITUTION.md`
- **Related:** ADR 002, ADR 005, ADR 010, ADR 011
- **Supersedes:** the `plans` / `plan_steps` schema introduced in
  `migrations/024_plans.sql`

## Context

ADR 011 makes MindVault a sovereign context and coordination fabric and lists
the interoperability kernel as the first implementation gate. Much of that
kernel now exists: canonical identities, a versioned event envelope, immutable
public schema registration, Context Node descriptors, Context and Tool Grants, a
transactional outbox with leases and action receipts, and a durable consumer
inbox with checkpoints.

`PROTOCOL_BOUNDARIES.md` declares that the internal domain also owns
`WorkOrder`, `AgentRun`, and `Artifact`. The constitution lists Work Orders,
Agent Runs, artifacts, and budgets among centrally governed concerns, and its
initial implementation gate ends in Work Order execution producing a verified
artifact and an approved delivery with a complete action receipt.

None of these exist. No type is defined in `mv-core`, no migration creates the
tables, and no store method exposes them. The constitution's acceptance gate
cannot currently be reached, and the A2A mapping table in `PROTOCOL_BOUNDARIES.md`
maps external tasks onto internal concepts that have no implementation.

Three surfaces partially occupy the space and none can serve:

- `migrations/024_plans.sql` defines `plans` and `plan_steps` as a linear
  ordered sequence. Verification shows no code reads or writes either table and
  no store trait exists for them.
- `crates/mv-engine/src/planner.rs` produces in-memory plans that are never
  persisted.
- `crates/mv-server/src/rest/plans.rs` exposes an approval endpoint that returns
  a hardcoded success for any identifier without performing a lookup. This is
  the false canonical success that `PROTOCOL_BOUNDARIES.md` prohibits.

Separately, `crates/mv-engine/src/intent_executor.rs` performs effectful work
without consulting `AutonomyGate`. The gate defers correctly by default but is
invoked by callers rather than by the execution path, so any new caller silently
bypasses the human-led guarantee established by System Principles 1 and 3.

An external engineering standard was evaluated as a candidate specification.
`docs/architecture/AGENT_EXECUTION_GRAPH_ASSESSMENT.md` records the full
analysis, including the parts rejected for violating constitutional law.

Four approaches were compared:

1. **Extend `plans` / `plan_steps` in place.** Lowest apparent cost, but forces
   a single `action` column to absorb typed edges, declared scope, budgets, and
   gate evidence, producing a schema strictly worse than a purpose-built one,
   and preserves a vocabulary that does not compose with `IntentType`.
2. **Leave the layer unimplemented and adapt A2A tasks directly.** Fastest route
   to an external integration demo, but makes a protocol object the internal
   model, which ADR 011 and constitutional law 9 prohibit.
3. **Implement an execution engine with prompt-level scope discipline**, as the
   external standard describes. Cheap to build, but unenforceable against
   untrusted agents, which `PROTOCOL_BOUNDARIES.md` requires MindVault to
   assume.
4. **Implement a governed execution graph whose declared scope resolves through
   the existing grant layer**, with typed edges, lease-based conflict control,
   layered verification gates, and enforced budgets.

Option 4 is chosen. It is the only option that makes the declared boundary an
enforced boundary, and it is more reversible than option 1 because it adds
tables rather than overloading existing columns.

## Decision

MindVault implements a governed agent execution graph as an internal domain
capability.

`WorkOrder` is the unit of governed intent. It carries a goal, explicit
non-goals, referenced anchors, budget ceilings, and success criteria. It
contains node contracts connected by typed edges. `AgentRun` is one bounded
execution attempt of one node contract. `Artifact` is a versioned, digested,
provenance-linked output of a run.

The following are binding:

- **Declared write scope is the AuthorityGrant target set.** A node contract may
  not declare a write target outside the resolved Tool Grant. Resolution uses
  the existing closed algorithm; failure rejects the Work Order before any run
  becomes schedulable. Scope is never enforced by instruction text alone.
- **Edges are typed.** Data, artifact, state, conflict, resource, policy,
  verification, and temporal dependencies are distinct and separately evaluated.
  Chronological narration is not a dependency.
- **Conflict control is lease-based per governing node**, reusing the claim,
  bounded lease, attempt counter, and atomic expiry replacement already proven
  for outbox dispatch. Global cross-node write scheduling is a non-goal, because
  constitutional law 13 forbids requiring wholesale centralization.
- **Leases are held across execution only, never across approval.** A run
  releases its write leases when entering `awaiting_approval` and re-acquires
  them with a fresh conflict check on resume.
- **`awaiting_approval` is unbounded.** It is never a timeout and never an
  error, because human-led behavior is the default path.
- **Budgets are enforced, not advisory.** Budget decrement, run state
  transition, and event emission commit in one immediate transaction. Exhaustion
  is terminal and fails closed; silent scope reduction is prohibited.
- **A run can never satisfy its own review gate.**
- **The autonomy gate becomes structurally non-bypassable.** A single admission
  function in `mv-engine` mediates every effectful path, including existing
  intent execution.

The capability must satisfy the constitution's feature completeness contract in
full: typed versioned object, command API, query API, emitted events, permission
and grant definitions, provenance behavior, Trust Ledger behavior, portable
export and restore, safe extension access, and automated contract and
conformance tests.

## Scope boundary

This decision governs internal runs only. It does not run an external
dispatcher, contact a provider, execute a third-party agent, verify a remote
signature, or authorize any external side effect.

`AUTHORITY_GRANT_MODEL.md` states that retention enforcement, action-envelope
admission, external tool execution, and public grant APIs remain later
integration gates, and that no external side effect is authorized by the grant
storage slice alone. That boundary is preserved here. Public grant admission and
enforcement are prerequisites for outbound execution, not parallel work.

## Relationship to migration 024

`plans` and `plan_steps` are superseded. Verification confirms no code reads or
writes either table, so no data migration is required; the superseding migration
records that fact rather than assuming it.

The plan approval endpoint that returns unconditional success is removed and
replaced with an explicit gone response naming the Work Order API. Leaving it
would preserve a surface that reports success for work that never happened.

`crates/mv-engine/src/planner.rs` retains its role as a goal-decomposition
helper. Its output becomes a candidate Work Order subject to admission rather
than an unpersisted result.

## Consequences

### Positive

- The constitution's initial implementation gate becomes reachable.
- Declared scope becomes an enforced boundary rather than documentation.
- Two concurrent runs can no longer silently corrupt one target in the owner's
  vault.
- The A2A mapping in `PROTOCOL_BOUNDARIES.md` acquires the internal concepts it
  already references.
- A live autonomy bypass is closed, independent of the rest of the capability.
- Verification evidence becomes queryable history rather than a transient
  boolean.

### Costs and risks

- Execution scheduling, lease management, and budget accounting become
  security-critical infrastructure.
- Grant resolution moves onto the hot path of every admission, adding latency
  proportional to delegation depth.
- An unbounded `awaiting_approval` state means work orders can remain open
  indefinitely; the lease release rule prevents this from blocking other runs,
  and that rule must be tested rather than assumed.
- Layered gates increase the cost of trivial runs unless gate requirements are
  genuinely risk-scaled.
- A second orchestration vocabulary exists during the transition until the
  superseded surfaces are removed.

## Implementation order

1. Ratify this decision and the supporting contracts.
2. Land schema, core types, and storage with trigger-enforced invariants.
3. Land engine operations and the non-bypassable admission function.
4. Land command and query transport surfaces together.
5. Land events, export and restore, and governed extension access.
6. Land the operator surface.
7. Retire the superseded plan schema and endpoints.

Scaffolding, endpoint count, or a rendered run view do not count as progress
against these gates. Conformance tests do.

## Supporting contracts

- `docs/architecture/AGENT_EXECUTION_GRAPH_ASSESSMENT.md`
- `docs/architecture/WORK_ORDER_MODEL.md`
- `docs/architecture/EXECUTION_ISOLATION_MODEL.md`
- `docs/architecture/AUTHORITY_GRANT_MODEL.md`
- `docs/architecture/ACTION_RECEIPT_MODEL.md`
- `docs/architecture/SOURCE_AUTHORITY_MODEL.md`
