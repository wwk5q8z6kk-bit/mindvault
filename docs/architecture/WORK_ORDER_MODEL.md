# Work Order Model

- **Status:** Required architecture contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`
- **Decision record:** `docs/adr/012-governed-agent-execution-graph.md`
- **Review trigger:** Any change to grant resolution, lease semantics, or gate
  requirements

## Purpose

A `WorkOrder` is MindVault's unit of governed intent. It answers what outcome is
being pursued, which node contracts pursue it, what each one may read and write,
in which order and under which constraints, what evidence is required before a
result is accepted, and what ceilings bound the whole effort.

Declaring a capability is not authorization to exercise it, and completing
execution is not evidence of correctness. A Work Order records the intent; the
grant layer authorizes it; the gate results prove it.

## Objects

```text
WorkOrder                goal, non-goals, anchors, budgets, success criteria
  └── WorkOrderNode      one bounded contract: purpose, scope, gates, risk tier
        └── edges        typed constraints between nodes
        └── AgentRun     one bounded execution attempt of one node
              ├── write leases      exclusive claims on target URIs
              ├── gate results      immutable verification evidence
              └── Artifact          versioned, digested, provenance-linked output
```

A Work Order is not an agent invocation. A node contract may be executed by an
agent, a deterministic engine operation, a local model, or an owner decision.

## Work Order record contract

Every Work Order carries:

- stable work-order, requesting-principal, acting-actor, and governing-node URIs;
- a bounded goal statement and an explicit non-goal set;
- referenced anchors, each a stable URI whose source authority the run may read
  but must not silently rewrite;
- budget ceilings and their remaining balances;
- success criteria and prohibited outcomes;
- sensitivity and retention ceilings, at or below the authorizing grant's;
- revisioned lifecycle state and reason;
- idempotency key, causation, and correlation identifiers.

Non-goals are recorded because a run that satisfies its goal by violating a
non-goal has failed. Anchors are recorded because the constitution requires that
derived knowledge never erase the authority of its evidence.

## Node contract

Every node contract carries:

- a single purpose;
- typed inputs, referencing artifacts or anchors by stable URI and version;
- a declared **read scope** and **write scope**, each an exact stable URI set;
- an executor kind (`engine`, `local_model`, `owner`, `external_agent`);
- a risk tier (`low`, `standard`, `high`);
- the set of gates required at that tier;
- a per-node share of the Work Order budget;
- a timeout, maximum attempts, and retry classification;
- a rollback disposition.

### Declared write scope is the grant target set

This is the binding rule of the entire model.

`AUTHORITY_GRANT_MODEL.md` requires grant targets to be exact stable URI
matches, and rejects wildcards, prefixes, endpoint URLs, and provider-specific
identifiers. A node contract's write scope is resolved against the effective
Tool Grant using `find_authorizing_grant`. A write target outside that set makes
the Work Order inadmissible.

Read scope resolves independently against a Context Grant. A Context Grant never
authorizes an effectful operation and a Tool Grant never implicitly authorizes
reading; a node needing both resolves both.

Scope is never enforced by instruction text. `PROTOCOL_BOUNDARIES.md` states
that external MCP content and tool descriptions are untrusted; a compromised
executor ignores prompts but cannot forge a grant.

## Edge taxonomy

`edge_kind` is one of eight values. An edge exists whenever the downstream node
cannot safely execute without respecting something produced, changed, reserved,
authorized, or verified by another node. Chronological narration is not an edge.

| Kind | Meaning | Satisfied when |
| --- | --- | --- |
| `data` | consumes a value produced upstream | predecessor `completed` and named output present |
| `artifact` | consumes an upstream artifact | artifact exists and its digest resolves |
| `state` | reads state the predecessor changed | predecessor committed and its event is durable in the outbox |
| `conflict` | write scopes overlap | no other run holds an active write lease on any overlapping target URI |
| `resource` | competes for a bounded resource | the named resource has capacity |
| `policy` | requires an authorization or approval outcome | `AutonomyGate::evaluate` returned `AutoApply`, or the approval queue recorded an owner decision |
| `verification` | requires upstream evidence | every gate required of the predecessor has outcome `pass` |
| `temporal` | must follow a wall-clock condition | `not_before` reached and quiet hours not active |

Conflict edges are derived, not authored. Two nodes whose declared write scopes
intersect acquire a conflict edge at admission whether or not the author noticed
the overlap. This is the primary defense against two runs corrupting one target
in the owner's vault, and it is why write scope must be declared before
execution rather than discovered during it.

## Work Order lifecycle

```text
draft -> admitted -> running -> completed
draft -> rejected
admitted|running -> cancelled
running -> failed
running -> budget_exhausted
```

Admission is the point at which grant resolution, scope validation, edge
derivation, and cycle detection occur. A Work Order that cannot be admitted is
`rejected` with a reason; it is never partially admitted.

## Agent run lifecycle

```text
ready -> leased -> running -> gated -> completed
running -> awaiting_approval -> ready
gated -> failed
gated -> ready              (requeued with new evidence)
ready|leased|running -> cancelled
any -> budget_exhausted     (terminal)
```

A node becomes `ready` only when every inbound edge is satisfied. Readiness is
recomputed on each state change rather than cached.

### Approval is unbounded and never holds leases

`awaiting_approval` has no expiry. MindVault's first system principle makes
human-led behavior the default path, with deferral and quiet hours; a run parked
on owner approval for a week is the system working correctly.

Because the state is unbounded, **a run releases all write leases when entering
`awaiting_approval` and re-acquires them with a fresh conflict check on resume.**
Holding leases across an unbounded approval would let one parked run block every
other run touching those URIs — a deadlock on the owner's own vault, caused by
the owner's own absence.

If the write set is no longer free on resume, the run returns to `ready` and
waits. Approval authorizes the action; it does not preserve a stale write set.

### Failure classification

A failed run is classified before any retry. Blind repetition is not recovery.

| Class | Disposition |
| --- | --- |
| transient | bounded retry with backoff; attempt number advances |
| deterministic | no identical retry; failure evidence attached and requeued or failed |
| specification | subgraph paused; the contract is at fault, not the executor |
| authorization | fail closed; never retried around |
| budget | terminal; never a silent scope reduction |
| conflict | return to `ready`; wait for the lease holder |

## Gate hierarchy

Gates produce immutable evidence, not boolean flags. A gate result records the
gate, outcome (`pass`, `fail`, `blocked`), evaluating actor URI, evidence
digest, and evaluation time.

| Gate | Check | Enforcement point |
| --- | --- | --- |
| G0 | contract validation; declared write scope is a subset of the grant target set | `find_authorizing_grant` |
| G1 | the emitted envelope validates against a registered public schema | existing fail-closed schema admission |
| G2 | every declared output exists with a matching digest | artifact records |
| G3 | provenance references resolve and source authority is respected | `SOURCE_AUTHORITY_MODEL.md` |
| G4 | derived knowledge has not overwritten its evidence | materialization mode of the affected binding |
| G5 | independent review by an actor URI distinct from the implementer | run attribution |
| G6 | budget, sensitivity, and retention remain at or below their ceilings | grant ceilings |
| G7 | owner approval | `AutonomyGate` and the approval queue |

### Risk-scaled requirements

| Risk tier | Required gates |
| --- | --- |
| `low` | G0, G1, G2, G6 |
| `standard` | G0, G1, G2, G3, G4, G6 |
| `high` | all gates, including G5 and G7 |

G0 and G6 are required at every tier. G0 is the scope boundary; G6 is the
ceiling boundary. Neither is ever waived.

### G5 in a single-owner vault

ADR 002 establishes single-owner sovereignty, external agents are outside this
contract's scope, and external content is untrusted by design, so there is no
independent third-party reviewer to import.

G5 is therefore **not required below the high-risk tier.** At the high-risk
tier it is satisfied by exactly one of:

- the owner acting as reviewer through the approval queue; or
- a distinct local model run whose actor URI differs from the implementer's.

The invariant that holds at every tier is narrower but absolute: **a run can
never satisfy its own G5.** Attribution is checked against the run's acting
actor, not against a role label.

## Budgets

A Work Order declares ceilings for wall-clock duration, run attempts, model
tokens, and effectful actions. Each is a non-negative remaining balance.

A `CHECK` constraint prevents a negative row; it does not prevent a double
spend. Therefore **budget decrement, run state transition, and event emission
commit in one immediate transaction**, exactly as action receipt insertion is
bound to outbox state transition in `ACTION_RECEIPT_MODEL.md`. Two concurrent
runs must not both pass a read-time check and both commit.

Exhaustion moves the run to `budget_exhausted`, which is terminal and fails
closed. Reducing scope to fit a budget without recording the reduction is
prohibited by the constitution's prohibition on silent scope reduction.

## Artifacts

An artifact is immutable after insertion. It carries a stable URI, kind, content
digest, producing run, schema reference where applicable, provenance references
to the anchors and inputs it derives from, and sensitivity and retention labels
inherited from the producing run's ceilings.

An artifact never replaces its evidence. A derived artifact references the
source binding it was derived from, and G4 fails if the materialization mode of
that binding does not permit the derivation.

## Relationship to existing execution

`AutonomyGate::evaluate` remains the policy decision point and continues to
defer by default. It is reached through a single admission function in
`mv-engine` that every effectful path must call, including existing intent
execution. Making the gate structurally non-bypassable is a correctness fix, not
a new capability: the gate previously depended on caller discipline.

`crates/mv-engine/src/planner.rs` remains a goal-decomposition helper. Its
output is a candidate Work Order subject to admission.

`plans` and `plan_steps` are superseded per ADR 012.

## Observation surface

Run transitions and gate recordings are announced on `/ws/agent` as
`agent_run_transitioned` and `agent_run_gate_recorded`. The stream carries
identifiers and status only — never artifact content or declared write scope.
Those are governed detail and belong behind the query API.

Work Orders are not namespace-scoped. Notifications therefore emit with
`namespace: None`. A namespace-scoped WebSocket client is filtered by
`handle_agent_socket` and receives none of them; that client must use the query
API (or the operator UI Refresh control). Unscoped clients receive the events.
This is deliberate: broadening the filter would push cross-scope signal to a
client that asked to be limited.

## Explicit non-claims

This contract governs internal runs. It does not run an external dispatcher,
contact a provider, execute a third-party agent, verify a remote signature, or
authorize any external side effect. Public grant admission and enforcement
remain prerequisites for outbound execution, per `AUTHORITY_GRANT_MODEL.md`.

Cross-node write coordination is a non-goal. Conflict control is per governing
node, because constitutional law 13 forbids requiring wholesale centralization.

## Conformance gates

- A node declaring a write target outside its effective Tool Grant is rejected
  at admission, before any run is created.
- A Context Grant cannot authorize a write scope, and a Tool Grant cannot
  authorize a read scope.
- Two admitted nodes with intersecting write scopes always carry a conflict
  edge, whether or not it was authored.
- A run in `awaiting_approval` holds no write leases.
- `awaiting_approval` never expires and never fails.
- Budget decrement and run state transition are not observable independently.
- A run's acting actor cannot appear as the evaluating actor of its own G5.
- G0 and G6 results exist for every completed run at every risk tier.
- Gate results, artifacts, and lease history cannot be updated or deleted.
- Retrying admission with the same idempotency key returns the original Work
  Order revision rather than creating a second one.
- Export and restore preserve Work Order identity, edges, gate evidence,
  artifact digests, and provenance.
