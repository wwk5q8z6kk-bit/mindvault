# Interoperability Current-State Baseline

- **Observed:** 2026-07-26
- **Owners:** MindVault
- **Scope:** API transports, MCP, federation, plugins, connectors, identity,
  events, provenance, source authority, grants, and portability
- **Purpose:** Establish the migration baseline for ADR 011 and the
  Interoperability Constitution

## Executive finding

MindVault has valuable interoperability components and now has the first
transactional slice of an interoperability kernel. It does not yet have the
complete kernel required by the constitution.

The current repository provides a local MCP stdio server, REST/gRPC/WebSocket
transports, generated OpenAPI, bearer/JWT/local OAuth client-credential
authentication, namespace-aware keys, WASM plugin hooks, adapter traits,
read-only peer queries, file-first document contracts, export/backup features,
stable core URIs, a versioned internal event envelope, an atomic
node-create/outbox boundary with principal-scoped idempotency, immutable public
schema records, governed Source Bindings with explicit rebind history,
revisioned Context Node descriptors, and purpose-bound Context/Tool Grants.
The outbox now also has a transport-neutral lease/retry/dead-letter lifecycle
with immutable, attributable action receipts. A durable consumer inbox records
idempotent admission, application leases and receipts, and local stream
checkpoints. These components still do not share a live transport publisher,
authenticated remote admission, CloudEvents
profile, extension manifest, federated trust model, public grant admission
boundary, or semantic portability contract.

Existing components should be migrated behind the constitution’s contracts.
They should not be relabeled as proof that the contracts already exist.

## Current foundation versus required platform

| Area             | Current repository                                                                                                  | Required disposition                                                                                                       | Evidence                                                                                                                         |
| ---------------- | ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| MCP              | Local stdio server and connector marketplace records                                                                | Add governed server profiles plus client/host manager; retain proposal-only mutation                                       | `crates/mv-mcp/`; `crates/mv-engine/src/engine/mcp_ops.rs`; `migrations/027_mcp_connectors.sql`                                  |
| Public API       | REST with generated OpenAPI; gRPC and WebSocket transports                                                          | Define one governed command/query contract and transport parity policy                                                     | `crates/mv-server/src/openapi.rs`; `crates/mv-server/src/grpc.rs`; `crates/mv-server/src/websocket.rs`                           |
| Authentication   | Shared bearer token, HS256 JWT, access keys, local OAuth client credentials                                         | Add OIDC, distinct actor/device/agent identities, short-lived grants, and audience binding                                 | `crates/mv-server/src/auth.rs`; `crates/mv-server/src/rest/oauth.rs`                                                             |
| Authorization    | Admin/write/read plus optional namespace; internal exact-target Context/Tool Grants with bounded delegation         | Wire grant resolution to command/query admission and add Space membership and policy decisions                             | `crates/mv-server/src/auth.rs`; `crates/mv-core/src/model/interoperability.rs`; `migrations/035_authority_grants.sql`             |
| Federation       | Process-memory peers; operator-supplied endpoints; optional shared-secret request signing; read-only recall fan-out | Add durable node identity, endpoint policy, asymmetric trust, scoped grants, signed results, revocation, and SSRF controls | `crates/mv-engine/src/federation.rs`; `crates/mv-server/src/rest/federation.rs`                                                  |
| Plugins          | Manifest permissions for node read/write/search; WASM host gate and fuel                                            | Replace with signed extension manifest, brokered secrets, egress/file scopes, limits, receipts, and revocation             | `crates/mv-plugin/src/manifest.rs`; `crates/mv-plugin/src/sandbox.rs`; `crates/mv-plugin/src/wasm_plugin.rs`                     |
| Connectors       | Adapter trait and per-provider implementations/configuration                                                        | Standardize discover/read/search/subscribe/import/export/execute/health plus source bindings and durable cursors           | `crates/mv-engine/src/adapters/`; `crates/mv-server/src/rest/adapters.rs`                                                        |
| Events           | Versioned envelope, active schema admission, atomic outbox commits, producer and consumer leases, immutable receipts, and local checkpoints | Add authenticated live publishers, CloudEvents/AsyncAPI mappings, and coverage for all mutations | `crates/mv-core/src/model/interoperability.rs`; `migrations/032_interoperability_kernel.sql`; `migrations/036_outbox_dispatch_and_action_receipts.sql`; `migrations/037_consumer_inbox_checkpoints.sql` |
| Source authority | Governed portable bindings, hashed lookups, sealed payloads, active-tuple uniqueness, and explicit rebind history   | Add connector-driven freshness/materialization transitions and governed deletion execution                                 | `crates/mv-core/src/model/interoperability.rs`; `migrations/033_governed_interoperability_registries.sql`                         |
| Provenance       | Node sources, evidence links, proposal history, request audit, and immutable publication and consumer-application receipts | Require the same receipt boundary across remaining effectful actions                                                   | `crates/mv-core/src/model/`; `crates/mv-storage/src/sqlite.rs`; `crates/mv-server/src/audit.rs`                                  |
| Audit            | Optional request audit with subject/role/namespace                                                                  | Keep as telemetry; add mandatory Trust Ledger with principal/actor/grants/effects                                          | `crates/mv-server/src/audit.rs`                                                                                                  |
| Portability      | JSON/export, backup, Markdown workspace contracts                                                                   | Add signed semantic bundle, identity/source/provenance preservation, conformance fixtures                                  | `crates/mv-engine/src/export.rs`; `crates/mv-engine/src/backup.rs`; `docs/architecture/knowledge-workspace-document-contract.md` |

## Highest-risk mismatches

### Federation trust is not a production node model

Peer endpoints and metadata are held in process memory. Handshake fetches a
caller-provided endpoint and registers the returned identity. Optional HMAC
signing protects request bodies within a clock window, but endpoint discovery,
network destination policy, durable trust, key rotation, result signatures,
grant scope, and revocation are not the target federation boundary.

### Authentication is not delegated authority

Current role and namespace checks are reusable middleware foundations. They
cannot distinguish a human principal, acting agent, device, connector, run,
Context Grant, Tool Grant, policy decision, or source resource.

### Plugins and adapters do not share one extension contract

The WASM runtime has useful permission checks and resource metering. Adapters,
MCP connector records, relay providers, and plugins use separate capability and
lifecycle concepts. A shared Source Binding now exists, but none implements the
full signed manifest, secret-broker, data-class, egress, outbox-dispatch, or
receipt contract.

### API access is not semantic interoperability

OpenAPI and transport parity expose operations, but the governed schemas and
Source Bindings currently have internal command/query contracts rather than
public transport parity, and no explicit translation-loss policy exists. Two
clients can still exchange JSON while disagreeing about identity, authority,
freshness, or provenance.

## Reusable foundations

- Rust domain/engine/storage separation and migration discipline;
- REST auth middleware, OpenAPI generation, gRPC parity tests, and WebSocket
  plumbing;
- local MCP server and proposal-only external mutation posture;
- WASM isolation, permission gate, fuel, and plugin lifecycle;
- adapter trait, provider-specific implementations, and credential store;
- HMAC federation request signing and timeout/error isolation;
- file-first Markdown identity and guarded mutation contracts;
- proposal, policy, evidence, backup, and export concepts.
- canonical `mindvault://` URI validation and durable local node identity;
- versioned event envelope plus atomic node-create/outbox/idempotency boundary;
- immutable public schemas with database-enforced event admission;
- governed Source Bindings with hashed lookups, sealed payload support, and
  explicit atomic rebind history.
- revisioned Context Node descriptors with capability manifests and governed
  lifecycle history;
- disjoint Context/Tool Grants with exact targets, bounded delegation, sealed
  payloads, and ancestor-aware authorization resolution.
- bounded outbox leases, scheduled retries, terminal dead letters, and
  immutable action receipts with sealed provider details.
- idempotent consumer admission, per-source application ordering, recoverable
  leases, immutable application receipts, and terminal local checkpoints.

## Phase 0 completion gate

Phase 0 architecture approval is complete. ADR 011 and the constitution are
ratified, the six supporting contracts are present, and implementation may
proceed incrementally behind their release gates.

The following implementation-program obligations remain active:

1. assign durable owners for schemas, grants, events, outbox delivery, receipts,
   connector runtime, and conformance;
2. define migrations and rollback for existing federation peers, OAuth
   consumers, MCP connector records, plugins, adapters, relay identities, and
   exported objects;
3. select the first federation deployment profile and validate its threat-model
   assumptions.

## Phase 1 kernel-slice status

The initial executable slice is documented in
`interoperability-kernel-v1.md`. It establishes stable URIs, a durable local
node identity, the versioned internal event envelope, atomic,
principal-scoped idempotent node creation, immutable schema registration, and
governed Source Binding registration and rebinding, revisioned Context Node
descriptors, internal Context/Tool Grant issuance and lifecycle resolution,
the durable outbox dispatch state machine with immutable per-attempt action
receipts, and a durable consumer inbox with application receipts and local
stream checkpoints.

It does not claim completion of the constitutional kernel. Governed Context
Node remote trust proof, schema lifecycle commands, authenticated inbox
transport admission, outbox transport publication, connector lifecycle,
public registry and grant transports, admission enforcement, and conformance
fixtures remain required before the end-to-end proof. A published receipt
currently proves acknowledgement by one declared destination, not downstream
consumer application or global exactly-once delivery. Consumer checkpoints use
local admission sequence numbers and therefore cannot detect gaps in a remote
issuer's stream.

## First implementation proof

The first proof is the external-meeting-to-team-report slice in the
constitution. It is accepted only when:

- no vendor-specific type enters the core;
- duplicate delivery creates no duplicate canonical action;
- every result retains source authority and provenance;
- candidate knowledge requires explicit or policy-approved promotion;
- two independent MCP hosts can retrieve only granted context;
- two independent A2A-adapted agents can execute bounded work;
- external delivery uses an outbox and complete action receipt;
- export/restore and connector uninstall preserve canonical integrity.

## Conclusion

ADR 011 is the highest architecture direction. ADR 010 supplies the owned
Personal Vault and Space boundaries inside it. The repository has enough
foundations for incremental migration, but federation, MCP, plugins, and
connectors must converge on the interoperability kernel before broad connector
or UI work begins.
