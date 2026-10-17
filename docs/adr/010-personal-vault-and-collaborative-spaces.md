# ADR 010: Personal Vaults and Collaborative Spaces

- **Status:** Proposed
- **Date:** 2026-07-26
- **Owners:** MindVault
- **Supersedes:** ADR 002 as a platform-wide prohibition on shared state
- **Preserves:** ADR 002 for the Personal Vault trust and ownership boundary
- **Related:** ADR 001, ADR 004, ADR 008, ADR 009, ADR 011
- **Subordinate to:** `INTEROPERABILITY_CONSTITUTION.md` for interoperability

## Context

MindVault currently treats one local vault as the sole canonical state. External
people and agents are relay contacts or delegated API consumers, and writes are
mediated through vault-wide roles, namespaces, proposals, and autonomy rules.
That model protects a private second brain, but it cannot represent a team that
shares projects, tasks, decisions, documents, delegated agent work, and durable
communication under explicit membership and policy.

Adding richer chat to the existing relay would preserve the wrong boundary. A
relay channel is a transport and grouping record; it is not an authority,
ownership, context, or work boundary. Treating it as one would make permissions
implicit, mix private memory with shared communication, and leave agent actions
without a durable principal or delegation chain.

Four directions were compared:

1. **Keep a single-owner vault and add chat features.** Lowest immediate cost,
   but cannot provide canonical shared work or defensible authorization.
2. **Federate only between personal vaults.** Preserves sovereignty and is
   reversible, but makes shared truth, membership changes, and concurrent team
   workflows needlessly complex.
3. **Turn the local vault into a multi-tenant database.** Reuses current
   storage, but weakens the private boundary and couples local/offline behavior
   to server concerns.
4. **Keep Personal Vaults and add governed Collaborative Spaces.** Requires new
   identity and shared-state components, but preserves local sovereignty while
   making collaboration an explicit domain.

Option 4 has the best expected outcome. Its execution cost is higher than a
chat overlay, but it is safer and more reversible than converting the Personal
Vault into a shared store.

## Decision

### Constitutional relationship

ADR 011 places these ownership domains inside a wider network of Context Nodes.
The Personal Vault and Collaborative Space are the two MindVault-owned
canonical state domains; they are not a claim that MindVault owns every
connected object. External applications, devices, repositories, services, and
other nodes may remain authoritative through explicit Source Bindings and
materialization policies.

Space, identity, work, and action contracts must therefore use the shared
interoperability kernel. Channels and vendor payloads remain adapters, while
MCP, A2A, OpenAPI, AsyncAPI, CloudEvents, and other protocols remain external
translations rather than internal database schemas.

### Product topology

MindVault will support two canonical state domains:

- A **Personal Vault** is local-first, private to one human owner, and remains
  canonical for that person's private knowledge, preferences, credentials, and
  private agent context.
- A **Collaborative Space** is canonical shared state for an explicit set of
  human and agent actors. It owns its membership, policy, communication, work,
  artifacts, approvals, and audit history.

The root collaboration primitive is `Space`. Channels, threads, projects,
tasks, documents, decisions, work orders, agent runs, artifacts, automations,
and notifications belong to a Space. A product-level `Workspace` is an
administrative container for Spaces, identities, organization policy, and
service configuration; it is not the primary authorization shortcut.

ADR 008 and ADR 009 use “workspace” for a mounted document root. Before shared
Workspace APIs ship, implementation names must distinguish that concept as
`DocumentWorkspace` (or an equally explicit name) from the collaboration
`Workspace`. Neither may inherit the other's authorization semantics by name.

### Actors and attribution

`Actor` is a first-class principal with an explicit kind:

- `human`;
- `agent`;
- `service`;
- `integration`.

Agents participate in Spaces but are not represented as human users. Every
state-changing action records both the accountable principal and, when
different, the acting agent or integration. Messages sent on behalf of a human
must identify the human principal, acting actor, delegation source, and policy
decision. The product must not impersonate a human or collapse these identities
into one sender string.

External identities are mappings to Actors, not the Actors themselves. Slack,
Discord, email, OAuth, MCP, and future A2A identifiers therefore attach through
versioned identity records.

### Membership and authorization

Authorization is default-deny and evaluated against:

1. authenticated actor identity;
2. active Space membership and role;
3. resource and operation;
4. context and tool grants;
5. delegation chain and applicable policy;
6. expiry, budget, rate, and approval constraints.

Namespaces remain useful for local partitioning and search, but they are not
membership or authorization boundaries. Relay contact trust levels are hints
for migration, not Space roles.

Human ownership and escalation are explicit. An agent can receive narrowly
scoped membership or a time-bound grant, but it cannot silently expand its
scope, approve its own elevated action, or create a new human principal.

### Work and agent execution

Agent delegation uses structured records:

- `WorkOrder` captures goal, requester, assignee, Space, inputs, constraints,
  approval policy, budgets, expected artifacts, and acceptance criteria.
- `AgentRun` captures the concrete execution attempt, model/runtime identity,
  context and tool grants, status, timing, usage, outputs, evidence, errors, and
  receipts.
- `Artifact` is a versioned result with provenance and verification state.

Existing tasks and plans may be adapted into this model, but a task title,
free-form assignee string, or plan step is not an agent run.

### Action envelope and trust ledger

Every durable command or external effect uses one action envelope with, at
minimum:

- action and correlation IDs;
- Workspace and Space IDs;
- accountable principal and acting actor IDs;
- delegation, work order, and agent run IDs when applicable;
- resource, operation, and expected version or idempotency key;
- context and tool grant IDs;
- policy decision and approval references;
- budget/usage data;
- outcome, artifacts, evidence, error, and timestamps.

The append-only action history is the Trust Ledger. Request telemetry,
proposal history, autonomy logs, and agent activity can feed it, but none is a
substitute until it carries the complete envelope and is durable by default.

### Communication and knowledge promotion

Communication and canonical knowledge are separate states:

```text
raw communication
        ↓ explicit or policy-governed extraction
candidate knowledge
        ↓ review, evidence, or accepted automation
canonical knowledge
```

Inbound and outbound messages may be indexed in a retention-controlled
communication store. They do not automatically become canonical Personal Vault
or Space knowledge. Promotion records source message IDs, extractor or actor,
evidence, confidence, policy, and approval.

Relay send and receive enforce this boundary by storing `RelayMessage` records
without creating `KnowledgeNode::Conversation` records. The optional
`vault_node_id` remains for compatibility with existing records; this change
does not migrate, delete, or alter those records. A future remediation for
legacy links must be separately approved and non-destructive: first produce a
reviewable inventory with provenance and retention decisions, then apply only
explicitly approved actions.

Inbound email attachment bytes and metadata remain relay-scoped data. They are
not attached to or text-indexed through a canonical knowledge node.

### Storage and synchronization

The Personal Vault keeps the local-first SQLite and derived-index architecture.
Collaborative Spaces use a server-authoritative transactional store, initially
PostgreSQL, plus:

- an append-only event/action log;
- an outbox for reliable external effects;
- object storage for larger artifacts;
- a realtime gateway for bounded subscriptions;
- projection workers that can rebuild search/read models.

A modular monolith is the initial server shape. Kafka, a broad microservice
split, general CRDT editing, and an unrestricted workflow engine are deferred
until measured scale or product requirements justify them.

Private data is never copied into a Collaborative Space merely because a person
or agent joined it. Every transfer is an explicit, attributed action governed
by policy. Offline clients cache authorized shared data and reconcile through
versioned events; they do not become competing authorities for shared state.

### Integration boundary

Slack, Discord, email, MCP, OAuth/OIDC, and future A2A support are adapters over
the same identity, Space, action, policy, and outbox contracts. Adapter-specific
IDs and delivery receipts remain at the edge.

MindVault owns knowledge, context, work, policy, and durable collaboration
state. DevX remains a distinct execution environment and may act as an agent or
tool provider through explicit grants and recorded runs.

### First collaboration segment

The first collaboration segment participates in the constitution's broader
external-meeting-to-team-report interoperability slice:

```text
approved meeting context enters a Project Space
  → human creates structured work
  → human delegates a WorkOrder to an agent
  → agent executes under context/tool/budget grants
  → artifacts and evidence are attached to an AgentRun
  → human verifies or rejects the result
  → an attributed team report is published
```

This segment must use one Space authorization path, Source Bindings for
external evidence, and one action/event envelope from the start. It must not
depend on Slack, Discord, an MCP object, or an A2A task as the canonical state
store.

## Consequences

### Positive

- Personal knowledge remains sovereign and local-first.
- Team truth has an explicit authority, membership, and policy boundary.
- Humans and agents can collaborate without identity collapse or impersonation.
- Communication adapters become replaceable edges rather than product
  architecture.
- Work, evidence, approval, and external effects share one provenance model.

### Costs and risks

- Identity, membership, authorization, shared persistence, and sync are new
  production-critical subsystems.
- Existing namespaces, relay contacts, tasks, plans, and proposals need
  migrations rather than cosmetic renaming.
- Local and shared schemas will evolve at different rates and need explicit
  compatibility contracts.
- Collaboration introduces abuse, tenant isolation, retention, recovery, and
  operational risks not present in a single-owner local vault.

## Non-goals for the initial implementation

- replacing Slack, Discord, or email;
- autonomous agents with unrestricted standing privileges;
- promoting every message into durable knowledge;
- arbitrary plugin code in the shared server;
- general multi-master synchronization;
- full CRDT document editing;
- Kafka or premature service decomposition;
- enterprise billing and marketplace scope.

## Acceptance gate

This ADR can become Accepted when:

1. the current-state audit has an owner and disposition for every reused
   relay, task, agent, adapter, proposal, policy, and audit component;
2. Actor, Workspace, Space, Membership, WorkOrder, AgentRun, Artifact, grant,
   action-envelope, and outbox schemas have migration and rollback plans;
3. an authorization matrix proves cross-Space isolation and human/agent
   attribution;
4. the communication-to-knowledge promotion contract has retention, review,
   and provenance tests;
5. the collaboration segment passes the constitution's end-to-end
   interoperability acceptance and threat-model cases;
6. failure recovery proves idempotent commands and external effects;
7. Personal Vault data remains private unless an explicit transfer is approved.
