# Interoperability Kernel v1: Transactional, Registry, Grant, and Delivery Slices

- **Status:** Implemented transactional, governed-registry, Context Node, grant, and durable producer/consumer delivery-control slices
- **Date:** 2026-07-26
- **Decision:** ADR 011
- **Scope:** Canonical node creation, public schemas, Source Bindings, Context Nodes, Authority Grants, outbox delivery, consumer inboxes, immutable receipts, and local checkpoints

## Purpose

These slices establish executable proofs of constitutional laws 4 and 5: a
committed canonical mutation and its attributable, versioned event share one
durable transaction, a retried command cannot create duplicate canonical
state, and external-object authority is represented as governed core data.

It is a kernel foundation, not completion of the full interoperability kernel.
Connector manifests, federation, schema lifecycle commands, public registry
APIs, action-envelope enforcement, authenticated live transport publishers,
remote signature verification, and protocol translation remain later gated
work.

## Stable identifiers

Core identities use credential-free, canonical `mindvault://` URIs:

```text
mindvault://{node_uuid}/node
mindvault://{node_uuid}/knowledge/{resource_uuid}
mindvault://{node_uuid}/identity/principal/{principal_uuid}
mindvault://schemas/{schema_name}
mindvault://schemas/{schema_name}/version/{version}
mindvault://{node_uuid}/source-binding/{binding_uuid}
mindvault://{node_uuid}/grant/{grant_uuid}
```

URIs are capped at 2,048 bytes and reject credentials, query strings,
fragments, relative path segments, noncanonical URL encoding or casing, and
schemes other than `mindvault`. The SQLite vault owns one durable local node
UUID in `interoperability_local_identity`; it does not derive identity from a
database path or HTTP endpoint.

## Event envelope

The internal envelope version is `mindvault.event-envelope/v1`. The first event
type is `dev.mindvault.knowledge.node.created.v1`. Every envelope contains:

- stable event, source, subject, principal, and actor identities;
- an explicit schema URI and version;
- occurrence time, correlation ID, optional causation ID, and idempotency key;
- a lowercase SHA-256 semantic payload digest;
- sensitivity, retention, and at least one provenance reference;
- an object-valued data payload.

The first event deliberately carries only `resource_kind`, `node_kind`, and
`namespace`. Node title and content remain in canonical storage and are not
copied into the plaintext outbox envelope.

Protocol adapters may map the envelope to CloudEvents, AsyncAPI, MCP, or A2A,
but must not drop its security-relevant fields. Those mappings are not part of
this slice.

## Governed public schemas

Migration `033_governed_interoperability_registries.sql` adds immutable,
content-addressed public schema versions. Every definition:

- uses `application/schema+json`;
- declares JSON Schema draft 2020-12;
- is capped at 1 MiB and carries a canonical SHA-256 digest;
- has a stable schema URI, explicit version, owner, and lifecycle;
- may declare `x-mindvault-event-type` to bind one schema version to one event
  type.

Schema registration and its `dev.mindvault.schema.registered.v1` event commit in
one immediate SQLite transaction. Schema rows cannot be updated or deleted in
this slice. Lifecycle transitions are deliberately blocked until they receive
their own governed command/event contract and replacement migration guard.

SQLite independently enforces event admission: every outbox insert must name an
active registered schema whose `x-mindvault-event-type` exactly matches the
envelope. Rust performs the same check before insertion, so unknown,
withdrawn, missing-type, and mismatched schemas fail closed even if a future
caller bypasses the intended storage helper.

## Governed Source Bindings

`SourceBinding` is the portable source-authority record. It captures the
canonical resource and Context Node, external system/account/object identity,
authoritative source and fields, sync direction and cursor, materialization,
freshness, conflict and deletion policies, retention, sensitivity, provenance,
and lifecycle status.

New bindings default to reference-only materialization, inbound sync, unknown
freshness, and explicit review for conflicts and deletion. The database permits
only one active binding for each Context Node plus external account/object
tuple. Indexed lookup identities use SHA-256 in unsealed mode and keyed HMAC in
sealed mode; the complete record is encrypted with an `mvenc-v1` envelope when
sealed mode is enabled.

Registration and explicit rebinding each share a transaction with their
versioned event. Rebinding retires the predecessor, archives its prior revision,
and installs the successor atomically. It cannot implicitly transfer authority
to a different Context Node. Registration and rebinding require that node to
be an active, registered local Context Node; a Source Binding cannot turn an
unregistered connector identity into authority. Connector polling, credential
access, content materialization, and external execution are outside this
slice.

## Context Nodes and Authority Grants

Migration `034_context_node_registry.sql` adds governed, revisioned Context
Node descriptors. Stable identity and governance fields are immutable while
capability manifests and lifecycle changes use explicit revisions, registered
events, and archived history. Capability advertisement is validated against
active public schema versions and does not authorize access.

Migration `035_authority_grants.sql` adds purpose- and time-bounded Context and
Tool Grants. Their capability sets are disjoint, targets match exact stable
URIs, and immutable terms cannot change after issuance. Delegated grants must
strictly narrow an effective parent. Authorization resolves the complete
parent chain, so ancestor suspension, revocation, or expiry invalidates
descendants immediately.

Issuance and lifecycle transitions share the governance transaction and
idempotent event contract. Complete grant records use sealed payload storage
when vault sealing is enabled. See `CONTEXT_NODE_MODEL.md` and
`AUTHORITY_GRANT_MODEL.md`.

## Command and replay contract

`POST /api/v1/nodes` accepts:

```http
Idempotency-Key: 1-200 visible ASCII characters
X-Correlation-Id: UUID (optional)
X-Causation-Id: UUID (optional)
```

If `Idempotency-Key` is absent, the server generates a UUIDv7 key. This supports
ordinary callers but does not give an unkeyed network retry replay guarantees.

The durable replay scope is:

```text
(source_uri, principal_uri, idempotency_key)
```

The server hashes a canonical representation of the semantically relevant
request fields. Object keys and tags are order-normalized before hashing.

| Condition | Result |
| --- | --- |
| First key and digest | Create node and event atomically; return `201 Created` |
| Same scope, key, and digest | Return the original node and event; return `200 OK` |
| Same scope and key, different digest | Reject with `409 Conflict` |
| Mutation failure | Roll back both canonical state and outbox event |

The authenticated subject resolves to a principal URI under the local Context
Node through the governed identity registry (migration `040_identity_registry.sql`).
Bootstrap registers `local-system` and `local-context-owner` with the same v5
principal UUIDs as the transitional derivation. Unknown subjects fail closed unless
`MINDVAULT_IDENTITY_LEGACY_FALLBACK=1` enables the legacy v5 derivation path.

## Transactional outbox

Migration `032_interoperability_kernel.sql` adds:

- `interoperability_local_identity`, containing the durable local node UUID;
- `interoperability_outbox`, containing the complete serialized envelope,
  replay key and digest, delivery state, attempt count, and publication fields.

SQLite uses an immediate transaction for replay lookup, node insertion,
tag/changelog writes, and outbox insertion. The unique replay constraint
prevents concurrent duplicate commits.

Migrations `033` through `035` add the schema, Source Binding, Context Node,
Authority Grant, and history tables plus database-enforced schema admission.

Migration `036_outbox_dispatch_and_action_receipts.sql` adds the
transport-neutral dispatch control plane:

- eligible pending events are claimed under exclusive, bounded leases whose
  expiration instant is not part of the valid completion interval;
- expired leases can be reclaimed and advance the attempt counter exactly once;
- each completion atomically records an immutable action receipt and moves the
  event to retry, published, or dead-letter state;
- retry timestamps prevent early reclamation, while published and dead-letter
  states are terminal;
- database triggers bind each receipt to the exact active event, attempt,
  executor, destination, and lease, and forbid receipt mutation or deletion.

Receipts retain the exact claim ID, principal, actor, subject, correlation,
sensitivity, retention, provenance, request digest, timing, outcome, and
destination attribution. In sealed mode, provider references and error
summaries reside in encrypted receipt payloads; only bounded routing and lookup
fields remain indexed.
Idempotent completion replay returns the original receipt, while a different
result for the same attempt is rejected.

This slice supplies storage contracts for a publisher and retry worker, but it
does not run a background worker or implement an HTTP, Slack, email, MCP, or A2A
publisher. `pending` means eligible now or at `next_attempt_at`; an active lease
means one dispatcher currently owns an attempt. `published` means the declared
destination acknowledged that attempt. It does not mean every downstream
consumer applied the event.

## Durable consumer inbox and checkpoints

Migration `037_consumer_inbox_checkpoints.sql` adds transport-neutral consumer
admission and application state:

- exact redelivery for the same consumer is idempotent, while changed envelope
  content under an existing event ID fails closed;
- active public-schema admission is enforced by both Rust and SQLite;
- claims are exclusive, recoverable, bounded to one hour, and ordered within a
  local `(consumer, source)` stream;
- each attempt produces an immutable application receipt bound to its exact
  claim, processor, event digest, policy metadata, provenance, and outcome;
- retries remain pending without advancing a checkpoint;
- applied and dead-letter outcomes atomically advance the stream checkpoint.

The lease interval is `[claimed_at, lease_expires_at)`. A completion at or after
expiration is invalid, and reclaiming an expired event invalidates its previous
claim. Local inbox sequence and checkpoint state prove only local admission and
disposition order; they do not prove a remote stream was gap-free.

This slice provides the durable boundary a future subscriber and domain
handler must use. It does not run a transport listener, invoke a domain
projection, infer an issuer sequence, or requeue a dead letter. Dead-letter
redrive requires a separate governed command so a prior terminal receipt and
checkpoint are never rewritten.

## In-process notification behavior

The first commit emits one WebSocket change notification containing the
versioned event envelope. A replay returns the original result without
re-indexing, re-enriching, auto-linking, or broadcasting another notification.
WebSocket notification is an immediate local projection, not the durable
delivery mechanism; the outbox remains authoritative.

Full-text, vector, graph, and enrichment work runs after the canonical
transaction. Projection failure is logged without turning a successful durable
commit into an HTTP failure. The pending outbox event is the future
reconciliation hook; a durable projection checkpoint/worker is still required
before projection recovery can be considered complete.

## Security and operational boundaries

- Idempotency is principal-scoped, preventing one authenticated principal from
  replaying another principal's key.
- Payload digests detect key reuse with changed semantics; they are not content
  authentication signatures.
- The event contains stable identities and policy metadata but excludes node
  content.
- Existing REST authorization and quota checks remain in place. The grant
  resolver is now wired to public command admission on `POST /api/v1/nodes`,
  behind `MINDVAULT_COMMAND_ADMISSION_MODE` (`off` by default). Admission is
  additive — it never replaces the role, namespace, or quota checks — and runs
  after the idempotent-replay lookup and before the quota check. See
  `AUTHORITY_GRANT_MODEL.md` § Command admission.
- Unknown envelope versions, invalid schema-version tokens, malformed URIs,
  invalid digests, empty provenance, non-object data, and unregistered or
  mismatched event schemas fail closed.
- Source account/object lookup columns never contain raw provider identifiers;
  sealed mode encrypts the full binding record and cursor.
- Delivery leases are bounded to one hour and stale completions fail closed.
  Error summaries and provider response details are retained in immutable
  receipts rather than copied into plaintext outbox status.
- A published receipt is evidence of destination acknowledgement, not a claim
  of end-to-end or consumer-level exactly-once execution.
- No connector, extension, MCP host, or federated peer receives direct access
  to the outbox table.

## Verification

Tests cover URI and header validation, envelope round trips and required
fields, schema digest and dialect validation, Context Node revision history,
grant-kind separation, bounded delegation, ancestor invalidation, immutable
registration, binding uniqueness, encrypted governed storage, explicit rebind
history, atomic rollback, replay, digest conflict, one emitted versioned
notification, durable outbox records, exclusive and recoverable delivery
leases, retry timing, terminal delivery states, stale-claim rejection,
idempotent completion, immutable attempt receipts, sealed receipt details,
consumer redelivery replay and conflict rejection, same-source ordering,
consumer retry timing, expired-claim recovery, stale-completion rejection,
exact application-claim binding, terminal checkpoint advancement, and sealed
consumer payloads.

Grant admission is now wired to public command admission on `POST /api/v1/nodes`
(`IK-001`), ahead of any live provider publisher, as this section previously
required. It ships `off` by default and is additive to the existing role,
namespace, and quota checks.

Local Context Node registration (`IK-001a`) and admin grant issuance
(`IK-001b`, `POST /api/v1/authority-grants`) are both available. An operator can
run `enforce` on a real vault after registering the local node and issuing a
Tool Grant to the acting principal. `observe` remains the recommended rollout
position.

Versioned action envelopes (`IK-002`, ADR 014) are constructed for node create,
update, and delete whenever command admission is active. The envelope carries
action/correlation IDs, principal, acting actor, resource, operation, grant IDs,
and the policy decision. Space, work-order, budget, and outcome fields from
ADR 010 remain deferred to the Trust Ledger slice.

The outbox dispatcher runtime (`IK-004`) claims under a lease, completes through
the existing receipt binding, and ships a local-ack publisher so pending events
are no longer inert. Authenticated live transport publishers remain `IK-005`.
End-to-end completion may be claimed only when authenticated delivery can be
joined to the independently queryable consumer application receipt and
checkpoint.
