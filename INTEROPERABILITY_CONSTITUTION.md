# MindVault Interoperability Constitution

- **Status:** Ratified
- **Effective:** 2026-07-26
- **Authority:** Highest-priority product and architecture law
- **Decision record:** `docs/adr/011-sovereign-interoperability-fabric.md`

## Product definition

MindVault is a sovereign, interoperable context fabric through which humans and
organizations connect their knowledge, applications, devices, people, models,
agents, workflows, and external services.

The platform is:

> **Logically central and physically decentralized.**

It is logically central because it governs identity, relationships, policy,
source authority, provenance, context routing, agent authority, approvals,
activity history, and action receipts. It is physically decentralized because
source data may remain in a Personal Vault, Shared Space, device, application,
repository, external service, private server, or another authorized Context
Node.

The platform is the coordination authority. It is not required to become the
physical warehouse or canonical source for every byte.

## Priority and conflict rule

This constitution sits above knowledge management, collaboration, agents,
tasks, automations, integrations, and user-interface design.

When a feature plan, ADR, API, storage choice, or implementation conflicts with
this constitution, the constitution wins unless a dated, scoped exception is
recorded in an ADR with an owner, expiry, migration path, and security review.
Convenience, vendor limitations, and existing code are not sufficient reasons
for an implicit exception.

ADR 010 remains authoritative for Personal Vaults and Collaborative Spaces, but
those domains are Context Node types within this broader fabric.

## Constitutional laws

1. **No vendor-specific object in the core.** Vendor payloads terminate at
   adapters and map to versioned public schemas or retained source artifacts.
2. **No UI-only capability.** First-party features require governed command,
   query, event, permission, provenance, audit, export, and conformance
   contracts.
3. **No extension receives direct canonical database access.** Extensions use
   the same policy-enforcing command and query boundaries as first-party
   clients.
4. **Every committed mutation emits a durable event.** The state change and
   outbox record share one transaction or equivalent atomic boundary.
5. **Every event is attributable and interpretable.** It carries source,
   identity, schema, subject, causation, correlation, sensitivity, provenance,
   retention, and idempotency semantics.
6. **Every external object has declared source authority.** Source ownership,
   materialization, sync direction, conflict, deletion, and freshness policies
   are explicit.
7. **Every AI receives an explicit Context Grant.** A model or agent sees only
   the context authorized for a stated purpose and duration.
8. **Context access and action authority are separate.** Reading data never
   implies permission to mutate, execute, transmit, or spend.
9. **Protocols remain adapters.** MCP, A2A, OpenAPI, AsyncAPI, CloudEvents,
   Matrix, Solid, and later standards do not become the internal domain model.
10. **No mandatory central extension registry.** Users may install from
    official, enterprise, self-hosted, signed, local, or development sources
    under explicit trust policy.
11. **All user data remains portable.** Export, restore, and migration preserve
    identity, provenance, relationships, source authority, and receipts.
12. **Core workflows function locally.** Cloud services may enhance but cannot
    become an undeclared dependency for Personal Vault ownership or recovery.
13. **Federation does not require surrendering ownership.** Nodes answer scoped
    queries or exchange signed packages without wholesale centralization.
14. **Protocol and schema versions are negotiated.** Unknown or incompatible
    versions fail explicitly and preserve source data for later recovery.
15. **Unknown extensions fail closed.** Undeclared capability, network, secret,
    file, schema, or data-class access is denied and recorded.

## Governed centrally

The fabric governs one coherent answer for:

- stable actor, resource, node, source, and relationship identities;
- permissions, Context Grants, Tool Grants, delegation, and approvals;
- source authority, materialization, synchronization, and conflict policy;
- public schemas, compatibility, provenance, and evidence;
- agent discovery, Work Orders, Agent Runs, artifacts, and budgets;
- command authorization, event publication, action receipts, and rollback;
- federated catalog/search routing, freshness, trust, and revocation.

Governance may be enforced by a Personal Node, Space Node, Organization Node,
or delegated control plane. “Logically central” does not mean one vendor cloud,
one database, or one globally privileged operator.

## Data placement and materialization

Each source binding selects one governed materialization mode:

- `reference_only`;
- `metadata_mirror`;
- `search_projection`;
- `cached_excerpt`;
- `full_replica`;
- `canonical_import`;
- `derived_knowledge`.

The default is the least materialization needed for the approved outcome.
Copying source data requires a purpose, retention rule, sensitivity label, and
provenance link. Derived knowledge never erases the authority of its evidence.

## Feature completeness contract

A first-party capability is complete only when it has:

1. a typed and versioned domain object;
2. a command API;
3. a query API;
4. emitted events;
5. permission and grant definitions;
6. provenance behavior;
7. Trust Ledger behavior;
8. portable export and restore behavior;
9. safe extension access where applicable;
10. automated contract and conformance tests.

A visual interface may lead implementation, but it cannot be the only usable
or authoritative contract.

## Open interoperability layer

Public schemas, event formats, connector manifests, Context Capsule formats,
agent profiles, import/export formats, SDKs, reference adapters, signature
formats, and conformance suites should be openly specified. The durable product
advantage should come from retrieval quality, policy enforcement, trustworthy
coordination, synchronization, human experience, and operational excellence,
not preventable lock-in.

## Initial implementation gate

Feature implementation under this constitution begins with the
interoperability kernel:

- stable URIs and external identity mappings;
- schema and source-authority registries;
- Context Node and capability records;
- Context and Tool Grants;
- a versioned action and event envelope;
- transactional inbox/outbox and idempotency;
- provenance mappings and action receipts;
- connector manifests, health, cursors, and revocation.

The first proof is:

```text
External meeting source
  -> universal governed ingestion
  -> candidate decisions and tasks
  -> human-approved canonical context
  -> retrieval from at least two independent MCP hosts
  -> Work Order execution by an A2A-adapted agent
  -> verified artifact and team report
  -> approved delivery with a complete action receipt
```

Vendor-specific UI, marketplace scale, unrestricted automation, and broad
connector quantity do not unlock this slice. Contract conformance does.

## Supporting contracts

- `docs/architecture/SOURCE_AUTHORITY_MODEL.md`
- `docs/architecture/CONTEXT_NODE_MODEL.md`
- `docs/architecture/EXTENSION_SECURITY_MODEL.md`
- `docs/architecture/PROTOCOL_BOUNDARIES.md`
- `docs/architecture/FEDERATION_THREAT_MODEL.md`
- `docs/architecture/DATA_PORTABILITY_CONTRACT.md`
- `docs/architecture/interoperability-baseline.md`
- `docs/architecture/interoperability-kernel-v1.md`
- `docs/architecture/AUTHORITY_GRANT_MODEL.md`
- `docs/architecture/ACTION_RECEIPT_MODEL.md`
- `docs/architecture/CONSUMER_INBOX_MODEL.md`
- `docs/architecture/WORK_ORDER_MODEL.md`
- `docs/architecture/EXECUTION_ISOLATION_MODEL.md`

## Execution tracking

`IMPLEMENTATION_BACKLOG.md` is the authoritative record of every deferred,
gated, or blocked item mandated by this constitution and its supporting
contracts. It supersedes `DEVELOPMENT_PLAN.md` for execution tracking. An
obligation stated in a contract but absent from the backlog is a defect in the
backlog; an item marked complete without a runnable verification command is not
complete.
