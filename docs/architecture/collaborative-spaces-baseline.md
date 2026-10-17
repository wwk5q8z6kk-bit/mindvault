# Collaborative Spaces Current-State Baseline

- **Observed:** 2026-07-26
- **Owners:** MindVault
- **Scope:** Relay, tasks, plans/agents, Slack, Discord, email, proposals,
  autonomy/policy, authorization, MCP/OAuth-facing identity, realtime, and audit
- **Purpose:** Identify reusable foundations and migration blockers before
  implementing ADR 010 under ADR 011 and the Interoperability Constitution

## Executive finding

MindVault has useful local collaboration ingredients, but no Collaborative
Space boundary or shared interoperability kernel.

The current system is a single-owner vault with global relay contacts and
channels, namespace-scoped API credentials, knowledge-node tasks, local agent
plans, adapters, and proposal/autonomy controls. It does not have first-class
Workspace, Space, Actor, Membership, WorkOrder, AgentRun, context/tool grant,
artifact, delegated-message, outbox, or complete action-envelope records.

The most important mismatch is semantic, not cosmetic: every non-blocked relay
message is immediately inserted as a canonical `Conversation` knowledge node.
ADR 010 requires retained communication to pass through a governed candidate
and promotion boundary before becoming canonical knowledge.

The repository supports incremental migration. Reuse the domain/engine/storage
layering, proposals, policy patterns, namespace-aware authorization,
notifications, and adapter trait. Do not rename relay channels to Spaces,
tasks/plans to WorkOrders/AgentRuns, or provider metadata to Source Bindings.
ADR 011 additionally requires these collaboration records to share stable
identity, schema, event, grant, provenance, and portability contracts with
other Context Nodes.

## Evidence and disposition

| Area                    | Current behavior                                                                                                                                                                         | Evidence                                                                                                                                       | Disposition                                                                                                                                            |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Product boundary        | Active plan says single owner, no shared state, and no shared-state multi-user accounts.                                                                                                 | `DEVELOPMENT_PLAN.md:12-26,53-62,105-116`                                                                                                      | Retain as implementation baseline; ADR 010 supersedes it as target topology.                                                                           |
| Relay contacts          | Contact identity is display name, public key, optional vault address, trust level, and optional autonomy rule.                                                                           | `crates/mv-core/src/model/exchange.rs:517-546,675-713`; `migrations/008_relay.sql:2-12`                                                        | Migrate external identifiers to Actor identity mappings; do not treat trust level as membership.                                                       |
| Relay channels          | Direct/group channel stores member contact UUIDs as a JSON list; it has no owner, Space, membership role, policy, or version.                                                            | `crates/mv-core/src/model/exchange.rs:715-748`; `migrations/008_relay.sql:16-23`                                                               | Preserve as communication projection after adding Space ownership; never promote it to the authority boundary.                                         |
| Relay messages          | Message has direction, contacts, content, status, optional vault node, and free-form metadata; no principal/acting actor/delegation/policy fields.                                       | `crates/mv-core/src/model/exchange.rs:750-803`; `migrations/008_relay.sql:27-46`                                                               | Replace write contract with the ADR 010 action envelope; migrate legacy rows as unattributed historical communication.                                 |
| Knowledge registration  | Relay retains messages without inserting knowledge nodes; promotion is explicit via `promote_relay_message` with provenance; retraction preserves source communication.                 | `crates/mv-engine/src/relay.rs`; `crates/mv-engine/src/engine/relay_ops.rs`                                                                    | Candidate extraction object and promote/retract API/UX remain follow-ups.                                                                              |
| Relay authorization     | Relay routes check coarse read/write roles and select an auth namespace, but do not authorize channel membership.                                                                        | `crates/mv-server/src/rest/relay.rs:360-426,461-517`; `crates/mv-server/src/auth.rs:58-113,391-419`                                            | Reuse auth middleware and default-deny patterns; add Actor and Space membership/resource checks.                                                       |
| Tasks                   | Tasks are `KnowledgeNode(kind=task)` records; status, assignee, dependencies, due date, and other fields are mapped through metadata.                                                    | `frontend/src/lib/api/tasks.ts:5-97`; `crates/mv-server/src/rest.rs:8790-8857`                                                                 | Reuse user-facing task behavior during migration. Add typed ownership, Space, assignment, revision, and provenance before shared use.                  |
| Agent plans             | Plans and steps have goals, actions, status, JSON input/output, and errors, but no requester, agent identity, grants, budgets, artifacts, evidence, or approval chain.                   | `crates/mv-engine/src/planner.rs:25-125`; `migrations/024_plans.sql`                                                                           | Adapt decomposition/execution logic behind new WorkOrder and AgentRun records; do not equate a plan with a run.                                        |
| Watcher agent           | A singleton local process scans recent vault nodes, extracts intents/insights, and writes Chronicle entries.                                                                             | `crates/mv-engine/src/watcher.rs:27-73,94-183`                                                                                                 | Reuse as a Personal Vault automation. If allowed in a Space, register it as an Agent actor with an explicit run and grants.                            |
| Proposals               | Proposal sender is an enum, action/payload are flexible, and approval can execute node CRUD or `relay.reply`.                                                                            | `crates/mv-core/src/model/exchange.rs:6-197`; `crates/mv-server/src/rest/exchange.rs:133-290`; `migrations/004_exchange.sql`                   | Reuse approval, diff, undo, and state-machine concepts. Replace sender enum with actor/principal/delegation references and bind decisions to versions. |
| Autonomy                | Rules support broad scope keys, confidence thresholds, rate caps, allow/block lists, quiet hours, and action logs.                                                                       | `crates/mv-core/src/model/exchange.rs:443-511`; `migrations/007_autonomy.sql`                                                                  | Reuse evaluator concepts. Add Space, actor, grant, operation, resource, budget, and approval semantics.                                                |
| Secret access policy    | ABAC-style policy is keyed by secret and consumer with scopes, TTL, expiry, and HITL flag.                                                                                               | `crates/mv-core/src/model/policy.rs:5-57`; `migrations/012_access_policies.sql`                                                                | Reuse default-deny, TTL, and approval vocabulary for tool grants; keep secret policy distinct from resource authorization.                             |
| Request identity        | Auth context carries subject, role, optional namespace, and optional consumer name. JWTs without a role default to write.                                                                | `crates/mv-server/src/auth.rs:58-113,360-378`                                                                                                  | Add stable Actor identity and membership-derived roles. Remove permissive role default before shared deployment.                                       |
| Adapter abstraction     | Slack, Discord, and email implement a common send/poll/health trait. Registry and configuration are process memory.                                                                      | `crates/mv-engine/src/adapters/mod.rs:27-60,99-178,181-229`; `crates/mv-server/src/rest/adapters.rs:102-152`                                   | Reuse the trait at the edge. Persist non-secret config, move secrets to credential references, and bind adapters to integration Actors and Spaces.     |
| Adapter polling         | Cursor persists, but failed individual ingests are logged and the cycle still advances the cursor. External IDs are metadata only. Generated auto-replies are marked “delivery pending.” | `crates/mv-server/src/adapter_poll.rs:67-137,139-199`; `migrations/022_adapter_poll_state.sql`                                                 | Add inbox/outbox, unique delivery keys, retry/dead-letter state, and receipts before collaborative use. Do not advance past failed ingestion.          |
| Slack                   | Webhook send and one-channel Web API polling exist; configuration requires a webhook and optionally a bot token/channel.                                                                 | `crates/mv-engine/src/adapters/slack.rs:30-47,60-187`                                                                                          | Keep as a thin adapter after durable config, identity mapping, idempotency, pagination, and outbox delivery exist.                                     |
| Discord                 | Webhook send and one-channel bot polling exist; adapter IDs and authors remain edge strings.                                                                                             | `crates/mv-engine/src/adapters/discord.rs:28-45,58-186`                                                                                        | Same as Slack; add safe content limits and self-message/deduplication handling.                                                                        |
| Relay outbound adapters | Relay send and approved `relay.reply` dispatch through the dedicated email path only; generic adapter sending is a separate endpoint.                                                    | `crates/mv-server/src/rest/relay.rs:380-458`; `crates/mv-server/src/rest/exchange.rs:211-285`; `crates/mv-server/src/rest/adapters.rs:195-219` | Route every external effect through one Space-aware outbox and adapter binding.                                                                        |
| Generic email adapter   | Process-memory settings include credentials; raw SMTP uses PLAIN auth without TLS and inbound polling is a no-op.                                                                        | `crates/mv-engine/src/adapters/email.rs:25-55,67-149,190-200`                                                                                  | Do not deploy. Remove or replace after migration to the dedicated credential-backed email implementation.                                              |
| Dedicated email path    | Separate server code uses credential-store SMTP and IMAP ingestion, persists local cursor/thread state, deduplicates message IDs, and can send auto-replies.                             | `crates/mv-server/src/email.rs:94-169,217-280,462-598`                                                                                         | Preferred implementation foundation. Move provider delivery state into the shared inbox/outbox contract.                                               |
| Conversations           | LLM conversations store only conversation and role/content turns; they are not team channels or identity-bearing communication.                                                          | `migrations/023_conversations.sql`                                                                                                             | Keep separate from Space communication.                                                                                                                |
| MCP connectors          | Connector records describe marketplace metadata and capabilities, not agent identity, grants, or runs.                                                                                   | `migrations/027_mcp_connectors.sql`                                                                                                            | Reuse discovery metadata; bind an installed connector to Integration/Agent actors and grants.                                                          |
| Realtime                | Engine changes and watcher discoveries can notify WebSocket clients.                                                                                                                     | `crates/mv-engine/src/watcher.rs:52-55,124-167`; `crates/mv-server/src/rest/exchange.rs:186-207`                                               | Reuse transport pattern; authorize subscriptions by Space and add resumable event positions.                                                           |
| Audit                   | Request audit captures subject, role, namespace, action, resource, result, and latency; it is disabled by default.                                                                       | `crates/mv-server/src/audit.rs:20-24,26-102`                                                                                                   | Preserve as operations telemetry. Add a mandatory durable Trust Ledger with action-envelope fields and tamper evidence.                                |

## Critical gaps

### P0: authority and isolation

There is no database-enforced Workspace/Space tenancy, Actor identity,
Membership lifecycle, resource ownership, or cross-Space authorization check.
Namespaces are optional string filters and cannot be relabeled as a security
boundary.

Unlock evidence:

- foreign-keyed Actor, Workspace, Space, Membership, and resource ownership
  schemas;
- deny-by-default authorization matrix tests for every actor kind;
- cross-Space read, write, subscription, search, and artifact isolation tests;
- migration rules for legacy owner, contacts, channels, namespaces, and API
  consumers.

### P0: attribution and delegated authority

Messages, proposals, plans, and autonomy logs cannot express accountable human,
acting agent, delegation, grants, work order, agent run, policy decision,
approval, and receipt together.

Unlock evidence:

- one versioned action envelope used by REST commands, agent execution,
  proposals, adapter ingestion, and external effects;
- WorkOrder/AgentRun state-machine and retry tests;
- no action can approve or broaden its own grant;
- on-behalf-of messages expose both principal and acting actor.

### P0: reliable effects

Adapter registration is volatile, inbound ingestion has no durable idempotency
constraint, polling can advance after partial failure, and Slack/Discord
outbound is not integrated with relay or proposal delivery.

Unlock evidence:

- persisted adapter bindings with credential references;
- transactional inbox/outbox with unique provider delivery IDs;
- retry, backoff, dead-letter, replay, and receipt behavior;
- crash tests covering each commit/delivery boundary.

### P0: communication is not canonical knowledge

**Status (2026-07-31):** Live auto-promotion removed. Relay send/receive retain
messages in the communication store only. Explicit/policy promotion via
`MindVaultEngine::promote_relay_message` records source message ID, extractor,
actor, evidence, confidence, policy, and approval; retraction removes the
knowledge node/indexes without rewriting source communication
(`cargo test -p mv-engine -- promotion_boundary`).

Remaining follow-ups:

- broader retention/deletion policy for raw communication;
- candidate extraction records (pre-canonical) as a first-class object;
- REST/UX surfaces for promote/retract.

### P1: execution evidence and trust

Plans have step output but not versioned artifacts, budgets, evidence,
verification, or receipts. Optional HTTP audit and Chronicle entries do not
form a Trust Ledger.

Unlock evidence:

- immutable artifact versions and verification state;
- per-run context/tool grant snapshots, usage, and budget enforcement;
- mandatory append-only action records and correlation across work, approval,
  delivery, and report publication.

## Reusable foundations

Extend these components:

- Rust domain/engine/storage boundaries and SQLite migration discipline;
- Personal Vault local-first storage and sealed credential store;
- authenticated REST middleware and namespace checks as an input to the new
  authorization layer;
- proposal approval/rejection, diffs, undo, and autonomy-gate patterns;
- task UX and plan decomposition behind typed shared-work contracts;
- adapter trait and the dedicated SMTP/IMAP implementation;
- WebSocket notification plumbing;
- hybrid search, graph, and knowledge-node projections;
- backup, validation, and request-audit infrastructure.

## Required first-class capabilities

1. Actor and external identity mapping;
2. Workspace, Space, Membership, role, and policy assignment;
3. Space ownership on every shared resource;
4. WorkOrder, AgentRun, Artifact, ContextGrant, and ToolGrant;
5. versioned action envelope and append-only Trust Ledger;
6. communication store, candidate knowledge, and promotion records;
7. durable adapter binding, inbox, outbox, delivery attempt, and receipt;
8. PostgreSQL shared-state repository and migration strategy;
9. Space-authorized realtime subscriptions and projection rebuild;
10. explicit Personal Vault ↔ Space transfer commands and provenance.
11. stable Context Node IDs, Source Bindings, and materialization policies;
12. public object/event schemas and protocol-adapter mappings;
13. semantic export/restore and interoperability conformance fixtures.

## Vertical-slice implementation gate

Do not start with Slack UI or broad schema coverage. The Project Space → Agent
Delegation → Verified Work → Team Report collaboration segment is unlocked
inside the constitution's external-meeting-to-team-report slice when these
contracts are approved:

1. Actor/Space/Membership schema and authorization matrix;
2. WorkOrder/AgentRun/Artifact state machines;
3. context/tool/budget grant semantics;
4. action envelope, Trust Ledger, and idempotency rules;
5. Personal Vault-to-Space transfer and communication-promotion boundaries;
6. PostgreSQL transaction, outbox, and realtime consistency model;
7. threat model, migration/rollback plan, and end-to-end acceptance fixtures.

The first slice should use an internal API or local test integration. Slack,
Discord, and email become validation adapters after canonical work, authority,
and reliable effects are proven.

## Baseline conclusion

ADR 011 and the Interoperability Constitution are the highest architectural
law. ADR 010 defines the Personal Vault and Space ownership domains within it.
The current repository provides strong local primitives, but collaboration
needs shared identity, source authority, grants, events, and provenance records
before feature work.

No feature code should be added until the P0 contracts above are reviewed.
After that review, implementation can proceed incrementally without replacing
the Personal Vault or discarding the existing proposal, policy, task, adapter,
and notification foundations.
