# Context Node Model

- **Status:** Implemented core and storage contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`

## Purpose

A `ContextNode` is an independently addressable participant that stores,
indexes, executes, relays, or governs context. It is the network primitive for
a logically unified but physically decentralized fabric.

A node is not automatically a tenant, process, database, or trust domain.
Those properties are declared by its type, owner, capabilities, and policy.

## Node types

| Type           | Primary responsibility                                      |
| -------------- | ----------------------------------------------------------- |
| `personal`     | Private knowledge, preferences, files, and personal agents  |
| `device`       | Laptop, phone, recorder, wearable, or home server           |
| `space`        | Shared project or team context                              |
| `organization` | Organization identity, policy, administration, and services |
| `application`  | External application or SaaS account                        |
| `agent`        | Discoverable work-performing actor or service               |
| `storage`      | Local disk, NAS, object store, drive, or database           |
| `execution`    | DevX, Codex, Claude Code, container, or remote worker       |
| `relay`        | Governed message, notification, or event transfer           |
| `index`        | Authorized search or semantic projection                    |

New node types require a schema version and cannot inherit authority merely by
their name.

## Stable identity

Every node has:

```text
node_id
node_uri
node_type
owner_actor_id
governing_node_id
display_name
capability_manifest
supported_protocols
supported_schema_versions
trust_class
public_keys
endpoints
data_residency
status
created_at
updated_at
```

`node_uri` is stable across endpoint, device, and display-name changes.
External identifiers map to the node through versioned identity records.
Endpoint discovery never proves node identity by itself.

## Control and data planes

The logical control plane governs:

- node and actor identity;
- capability and schema negotiation;
- trust, grants, policy, and revocation;
- source bindings and routing;
- provenance and receipts;
- freshness and synchronization state.

The data plane may remain distributed. A node may answer a scoped query,
return a signed Context Capsule, emit an event, accept a command, or provide a
reference without transferring its complete underlying store.

## Capability contract

Nodes advertise a versioned subset of:

- `discover`;
- `query`;
- `read`;
- `subscribe`;
- `propose`;
- `command`;
- `execute`;
- `notify`;
- `synchronize`;
- `export`;
- `import`;
- `health`;
- `revoke`.

Advertisement is not authorization. Each invocation still requires an
authenticated actor, applicable grants, policy evaluation, and an action
envelope. The implemented grant boundary is specified in
`AUTHORITY_GRANT_MODEL.md`.

## Discovery and trust establishment

1. Resolve a candidate endpoint through an operator-approved or policy-approved
   discovery path.
2. Fetch the node descriptor with strict size, redirect, network, and timeout
   limits.
3. Verify node identity and key proof out of band or through an already trusted
   authority.
4. Negotiate protocol, schema, and capability versions.
5. Record trust class, allowed operations, and data classifications.
6. Issue narrow, short-lived credentials or grants.
7. Record the relationship and all changes in the Trust Ledger.

Adding an endpoint does not make it trusted. Unknown nodes fail closed.

## Personal Vaults and Spaces

- A Personal Vault is the canonical private context domain of one human and is
  commonly hosted by a Personal Node and one or more Device Nodes.
- A Collaborative Space is canonical shared context and is represented by a
  Space Node governed by membership and policy.
- An Organization Node may govern many Space Nodes without owning Personal
  Vault contents.
- Transfers between nodes are explicit, attributed, and governed by source
  authority and materialization policy.

## Federated query

Federated query uses scoped subqueries:

```text
question
  -> local policy-aware planner
  -> authorized node selection
  -> bounded subqueries
  -> signed results with source and freshness
  -> local validation, deduplication, and ranking
  -> Context Capsule or cited response
```

A remote result is evidence from a source, not automatically local canonical
knowledge. Query planners must enforce budget, timeout, sensitivity, residency,
and minimum-trust policies.

## Context Capsules

A `ContextCapsule` is a signed, purpose-bound transfer package containing:

- objective and recipient;
- selected source excerpts or typed objects;
- source authority, provenance, and freshness;
- permitted operations and model-use restrictions;
- expiration, redistribution, and retention rules;
- revocation handle and issuer signature.

Capsules may be query-only, streamed, copied, local-only, one-time, or
time-limited. A capsule does not transfer source authority unless an explicit
canonical-import command says so.

## Lifecycle

Nodes transition through:

```text
discovered -> pending_trust -> active -> suspended -> revoked -> retired
```

Suspension blocks new access while preserving evidence needed for investigation
and recovery. Revocation invalidates credentials, grants, subscriptions, and
cached capability descriptors. Retirement preserves stable identifiers and
tombstones required for provenance.

## Conformance gates

- Node identity survives endpoint rotation.
- A malicious discovery response cannot select private or link-local targets.
- Capability advertisement cannot self-authorize an operation.
- Revocation terminates new queries, commands, and subscriptions within the
  documented bound.
- Cross-node results retain source, freshness, schema, and signature status.
- Offline queues replay idempotently without changing source authority.

## Implementation status

Migration `034_context_node_registry.sql` provides revisioned Context Node
descriptors, immutable identity and governance fields, lifecycle history,
sealed record payloads, active public-schema validation, and atomic registered
events for registration, descriptor updates, and lifecycle transitions.

Migration `035_authority_grants.sql` adds the separate authorization boundary.
The registry and grant store are internal contracts in this slice; public
discovery, remote key proof, signature verification, federated query, and
action-envelope enforcement remain release gates. Initial local bootstrap may
register the self-governed local node directly as active; every subsequent
Source Binding registration or rebind is admitted only for an active,
registered local Context Node.
