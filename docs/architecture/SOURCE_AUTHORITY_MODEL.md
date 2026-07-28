# Source Authority Model

- **Status:** Required architecture contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`

## Purpose

The Source Authority Registry prevents split-brain ownership when MindVault and
an external system describe the same object. It distinguishes identity,
custody, authority, materialization, and derivation.

## Core concepts

- **Source Authority** is the system authorized to define the current canonical
  state of an object or field.
- **Source Binding** maps a stable internal resource URI to an external system,
  account, and object identity.
- **Materialization** is the local representation retained for an authorized
  purpose.
- **Derived Resource** is a new object supported by source evidence but governed
  independently from that evidence.
- **Projection** is rebuildable state and never establishes authority.

Authority may be field-scoped. A calendar provider may own event time while a
Space owns an internal attendance decision linked to the event.

## Source Binding

Every external object represented by the fabric must have a versioned binding
containing at least:

```text
binding_id
resource_uri
node_id
external_system
external_account_id
external_object_id
authoritative_source
authoritative_fields
sync_direction
last_seen_version
last_seen_at
last_sync_cursor
content_hash
materialization_mode
conflict_policy
deletion_policy
retention_class
sensitivity
provenance_ref
status
```

The tuple `(node_id, external_account_id, external_object_id)` is unique for an
active binding. Rebinding requires an explicit migration event and preserves
the prior binding in history.

## Materialization modes

| Mode                | Locally retained state                     | Authority                           |
| ------------------- | ------------------------------------------ | ----------------------------------- |
| `reference_only`    | Identity, location, policy, freshness      | External                            |
| `metadata_mirror`   | Selected metadata and source pointer       | External                            |
| `search_projection` | Rebuildable authorized index content       | External                            |
| `cached_excerpt`    | Bounded excerpt with expiry                | External                            |
| `full_replica`      | Encrypted copy with sync metadata          | External unless imported            |
| `canonical_import`  | Complete owned resource                    | MindVault node                      |
| `derived_knowledge` | Approved claim, decision, task, or summary | MindVault node, with evidence links |

The least-retentive mode that satisfies the approved purpose is the default.

## Mutation routing

Commands against externally authoritative state must:

1. resolve the active Source Binding;
2. authorize the actor, principal, Context Grant, and Tool Grant;
3. check source version and conflict policy;
4. enqueue the external command through the transactional outbox;
5. retain the provider response and delivery identity;
6. refresh or invalidate local projections;
7. append an action receipt with causation and provenance.

The UI, agent, extension, and automation paths use this same sequence. A local
database update cannot simulate success against an external authority.

## Conflict and freshness rules

- Freshness is explicit: `fresh`, `stale`, `unavailable`, `conflicted`, or
  `unknown`.
- Last-write-wins is not a default conflict policy.
- Conflicts preserve both observed versions and identify the authoritative
  field owner.
- Search results expose source, materialization mode, and freshness.
- A stale projection may support discovery but cannot silently authorize a
  consequential action.

## Deletion and revocation

Deletion semantics are binding-specific:

- source deletion may create a tombstone, purge a cache, retain a legal hold,
  or require review;
- uninstalling a connector removes credentials and active synchronization but
  must not corrupt independently canonical derived resources;
- revoked access makes the source unavailable and invalidates unauthorized
  caches;
- provenance records retain non-sensitive identifiers needed to explain prior
  actions, subject to policy.

## Examples

| Object                    | Authority                | MindVault responsibility                     |
| ------------------------- | ------------------------ | -------------------------------------------- |
| GitHub pull-request state | GitHub                   | Reference, projection, report, receipt       |
| Meeting recording         | Meeting source or device | Reference/replica by policy, derived summary |
| Approved meeting decision | Space Node               | Canonical decision with evidence             |
| Calendar event time       | Calendar provider        | Mirror and proposed mutation                 |
| Personal Markdown note    | Personal/Device Node     | Canonical file plus derived indexes          |
| Team document             | Space Node               | Canonical collaborative state                |
| Slack message             | Slack                    | Reference or policy-selected mirror          |
| Platform Space message    | Space Node               | Canonical communication state                |
| Agent Run                 | Agent control domain     | Canonical run, artifacts, and receipts       |

## Conformance gates

- Duplicate delivery cannot create duplicate active bindings.
- Connector failure cannot advance a cursor past uncommitted source records.
- Uninstall and reinstallation preserve canonical resources and explainable
  provenance.
- Conflict, deletion, stale-source, and authority-transfer fixtures are
  portable across connector implementations.
- No vendor SDK type crosses the connector-to-domain boundary.

## Implementation status

Migration `033_governed_interoperability_registries.sql` and the core
interoperability model implement the first registry slice:

- active-binding uniqueness for the constitutional identity tuple;
- atomic, idempotent registration plus an attributable outbox event;
- explicit rebinding with archived predecessor history and a migration event;
- sealed record encryption and keyed-HMAC external-identifier lookups;
- typed authority, freshness, materialization, conflict, deletion, retention,
  sensitivity, provenance, and lifecycle fields.

Connector observation ingestion, cursor advancement, external command
delivery, conflict execution, deletion enforcement, and uninstall/reinstall
fixtures remain gated work. No standalone cursor mutation API exists in this
slice; cursor progress must later commit atomically with the governed source
observation it acknowledges.
