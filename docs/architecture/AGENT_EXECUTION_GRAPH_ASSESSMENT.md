# Agent Execution Graph — External Standard Assessment

- **Status:** Accepted analysis
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`
- **Decision record:** `docs/adr/012-governed-agent-execution-graph.md`

## Purpose

An external engineering standard ("Evidence-Grounded Dynamic Graph
Engineering") proposes contract-bound nodes, typed edges, artifacts, anchors,
verification gates, schedulers, five isolation dimensions, and a multi-agent
fleet process. This document records which parts MindVault adopts, which parts
MindVault already specifies more strictly, and which parts are rejected because
they would violate constitutional law.

The assessment exists so that later sessions inherit the reasoning rather than
relitigating it, and so that the adopted parts carry their rationale into
`WORK_ORDER_MODEL.md` and `EXECUTION_ISOLATION_MODEL.md`.

## Why the standard is relevant at all

`PROTOCOL_BOUNDARIES.md` declares that the internal domain owns `WorkOrder`,
`AgentRun`, and `Artifact`. The constitution lists Work Orders, Agent Runs,
artifacts, and budgets among the concerns governed centrally, and its initial
implementation gate terminates in Work Order execution by an adapted agent
producing a verified artifact and an approved delivery with a complete action
receipt.

No such type, table, or store method exists. The constitution's own acceptance
gate is therefore unreachable, and the standard is a candidate specification for
the missing layer.

## What exists today, and why it cannot serve

| Surface | Assessment |
| --- | --- |
| `migrations/024_plans.sql` | `plan_steps` is a linear `step_order` sequence. Its `action` vocabulary does not compose with `IntentType`. |
| `plans` / `plan_steps` tables | No code reads or writes them. No `PlanStore` trait exists. Dead schema. |
| `crates/mv-engine/src/planner.rs` | Produces in-memory plans that are never persisted. |
| `crates/mv-server/src/rest/plans.rs` | `approve_plan` returns a hardcoded approved status for any identifier without a lookup; `get_plan` always reports not found. This is false canonical success, which `PROTOCOL_BOUNDARIES.md` prohibits. |
| `crates/mv-engine/src/intent_executor.rs` | Executes one intent and reports the affected node after the fact. Write scope is discovered during execution rather than declared before it. No budget, verification, or attributable receipt. |
| `crates/mv-engine/src/autonomy.rs` | `AutonomyGate::evaluate` correctly defers by default, but `IntentExecutor::execute` never calls it. The gate is caller-invoked, so any new execution path bypasses the human-led guarantee. |

The last row is a live defect, not a design gap. It is fixed as part of the
adopted work regardless of how much of the standard is taken.

## Adopted

| Concept | Gap it closes | Destination |
| --- | --- | --- |
| Typed edge taxonomy | MindVault has no dependency typing. `step_order` is the entire model. | `WORK_ORDER_MODEL.md`, `work_order_edges` |
| Conflict edges derived from declared write sets | Sync detects conflicts after the fact through vector clocks. Nothing prevents two runs from targeting one node. | `agent_run_write_leases` |
| Layered verification gates | The autonomy gate is a single pre-decision about one intent. A run has no accumulated verification evidence. | `agent_run_gate_results` |
| Five isolation dimensions | Only credential isolation exists. Data isolation is absent. | `EXECUTION_ISOLATION_MODEL.md` |
| Budgets as enforced ceilings | Named in the constitution, never implemented. | `work_orders` budget columns |
| Implementer is not approver | `IntentExecutor` both performs work and reports its success. | Gate G5 |

## Already specified more strictly in MindVault

These are recorded so that no future session imports a weaker version.

- **Anchors.** The standard describes an external source of truth that agents
  may read but not silently rewrite. `SOURCE_AUTHORITY_MODEL.md` and
  constitutional law 6 are stronger: every external object declares source
  ownership, materialization mode, sync direction, conflict policy, deletion
  policy, and freshness policy, and derived knowledge never erases the authority
  of its evidence. MindVault keeps its own model and borrows only the vocabulary.
- **Artifacts.** The standard asks for immutable or versioned results.
  `035_authority_grants.sql` already demonstrates the stronger pattern:
  revisioned records with archive tables and database triggers prohibiting
  deletion, immutable-term updates, and revision skips.
- **Receipts.** `ACTION_RECEIPT_MODEL.md` specifies lease intervals, attempt
  monotonicity, canonical request and response digests, and idempotency-conflict
  detection on replay. The standard has no equivalent.
- **Fail-closed authorization.** `AUTHORITY_GRANT_MODEL.md` specifies a
  five-condition closed algorithm that walks the complete delegation chain.
- **Merge is not release.** Already internalized: a `published` receipt proves
  destination acknowledgement only, not end-to-end exactly-once application.

## Rejected

| Proposal | Basis for rejection |
| --- | --- |
| LangGraph, LangSmith, and an accompanying Python package stack | Constitutional law 1 prohibits vendor-specific objects in the core; law 12 requires core workflows to function locally. MindVault is a local-first Rust system. A Python orchestration dependency in the core is a sovereignty regression. |
| The rewrite and language-port standard | No mechanical port is in scope. MindVault is implementing novel contracts, not translating an existing implementation. |
| A large parallel agent fleet to build this layer | The standard's own preconditions disqualify it: graph execution is described as ineffective when the problem is conceptually undefined and correctness is largely subjective. Implementing a governance kernel is that case. The conformance matrix is a legitimate future fan-out target because it is mechanical and partitionable; the kernel itself is not. |
| Majority vote among models as evidence | Contradicted by the standard itself and incompatible with a threat model that treats external model output as untrusted. |

## The translation error that would make adoption unsafe

The standard assumes an incompetent-but-cooperative agent supervised by a human
with continuous integration as the oracle. MindVault's threat model inverts
this. `PROTOCOL_BOUNDARIES.md` states that external MCP content and tool
descriptions are untrusted, and adapted external agents are third parties acting
against a private vault.

A node contract's declared write scope is, in the standard, a prompt-level
instruction. A hostile or compromised agent ignores prompts. MindVault therefore
binds the concept to an existing enforcement point:

> **Declared write scope is the AuthorityGrant target set.**

`AUTHORITY_GRANT_MODEL.md` requires exact stable URI matches and rejects
wildcards, prefixes, endpoint URLs, and provider-specific identifiers as grant
targets. A run may not declare a write target outside its resolved Tool Grant,
and gate G0 rejects the Work Order before any run becomes schedulable.

Adopting the write-scope concept without this binding would produce a system
that documents a boundary it does not enforce.

## Two structural tensions the standard does not address

### Global scheduling versus federation

The standard's scheduler requires global visibility of read and write sets in
order to detect conflicts. Constitutional law 13 states that federation does not
require surrendering ownership and that nodes answer scoped queries without
wholesale centralization. A cross-node Work Order therefore cannot have a
globally known write set.

MindVault resolves this by making conflict detection lease-based per governing
node, reusing the claim and lease pattern already proven for outbox dispatch,
rather than scheduler-based. Cross-node write coordination is an explicit
non-goal.

### The anchor is a live human

The standard treats anchors as frozen artifacts and human approval as an
occasional prompt. MindVault's first system principle makes human-led behavior
the default path, with deferral, quiet hours, and per-contact scope.

Consequently `awaiting_approval` is a first-class run state of unbounded
duration. It is never a timeout and never an error. A run parked on owner
approval during quiet hours is the system behaving correctly.

This inversion has a direct safety consequence: because approval is unbounded,
a run must not hold write leases while parked on it. See
`EXECUTION_ISOLATION_MODEL.md`.

## Conformance gates

- No adopted concept may be implemented as prompt-level guidance where an
  existing enforcement point exists.
- Declared write scope resolves through `find_authorizing_grant` or the Work
  Order is rejected.
- No dependency introduced by this assessment may make a core workflow require
  a network service or a non-Rust runtime.
- Any future proposal to import the rejected items must cite this document and
  the specific law it now satisfies.
