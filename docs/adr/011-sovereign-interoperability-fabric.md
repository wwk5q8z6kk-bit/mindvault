# ADR 011: Sovereign Interoperability Fabric

- **Status:** Accepted
- **Date:** 2026-07-26
- **Owners:** MindVault
- **Constitution:** `INTEROPERABILITY_CONSTITUTION.md`
- **Related:** ADR 001, ADR 002, ADR 005, ADR 008, ADR 009, ADR 010

## Context

ADR 010 establishes sovereign Personal Vaults and governed Collaborative
Spaces. That decision is necessary but not sufficient. A platform centered on
its own vaults and Spaces would still force applications, devices, agents, and
external services into one storage topology or a growing set of proprietary
integrations.

The system already exposes REST, gRPC, WebSocket, OpenAPI, a local MCP server,
WASM plugins, OAuth client credentials, and read-only federated queries. These
are useful components, but they do not yet share a stable source-authority,
schema, event, grant, provenance, extension, or portability contract.

Four approaches were compared:

1. **Continue adding product-specific integrations.** Lowest near-term cost,
   but creates vendor-shaped core objects and inconsistent permissions.
2. **Centralize all connected data.** Simplifies search, but weakens
   sovereignty, multiplies retention risk, and creates a new silo.
3. **Adopt one external protocol as the internal model.** Accelerates one
   integration class, but couples the domain to a protocol MindVault does not
   control.
4. **Govern a decentralized network through stable internal contracts and
   protocol adapters.** Higher initial design cost, but preserves source
   ownership, supports multiple protocols, and makes policy consistent.

Option 4 provides the best expected outcome. It is more reversible than a
central warehouse or protocol-shaped database because adapters and
materializations can evolve independently of source ownership.

## Decision

MindVault becomes a sovereign context and coordination fabric that is logically
central and physically decentralized.

`ContextNode` is the network primitive. Personal Vaults and Collaborative
Spaces remain canonical domains when MindVault owns the relevant state, while
applications, devices, agents, storage systems, execution environments, relay
services, indexes, and organizations participate as other node types.

The platform governs:

- identity and node relationships;
- source authority and materialization;
- public schemas and semantic compatibility;
- Context Grants and Tool Grants;
- delegation, approvals, provenance, and action receipts;
- query, command, event, synchronization, and portability contracts.

It does not automatically copy or become authoritative for externally owned
content.

External standards remain adapters over the internal domain:

- MCP for AI context and tool interoperability, in both server and host roles;
- A2A for external agent discovery, task exchange, progress, and artifacts;
- OpenAPI for public synchronous HTTP contracts;
- AsyncAPI and CloudEvents for event contracts and envelopes;
- JSON Schema and Protocol Buffers for public and internal typed contracts;
- OIDC, OAuth, and SCIM for identity, delegation, and provisioning;
- W3C PROV-compatible mappings for exchanged provenance;
- Matrix and Solid through optional bridges or adapters, not mandatory cores.

Every first-party feature must satisfy the constitution’s feature-completeness
contract. Every external object must have a Source Binding. Every AI access
requires a Context Grant, while external effects require a separate Tool Grant
and action envelope.

## Relationship to ADR 010

ADR 010 is preserved and narrowed:

- Personal Vault and Collaborative Space remain ownership and authorization
  boundaries.
- A Space is also a Context Node and may reference or materialize data owned by
  other nodes.
- “Two canonical state domains” means two MindVault-owned domains, not a claim
  that MindVault owns all connected state.
- The Project Space workflow becomes the governed work segment inside the
  broader meeting-to-agent-to-team-report interoperability slice.

If ADR 010 and this ADR appear to conflict, this ADR and the constitution govern
the interoperability boundary; ADR 010 governs Personal Vault and Space
semantics within it.

## Consequences

### Positive

- External systems can remain authoritative without becoming second-class.
- The same identity, policy, provenance, and receipt model governs humans,
  agents, applications, devices, and extensions.
- MCP, A2A, vendors, and transport choices can evolve without rewriting the
  core domain.
- Federated search and collaboration do not require a single global database.
- Open schemas, SDKs, and conformance tests support a real developer ecosystem.

### Costs and risks

- Source authority, schemas, grants, event delivery, and federation become
  security-critical infrastructure.
- “Universal connection” substantially expands SSRF, supply-chain, confused
  deputy, prompt-injection, exfiltration, and cross-node authorization risk.
- Existing federation, MCP, plugin, relay, and OAuth components require
  migration rather than relabeling.
- Interoperability contracts must be maintained across versions and languages.

## Implementation order

1. Ratify the constitution and Phase 0 contracts.
2. Implement the interoperability kernel.
3. Build the universal gateway and constrained extension runtime.
4. Add bidirectional MCP and A2A adapters.
5. Prove source authority and materialization semantics.
6. Enable federated nodes only after threat-model release gates pass.
7. Validate the complete interoperability vertical slice.

Empty crate scaffolding, connector quantity, or UI prototypes do not count as
progress against these gates.
