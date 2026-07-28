# Execution Isolation Model

- **Status:** Required architecture contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`
- **Decision record:** `docs/adr/012-governed-agent-execution-graph.md`
- **Review trigger:** Any new executor kind, or any change to lease or
  credential lifetime

## Purpose

Concurrent agent runs against a single sovereign vault can damage one another's
work. This contract states which isolation dimensions MindVault enforces, by
which mechanism, and — equally important — which are declared but not yet
enforced, so that no future session assumes a boundary that does not exist.

The governing observation is that isolating source state is not isolating
execution. A workspace or branch separates files; it does not separate
databases, ports, processes, caches, credentials, disk pressure, or shared
services. Systems that conflate the two discover the difference under
concurrency, in production.

## The five dimensions

| Dimension | What it separates | MindVault mechanism | Status |
| --- | --- | --- | --- |
| Source | which resources a run may read and write | declared scope resolved against the grant target set | **enforced** |
| Data | concurrent writes to the same resource | exclusive write leases per target URI | **enforced** |
| Credential | which secrets and identities a run may use | Tool Grant plus Sovereign Keychain | **enforced** |
| Runtime | processes, ports, sandboxes, model sessions | run credentials are short-lived and audience-bound; WASM plugin sandbox | **partial** |
| Build | generated files, caches, projections | workspace projection rebuild | **partial** |

No dimension is enforced by instruction text. `PROTOCOL_BOUNDARIES.md` treats
external MCP content and tool descriptions as untrusted, so any boundary that
depends on an executor choosing to respect it is not a boundary.

## Source isolation

A node contract declares an exact read scope and write scope as stable URI sets.
`AUTHORITY_GRANT_MODEL.md` requires grant targets to be exact stable URI
matches and rejects wildcards, prefixes, endpoint URLs, and provider-specific
identifiers.

Admission resolves declared scope through `find_authorizing_grant`. A write
target outside the effective Tool Grant makes the Work Order inadmissible. A
read target outside the effective Context Grant does likewise. The two resolve
independently: a Context Grant never authorizes an effectful operation, and a
Tool Grant never implicitly authorizes reading.

This is gate G0, and it runs before any `AgentRun` exists.

## Data isolation

Source isolation says what a run *may* touch. It does not prevent two authorized
runs from touching the same resource at the same time. Vector-clock conflict
detection in the sync engine finds such collisions afterward; that is recovery,
not isolation.

Data isolation is therefore an exclusive **write lease** per target URI, scoped
to the governing node.

### Lease semantics

Leases carry the semantics already proven for outbox dispatch in
`ACTION_RECEIPT_MODEL.md`, without variation:

- a lease is claimed by exactly one run for exactly one target URI;
- the lease interval is half-open, `[claimed_at, lease_expires_at)`;
- lease duration is bounded to at most one hour;
- completion at or after expiry fails closed;
- an expired lease is atomically replaceable, advancing the attempt number and
  invalidating completion by the prior claim.

The bound and the replacement rule are not optional. Without them, a run that
crashes or is abandoned locks a target URI permanently, and the owner's vault
acquires a resource it can never write again without manual repair.

A second lease shape must not be invented for this contract. Divergence between
two lease implementations is itself a defect.

### Leases and approval

A run releases all write leases when entering `awaiting_approval`, and
re-acquires them with a fresh conflict check on resume.

This rule exists because `awaiting_approval` is unbounded by design — MindVault's
first system principle makes human-led behavior the default, with deferral and
quiet hours. A run parked on owner approval over a weekend, or over an owner's
week of absence, would otherwise block every other run touching those URIs.

Approval authorizes the action. It does not preserve a stale write set. If the
set is no longer free on resume, the run returns to `ready` and waits.

## Credential isolation

Tool Grants bound what a run may cause; the Sovereign Keychain remains the
single secret store. `PROTOCOL_BOUNDARIES.md` requires that run credentials be
short-lived, audience-bound, and narrower than the requesting principal's
standing authority, and that devices, services, integrations, and agents have
distinct identity types.

A run never receives the owner's standing authority. Delegation follows the
subset rules in `AUTHORITY_GRANT_MODEL.md`: a child grant cannot expand targets,
capabilities, validity, sensitivity, retention, data use, or delegation depth,
and suspending or revoking a parent immediately makes descendants ineffective.

## Runtime isolation

Partially enforced, and stated plainly rather than assumed.

Enforced today: WASM plugin sandboxing per ADR 005; short-lived audience-bound
run credentials; fail-closed handling of undeclared extension capability,
network, secret, file, schema, or data-class access per constitutional law 15.

Not enforced today: process, port, and namespace separation between concurrent
local model runs; memory and disk quotas per run; protection against a
resource-heavy run exhausting local disk. A run's `resource` edges express
contention declaratively, but the engine does not yet reserve or meter the
underlying resource.

Until metering exists, `resource` edges are a scheduling constraint, not a
guarantee. External executor kinds must not be enabled on the strength of this
dimension alone.

## Build isolation

Partially enforced. Workspace projections are rebuilt through a governed
operation rather than mutated in place, which separates derived state from
source state. Generated artifacts carry digests, so a stale derivation is
detectable.

Not enforced today: per-run separation of projection rebuild. Two runs
rebuilding projections for the same workspace serialize through the workspace
write lease rather than through independent build directories.

## What isolation does not provide

Isolation prevents interference. It does not establish correctness. A run that
holds a valid lease, stays inside its grant, and respects its budget can still
produce a wrong artifact. That is the responsibility of the gate hierarchy in
`WORK_ORDER_MODEL.md`, and specifically of G2 through G5.

Nor does isolation establish delivery. Completing a run inside its leases proves
local state changed; it does not prove any external destination observed the
change. `ACTION_RECEIPT_MODEL.md` is explicit that a `published` receipt proves
destination acknowledgement only.

## Conformance gates

- A run holding no lease on a target URI cannot write that target.
- Two active leases cannot exist for one `(target_uri, governing_node_uri)` pair.
- A lease exceeding one hour cannot be created.
- Completion at or after `lease_expires_at` fails closed.
- An expired lease is claimable by another run, with an advancing attempt number.
- A run in `awaiting_approval` holds no leases.
- A resumed run re-checks conflicts before re-acquiring.
- Lease history is insert-only and cannot be updated or deleted.
- A run's credentials expire no later than the run's own terminal state.
- No executor kind whose isolation depends on an unenforced dimension is enabled
  by default.
