# Authority Grant Model

- **Status:** Implemented core and storage contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`

## Purpose

An `AuthorityGrant` is MindVault's portable, purpose-bound authorization
record. It answers who may do what, to which exact resources, through which
governing Context Node, for how long, and under which data-use ceilings.

Capability advertisement is not authorization. A Context Node descriptor can
announce what the node supports, but an operation remains denied unless an
effective grant and its complete delegation chain authorize it.

## Context and Tool Grants

Context and Tool Grants use one lifecycle and storage contract but have
disjoint capabilities:

| Kind | Permitted capabilities |
| --- | --- |
| `context` | `discover`, `query`, `read`, `subscribe`, `export`, `health` |
| `tool` | `propose`, `command`, `execute`, `notify`, `synchronize`, `import`, `revoke` |

A Context Grant never authorizes an effectful operation. A Tool Grant never
implicitly authorizes reading context. Callers that need both must resolve both
grants independently.

## Record contract

Every grant carries:

- stable grant, grantor, grantee, and governing-node URIs;
- an exact, nonempty target set and sorted capability set;
- sensitivity and retention ceilings;
- explicit redistribution and model-training flags;
- a bounded purpose statement;
- validity start and expiry timestamps;
- optional parent grant and remaining delegation depth;
- revisioned lifecycle state and reason.

Targets are exact stable URI matches. Wildcards, prefixes, endpoint URLs, and
provider-specific identifiers are not grant targets. New grants default to
internal sensitivity, operational retention, no redistribution, no model
training, no delegation, and active status.

### Creates target the governing node

Exact matching has a direct consequence for creation. A created resource's
identifier is minted while the command is being admitted, so no pre-existing
grant can name it. The only satisfiable target for a create is therefore the
governing Context Node URI, `mindvault://{node}/node`.

This is a stated property, not an accident of implementation: **a node-scoped
Tool Grant authorizes creating any resource under that node.** It is coarse by
construction. Per-resource scoping for creates remains with the existing
namespace checks, which are unchanged and still run. Expressing a finer create
scope would require namespace-typed targets or a create-under-collection target
form — a change to this model, not a change to the caller.

Two corollaries worth stating because both are easy to trip over:

- A grant issued with the defaults **cannot** admit a node create. Defaults set
  operational retention, while a node create declares durable retention, and
  rule 4 below refuses it. The issuer must raise `retention_ceiling` explicitly.
- The sensitivity and retention a caller declares must match the envelope the
  command will emit. If they drift apart, a grant could admit a command whose
  declared ceilings it does not actually cover.

## Delegation

A delegated grant is valid only when it is a strict subset of its parent:

- the child grantor is the parent grantee;
- kind and governing node are unchanged;
- targets and capabilities are subsets;
- validity, sensitivity, retention, and data-use permissions do not expand;
- remaining delegation depth strictly decreases.

Authorization walks the complete parent chain on every resolution. Missing,
cyclic, suspended, revoked, or expired ancestors invalidate all descendants
without rewriting the child records.

## Lifecycle and evidence

The lifecycle is:

```text
active -> suspended -> active
active|suspended -> revoked
active|suspended -> expired
```

Grant terms are immutable after issuance. A lifecycle transition advances the
revision, archives the previous revision, and changes only status, reason, and
update time. Issuance and transition commit atomically with their registered,
versioned event envelope. Idempotent replay returns the exact historical
revision referenced by the original event.

Migration `035_authority_grants.sql` stores searchable governance metadata and
the complete record payload. Sealed vaults use `mvenc-v1`; plaintext vaults use
`json-v1`. Database triggers prohibit deletion, immutable-term updates,
revision skips, and invalid lifecycle transitions.

## Authorization algorithm

An authorization lookup fails closed unless all of the following hold:

1. grantee and grant kind match;
2. the grant is active and inside its validity window;
3. target and capability match exactly;
4. requested sensitivity and retention are at or below their grant ceilings;
5. every parent exists, is effective, and still contains the child terms.

External tool execution, non-admin public grant APIs, and signatures remain
later integration gates. Admin-only grant issuance and lifecycle
(`POST/GET /api/v1/authority-grants`, suspend/revoke/resume) are implemented
(IK-001b, ADR 013). Immutable outbox publication-attempt receipts now exist, but
they do not themselves authorize an external side effect. No external side
effect is authorized by this storage slice alone.

### Command admission (implemented)

The resolver is wired to public command admission on `POST /api/v1/nodes`,
behind `MINDVAULT_COMMAND_ADMISSION_MODE` (`off` by default, then `observe`,
then `enforce`). Admission is **additive**: role, namespace, and quota checks
all still run, and grants are an additional axis rather than a replacement, per
constitutional law 8.

Ordering within the handler is load-bearing. Admission runs *after* the
idempotent-replay lookup, because a replay performs no mutation and the lookup
is principal-scoped — re-admitting would refuse a network retry whose original
commit already succeeded. It runs *before* the quota check, because ADR 010
orders authorization as identity → role → resource → grants → delegation →
budget.

Refusals are `403` with a stable `command_admission_denied` code. The bounded
denial reason is **never returned to the caller**: distinguishing "you hold no
grant" from "your grant expired" is a probing oracle. The reason is written to
tracing and the audit trail instead, satisfying law 15's requirement that a
denial be recorded rather than disclosed. A sealed vault yields `503`. A
resolver that cannot reach an answer denies and never falls through to a
successful mutation.

Enforcement is usable on a fresh vault after registering the local Context
Node (`POST /api/v1/context-nodes/local`) and issuing a Tool Grant
(`POST /api/v1/authority-grants`). `observe` remains the recommended rollout
position so an operator can watch denials fall to zero first.

## Conformance gates

- A Context Grant cannot carry `execute`, `command`, or another Tool capability.
- A Tool Grant cannot carry `read`, `query`, or another Context capability.
- Retrying issuance cannot create a duplicate grant or event.
- A child cannot expand targets, capabilities, time, data use, or delegation.
- Suspending or revoking a parent immediately makes descendants ineffective.
- Sealed storage does not expose purpose or target details in the record payload.
- Lifecycle history remains queryable for audit and idempotent replay.
