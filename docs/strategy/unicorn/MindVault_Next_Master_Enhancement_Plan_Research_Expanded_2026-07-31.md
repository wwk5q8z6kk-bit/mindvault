# MindVault Next — Research-Expanded Capability, Product, Architecture, Interoperability, Collaboration, Agent, Security, and Execution Plan

**Status:** Research-expanded superseding master plan  
**Date:** July 31, 2026  
**Working product name:** MindVault Next  
**Internal architecture name:** Sovereign Context Fabric  
**Owner:** Hassan El Jesr  

> This document supersedes the existing product direction and every earlier partial enhancement plan. It does not discard the current repository. It reclassifies the repository as a valuable implementation baseline that must be audited, preserved, migrated, and evolved into a broader product.

## How to Use This Document

- **Sections 0–5** define the immutable product direction.
- **Sections 6–18** define canonical objects, data authority, storage, sync, retrieval, and context.
- **Sections 19–28** define interoperability, MCP/A2A, connectors, extensions, and the developer ecosystem.
- **Sections 29–40** define collaboration, work, agents, delegated communication, automation, and differentiating workflows.
- **Sections 41–48** define AI/runtime choices, languages, hardware, UX, security, and the Trust Ledger.
- **Sections 49–53** define boundaries with DevX and how to evolve the existing repository.
- **Sections 54–73** provide the ordered migration and development roadmap.
- **Sections 74–78** define the first end-to-end vertical slices.
- **Sections 79–89** define evaluation, benchmarks, and release gates.
- **Sections 90–101** define ownership/licensing, naming, business model, immediate execution, and the original final product statement.
- **Sections 102–145** are a binding research expansion. They replace any sample-driven reading of the earlier plan with a capability-first, domain-pack, malleable-workspace architecture and a broader proof portfolio.

The constitutional and authority rules take precedence over feature requests. The roadmap is sequential by dependency and exit criterion, not a calendar estimate.

> **Scope rule:** Every example, named integration, reference vertical slice, domain, competitor, and workflow in this plan is a minimum proof of generality—not a boundary on what the platform may support. Sections 102–145 are binding where earlier examples appear narrower.

---

## 0. Locked Decisions

| Area | Decision |
|---|---|
| Ownership | MindVault is Hassan's product and codebase. Future development may be private, proprietary, dual-licensed, open-core, renamed, restructured, or rewritten as strategically appropriate. |
| Existing public release | Preserve and tag the exact public Apache-2.0 baseline before changing repository visibility or licensing. |
| Product scope | Expand from a personal knowledge vault into a sovereign human–AI context, collaboration, coordination, agent, and interoperability platform. |
| Strategic position | Rival Obsidian, Notion, Slack, Linear/Asana/ClickUp, automation systems, AI-memory products, and agent platforms through a unified architecture—not through disconnected clones. |
| Central principle | The product is **logically central and physically decentralized**. It coordinates identity, permissions, context, provenance, agents, work, and receipts while source data may remain distributed. |
| Personal knowledge | Obsidian remains a supported human authoring environment. MindVault becomes the durable context and intelligence substrate and may later provide its own first-class editor. |
| Collaboration | Add private Vaults and governed shared Spaces with channels, DMs, threads, documents, projects, decisions, files, people, agents, automations, and activity. |
| Agents | Agents are first-class but visibly non-human actors. Human ownership remains explicit while execution may be delegated. |
| Interoperability | Any authorized AI, application, device, extension, storage service, or agent must be able to connect through open contracts. |
| Model independence | Support ChatGPT, Claude, Codex, Claude Code, Cursor, local models, future models, and external agents without making one vendor foundational. |
| Language strategy | Rust remains the systems and authority kernel. C++ is added only for proven hot-path compute. Swift/AppKit serve premium Apple UX; Kotlin/Compose serves Android; TypeScript/Svelte remains for portable web/Tauri; Python serves research/evals; Java primarily serves SDK and enterprise compatibility. |
| Local/cloud strategy | Local-first, cloud-optional, self-hostable, and hardware-adaptive. |
| AI writes | No silent direct canonical AI writes. AI proposes; policy or humans approve; every change is attributable, evidence-linked, auditable, and reversible. |
| External actions | Separate context access from action authority. Consequential external actions require bounded policy or explicit approval. |
| DevX boundary | MindVault coordinates context, people, Spaces, work orders, and reports. DevX Runtime remains the engineering execution authority when engineering tasks are delegated. |
| Budget/ambition | Optimize for best-in-class product quality, not the cheapest implementation or minimum feature count. |
| Examples and use cases | Examples are conformance floors, never scope ceilings. New domains should arrive through reusable capabilities and governed Domain Packs rather than leaking domain-specific objects into the core. |

---

# Part I — Product Definition

## 1. New Product Thesis

MindVault Next is not merely:

- a note-taking application;
- a vector database;
- a chatbot over files;
- a Slack clone;
- a Notion clone;
- an automation builder;
- an agent framework;
- or a personal second brain.

It is:

> **A sovereign, interoperable context and coordination fabric where humans, teams, applications, devices, AI models, and agents can share governed context, communicate, perform work, preserve evidence, and act under explicit human authority.**

### Product formula

\[
\boxed{
\text{Private Vaults}
+
\text{Shared Spaces}
+
\text{Evidence-backed Knowledge}
+
\text{Structured Work}
+
\text{Agent Delegation}
+
\text{Universal Interoperability}
+
\text{Trust and Human Authority}
}
\]

### North-star promise

> **Connect anything. Preserve evidence. Understand context. Delegate safely. Act with authority. Retain control.**

### Category

**Sovereign Human–AI Workspace and Context Fabric**

### Differentiation

The product wins not by having the largest checklist, but by providing one consistent system for:

1. identity;
2. source authority;
3. context boundaries;
4. permissions;
5. evidence and provenance;
6. structured work;
7. agent delegation;
8. human approvals;
9. external actions;
10. synchronization and federation;
11. portability;
12. trust receipts.

---

## 2. Competitive Ambition

| Competitor class | Parity target | Our differentiator |
|---|---|---|
| Obsidian / Logseq | Local files, Markdown, backlinks, graph, plugins, offline use, fast capture | Governed AI context, typed knowledge, provenance, permissions, agents, Spaces, federation |
| Notion | Documents, databases, templates, collaboration, sharing, search, workspace organization | Local-first ownership, open schemas, decentralized nodes, model independence, evidence-first AI |
| Slack / Teams | Channels, DMs, threads, files, mentions, notifications, search, integrations, guests | Conversation-to-work, persistent knowledge, agent runs, context grants, action receipts, self-hosting |
| Linear / Asana / ClickUp | Projects, tasks, milestones, dependencies, triage, status, views | Human ownership plus agent delegation, evidence-linked work, work replay, decision history |
| n8n / Zapier / Make | Triggers, actions, schedules, connectors, retries, logs | Context-aware automation, policy simulation, approvals, provenance, local/private execution |
| AI memory products | Recall, personalization, semantic retrieval | Canonical evidence, temporal truth, source authority, structured memory lifecycle, multi-model access |
| Agent platforms | Agent registry, tools, tasks, streaming, artifacts | Human-agent Spaces, private/shared context boundaries, trust plane, any-runtime interoperability |
| Mattermost / self-hosted collaboration | Channels, self-hosting, plugins, enterprise deployment | Personal Vault plus team Spaces, open AI/agent fabric, typed work and knowledge in one system |

### Competitive law

> Do not clone every competitor surface independently. Build one coherent object and policy model and expose multiple views over it.

---

## 3. Product Constitution

These rules are architectural law.

1. **The user owns the data, keys, policies, exports, and model choices.**
2. **Core capture, reading, writing, search, export, and recovery work offline.**
3. **Cloud inference and hosted collaboration are explicit, inspectable choices.**
4. **Original evidence is preserved separately from derived knowledge.**
5. **Every derived claim can point back to supporting evidence.**
6. **Human-authored documents remain readable outside the product.**
7. **AI may propose canonical changes but cannot silently apply them.**
8. **Every consequential action identifies the actor, principal, authority, evidence, and result.**
9. **Private Vault context never enters a shared Space without an explicit grant.**
10. **Agents never automatically inherit all permissions of their human sponsor.**
11. **Messages are events, not automatically canonical knowledge.**
12. **Every important state change is versioned and reversible where technically possible.**
13. **Current truth and historical truth are both preserved.**
14. **Search, vector, and graph indexes are derived and rebuildable.**
15. **External protocols are adapters, not the internal domain model.**
16. **Every first-party capability is exposed through governed APIs and events; no UI-only features.**
17. **No extension receives direct database access.**
18. **No single AI provider, agent framework, vector database, or cloud is irreplaceable.**
19. **Language choices follow responsibility and benchmarks, not fashion.**
20. **The platform is useful as a personal system before requiring a team or hosted service.**
21. **The platform can be self-hosted without losing essential capabilities.**
22. **The moat is quality, context, trust, and execution—not data lock-in.**
23. **The system explains what context was used, why it was used, and what was withheld.**
24. **Automation defaults to shadow mode before bounded autonomy.**
25. **When policy, provenance, identity, or state is uncertain, the system fails closed.**
26. **Examples are minimum conformance proofs, not product boundaries.** A new domain should normally be expressible through existing platform capabilities plus a versioned Domain Pack.
27. **Domain-specific semantics remain outside the universal kernel.** Health, finance, research, legal, industrial, education, career, and other vocabularies enter through profiles, mappings, policies, workflows, and views.
28. **Users may gradually reshape the workspace.** Freeform information can be enriched into typed objects, formulas, views, workflows, agents, and domain applications without surrendering portability or governance.
29. **The platform is judged by unknown-domain extensibility.** It must periodically prove that unplanned domains can be implemented without core schema changes or privilege bypasses.

---

## 4. Product Family and Modules

The final public brand may change. Until naming clearance, use **MindVault Next** for the platform and these module names internally.

| Module | Purpose |
|---|---|
| **Vault** | Private personal knowledge, files, memory, preferences, and private agents |
| **Spaces** | Team, project, client, research, and temporary shared contexts |
| **Knowledge** | Evidence, documents, entities, claims, relationships, timeline, search, and memory |
| **Work** | Projects, tasks, milestones, dependencies, decisions, goals, and status |
| **Agents** | Agent registry, skills, Work Orders, Runs, artifacts, budgets, and reviews |
| **Connect** | Applications, devices, APIs, MCP, A2A, connectors, extensions, and federation |
| **Automate** | Triggers, schedules, policies, workflows, watchers, and recurring reports |
| **Relay** | Internal and external messaging, delegated communication, notifications, email, Slack, Teams, Discord, and other delivery channels |
| **Trust** | Identity, permissions, Context Grants, Tool Grants, approvals, provenance, audit, retention, and action receipts |
| **Admin** | Deployment, billing, model providers, storage, security, compliance, and organization controls |

### Personal and shared structure

```text
Workspace / Deployment
├── Personal Vaults
│   ├── Private documents
│   ├── Personal memory
│   ├── Private agents
│   └── Personal integrations
├── Shared Spaces
│   ├── Channels and DMs
│   ├── Documents and databases
│   ├── Projects and tasks
│   ├── Decisions and files
│   ├── Humans and agents
│   └── Automations
└── Trust and Interoperability Fabric
    ├── Identity
    ├── Permissions
    ├── Context routing
    ├── Source authority
    ├── Provenance
    ├── Connectors
    └── Action receipts
```

---

## 5. Space as the Fundamental Collaboration Primitive

A **Space** is an isolated context, membership, policy, collaboration, work, and agent boundary.

### Space types

| Type | Example |
|---|---|
| Personal | Private life, private career, private research |
| Team | Product, engineering, operations, leadership |
| Project | MindVault redesign, DevX milestone, job-search campaign |
| Client | Restricted external collaboration |
| Research | Sources, hypotheses, experiments, evidence, reviews |
| Incident | Time-limited urgent debugging or crisis coordination |
| Agent Lab | Safe experimentation with agents and permissions |
| Guest | Narrowly shared context for contractors or advisors |
| Automation | Scheduled workflows, watchers, and system operations |

### Every Space can contain

- humans, guests, agents, applications, and connectors;
- channels, DMs, threads, reactions, mentions, and files;
- documents, structured databases, canvases, and collections;
- projects, tasks, milestones, dependencies, and decisions;
- meetings, transcripts, summaries, and follow-ups;
- agent skills, Work Orders, Runs, artifacts, and reports;
- automations, schedules, triggers, and notifications;
- policies, retention, sensitivity, and model-use rules;
- a Space-specific knowledge and memory graph;
- a complete Trust Ledger.

### Important rule

A private Vault may be represented as a private Space, but shared Spaces must never gain implicit access to private Vault content.

---

# Part II — Core Domain Model

## 6. Actors, Principals, and Identities

The platform must distinguish **who benefits from an action** from **what actually performed it**.

| Concept | Definition |
|---|---|
| Principal | Human or organization on whose authority an action occurs |
| Actor | Human, agent, application, connector, service, or device that performs an action |
| Sponsor | Human accountable for an agent or automation |
| Delegate | Human or agent assigned to perform work |
| Reviewer | Human or approved agent responsible for verification |
| Recipient | Human, Space, channel, or external destination receiving an output |

Example:

```text
Principal: Hassan
Actor: Project Reporter Agent v1.4
Action: Post weekly project update
Space: MindVault Engineering
Authority: Internal verified-status-report policy
Evidence: Task state + merged PRs + test receipts
```

### Actor types

- HumanIdentity
- DeviceIdentity
- AgentIdentity
- ApplicationIdentity
- ConnectorIdentity
- ServiceIdentity
- OrganizationIdentity

Every actor receives its own credentials, scopes, revocation state, audit history, and risk classification.

---

## 7. Canonical Objects

| Object | Purpose |
|---|---|
| Workspace | Administrative and deployment boundary |
| Vault | Private personal context boundary |
| Space | Shared or private collaboration and policy boundary |
| Membership | Actor role and permissions within a Space |
| SourceArtifact | Immutable original file, message, page, recording, image, transcript, or payload |
| Document | Human-readable logical document |
| Block | Addressable paragraph, table, code block, image, quote, or embedded object |
| Entity | Person, organization, project, product, concept, place, role, or other identity |
| Claim | Evidence-linked statement with confidence and temporal validity |
| Relationship | Typed connection between objects |
| Event | Something that happened at a time |
| Message | Communication event from a human, agent, application, or connector |
| Channel | Ordered communication stream within a Space |
| Thread | Focused discussion attached to a message or object |
| Meeting | Conversation event with participants, agenda, evidence, decisions, and actions |
| Project | Goal-oriented body of work |
| Task | Action with owner, delegate, status, criteria, and dependencies |
| Milestone | Significant target or checkpoint |
| Decision | Choice, rationale, alternatives, evidence, owner, and outcome |
| Procedure | Repeatable, versioned process or playbook |
| Preference | Scoped user or organization preference with history |
| MemoryRecord | Governed durable context intended for future retrieval |
| WorkOrder | Structured assignment to a human or agent |
| AgentRun | One execution of a Work Order |
| Artifact | Report, code, file, dataset, design, diff, or other output |
| ContextGrant | Exact context an actor may access, for what purpose and duration |
| ToolGrant | Exact actions an actor may perform, on which resources and for how long |
| ActionProposal | Proposed state change or external action |
| Approval | Human or policy decision on a proposal |
| DelegatedMessage | Message sent under a human or organization’s authority by another actor |
| Automation | Trigger, conditions, actions, authority, and lifecycle |
| Notification | Attention event, routing, delivery, acknowledgment, and escalation state |
| Connector | External source or action integration |
| Extension | Installable agent, connector, view, transformer, automation, model, or policy pack |
| AuditEvent | Immutable record of access, transformation, decision, or action |
| ActionReceipt | Final record of authority, actor, context, action, evidence, and result |
| ContextPackage | Permission-filtered context compiled for an AI or application |
| ContextCapsule | Portable, signed, revocable context package for another node or party |

---

## 8. Required Metadata and Temporal Model

Every canonical object should support:

| Field | Purpose |
|---|---|
| UUIDv7 | Stable, sortable internal identity |
| Global resource URI | Cross-node identity |
| Content hash | Exact identity, deduplication, integrity |
| Schema version | Safe evolution and migrations |
| Workspace / Vault / Space | Context boundary |
| Actor / creator | Accountability |
| Principal | On-behalf-of authority |
| Source connector | Provenance and refresh |
| External source ID | Deduplication and synchronization |
| `created_at` | Local record creation |
| `observed_at` | When the platform learned it |
| `valid_from` | When the fact became true |
| `valid_until` | When it stopped being true |
| `superseded_by` | Change lineage |
| Version / revision | Rollback and concurrency |
| Confidence | Uncertainty of derived information |
| Verification state | Candidate, verified, disputed, rejected, superseded |
| Sensitivity | Public, internal, private, confidential, highly restricted |
| Retention class | Lifecycle and deletion policy |
| Evidence references | Supporting source material |
| Signature / checksum | Integrity and trust boundary verification |

### Bitemporal requirement

The platform must distinguish:

- **when something was true in the world**; and
- **when the platform learned or recorded it**.

This enables questions such as:

- What did we believe on March 1?
- What was actually true on March 1?
- When did the team learn that the deadline changed?
- Which decision relied on now-superseded information?

---

## 9. Memory Taxonomy and Lifecycle

### Memory types

| Type | Example |
|---|---|
| Semantic | Stable fact or concept |
| Episodic | What happened during a meeting or project event |
| Procedural | How to perform a workflow |
| Preference | User or team preference |
| Relationship | Person, organization, project, or dependency relation |
| Project | Architecture, status, decisions, risks, and artifacts |
| Temporary | Short-lived task context |
| Sensitive | Health, finance, identity, legal, or private correspondence |
| Derived | AI-inferred statement requiring evidence |
| Contradiction | Incompatible active claims |
| Superseded | Previously valid information replaced by newer truth |

### Lifecycle

```text
Captured
  → Candidate
  → Normalized
  → Evidence-linked
  → Proposed
  → Verified / Policy-approved
  → Active
  → Stale / Disputed / Superseded
  → Archived / Deleted
```

No imported message, transcript, or model output may jump directly from capture to active canonical memory.

---

# Part III — System Architecture and Data Authority

## 10. Target Architectural Shape

MindVault Next must be **logically central and physically decentralized**.

- Logically central: one coherent identity, context, policy, provenance, source-authority, work, agent, and receipt model.
- Physically decentralized: data and execution may remain on personal devices, team servers, external applications, storage providers, AI services, and agent runtimes.

### Architectural planes

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ EXPERIENCE PLANE                                                           │
│ Native Apple │ Android │ Web │ Tauri Desktop │ Obsidian │ CLI │ Extensions │
└──────────────────────────────────┬─────────────────────────────────────────┘
                                   │ commands, queries, subscriptions
┌──────────────────────────────────▼─────────────────────────────────────────┐
│ TRUST AND AUTHORITY PLANE                                                  │
│ Identity │ Membership │ Policy │ Context Grants │ Tool Grants │ Approvals  │
│ Data Classification │ Egress Control │ Audit │ Action Receipts             │
└──────────────────────────────────┬─────────────────────────────────────────┘
                                   │ governed operations
┌──────────────────────────────────▼─────────────────────────────────────────┐
│ CONTEXT AND KNOWLEDGE PLANE                                                │
│ Evidence │ Documents │ Entities │ Claims │ Relations │ Memory │ Retrieval  │
│ Temporal Truth │ Contradictions │ Context Compiler │ Context Capsules      │
└────────────────────────┬─────────────────────────┬─────────────────────────┘
                         │                         │
┌────────────────────────▼──────────────┐  ┌───────▼─────────────────────────┐
│ COLLABORATION AND WORK PLANE          │  │ AGENT AND AUTOMATION PLANE     │
│ Spaces │ Channels │ Threads │ Docs    │  │ Agent Registry │ Work Orders   │
│ Projects │ Tasks │ Decisions │ Files  │  │ Runs │ Schedules │ Watchers    │
│ Meetings │ Notifications              │  │ Artifacts │ Verification       │
└────────────────────────┬──────────────┘  └───────┬─────────────────────────┘
                         │                         │
┌────────────────────────▼─────────────────────────▼─────────────────────────┐
│ INTEROPERABILITY PLANE                                                     │
│ MCP │ A2A │ OpenAPI │ AsyncAPI │ Events │ Connectors │ Extensions │ Relay │
│ Federation │ SDKs │ Schema Registry │ Source Authority Registry           │
└──────────────────────────────────┬─────────────────────────────────────────┘
                                   │
┌──────────────────────────────────▼─────────────────────────────────────────┐
│ DATA AND EXECUTION PLANE                                                   │
│ Markdown │ SQLite │ PostgreSQL │ Content-addressed Blobs │ Object Storage  │
│ Event Log │ Search/Vector/Graph Projections │ Local/Cloud Model Backends   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Architectural rule

Every plane may evolve independently, but no plane may bypass the Trust and Authority Plane when accessing context or causing side effects.

---

## 11. Node Architecture

The decentralized system is composed of independently deployable **Context Nodes**.

| Node type | Role |
|---|---|
| Personal Node | Private Vault, personal memory, local files, local models, private agents |
| Device Node | Laptop, phone, tablet, wearable, recorder, home server, or workstation |
| Shared Space Node | Team/project collaboration, shared knowledge, work, messages, and agents |
| Organization Node | Identity, policy, compliance, shared services, and administrative controls |
| Application Node | Obsidian, Slack, GitHub, Granola, interview tool, calendar, CRM, or other source |
| Agent Node | Independent agent or agent platform that accepts Work Orders and returns artifacts |
| Execution Node | DevX Runtime, Claude Code, Codex, container, cluster, or remote worker |
| Storage Node | Local disk, NAS, cloud drive, S3-compatible object store, database, or archive |
| Relay Node | Encrypted event, message, notification, or synchronization transport |
| Index Node | Authorized search or semantic projection without necessarily owning source data |

### Node identity

Every node must publish or expose:

- a stable node identity;
- supported protocol versions;
- capabilities;
- authentication methods;
- schemas;
- trust class;
- data residency and retention information;
- health and availability;
- public keys or signing metadata;
- federation and revocation endpoints where applicable.

### Node trust classes

| Trust class | Meaning |
|---|---|
| Owner-controlled | Operated on the user’s device or infrastructure |
| Organization-controlled | Operated by the user’s organization |
| Verified third party | Identity and implementation reviewed |
| Community | Publicly available but not independently verified |
| Ephemeral | Temporary execution or relay node |
| Untrusted | May be queried or used only through narrow, sanitized boundaries |
| Blocked | Revoked, compromised, malicious, or incompatible |

---

## 12. Personal and Shared Deployment Model

The platform must preserve personal local-first ownership without forcing shared collaboration into a single-device database.

### Personal node

```text
Personal Device
├── Markdown workspace / Obsidian vaults
├── SQLite canonical private state
├── Content-addressed encrypted artifacts
├── Local event and audit log
├── Derived full-text, vector, and graph indexes
├── Device identity and local keys
├── Local model and connector runtimes
└── Encrypted cache of authorized shared Spaces
```

### Shared workspace server

```text
Shared Workspace Server
├── PostgreSQL canonical shared state
├── Append-only command/event ledger
├── Encrypted object storage
├── Realtime gateway
├── Identity and policy services
├── Agent control plane
├── Notification and Relay services
├── Connector workers
├── Search/index workers
└── Organization administration
```

### Why the stores differ

| Personal context | Shared context |
|---|---|
| Single owner is authoritative | Multiple authorized actors modify state |
| SQLite and files are appropriate | Transactional shared database is appropriate |
| Local keys and local-only data | Workspace/Space key hierarchy and membership |
| Full offline operation | Realtime plus offline synchronization |
| User chooses what is shared | Space policy determines shared access |
| Private agent memory | Shared project and team context |

### Non-negotiable boundary

Shared services never receive unrestricted access to a Personal Node. Personal context crosses the boundary only through an explicit `ContextGrant`, accepted canonical import, or signed `ContextCapsule`.

---

## 13. Canonical Storage and Derived Projections

### Canonical data

| Data class | Canonical form |
|---|---|
| Human-authored personal documents | Markdown or another open, human-readable document format |
| Structured personal context | SQLite |
| Shared collaboration/work state | PostgreSQL or equivalent validated relational store |
| Original artifacts | Content-addressed encrypted blob/object storage |
| Immutable action history | Append-only command/event/audit ledger |
| Secrets | OS keychain, secure enclave, HSM, or dedicated secret broker |
| Collaborative documents | Versioned operation/CRDT representation with open exports |
| Public schemas and manifests | Version-controlled JSON Schema / protocol definitions |

### Derived data

| Projection | Rule |
|---|---|
| Full-text index | Rebuildable from canonical documents and records |
| Vector index | Rebuildable and model-versioned |
| Graph projection | Derived from canonical typed relationships |
| Reranker cache | Disposable |
| Summaries | Derived artifacts with provenance and model/version metadata |
| Entity extraction | Candidate knowledge until verified or policy-approved |
| Analytics | Recomputable from canonical events where feasible |

### Storage invariants

1. No connector writes directly to a search, vector, or graph index.
2. No index is the sole repository of user knowledge.
3. Every source artifact has an exact identity and integrity hash.
4. Every ingest path is idempotent.
5. Every schema migration supports dry-run, verification, and rollback or export-first recovery.
6. Database corruption cannot silently become missing context.
7. Backup and synchronization are separate systems.
8. A user can export human-readable content and machine-readable structured state without an active subscription.
9. Model-dependent derived records retain the model, prompt/recipe, version, timestamp, and supporting sources.
10. Deleting a derived index never deletes canonical user data.

---

## 14. Source Authority Registry

Interoperability fails when several systems believe they own the same state. Every external or internal object must declare its authoritative source.

### Example authority mapping

| Object | Authoritative source | MindVault responsibility |
|---|---|---|
| GitHub pull-request status | GitHub | Reference, index, relate, display, and report |
| Meeting recording | Meeting application or local device | Reference or replica according to policy; derive approved outcomes |
| Approved meeting decision | MindVault Space | Canonical decision and provenance |
| Calendar event time | Calendar provider unless imported canonically | Mirror, propose, and record receipts |
| Personal note | User-selected Markdown workspace | Index, enrich, synchronize, and preserve identity |
| Team document | Shared Space | Canonical collaborative state and open export |
| Slack message | Slack | Reference or mirror under policy |
| MindVault channel message | MindVault Space | Canonical platform communication event |
| Agent Run | MindVault Agent Control Plane | Canonical task/run state, even when execution is external |
| Source code | Git repository | Reference, contextualize, and connect to work/evidence |

### Source binding record

Every bound object should record:

```text
internal_resource_id
external_system_id
external_account_id
external_object_id
canonical_authority
sync_direction
last_seen_version
last_sync_cursor
content_hash
materialization_mode
conflict_policy
deletion_policy
sensitivity
retention
provenance
```

### Materialization modes

| Mode | Meaning |
|---|---|
| Reference only | Keep identity, location, permissions, and metadata |
| Metadata mirror | Store title, participants, timestamps, status, and source pointer |
| Search projection | Index authorized content without claiming canonical ownership |
| Cached excerpt | Keep selected passages required for context or offline work |
| Full encrypted replica | Preserve a governed local or organizational copy |
| Canonical import | Transfer ownership into the platform |
| Derived knowledge | Store approved claims, decisions, tasks, relationships, or summaries |

The user or organization must be able to inspect and change the materialization policy for each connector, Space, or data class.

---

## 15. Command, Query, and Event Architecture

No first-party capability may exist only inside an official UI.

### Completion contract

A capability is complete only when it has:

1. a typed domain object or explicit stateless contract;
2. a command API for mutations;
3. a query API for reads;
4. emitted domain events;
5. permission and policy definitions;
6. provenance behavior;
7. audit and receipt behavior;
8. export/import behavior where applicable;
9. extension access where safe;
10. automated conformance tests.

### Command path

```text
Actor request
  → Authenticate actor
  → Resolve principal
  → Validate command schema
  → Load ContextGrant / ToolGrant
  → Evaluate policy
  → Verify expected resource version
  → Execute transactional state transition
  → Append event and audit record
  → Produce ActionReceipt
  → Update derived projections asynchronously
  → Notify subscribers
```

### Query path

```text
Query
  → Authenticate actor
  → Resolve context boundary
  → Apply authorization and sensitivity filters
  → Query canonical state and permitted projections
  → Attach provenance, freshness, and confidence
  → Redact or withhold unauthorized content
  → Return result plus explanation metadata
```

### Event guarantees

- globally unique event identity;
- stable source identity;
- schema version;
- causal parent and workflow correlation ID;
- actor and principal;
- Space/Vault boundary;
- sensitivity and retention class;
- idempotency behavior;
- provenance references;
- signature when crossing trust boundaries;
- at-least-once delivery with consumer deduplication unless a stronger guarantee is proven.

---

## 16. Synchronization, Collaboration, and Federation

### Synchronization layers

| Layer | Mechanism |
|---|---|
| Single-object state | Optimistic versioning and explicit conflict detection |
| Personal multi-device state | Signed operation log, version vectors, snapshots, deterministic merge policies |
| Collaborative text/docs | CRDT or operation-based model only where simultaneous editing requires it |
| Binary artifacts | Content-addressed, chunked, resumable transfer |
| Shared relational state | Transactional server authority plus offline command queue |
| Cross-node federation | Signed events, capability negotiation, cursors, and selective replication |

### Conflict policy

Every object type must declare one of:

- last-writer-wins only for low-risk ephemeral metadata;
- field-level deterministic merge;
- human resolution;
- authoritative-source-wins;
- append-only coexistence;
- CRDT merge;
- reject and retry.

Silent loss of a human edit is prohibited.

### Federation stages

1. Local personal node.
2. Multiple owner-controlled devices.
3. Personal node plus shared Space server.
4. Multiple self-hosted or managed organization nodes.
5. Selective cross-organization Spaces and Context Capsules.
6. Optional open federation and bridges.

### E2EE honesty rule

A server-side agent cannot read an end-to-end encrypted Space unless it is itself an explicitly authorized cryptographic member or executes on a trusted member device. The product must present this tradeoff clearly rather than claiming both server invisibility and unrestricted server-side intelligence.

### Matrix and MLS position

- Matrix may later serve as a federation or messaging bridge.
- MLS may later support efficient group key establishment and membership changes.
- Neither becomes the internal domain model by default.
- Adoption requires threat modeling, interoperability testing, migration design, and proof that the added complexity is warranted.

---

## 17. Backup, Recovery, and Portability

### Backup is not sync

| Synchronization | Backup |
|---|---|
| Propagates current state | Preserves historical recovery points |
| Can propagate accidental deletion | Retains older snapshots |
| Optimized for continuity | Optimized for disaster recovery |
| Device/Space aware | Storage and restoration aware |
| Near-real-time | Scheduled, verified, and versioned |

### Required recovery capabilities

- encrypted local snapshot;
- encrypted user-selected cloud/object-store backup;
- offline archive;
- point-in-time restore;
- selective Vault/Space/object restore;
- restore into an isolated verification environment;
- schema migration during restore;
- lost-device revocation;
- recovery-key rotation;
- full human-readable export;
- full machine-readable export;
- connector/source-binding export;
- audit and receipt export.

### Release gate

No release is considered safe until backup creation, corruption detection, full restore, selective restore, and post-restore integrity verification are automated and tested.

---

## 18. Retrieval and Context Compilation

Vector search alone is not memory. Retrieval must combine exact, structured, lexical, semantic, graph, temporal, and policy-aware methods.

### Retrieval pipeline

```text
Question / task
  → Intent, entity, and scope analysis
  → Vault/Space and permission resolution
  → Sensitivity and egress filter
  → Exact ID/hash/path/name lookup
  → Structured relational query
  → Full-text candidate generation
  → Vector candidate generation
  → Graph expansion
  → Temporal validity and freshness filtering
  → Candidate fusion
  → Reranking
  → Contradiction and source-authority checks
  → Evidence-coverage validation
  → Context compilation
```

### Retrieval modes

| Mode | Best use |
|---|---|
| Exact | IDs, file paths, symbols, names, hashes, dates, known facts |
| Structured | Applications, tasks, meetings, decisions, ownership, status |
| Full text | Quotes, names, code, rare tokens, precise wording |
| Semantic | Paraphrases and conceptually related material |
| Graph | People, projects, dependencies, multi-hop relationships |
| Temporal | Current truth, history, superseded facts, time-machine queries |
| Procedural | Reconstructing processes, decisions, or prior work sequences |
| Hybrid | Most natural-language questions and agent tasks |

### ContextPackage

A `ContextPackage` delivered to an AI or application should contain:

- task/question;
- authorized entities, claims, documents, and excerpts;
- exact evidence references;
- freshness and temporal validity;
- active contradictions and uncertainty;
- data sensitivity and handling restrictions;
- token/size budget;
- requesting actor and principal;
- allowed tools and write scopes;
- omitted-context explanation;
- cache identity;
- expiration;
- provenance and policy decision IDs.

### ContextCapsule

A `ContextCapsule` is a portable, signed, revocable context package for another node, person, application, or agent.

It may be:

- query-only;
- streamed;
- copied;
- encrypted to the recipient;
- one-time-use;
- time-limited;
- local-only;
- revocable;
- non-redistributable;
- restricted to specified models or purposes.

Example capsules:

| Recipient | Capsule contents |
|---|---|
| Interview coach | Resume, target role, selected stories, no private Vault |
| Coding agent | Repository context, task, decisions, allowed files/tools |
| Meeting assistant | Agenda, participants, prior decisions, open actions |
| Contractor | Selected project docs/tasks with expiration |
| Research agent | Question, approved sources, citation and output requirements |
| Team reporter | Verified project state and approved recipient list |

---
# Part IV — Interoperability, Connectors, and the Extension Ecosystem

## 19. Interoperability Constitution

Interoperability is not a secondary integration feature. It is a top-level product law.

1. Any authorized AI, application, device, extension, service, or agent may connect through a documented contract.
2. No connection bypasses identity, policy, context boundaries, source authority, provenance, or audit.
3. Vendor-specific data models remain at adapters; the core uses versioned public domain schemas.
4. External protocols are translated into the internal domain model and can be replaced or versioned independently.
5. Context access and action authority are always separate.
6. Every external object declares its source of truth and synchronization policy.
7. Every incoming mutation is idempotent or rejected.
8. Every outgoing action creates an ActionReceipt.
9. Third-party extensions never receive direct database or unrestricted filesystem access.
10. Users may install from official, enterprise, self-hosted, community, local, or direct sources.
11. No mandatory central marketplace controls the user’s ability to extend a self-owned node.
12. All public data formats and APIs have conformance tests.
13. Breaking schema/protocol changes require migration and compatibility windows.
14. Unknown or unsupported capabilities fail closed.
15. The user can disconnect a source without corrupting canonical data.
16. A connector may be removed without erasing legitimately imported canonical records unless the user explicitly requests deletion.
17. Interoperability must include both reading and governed action, not only bulk import.
18. Every integration discloses data classes, retention, external destinations, permissions, and model use.
19. The platform provides open exits: data, receipts, schemas, and source bindings remain exportable.
20. Interoperability quality is measured through a public conformance suite, not marketing claims.

---

## 20. Protocol Strategy

The protocols below are interoperability tools, not the complete universe of future adapters. Their presence in this table is a floor, and any later standard must pass the same authority, source, provenance, versioning, and conformance rules.


Use open standards by responsibility while keeping the internal model independent.

| Need | Primary contract | Platform role |
|---|---|---|
| AI access to context and tools | MCP | Server and client/host |
| Agent discovery and delegated work | A2A | Server and client/adapter |
| Synchronous application integration | OpenAPI/HTTP | Public command/query API |
| Event-driven integration | AsyncAPI | Subscriptions, streams, and event documentation |
| Event envelope | CloudEvents-compatible envelope | Cross-system event identity and transport |
| Public object validation | JSON Schema | Versioned domain and extension schemas |
| Efficient typed internal RPC | Protocol Buffers where justified | Internal/native SDK contracts |
| Human identity | OpenID Connect and passkeys | Federated login and strong authentication |
| Delegated authorization | OAuth 2.x profiles | Revocable app/agent grants |
| Enterprise provisioning | SCIM | Users, groups, lifecycle |
| Provenance exchange | W3C PROV-compatible mapping | Cross-system derivation and attribution |
| Observability | OpenTelemetry | Traces, logs, metrics, correlation |
| Communication federation | Matrix bridge, evaluated later | Optional cross-server messaging |
| Group E2EE | MLS, evaluated later | Optional key-management foundation |
| User-controlled external data | Solid-compatible adapter, evaluated later | Optional pod/data-store interoperability |
| Documents | Markdown, HTML, PDF, JSON Canvas, open exports | Human portability |
| Calendar | iCalendar / CalDAV | Calendar interoperability |
| Contacts | vCard / CardDAV | Contact portability |
| Email | JMAP or IMAP/SMTP adapters | Ingestion and governed actions |
| Files | Local filesystem, WebDAV, S3-compatible interfaces | Storage portability |
| Code | Git | Repository and change interoperability |

### Protocol-version policy

- Maintain protocol adapters behind owned interfaces.
- Negotiate versions explicitly.
- Support the latest stable protocol by default.
- Treat release candidates and drafts as opt-in experimental adapters.
- Preserve recorded protocol version on every interaction receipt.
- Maintain compatibility fixtures and contract tests for each supported version.

---

## 21. Bidirectional MCP Architecture

MindVault must operate both as an **MCP server** and an **MCP client/host**.

### MCP server role

Authorized AI clients may:

- discover permitted Vaults and Spaces;
- search context;
- retrieve evidence-linked excerpts;
- inspect entities, relations, decisions, tasks, and project state;
- request compiled ContextPackages;
- submit artifacts;
- create proposals;
- request governed actions;
- report progress;
- ask for human input;
- subscribe to approved changes;
- inspect run/action receipts.

Potential clients include ChatGPT, Claude, Claude Code, Codex, Cursor, DevX, local model applications, enterprise assistants, and future AI systems.

### MCP client/host role

MindVault agents and automations may connect to external MCP servers such as:

- GitHub;
- filesystem and repository tools;
- browsers;
- calendars;
- databases;
- CRM systems;
- design applications;
- research services;
- company-specific internal tools;
- local devices and models.

### MCP policy router

```text
AI Client or MindVault Agent
  → MCP session identity
  → Connection profile
  → ContextGrant / ToolGrant
  → Resource and tool filtering
  → Parameter validation
  → Secret brokering
  → Action policy and approval
  → Call execution
  → Result sanitization
  → Provenance and ActionReceipt
```

### MCP connection profiles

| Profile | Typical permission |
|---|---|
| Personal Assistant | Broad private read; proposal-only writes |
| Coding Agent | Named project/repository context and engineering tools only |
| Meeting Agent | Meetings, participants, calendar, projects, and approved actions |
| Career Agent | Resume evidence, companies, applications, contacts, selected communications |
| Team Agent | One or more shared Spaces |
| External Advisor | Explicit ContextCapsule only |
| Untrusted Agent | Search metadata and user-approved excerpts |
| Local Model | Local-only context according to device policy |
| Auditor | Read-only receipts, evidence, and policy traces |

### MCP write rule

MCP tools that mutate canonical state or cause external effects must map to the same internal command/proposal path used by first-party clients. There is no privileged MCP bypass.

---

## 22. A2A and External Agent Interoperability

A2A should enable discovery, delegation, task state, streaming updates, and artifacts between independent agents while remaining an adapter over MindVault’s internal `WorkOrder` and `AgentRun` model.

### Mapping

| MindVault object | A2A concept |
|---|---|
| AgentIdentity/Profile | Agent Card |
| Skill | Agent Skill |
| WorkOrder request | Initiating message/task request |
| AgentRun | Task |
| Run status | Task status |
| Progress update | Status event/message |
| Report or file | Artifact |
| Human input required | Input-required state |
| Authentication needed | Auth-required state |
| Cancellation | Task cancellation |
| Runtime/version | Agent metadata and internal manifest |

### A2A boundary rules

1. External Agent Cards are discovered and imported as candidate agent profiles.
2. An administrator or owner approves trust, permissions, and allowed Spaces before use.
3. The external protocol does not define internal canonical permissions.
4. Every external task maps to a local WorkOrder and policy context.
5. Every incoming artifact is untrusted until scanned, validated, and linked to the originating run.
6. External agents receive short-lived, run-specific credentials.
7. Streaming status is rate-limited, normalized, and summarized for human attention.
8. Cancellation must revoke local grants and attempt remote cancellation.
9. External-agent errors never silently become successful local task state.
10. The platform records the A2A protocol version and remote agent version in the run receipt.

---

## 23. Universal Connector Gateway

External applications and devices connect through one governed gateway rather than bespoke direct database integrations.

### Supported connection methods

| Method | Best for |
|---|---|
| OAuth API connector | Applications with official APIs |
| API key/service-account connector | User-managed or enterprise systems |
| Webhook receiver | Applications that push events |
| Polling connector | APIs without event delivery |
| Local filesystem watcher | Obsidian, exports, recorder folders, local apps |
| Browser extension | Web applications without sufficient APIs |
| Desktop/mobile share extension | One-step capture from native applications |
| Email ingestion address | Tools that send transcripts, reports, receipts, or exports by email |
| Calendar-linked capture | Meeting and scheduling applications |
| Import dropbox | JSON, Markdown, PDF, audio, video, images, CSV, archives |
| Local socket / stdio | Local tools and desktop extensions |
| MCP | AI-accessible resources and tools |
| A2A | Independent agent services |
| WebDAV / S3 | File and object stores |
| Generic REST/OpenAPI connector | User-configured or internal APIs |
| Generic event connector | CloudEvents/webhook-compatible sources |
| Embedded SDK | Deep first-party or enterprise integrations |

### Example: meeting or interview capture device

A Scribe-like, meeting, or interview-note application can integrate through any supported path:

1. official API;
2. webhook;
3. emailed transcript or summary;
4. exported Markdown/PDF/JSON;
5. watched local folder;
6. browser extension;
7. mobile share sheet;
8. generic ingestion endpoint;
9. local companion application;
10. MCP resource/tool provider.

The platform should normalize all of these into the same `Meeting`, `Transcript`, `SourceArtifact`, `Participant`, `DecisionCandidate`, and `TaskCandidate` objects.

---

## 24. Connector Contract

Every connector must implement or explicitly decline the following capabilities.

| Capability | Meaning |
|---|---|
| `discover` | Enumerate available external objects |
| `read` | Retrieve one external object |
| `search` | Query the external source |
| `subscribe` | Receive source changes |
| `sync` | Incrementally reconcile external and local state |
| `import` | Normalize data into platform objects |
| `export` | Send platform data to a target |
| `append` | Add without replacing existing content |
| `update` | Modify an external object |
| `execute` | Perform an external action |
| `notify` | Deliver a message or alert |
| `identity.resolve` | Match external people, organizations, devices, or agents |
| `permissions.inspect` | Report source-level access and ownership |
| `attachments.read` | Retrieve linked files |
| `health` | Report availability and degraded behavior |
| `delete` | Delete only when explicitly authorized and supported |
| `revoke` | Remove credentials and subscriptions |
| `export.receipts` | Return upstream action/delivery receipts when available |

### Connector quality requirements

- stable external IDs;
- incremental cursor/checkpoint;
- idempotency;
- deduplication;
- bounded retries and backoff;
- rate-limit compliance;
- original payload or verifiable source reference;
- schema validation;
- data-class mapping;
- sensitivity classification;
- retention declaration;
- deletion semantics;
- conflict policy;
- complete audit;
- health monitoring;
- test fixtures;
- uninstall behavior;
- migration across connector versions.

---

## 25. Connector and Extension Manifest v2

```yaml
manifest_version: 2
id: com.example.meeting-connector
name: Example Meeting Connector
version: 1.2.0
publisher: Example Inc.
kind: connector
trust_request: community

runtime:
  type: wasm
  entrypoint: connector.wasm
  minimum_platform_version: 2.0.0

protocols:
  - oauth2
  - webhook
  - rest

capabilities:
  - meetings.discover
  - meetings.read
  - transcripts.read
  - participants.resolve
  - events.subscribe
  - artifacts.import

permissions:
  context:
    read:
      - selected_spaces
    propose:
      - meetings
      - people
      - tasks
  network:
    allow:
      - api.example.com
  secrets:
    - oauth_token
  filesystem: none
  external_actions: none

schemas:
  produces:
    - context.meeting.v1
    - context.transcript.v1
  consumes:
    - context.space.v1

classification:
  data_classes:
    - meeting_metadata
    - transcript
    - participant_identifiers
  highest_sensitivity: confidential

retention:
  raw_payload: 7d
  normalized_records: governed_by_space
  logs: 30d

endpoints:
  webhook: /events
  sync: /sync
  health: /health

supply_chain:
  source_repository: declared
  license: declared
  sbom: included
  signature: required
```

### Manifest-enforced controls

- installation permissions;
- network domain allowlist;
- secret access;
- Vault/Space scopes;
- supported object schemas;
- data classes and sensitivity;
- raw and normalized retention;
- UI contributions;
- agent/model access;
- external-action rights;
- update channel;
- signature and SBOM;
- compatibility and migrations.

---

## 26. Extension Types

| Extension type | Adds |
|---|---|
| Connector | External application, service, device, source, or action target |
| Agent | Task-performing AI or software actor |
| Model provider | Local or cloud inference backend |
| Transformer | Parser, extractor, OCR, transcription, classifier, summarizer, enrichment |
| Indexer | Search, vector, graph, or reranking backend |
| Storage provider | Local, self-hosted, peer, or cloud storage |
| Automation | Trigger, schedule, workflow, or watcher package |
| View | Domain-specific interface, card, dashboard, or inspector |
| Command | User-invoked operation |
| Policy pack | Organization/domain-specific governance rules |
| Importer | Legacy or vendor migration |
| Exporter | Portable output or external target |
| Notification channel | Push, email, chat, SMS, desktop, or custom delivery |
| Identity provider | Authentication, directory, and provisioning integration |
| Domain module | Career, research, healthcare, engineering, education, finance, or other vertical |
| Protocol adapter | New external interoperability protocol |

### UI extension tiers

| Tier | Capability |
|---|---|
| Declarative | Cards, tables, forms, commands, settings, and contextual actions |
| Sandboxed web | Isolated iframe/webview with strict CSP and message API |
| Sandboxed native/WASM | Capability-limited extension runtime |
| Trusted native | Signed, reviewed first-party or enterprise extension |

No UI extension may call the canonical database directly. All reads and actions go through the same query/command/policy interfaces as official clients.

---

## 27. Decentralized Registry and Trust Model

A registry aids discovery but must not become a mandatory gatekeeper for owner-controlled nodes.

### Installation sources

- official registry;
- verified community registry;
- enterprise/private registry;
- self-hosted registry;
- Git repository;
- signed release URL;
- local package;
- development directory;
- direct MCP endpoint;
- direct A2A Agent Card;
- organization policy bundle.

### Trust classes

| Class | Meaning |
|---|---|
| Official | Maintained by the product team |
| Verified | Identity, source, security, and conformance reviewed |
| Community | Signed and public, but not formally audited |
| Enterprise | Approved by an organization administrator |
| Local | Installed only on owner-controlled nodes |
| Development | Explicitly insecure/testing-only |
| Unverified | Narrow sandbox and prominent warning required |
| Revoked | Blocked due to compromise, malicious behavior, or incompatibility |

### Registry metadata

- publisher identity;
- release history;
- source availability;
- declared license;
- manifest and permissions;
- SBOM;
- security advisories;
- conformance results;
- supported schemas/protocols;
- data classes and retention;
- telemetry behavior;
- reviews and issue links;
- update channel;
- revocation status.

---

## 28. SDK and Developer Platform

### SDK priorities

1. Rust
2. TypeScript
3. Python
4. Swift
5. Kotlin
6. Java/JVM
7. C++
8. Generated OpenAPI clients for additional languages

### Developer tooling

- connector generator;
- agent generator;
- extension manifest editor;
- local sandbox;
- fake/test workspace;
- webhook inspector;
- event replay;
- schema validator;
- permission simulator;
- policy debugger;
- context inspector;
- source-authority debugger;
- compatibility checker;
- conformance runner;
- signing and packaging tool;
- SBOM generator;
- extension profiler;
- trace and receipt viewer;
- mocked third-party services;
- migration harness.

### Open interoperability layer

Even if the main product becomes proprietary, publish under a permissive license or open specification where strategically appropriate:

- public object schemas;
- event schemas;
- connector manifest;
- ContextCapsule format;
- ActionReceipt format;
- agent integration profile;
- protocol adapters or reference subsets;
- SDKs;
- example connectors;
- conformance suite;
- import/export formats;
- signing metadata format;
- CLI scaffolding tools.

The proprietary advantage should be product quality, context compilation, retrieval, synchronization, policy, Trust Ledger, agent coordination, hardware adaptation, native experiences, and managed services—not inability to leave.

---
# Part V — Collaboration, Structured Work, Agents, and Automation

## 29. Collaboration Model

MindVault should reach functional parity with serious collaboration systems without copying Slack or Notion as disconnected products.

### Communication surfaces

| Surface | Required capability |
|---|---|
| Channels | Public/private Space channels, topics, membership, retention, permissions |
| Direct messages | Human-human, human-agent, and explicitly authorized agent-agent conversations |
| Group messages | Small ad hoc collaboration groups |
| Threads | Focused discussion, resolution state, linked decisions/tasks/docs |
| Mentions | Humans, agents, teams, roles, objects, and workflows |
| Reactions | Lightweight acknowledgment and workflow triggers where appropriate |
| Presence | Available, focused, offline, agent working/waiting/blocked; privacy-controlled |
| Files | Upload, preview, version, provenance, permissions, retention |
| Search | Messages, files, people, decisions, tasks, artifacts, and source context |
| Guests | Time-limited and Space-limited external participation |
| Notifications | Ranked, deduplicated, preference-aware delivery |
| Huddles/calls | Integrate first; native real-time audio/video only after core maturity |

### Document and knowledge surfaces

| Surface | Required capability |
|---|---|
| Documents | Rich text/Markdown, blocks, comments, mentions, citations, versioning |
| Structured collections | Typed fields, filters, views, relations, formulas, permissions |
| Canvas | Visual arrangement of documents, entities, tasks, decisions, and evidence |
| Wiki | Verified and maintained team knowledge with owners and review dates |
| Templates | Space, project, document, decision, meeting, agent, and automation templates |
| Sources | Inspectable evidence panel and provenance chain |
| Timeline | Historical state, decisions, events, and agent activity |

### Collaboration principle

Messages, documents, tasks, decisions, agents, and artifacts are separate typed objects that can reference one another. A chat message should not be overloaded to represent every form of work.

---

## 30. Conversation-to-Work and Conversation-to-Knowledge

Any message or thread may be converted into:

- a task;
- a project;
- a decision;
- a document;
- a procedure;
- a meeting follow-up;
- a WorkOrder;
- an automation;
- a reminder;
- a candidate memory;
- a ContextCapsule;
- an external message draft.

The original conversation remains linked as evidence.

### Three-layer communication model

| Layer | Meaning |
|---|---|
| Raw communication | Message/thread retained according to policy |
| Candidate structured information | Extracted task, decision, claim, relationship, or preference |
| Canonical knowledge/work | Human-verified or narrowly policy-approved object |

### Anti-noise rules

- Do not convert every message into memory.
- Do not retain every transient conversation forever by default.
- Do not allow informal speculation to become verified fact.
- Do not let AI summaries replace the source conversation.
- Do not use hidden chain-of-thought as a canonical work artifact.
- Do preserve declared decisions, commitments, approvals, and action receipts.

---

## 31. Work Graph

The Work module should model execution as a typed graph rather than a flat task list.

### Core work objects

| Object | Key fields |
|---|---|
| Objective | Desired outcome, owner, measures, timeframe |
| Project | Objective, scope, members, status, health, policies |
| Milestone | Target, due date, criteria, dependencies |
| Task | Owner, delegate, reviewer, status, criteria, dependencies, evidence |
| Decision | Choice, alternatives, rationale, evidence, effective/review dates |
| Risk | Probability, impact, owner, mitigation, trigger |
| Blocker | Blocked object, cause, owner, escalation, resolution |
| Procedure | Versioned executable or human process |
| Resource | Budget, compute, person, equipment, or time allocation |
| Deliverable | Expected artifact and acceptance criteria |
| Verification | Tests, reviews, evidence, and sign-off |
| Outcome | Actual result and later evaluation |

### Work status model

```text
Proposed
  → Planned
  → Ready
  → In Progress
  → Waiting / Blocked / Paused
  → Verification
  → Review
  → Accepted / Rework / Rejected
  → Completed
  → Outcome Review
  → Archived
```

### Ownership model

A task may include:

```text
Accountable owner: Human
Delegate: Human or Agent
Reviewer: Human or approved Agent
Collaborators: Humans and Agents
Observers: Humans, Agents, Services
```

Human accountability remains visible even when execution is delegated.

### Required views

- list;
- board;
- timeline;
- calendar;
- dependency graph;
- workload;
- agent-run view;
- decision log;
- risk register;
- project health;
- evidence and verification;
- historical replay.

---

## 32. Decision System

Decisions must be first-class objects rather than messages buried in channels.

### Decision record

- question or problem;
- accountable owner;
- participants and consulted agents;
- alternatives;
- supporting and opposing evidence;
- assumptions;
- risks;
- selected choice;
- rationale;
- authority and approvals;
- effective date;
- review/expiry date;
- affected objects;
- resulting tasks;
- later outcome;
- supersession chain.

### Decision receipt

A finalized decision produces a signed receipt recording:

```text
who decided
who or what advised
what evidence was available
what context was withheld
what alternatives were considered
what policy authorized the decision
when it became effective
what work it triggered
how it can be reversed or superseded
```

### Decision-review agents

Optional agents may:

- test assumptions;
- search for missing evidence;
- identify conflicts with existing decisions;
- estimate cost and risk;
- run security/privacy review;
- propose review dates;
- monitor whether the decision’s assumptions remain valid.

They advise; they do not silently make organizational commitments.

---

## 33. Agents as First-Class, Explicitly Non-Human Actors

An agent may participate in Spaces, conversations, projects, tasks, documents, and automations, but its identity must never be confused with a human.

### Agent profile

| Field | Purpose |
|---|---|
| Agent name and icon | Recognizable identity |
| Sponsor/owner | Accountable human or organization |
| Provider/model/runtime | Technical execution identity |
| Version | Reproducible behavior |
| Skills | Supported work types |
| Input/output contracts | Expected requests and artifacts |
| Permitted Vaults/Spaces | Context boundaries |
| Context scopes | What it may read |
| Tool scopes | What it may invoke |
| Write scopes | What it may propose or modify |
| External-action rights | What it may send, publish, schedule, or execute |
| Schedule/availability | Manual, scheduled, event-triggered, continuous |
| Cost/token/time/compute budgets | Resource governance |
| Risk class | Read-only, draft, internal action, external action, privileged |
| Escalation policy | When it must stop and request human input |
| Retention policy | Run context and output lifecycle |
| Trust state | Verified, local, community, experimental, suspended |
| Evaluation history | Quality, failures, incidents, acceptance rates |

### Agent state

- available;
- queued;
- planning;
- waiting for approval;
- working;
- waiting for input;
- blocked;
- paused;
- verifying;
- review-ready;
- failed;
- cancelled;
- completed;
- suspended.

### Agent policy law

An agent’s permissions are never inferred solely from its sponsor’s permissions. The agent receives explicit capability grants, and high-risk capabilities may be granted per run only.

---

## 34. Work Orders and Agent Runs

### WorkOrder

A `WorkOrder` is the durable request and contract for delegated work.

Required fields:

- objective;
- accountable owner;
- selected delegate or agent-selection policy;
- Space/Vault context;
- acceptance criteria;
- constraints;
- evidence and source requirements;
- deliverable types;
- context-request policy;
- tool needs;
- budget and deadline;
- risk classification;
- approval checkpoints;
- verification strategy;
- notification policy.

### AgentRun lifecycle

```text
Draft request
  → WorkOrder validation
  → Context compilation
  → Plan proposal
  → Permission/budget/policy check
  → Human or policy approval
  → Queue and credential issuance
  → Execution
  → Progress/checkpoints
  → Waiting / blocked / escalation as needed
  → Verification
  → Report and artifacts
  → Human review
  → Accepted / revised / rejected
  → Canonical updates through proposals
  → Run closure and receipt
```

### Run record

| Component | Contents |
|---|---|
| Request | Exact original request and revisions |
| Owner | Accountable principal |
| Agent | Identity, version, model, runtime |
| Context snapshot | Exact sources and records supplied |
| Plan | Proposed/approved execution plan |
| Grants | Context and tool capabilities |
| Budget | Cost, tokens, compute, storage, time |
| Timeline | Started, paused, resumed, completed |
| Actions | Tool calls and side effects |
| Progress | Structured status and checkpoints |
| Artifacts | Reports, code, files, data, designs, diffs |
| Evidence | Sources, tests, logs, receipts, screenshots |
| Verification | Automated and human review |
| Result | Completed, partial, blocked, failed, cancelled |
| Follow-up | Proposed tasks, decisions, automations |
| Receipt | Authority, behavior, outcome, costs, and final state |

### Run cards

Channels should receive concise state cards rather than low-level model traces.

```text
Research Agent — Working
17 of 24 sources reviewed
Current stage: evidence synthesis
Blocker: one source requires access
Cost used: $4.21 of $10
[Open Run] [Provide Access] [Change Scope] [Pause] [Cancel]
```

```text
Research Agent — Review Ready
Recommendation: Option A
Confidence: medium-high
Unresolved uncertainties: 3
Artifacts: report, comparison table, source map
[Inspect Evidence] [Accept] [Request Revision] [Reject]
```

### Reasoning privacy

Store plans, actions, evidence, structured rationales, intermediate artifacts, and verification. Do not require or preserve private hidden chain-of-thought. Agents should provide concise, inspectable decision summaries appropriate for audit.

---

## 35. Agent Selection and Multi-Agent Collaboration

### Agent selection

A router may select an agent based on:

- required skill;
- context sensitivity;
- local-only requirement;
- quality history;
- latency;
- cost;
- hardware availability;
- model/provider policy;
- tool support;
- workload;
- user preference;
- conflict-of-interest policy;
- verification requirements.

The selected agent and rationale are shown before high-risk execution.

### Multi-agent patterns

| Pattern | Use |
|---|---|
| Specialist handoff | Research agent hands evidence to writing or implementation agent |
| Parallel investigation | Multiple agents examine independent hypotheses |
| Review council | Architecture, security, privacy, cost, and red-team agents review a proposal |
| Supervisor/delegates | Coordinator decomposes and monitors scoped sub-work |
| Debate with arbiter | Agents present opposing analyses; human or designated synthesis stage resolves |
| Builder/reviewer | One agent creates; another independently verifies |
| Watcher/responder | Monitor detects condition and proposes or executes bounded response |
| Human-agent swarm | Humans own tasks while agents provide execution capacity |

### Anti-collusion and verification rules

- Critical verification should not be performed by the identical model/configuration that produced the work when independence matters.
- Agent outputs preserve attribution and disagreement.
- A synthesis agent may summarize disagreements but cannot erase dissenting evidence.
- Parallelism is bounded by budget and value, not used indiscriminately.
- Agent-to-agent delegation cannot expand permissions.
- Sub-agent work inherits stricter or equal constraints, never broader constraints.

---

## 36. Delegated Communication and Action Outbox

Agents may communicate on behalf of users or organizations only through an explicit delegated-action model.

### Communication modes

| Mode | Behavior |
|---|---|
| Suggest | Agent recommends that a message be sent |
| Draft | Agent writes; human edits and sends |
| Approve once | Agent prepares; human approves delivery |
| Approved template | Agent fills verified fields in an approved template |
| Internal bounded autonomy | Agent may post low-risk verified operational updates in approved Spaces |
| External bounded autonomy | Agent may send only to approved recipients for approved purposes |
| Owner-only | Agent cannot send or commit the action |

### Default authority matrix

| Action | Default |
|---|---|
| Post “tests passed; review ready” internally | Bounded internal autonomy after verification |
| Notify that a task is blocked | Bounded internal autonomy |
| Publish scheduled internal project digest | Bounded internal autonomy if evidence checks pass |
| Meeting follow-up summary | Draft or approve once |
| Schedule inside explicit calendar rules | Bounded automation |
| Remind assigned teammate | Approved workflow with rate limits |
| Reply to recruiter/professional contact | Draft first |
| Client commitment | Human approval |
| Legal, medical, HR, financial, or contractual statement | Human approval |
| Public post | Human approval |
| Purchase, transfer, or financial transaction | Owner-only or transaction-specific strong approval |
| Delete shared/canonical data | Human approval with recovery window |

### Attribution

Internal display should preserve both identities:

- `Project Reporter · acting for Hassan`
- `Hassan via Project Reporter`

External presentation may vary according to destination rules, but the Trust Ledger always records:

- principal;
- acting agent;
- agent/model/version;
- source evidence;
- human review state;
- policy decision;
- recipients;
- delivery receipt;
- replies and downstream effects.

### Action Outbox

All external side effects enter a durable Outbox with:

- proposed action;
- exact payload preview;
- recipient/resource;
- authority mode;
- policy result;
- approval requirement;
- idempotency key;
- scheduled time;
- retries;
- cancellation window;
- compensation/rollback where available;
- final external receipt.

---

## 37. Scheduling, Reports, and Notifications

### Scheduled agent examples

- daily project digest;
- Monday standup preparation;
- Friday status report;
- monthly strategy review;
- deadline-risk watch;
- stale-decision review;
- dependency/security update;
- competitor monitoring;
- meeting follow-up;
- onboarding progress;
- knowledge-debt review;
- application/job-search review;
- recurring research refresh.

### Evidence-based project report

A Project Reporter should compile from verified state:

- completed tasks;
- accepted artifacts;
- merged changes;
- test and release receipts;
- open work;
- blockers;
- risks;
- decisions;
- scope/deadline changes;
- agent-run outcomes;
- meetings and commitments;
- next priorities.

It should not infer project status solely from recent chat.

### Notification triggers

| Trigger | Example |
|---|---|
| Time | Weekly digest or scheduled check |
| Task state | Blocked, ready for review, overdue |
| Agent state | Needs approval, failed, completed |
| Repository event | CI passed, security issue found, release ready |
| Deadline | Milestone risk crosses threshold |
| Decision | Affected members must review or acknowledge |
| Meeting | Summary/action proposals are ready |
| Document change | Material specification change requires review |
| External event | Customer issue, regulatory update, competitor change |
| Inactivity | No progress for defined period |
| Confidence | Evidence conflict or stale assumption detected |
| Policy | Agent requests new capability |
| Security | Suspicious behavior or credential failure |

### Attention Router

The Attention Router must:

- deduplicate related events;
- thread updates by object/run;
- rank urgency and importance;
- respect quiet hours and focus modes;
- distinguish informational from action-required;
- summarize low-priority activity;
- escalate unresolved critical items;
- apply per-Space preferences;
- learn only from approved preference signals;
- explain why an alert was delivered;
- provide one-click mute, defer, delegate, or convert-to-task.

### Delivery channels

- in-app Inbox;
- channel/thread;
- direct message;
- desktop notification;
- mobile push;
- email;
- Slack, Teams, Discord, Matrix bridge;
- calendar reminder;
- SMS only for explicitly urgent policies;
- external webhook.

---

## 38. Automation Engine

Automation should combine deterministic orchestration with governed AI steps.

### Automation object

- trigger;
- preconditions;
- actor/principal;
- input schema;
- deterministic steps;
- model/agent steps;
- context policy;
- tool policy;
- approval checkpoints;
- retry/backoff;
- idempotency;
- timeout;
- compensation;
- notification policy;
- observability;
- version;
- test cases;
- deployment state.

### Lifecycle

```text
Draft
  → Validate
  → Test with fixtures
  → Shadow mode
  → Compare proposed vs desired outcomes
  → Limited pilot
  → Bounded production autonomy
  → Continuous monitoring
  → Pause / rollback / retire
```

### Execution classes

| Class | Example | Default control |
|---|---|---|
| Deterministic local | Rename/tag/import based on fixed rules | May auto-run |
| AI-assisted proposal | Extract tasks from a meeting | Review required initially |
| Internal low-risk action | Post verified build result | Bounded policy |
| External communication | Send follow-up email | Draft/approval by default |
| Destructive action | Delete records or revoke members | Strong approval |
| Financial/legal action | Purchase, sign, commit | Owner-only/transaction approval |

### Automation invariants

1. Every run has an idempotency key.
2. Retries cannot duplicate external side effects.
3. AI steps cannot alter workflow permissions.
4. Secrets are brokered, never embedded in prompts or workflow definitions.
5. Every external action returns or records a receipt.
6. Failed compensation is escalated.
7. Shadow-mode evidence is retained before autonomy increases.
8. A user can pause all automations or one Space immediately.
9. A policy change invalidates or revalidates affected automations.
10. Automation versions are immutable once executed; edits create new versions.

---

## 39. High-Value First-Party Agents

### Project and engineering

| Agent | Responsibility |
|---|---|
| Project Coordinator | Milestones, dependencies, owners, risk, and status |
| Build Agent | Approved implementation through DevX Runtime, Claude Code, Codex, or other runtime |
| Test Agent | Tests, regression detection, and acceptance evidence |
| Review Agent | Code/spec/document review and contradiction detection |
| Release Agent | Release candidate, gates, notes, and receipts |
| Documentation Agent | Updates docs from accepted changes |
| Incident Agent | Investigation, timeline, coordination, and response proposals |
| Dependency Watcher | Security, compatibility, license, and maintenance changes |

### Knowledge and context

| Agent | Responsibility |
|---|---|
| Knowledge Curator | Duplicates, stale content, contradictions, orphaned records |
| Citation Auditor | Whether claims are actually supported |
| Context Compiler | Concise actor-specific ContextPackages |
| Decision Historian | Why a decision was made and what was known |
| Memory Steward | Candidate durable memory and review queues |
| Onboarding Agent | Role-specific handoff and onboarding capsules |
| Taxonomy Steward | Schema/category consistency without silent rewriting |

### Communication and coordination

| Agent | Responsibility |
|---|---|
| Project Reporter | Scheduled evidence-backed updates |
| Meeting Follow-up Agent | Summary, decision/task proposals, approved follow-up |
| Stakeholder Reporter | Tailored updates for executives, engineers, clients, investors |
| Inbox Triage Agent | Classify messages and propose responses |
| Scheduling Agent | Coordinate within explicit calendar policies |
| Community Agent | Answer routine questions from approved public knowledge |

### Research and decisions

| Agent | Responsibility |
|---|---|
| Research Agent | Evidence gathering, evaluation, synthesis, citations |
| Competitive Intelligence Agent | Market/product/pricing/regulatory monitoring |
| Red-Team Agent | Failure modes and adversarial challenge |
| Cost Agent | Resource and operational alternatives |
| Security Agent | Threats, vulnerabilities, and control verification |
| Privacy Agent | Data flows, minimization, retention, and egress |
| Scenario Agent | Alternative plans and outcome modeling |

---

## 40. Additional Differentiating Capabilities

### Agent Standups

At a configured time, humans and agents submit structured updates. A coordinator detects blockers/dependencies and prepares a concise standup. Project state changes only from verified evidence or approved updates.

### Project Pulse

A live evidence-backed health surface:

- progress and confidence;
- scope change;
- deadline risk;
- unresolved blockers;
- aging tasks;
- unreviewed agent work;
- dependency risk;
- resource consumption;
- stale assumptions;
- next critical action.

### Knowledge Debt

Track:

- obsolete documents;
- uncited claims;
- unresolved contradictions;
- duplicate records;
- ownerless tasks;
- stale procedures;
- abandoned proposals;
- decisions without outcome review;
- disconnected artifacts;
- undocumented external actions.

### Meeting-to-Execution

```text
Meeting evidence
  → summary candidates
  → decisions/tasks/commitments proposed
  → owners confirm
  → WorkOrders created
  → humans/agents execute
  → results verified
  → follow-up drafted/sent
  → approved knowledge updated
```

### Context Handoff Capsules

When work moves between people, agents, teams, or organizations, compile:

- objective;
- current state;
- accepted decisions;
- open questions;
- artifacts;
- blockers;
- permissions;
- next action;
- expiry/revocation.

### Work Replay and Time Machine

Reconstruct:

- what was known at a time;
- messages and meetings;
- active tasks and decisions;
- agent plans/actions;
- artifacts and tests;
- failures and corrections;
- permissions then in effect;
- final outcome.

### Agent Office Hours

Agents may be available only:

- during specified times;
- in named Spaces;
- to approved roles;
- for defined task types;
- within cost/compute quotas;
- while required supervisors are available.

### Context Subscriptions

A user or agent may subscribe to semantic conditions such as:

- “Notify me when evidence contradicts this decision.”
- “Track material changes to this company or regulation.”
- “Alert the project when any dependency becomes unmaintained.”
- “Refresh this research dossier every month.”

### Policy Simulation

Before granting autonomy, replay historical events and show:

- which actions would have run;
- which data would have left the system;
- which messages would have been sent;
- errors or false positives;
- cost and attention impact.

### Multi-Agent Review Council

Architecture, security, privacy, cost, accessibility, and red-team agents may independently review a proposal. Their opinions remain separately attributable, and synthesis cannot hide unresolved disagreement.

### External Guest Capsule

Instead of adding an external person to a whole workspace, issue a revocable capsule with exactly the documents, tasks, messages, and tools required, plus expiry and redistribution policy.

---
# Part VI — AI Runtime, Languages, Hardware, Experience, and Security

## 41. Model-Independent AI Architecture

AI is a replaceable capability used by the platform. It is not the database, the authority system, or the product’s sole interface.

### Model capability classes

- embedding;
- reranking;
- classification;
- extraction;
- entity resolution;
- OCR;
- transcription;
- translation;
- summarization;
- generative reasoning;
- code generation;
- vision;
- speech;
- safety/moderation;
- anomaly detection.

### Model registry

Every model/backend record should include:

| Field | Purpose |
|---|---|
| Model and provider ID | Stable routing identity |
| Version and hash | Reproducibility |
| License/terms | Distribution and use constraints |
| Task capabilities | Routing eligibility |
| Input/output modalities | Compatibility |
| Context/input limits | Resource planning |
| Quantization/runtime | Hardware planning |
| Required memory/compute | Admission control |
| Quality evaluations | Task-specific routing |
| Latency distributions | User experience |
| Cost | Budget control |
| Energy profile | Local resource governance |
| Local/cloud status | Privacy and availability |
| Data retention/training policy | Egress decision |
| Supported hardware backends | Runtime selection |
| Known failure modes | Risk control |
| Approval status | Organization governance |

### Model routing

Routing considers:

- task and required quality;
- sensitivity;
- local-only/cloud-eligible classification;
- user/organization provider policy;
- device capabilities;
- latency target;
- cost budget;
- energy/thermal state;
- offline/network state;
- context size;
- modality;
- evaluation history;
- data residency;
- current provider availability.

### Local/cloud policy examples

| Task | Default direction |
|---|---|
| Search-as-you-type embedding | Small local model |
| Background indexing | Local, throttled on battery |
| Sensitive extraction | Local-only or approved private endpoint |
| Complex synthesis | Best approved model after context/egress check |
| Meeting tagging | Local or approved organizational service |
| Quick mobile capture | Lightweight local processing; deeper work deferred |
| High-stakes external message | Best approved model plus human review |
| Simple deterministic classification | Rules before model where reliable |

### AI-output requirements

Every derived output must retain:

- model/provider/version;
- time;
- input ContextPackage identity;
- transformation recipe/prompt version where appropriate;
- tool actions;
- sources;
- confidence/uncertainty;
- review state;
- cost and resource use;
- retention and sensitivity.

---

## 42. Experimental Neural Memory

Compiled parametric memory, Hebbian modules, adapters, LoRA, or other neural-memory techniques may be explored inside an isolated **Memory Lab**.

### Experimental rules

1. Neural memory is never canonical truth.
2. It must be rebuildable from approved source data.
3. It cannot silently write to the Knowledge Plane.
4. It is benchmarked against exact lookup, relational queries, full-text search, vector retrieval, graph traversal, and simpler caches.
5. Tests measure factual recall, paraphrase robustness, contradictions, update/delete behavior, latency, memory, energy, and hardware needs.
6. Experiments remain optional and removable.
7. Promotion requires meaningful quality-per-compute or latency gains under reproducible conditions.
8. Sensitive data use requires explicit policy and purge verification.
9. Failed recall or uncertain decoding must fall back to evidence retrieval.
10. The user can disable or erase learned parameters and rebuild from source.

---

## 43. Language and Runtime Allocation

Use a polyglot architecture by clear boundary, not by fashion or feature team preference.

| Language/technology | Recommended ownership | Avoid using for |
|---|---|---|
| Rust | Domain kernel, commands, policies, storage orchestration, cryptography, sync, server, connectors, audit, MCP/A2A adapters, extension runtime | Every UI or experimental model notebook |
| C++ | Proven hot-path compute, local inference, vector/ANN kernels, rerankers, OCR/layout/audio/image processing, native acceleration | Broad business logic, policy, schema migrations, direct canonical storage |
| Swift/SwiftUI/AppKit | Premium macOS/iOS experience, system integration, accessibility, Keychain/Secure Enclave, Apple ML bridges | Cross-platform server/domain kernel |
| Objective-C++ | Narrow Swift/Apple-to-C++ bridge | Product/domain logic |
| Kotlin/Compose | Native Android application and Android services | Canonical server core |
| Java | JVM SDK, enterprise compatibility, selected integrations | Default new Android UI or systems kernel |
| TypeScript/Svelte | Web, browser extension, existing Tauri UI, declarative extension UI, admin surfaces | Trusted canonical storage and security logic |
| Python | Evaluation, ML experimentation, data science, migration prototypes, offline research tooling | Long-running canonical authority service |
| WebAssembly | Sandboxed third-party extensions and portable transforms | Unrestricted system integration |
| SQL | Schema, constraints, migrations, exact queries | Semantic reasoning |
| JSON Schema/OpenAPI/Protobuf | Cross-language contracts | Internal domain behavior |

### Rust remains the authority kernel

Rust should continue to own:

- canonical state transitions;
- transactions and invariants;
- identity and authorization;
- policy evaluation;
- context/tool grants;
- audit and receipts;
- sync state;
- connector lifecycle;
- extension capability enforcement;
- process and service orchestration.

### C++ admission gate

A subsystem moves to or adds C++ only when identical-workload benchmarks show one or more material advantages:

- lower median/p95/p99 latency;
- lower peak or steady-state memory;
- better energy efficiency;
- higher indexing/inference throughput;
- necessary hardware/runtime access;
- reduced dependency footprint;
- better cross-platform native compatibility.

### C++ boundary rules

- narrow C ABI or audited bridge;
- explicit ownership and lifetime rules;
- no exceptions across FFI;
- no direct canonical database access;
- no shared mutable authority state;
- sanitizers and fuzzing;
- deterministic fixtures;
- crash containment where practical;
- versioned interface;
- benchmark regression gate.

### Frontend strategy

| Surface | Direction |
|---|---|
| Cross-platform desktop now | Preserve and refactor SvelteKit/Tauri while core contracts stabilize |
| macOS/iOS premium experience | Native SwiftUI/AppKit client once APIs/domain model stabilize |
| Android | Kotlin/Jetpack Compose |
| Web | SvelteKit or later evidence-backed choice |
| Browser | TypeScript extension with narrow connector/command APIs |
| CLI | Rust |
| Developer SDKs | Native language SDKs generated/maintained from public contracts |

Java should be supported where users or enterprises need JVM interoperability, but it should not be selected as the default UI language simply because it is broadly portable.

---

## 44. Hardware-Adaptive Runtime

The platform should optimize for the available machine rather than assuming one universal configuration.

### Capability profiler

Detect and periodically refresh:

- CPU architecture and SIMD capabilities;
- physical and available memory;
- unified versus discrete memory;
- GPU/NPU/accelerator availability;
- supported runtime backends;
- battery and charging state;
- thermal state;
- free storage and I/O throughput;
- model-cache capacity;
- network availability/quality;
- local-only and egress policies.

### Backend candidates

| Platform | Candidate backends |
|---|---|
| Apple Silicon | Core ML, MLX, Metal, Accelerate, llama.cpp/native C++ |
| NVIDIA | CUDA, TensorRT, ONNX Runtime CUDA, optimized native runtimes |
| AMD | ROCm/MIGraphX-compatible paths where mature and supported |
| Intel | OpenVINO, optimized CPU SIMD, ONNX Runtime |
| Windows ARM/Qualcomm | QNN/DirectML/ONNX Runtime where supported |
| Generic CPU | Optimized Rust/C++, ONNX Runtime CPU, llama.cpp |
| Browser | WASM/WebGPU for constrained tasks |

### Resource governor

Every local AI or compute job declares:

- latency class;
- memory ceiling;
- compute/GPU ceiling;
- energy priority;
- battery behavior;
- foreground/background status;
- interruptibility;
- maximum context;
- data classification;
- local-only requirement;
- cloud eligibility;
- deadline;
- cache policy.

### Resource classes

| Class | Behavior |
|---|---|
| Interactive | Prioritize response latency and UI smoothness |
| Foreground heavy | Use available resources with visible progress and cancel |
| Background normal | Throttle around user activity |
| Background opportunistic | Run only when charging/idle or resource headroom exists |
| Critical | Reserve resources and fail over according to policy |
| Mobile constrained | Prefer small models, staged processing, and deferred enrichment |

### Hardware benchmarks

Maintain named baseline profiles:

- lower-memory Apple Silicon laptop;
- current mainstream Apple Silicon system;
- high-memory Apple Silicon workstation;
- mainstream Windows x86 system;
- NVIDIA workstation;
- generic CPU-only Linux server;
- current iPhone and Android device classes.

Do not publish “fast” or “efficient” without device, workload, corpus, model, and percentile details.

---

## 45. Human Experience Architecture

### Top-level navigation

| Surface | Purpose |
|---|---|
| Home | Priorities, recent context, project pulse, personal dashboard |
| Inbox | Messages, mentions, approvals, blockers, agent requests, notifications |
| Spaces | Personal, team, project, client, research, and guest Spaces |
| Work | Projects, tasks, milestones, decisions, risks, schedules |
| Agents | Agent roster, skills, availability, Work Orders, Runs, budgets, history |
| Knowledge | Documents, evidence, entities, memory, graph, timeline, search |
| Connect | Applications, devices, nodes, connectors, protocols, extensions |
| Automate | Workflows, schedules, watchers, shadow-mode results |
| Trust | Permissions, grants, approvals, provenance, receipts, retention, egress |
| Admin | Deployment, identity, storage, models, security, organization controls |

### Inside each Space

```text
Overview
Channels
Docs
Work
Decisions
Meetings
Files
Agents
Automations
Memory
Activity
Settings
```

### Essential interface primitives

- global command/search palette;
- channel/thread view;
- source citation and evidence inspector;
- ContextPackage inspector;
- agent Run Card;
- WorkOrder builder;
- diff/proposal review;
- decision record and receipt;
- approval center;
- Action Outbox;
- connector health panel;
- permissions/grant inspector;
- policy simulation;
- Space activity timeline;
- conflict resolver;
- sync/recovery status;
- privacy and egress indicator;
- project pulse;
- notification/attention controls.

### UX laws

1. Chat is not the only or default representation of work.
2. Every AI answer can expose supporting sources.
3. Every AI-originated change shows a diff.
4. Every external action shows destination and authority.
5. Every screen communicates local/cloud/remote processing when relevant.
6. Advanced complexity is progressively disclosed.
7. Search is globally accessible.
8. Users can inspect why content or a notification was shown.
9. Keyboard navigation is complete for core workflows.
10. Screen-reader behavior is a release gate.
11. Offline, syncing, stale, conflicting, and failed states are explicit.
12. Destructive actions state consequences and recovery options.
13. Agents have unmistakable non-human visual identity.
14. Permission prompts describe actual data and actions rather than abstract scopes alone.
15. Settings are decomposed by domain rather than placed in one giant page.

### Native Apple experience targets

- proper application/menu/window lifecycle;
- command menus and keyboard navigation;
- drag and drop;
- Share extensions;
- Finder/Quick Look integration;
- Spotlight integration where privacy permits;
- Services integration;
- secure Keychain/Secure Enclave use;
- background tasks and notifications;
- system accessibility APIs;
- energy/thermal integration;
- native document and file coordination.

---

## 46. Design System and Accessibility

Before expanding UI surface, build a shared, tested design system.

### Required primitives

- Button and IconButton;
- Link and navigation item;
- Menu/context menu;
- Dialog/AlertDialog;
- Sheet/drawer;
- Popover/tooltip;
- Combobox/autocomplete;
- Command palette;
- Tabs;
- Data table/tree/grid;
- Form controls and validation;
- Empty/loading/error/offline states;
- SourceCitation;
- EvidencePanel;
- DiffViewer;
- PermissionBadge;
- SensitivityLabel;
- AgentBadge;
- RunStatus;
- ApprovalCard;
- ActionReceipt;
- Toast and Undo;
- Timeline and activity item;
- ConflictResolver;
- Skeleton/progress;
- accessible rich-text editor surface.

### Accessibility release requirements

- full keyboard operability;
- visible focus;
- logical focus order;
- accessible names/descriptions;
- screen-reader announcements for dynamic updates;
- reduced motion;
- sufficient contrast;
- text scaling;
- no color-only meaning;
- accessible data tables and graphs;
- caption/transcript support;
- VoiceOver, TalkBack, and desktop screen-reader manual testing;
- automated checks plus human verification.

---

## 47. Security and Privacy Architecture

MindVault may hold a broader and more sensitive context set than email alone. Security is a product plane and release gate.

### Threat categories

| Threat | Required control |
|---|---|
| Stolen device | Vault encryption, OS authentication, device revocation |
| Malicious plugin | Capability sandbox, signed manifest, network/file restrictions |
| Prompt injection | Untrusted-content quarantine and instruction/data separation |
| Cloud leakage | Egress gateway, classification, preview, provider policy |
| Compromised connector | Per-connector identity, scopes, revocation, source validation |
| Malicious/compromised agent | Run-specific grants, sandbox, budgets, kill switch, audit |
| Supply-chain compromise | Lockfiles, SBOM, signatures, provenance, dependency policy |
| Sync replay/tampering | Signed operations, sequence/cursor protection, integrity checks |
| Database corruption | Checksums, transactional writes, snapshots, verified restore |
| Unauthorized local process | Authenticated local IPC and OS permissions |
| Silent canonical mutation | Proposals, version history, immutable receipt |
| Hallucinated knowledge | Evidence requirements, abstention, contradiction checks |
| Cross-Space leakage | Mandatory boundary filters and adversarial tests |
| Secret exposure to models | Secret broker and scoped proxy actions |
| Excessive notification/action spam | Rate limits, attention policy, anomaly detection |
| Insider misuse | Least privilege, approval separation, audit export |
| Account takeover | Passkeys/MFA, session/device management, risk signals |

### Zero-trust actor model

- humans, devices, agents, applications, connectors, extensions, and services each have separate identities;
- no implicit trust from network location;
- short-lived capability credentials;
- explicit Vault/Space scopes;
- read and action permissions separated;
- high-risk actions require stronger authentication or approval;
- suspicious behavior can automatically suspend an identity;
- revocation propagates to active runs and sessions.

### ContextGrant

A ContextGrant specifies:

- actor and principal;
- purpose;
- Vaults/Spaces/resources;
- allowed fields/excerpts;
- sensitivity ceiling;
- local/cloud handling;
- duration and expiry;
- query/export limits;
- redistribution limits;
- model/provider restrictions;
- revocation;
- receipt requirements.

### ToolGrant

A ToolGrant specifies:

- exact tool/action;
- target resources;
- allowed parameters or templates;
- rate and cost limits;
- transaction limits;
- recipients/domains;
- time window;
- human-approval rules;
- rollback/compensation requirements;
- audit level.

A ContextGrant never implies a ToolGrant.

### Encryption and keys

- encryption at rest is default-on;
- private and shared contexts use an explicit key hierarchy;
- keys are not stored beside encrypted data in plaintext;
- device-bound keys use OS/hardware facilities where available;
- recovery keys are user-controlled and testable;
- namespaces/Spaces may use separate data-encryption keys;
- highly restricted Spaces may forbid cloud egress;
- key rotation and member removal are supported;
- backups use independent encryption;
- cryptographic formats are versioned and migratable.

### Secret broker

Agents and extensions do not receive raw long-lived credentials when a brokered action is possible.

```text
Agent requests approved action
  → Policy verifies ToolGrant
  → Secret broker calls external service
  → Result is filtered
  → Credential remains hidden
  → ActionReceipt records the call
```

### Egress firewall

Every outbound data flow passes through:

- destination allowlist;
- data classification;
- user/organization policy;
- model/provider policy;
- redaction/minimization;
- purpose binding;
- payload preview for sensitive actions;
- logging and receipt.

### Prompt-injection containment

Imported pages, messages, transcripts, documents, code, and files are data—not authority.

The platform must distinguish:

- trusted system policy;
- human instruction;
- agent instruction;
- source content;
- embedded untrusted instruction;
- executable proposal.

No imported content may grant permissions, reveal secrets, change policy, or trigger an external action without passing the normal authority path.

### Extension sandbox

- WASM or isolated process boundary;
- deny-by-default filesystem and network access;
- declared domains;
- memory/time/CPU limits;
- no raw database connection;
- no unrestricted process spawning;
- content scanning;
- signed packages and update verification;
- per-extension data and action audit;
- emergency revocation.

### Privacy defaults

- telemetry off or minimal opt-in;
- model training on user content off by default;
- private-by-default Spaces and artifacts;
- retention visible and configurable;
- raw imported payloads minimized;
- cloud processing explicit;
- no sale of personal data;
- no dark patterns around export/deletion;
- user-controlled portability;
- per-connector data-flow map.

---

## 48. Trust Ledger and Action Receipts

The Trust Ledger is the immutable, inspectable history of consequential access, decisions, transformations, and actions.

### Action envelope

```text
actor_id
actor_type
principal_id
on_behalf_of
workspace_id
vault_or_space_id
agent_run_id
connector_or_extension_id
action_type
resource_ids
context_grant_id
tool_grant_id
policy_decision_id
approval_id
causation_id
correlation_id
idempotency_key
provenance_refs
timestamp
signature
result
external_receipt
compensation_or_rollback
```

### Receipt types

- access receipt;
- context-compilation receipt;
- model-inference receipt;
- canonical-change receipt;
- approval receipt;
- delegated-message receipt;
- external-action receipt;
- agent-run receipt;
- import/sync receipt;
- export/share receipt;
- deletion/retention receipt;
- policy-change receipt;
- backup/restore receipt.

### Human questions the ledger must answer

- Who or what acted?
- On whose behalf?
- What context did it receive?
- What context was withheld?
- Which permissions and policy authorized it?
- Was a human involved?
- Which model and tools were used?
- What source evidence supported it?
- What external system changed?
- What did it cost?
- What was the result?
- Can it be reversed?
- Which later actions depended on it?

---
# Part VII — Product Boundaries and Evolution of the Existing Repository

## 49. Boundary with DevX Platform

MindVault Next and DevX remain distinct products with explicit contracts.

| MindVault Next owns | DevX Runtime owns |
|---|---|
| Personal and organizational context | Engineering execution |
| Vaults and Spaces | Sandboxed tools and worker processes |
| Humans, teams, memberships, and communication | Builds, tests, debugging, and verification execution |
| Projects, tasks, decisions, and Work Orders | Runtime scheduling and resource governance for engineering work |
| Agent registry and run presentation | Engineering-task execution truth |
| Context compilation and grants | Workspace/repository execution controls |
| Approvals, delegated communication, and reports | Execution evidence and artifacts |
| Long-term knowledge and provenance | Engineering runtime logs and execution receipts |
| Cross-domain integrations and notifications | Code-oriented tool adapters and worker lifecycle |

### Delegation example

```text
MindVault Project Space
  → Human creates/approves engineering WorkOrder
  → ContextPackage and ToolGrant issued
  → DevX Runtime / Claude Code / Codex executes
  → Tests, diffs, artifacts, and runtime evidence returned
  → MindVault AgentRun enters review
  → Human accepts or requests revision
  → Project state, decision, and report update
```

MindVault may invoke Claude Code, Codex, Cursor, or other tools directly for personal workflows, but DevX Runtime remains the governed execution owner when the DevX architecture is used.

---

## 50. Documented Existing Baseline

The current repository appears to provide a valuable starting point, including a Rust workspace, SQLite, vector/full-text/graph components, REST/gRPC/WebSocket surfaces, MCP, a Tauri/Svelte interface, WASM plugins, connectors, relay concepts, scoped proposals, audit/security work, and synchronization/federation experiments.

These are **audit hypotheses**, not final truth. A filename, checklist, or “complete” marker does not establish that a capability is production-integrated, secure, performant, or usable.

### Feature-truth classifications

Every existing capability must be classified as:

| Classification | Meaning |
|---|---|
| Production-integrated | Reachable through the real production path and meets release gates |
| Implemented but unverified | Code exists; behavior has not been sufficiently validated |
| Partial | Only some required behavior exists |
| Prototype | Experimental and not suitable for production dependency |
| Stub/scaffold | Interfaces or placeholders without complete behavior |
| Dead | Unused or unreachable code |
| Documentation-only | Claimed but not found in executable implementation |
| Missing | Required but not present |
| Superseded | Existing behavior intentionally replaced by v2 architecture |

### Audit law

No existing component is preserved, rewritten, or removed based solely on language preference or documentation. Decisions require call-path inspection, tests, benchmarks, dependency risk, data-migration analysis, and product fit.

---

## 51. Preserve, Refactor, Modularize, Replace, or Retire

### Preserve and harden

| Existing direction | Treatment |
|---|---|
| Modular Rust workspace | Preserve as systems foundation |
| SQLite structured storage | Preserve for Personal Node canonical state |
| Markdown and local-first principles | Preserve and strengthen |
| Evidence/provenance concepts | Elevate into core schema and Trust Plane |
| MCP server | Preserve, update, and make bidirectional |
| Proposal-based AI writes | Make universal canonical-write invariant |
| WASM/Wasmtime extension direction | Preserve and harden capability model |
| Import/export | Preserve and expand into portability contract |
| Attachments/extraction | Preserve behind evidence pipeline |
| Audit logging | Upgrade to tamper-evident Trust Ledger |
| Encryption/keychain foundations | Preserve; make secure defaults mandatory |
| Browser capture | Preserve as first-party connector |
| Relay/adapters | Preserve as input to Relay and connector architecture |
| Existing Tauri/Svelte UI | Preserve as migration and portable client foundation |
| CLI | Preserve and expand for recovery/developer operations |

### Refactor deeply

| Current direction | New direction |
|---|---|
| Generic “node” object | Typed public domain objects plus compatibility layer |
| Broad engine responsibilities | Context, retrieval, memory, policy, work, agent, and interop services |
| Multiple parallel indexes | Derived, benchmarked projections with one update pipeline |
| Local MCP only | Local/remote, client/server, profile-based MCP gateway |
| Namespace/role model | Actor/principal identity plus ContextGrants, ToolGrants, ABAC/capabilities |
| Plugin direct write capabilities | Commands/proposals through policy layer |
| Relay as single-owner messaging | Native private/shared Spaces and external Relay adapters |
| Single-owner sync assumptions | Personal nodes plus shared-server and federation model |
| Agent activity streams | Structured WorkOrders, Runs, artifacts, verification, receipts |
| AI provider status | Model registry, explicit local/cloud route and egress |
| Settings monolith | Domain-based settings and admin information architecture |
| Search | Permission-aware exact/structured/full-text/vector/graph/temporal pipeline |
| Memory | Lifecycle, evidence, contradictions, supersession, temporal validity |
| Public sharing | Signed, revocable ContextCapsules and explicit share policies |
| Metrics | Release SLOs and longitudinal quality/performance evaluations |

### Move out of the core

| Capability | Destination |
|---|---|
| Goals and habits | Optional personal-development module |
| Flashcards/study | Education module |
| Career workflows | Career domain module |
| Calendar | Connector plus optional view/module |
| Slack/Discord/Teams | External Relay/connectors, not canonical core protocol |
| Public publishing | Sharing/publishing module |
| CRM-like relationships | Relationship module built on core Person/Organization model |
| Voice/video rooms | Later communications module |
| Federation experiments | Dedicated interoperability/federation subsystem |

### Retire or disable until justified

- the constitutional assumption that no shared multi-user state will exist;
- automatic treatment of every interaction as durable memory;
- encryption disabled by default;
- unrestricted direct plugin or AI canonical writes;
- implicit cloud/model egress;
- opaque “auto” provider modes that hide whether AI ran locally, remotely, or heuristically;
- unbounded autonomous replies;
- duplicate admin and UI surfaces without unique responsibility;
- transport protocols with no validated consumer or operational need;
- public sharing enabled by default;
- raw model reasoning treated as product evidence;
- documentation claims that exceed verified implementation.

---

## 52. Proposed Crate and Service Evolution

The exact migration follows the repository audit; this is the target responsibility map.

| Existing area | Target evolution |
|---|---|
| `mv-core` | `context-domain`, typed IDs, schemas, domain rules |
| `mv-storage` | personal store, shared store adapters, artifact store, migrations |
| `mv-index` | retrieval projections and benchmarked index backends |
| `mv-graph` | derived relationship projection and algorithms |
| `mv-engine` | split into ingest, retrieval, context compiler, memory lifecycle, work, and agent services |
| `mv-server` | public gateway, realtime, admin, health, event subscriptions |
| `mv-mcp` | bidirectional MCP gateway and connection profiles |
| plugin crate/runtime | capability-based extension runtime and registry client |
| connector code | isolated connector runtime with manifests and source bindings |
| relay code | native collaboration/Relay adapters under shared identity and receipt model |
| sync/federation | explicit operation log, node identity, selective replication |
| CLI | administration, recovery, migration, conformance, development |
| web/Tauri frontend | new information architecture over stable APIs |
| native clients | Swift/Apple and Kotlin/Android applications after contracts stabilize |

### Target repository responsibility map

```text
apps/
├── desktop-tauri/
├── macos/
├── ios/
├── android/
├── web/
└── cli/

crates/
├── context-domain/
├── identity/
├── source-authority/
├── evidence-store/
├── personal-store/
├── shared-store/
├── artifact-store/
├── retrieval/
├── context-compiler/
├── memory-lifecycle/
├── collaboration/
├── work-graph/
├── agent-control/
├── automation-engine/
├── policy-engine/
├── context-grants/
├── action-grants/
├── trust-ledger/
├── sync-engine/
├── federation/
├── connector-runtime/
├── extension-runtime/
├── model-registry/
├── resource-governor/
├── event-router/
├── notification-router/
├── relay/
├── api-gateway/
├── mcp-adapter/
├── a2a-adapter/
└── observability/

native/
├── compute-cpp/
├── apple-bridge/
└── platform-backends/

schemas/
├── core/
├── knowledge/
├── collaboration/
├── work/
├── agents/
├── events/
├── connectors/
├── approvals/
└── provenance/

sdk/
├── rust/
├── typescript/
├── python/
├── swift/
├── kotlin/
├── java/
└── cpp/

connectors/
├── filesystem/
├── obsidian/
├── browser/
├── webhook/
├── email-ingest/
├── meetings/
├── calendar/
├── github/
├── messaging/
└── career/

evals/
├── retrieval/
├── context/
├── memory/
├── agents/
├── automation/
├── interoperability/
├── security/
├── performance/
└── user-journeys/

conformance/
├── connector/
├── extension/
├── mcp/
├── a2a/
├── events/
├── schemas/
├── federation/
└── security/
```

Do not create empty scaffolding for this entire map before the audit. Create boundaries only when a real implementation or migration step requires them.

---

## 53. Obsidian Relationship

Obsidian remains a first-class human authoring environment, not a permanent architectural requirement or competitor that must be replaced immediately.

### Stage 1 — Read-only integration

- select one or more vaults;
- index Markdown, tags, aliases, links, embeds, canvases, and attachments;
- preserve exact paths and hashes;
- detect moves and renames;
- never alter files;
- expose evidence-backed search and AI context.

### Stage 2 — Stable identity and metadata

- internal stable IDs in sidecar state by default;
- optional frontmatter IDs;
- duplicate detection;
- canonical source bindings;
- support link/path changes;
- map Obsidian concepts into public schemas without losing original form.

### Stage 3 — Guarded bidirectional editing

- exact diff preview;
- formatting-preserving writes;
- write-loop prevention;
- conflict detection;
- version/rollback;
- AI edits remain proposals;
- no hidden conversion into proprietary format.

### Stage 4 — Native workspace parity

MindVault may later offer a complete first-party editor with:

- Markdown/open export;
- blocks and structured content;
- collaborative editing;
- source citations;
- entity and decision views;
- agent collaboration;
- offline/native performance.

Users should not have to abandon Obsidian before MindVault’s human experience is genuinely better for their workflow.

---
# Part VIII — Sequential Development and Migration Roadmap

## 54. Program Rules

1. Preserve the existing repository and user data before architectural work.
2. Do not begin broad feature implementation until the product constitution, repository truth audit, target domain model, and migration strategy are approved.
3. Ship vertical slices through real production paths rather than isolated demos.
4. Every phase has measurable exit criteria.
5. Prefer incremental replacement behind stable contracts over a blind rewrite.
6. Maintain a compatibility test corpus for every historical data format.
7. Keep the application usable during migration wherever practical.
8. Do not introduce C++, microservices, federation, CRDTs, or new databases without a demonstrated requirement and benchmark.
9. Security, privacy, accessibility, backup, and recovery are implementation requirements—not end-of-project audits.
10. A phase is not complete because code exists; it is complete when the production path, evidence, tests, operations, and user workflow meet the exit criteria.

---

## 55. Phase 0 — Baseline Protection, Ownership, and Repository Safety

### Objectives

- preserve every current asset;
- establish the exact public/private and license boundary;
- prevent accidental data loss;
- inventory code, dependencies, data, and contributors;
- create a reproducible legacy baseline.

### Work

| Workstream | Deliverable |
|---|---|
| Git baseline | Immutable tag, commit hash, bundle, source archive, branch/working-tree record |
| Repository visibility | Deliberate private/new-repo transition plan |
| Data inventory | Every database, vault, artifact, key, cache, config, and migration path |
| Authorship/IP | Contributor, generated-code, copied-code, and provenance register |
| Dependencies | SBOM, licenses, vulnerabilities, maintenance status |
| Secrets | Secret/credential/private-path scan and remediation |
| Build | Reproducible build and packaging instructions |
| Runtime | Reproducible local start and smoke test |
| Data recovery | Legacy backup and restore runbook |
| Branch protection | No destructive force-push or unreviewed migration path |

### Exit criteria

- exact public baseline preserved;
- all uncommitted work protected;
- no unresolved critical secrets in tracked history or active configuration;
- legacy build reproduced on a clean environment;
- test data and real user data locations understood;
- full backup restored successfully;
- dependency/license report exists;
- ownership/provenance issues are recorded, not silently assumed away.

---

## 56. Phase 1 — Constitution, Boundaries, Naming Brief, and Commercial Strategy

### Required documents

- `PRODUCT_CONSTITUTION.md`
- `PRODUCT_BOUNDARIES.md`
- `INTEROPERABILITY_CONSTITUTION.md`
- `DATA_OWNERSHIP_CONTRACT.md`
- `AI_AUTHORITY_MODEL.md`
- `SOURCE_AUTHORITY_MODEL.md`
- `PRIVACY_CONTRACT.md`
- `EXTENSION_TRUST_MODEL.md`
- `NAMING_BRIEF.md`
- `LICENSING_AND_COMMERCIALIZATION_OPTIONS.md`

### Decisions

- target user sequence: owner/power user → small trusted team → organization;
- Vault versus Space boundary;
- MindVault versus DevX boundary;
- core versus optional modules;
- private/self-hosted/managed deployment options;
- future license architecture;
- what is open specification versus proprietary implementation;
- platform naming requirements;
- data sale/training/telemetry prohibitions;
- human-authority and agent-autonomy limits.

### Exit criteria

- one approved north star;
- one approved architecture boundary map;
- one approved set of immutable product laws;
- conflicting legacy assumptions explicitly superseded;
- naming and licensing work can proceed without changing core architecture.

---

## 57. Phase 2 — Complete Repository Truth Audit

### Audit areas

| Area | Required output |
|---|---|
| Source/code inventory | Crates, applications, connectors, scripts, generated code, tests |
| Architecture | Actual dependency and process diagrams |
| Production call paths | User action to state change, AI call, connector action, and response |
| Data | Schemas, migrations, invariants, ownership, backups |
| Search | Exact/full-text/vector/graph behavior and consistency |
| AI | Providers, local models, fallbacks, prompts, model-data flow |
| Memory | What is captured, consolidated, retrieved, changed, and deleted |
| Security | Threat model, authentication, authorization, encryption, keys, secrets |
| Sync/federation | Current consistency, conflicts, failure recovery, trust |
| Relay/collaboration | Channels/messages/adapters and real integration state |
| Agents | Proposals, watcher behavior, permissions, autonomous action paths |
| MCP/APIs | Protocol versions, capabilities, auth, direct writes, gaps |
| Plugins | Sandboxing, permissions, update model, registry, supply chain |
| UI/UX | Every route, workflow, component, design-system gap |
| Accessibility | Automated plus manual baseline |
| Performance | Startup, search, ingest, memory, CPU, battery, large corpus |
| Reliability | Crashes, retries, corruption, degraded modes |
| DevOps/release | CI, signing, packaging, updates, observability |
| Documentation | Claim-versus-implementation matrix |
| Dead code | Unreachable, unused, superseded, or duplicate behavior |

### Required artifacts

- `CURRENT_STATE.md`
- `FEATURE_TRUTH_MATRIX.md`
- `ARCHITECTURE_MAP.md`
- `PRODUCTION_CALL_PATHS.md`
- `DATA_FLOW_MAP.md`
- `SECURITY_THREAT_MODEL.md`
- `PRIVACY_AND_EGRESS_MAP.md`
- `DEPENDENCY_LICENSE_SBOM_REPORT.md`
- `DATA_MIGRATION_AUDIT.md`
- `SYNC_AND_RECOVERY_AUDIT.md`
- `INTEROPERABILITY_AUDIT.md`
- `UI_UX_ACCESSIBILITY_AUDIT.md`
- `PERFORMANCE_BASELINE.md`
- `KEEP_REFACTOR_MODULARIZE_REPLACE_RETIRE.md`
- `BLOCKER_REGISTER.md`

### Exit criteria

- every advertised feature classified;
- every canonical write path known;
- every external data flow known;
- every persistent store known;
- real test status known;
- no architecture decision depends on a presumed but unverified implementation.

---

## 58. Phase 3 — Architecture Decision Records and Benchmark Design

### Required ADRs

1. Product constitution and boundary model.
2. Vault, Space, Workspace, and Node hierarchy.
3. Canonical typed domain model.
4. Personal Markdown/file authority.
5. Personal SQLite authority.
6. Shared PostgreSQL authority.
7. Artifact/content-addressed storage.
8. Append-only event and Trust Ledger.
9. FTS5 versus Tantivy or alternative.
10. Vector backend and embedding lifecycle.
11. Graph projection and algorithms.
12. Rust/C++ boundary.
13. Tauri/web versus native clients.
14. Collaborative-document representation.
15. Personal sync and shared offline model.
16. Federation and node trust.
17. Local/cloud model routing.
18. MCP versioning and bidirectional gateway.
19. A2A adapter and external agent trust.
20. Connector/extension runtime and manifest.
21. Policy, ContextGrant, and ToolGrant model.
22. Proposal, approval, and Action Outbox.
23. Public API and event contracts.
24. Open/proprietary component boundary.
25. Legacy migration and compatibility.

### ADR standard

Every ADR includes:

- problem;
- current state;
- decision drivers;
- alternatives;
- evidence/measurements;
- security/privacy/accessibility implications;
- operational and migration implications;
- chosen direction;
- rejected alternatives;
- reversal path;
- acceptance criteria.

### Exit criteria

- architecture is decided by evidence and reversible interfaces;
- benchmarks exist before backend/language substitutions;
- migration path is coherent from the actual repository state.

---

## 59. Phase 4 — Domain Model v2 and Compatibility Layer

### Deliver

- stable typed IDs and global resource URIs;
- Workspace/Vault/Space hierarchy;
- Actor/Principal identity model;
- SourceArtifact, Document, Block, Entity, Claim, Relationship;
- Message/Channel/Thread/Meeting;
- Project/Task/Milestone/Decision/Risk;
- Agent/WorkOrder/Run/Artifact;
- ContextGrant/ToolGrant/Proposal/Approval/Receipt;
- bitemporal metadata;
- sensitivity and retention;
- source bindings;
- versioned schemas;
- legacy generic-node compatibility adapter;
- migration dry-run/report/rollback.

### Exit criteria

- representative legacy datasets migrate and round-trip without loss;
- old clients can operate through compatibility boundaries or receive clear migration errors;
- typed invariants are enforced transactionally;
- every new object exports to a documented format.

---

## 60. Phase 5 — Evidence, Canonical Storage, and Trust Ledger

### Deliver

- immutable/evidence-preserving ingest;
- content-addressed artifact storage;
- canonical personal SQLite store;
- shared-store contract and initial PostgreSQL implementation;
- append-only command/event/audit ledger;
- version history and rollback;
- source-authority registry;
- materialization policies;
- checksums and corruption detection;
- encrypted backup and verified restore;
- projection rebuild framework.

### Exit criteria

- original evidence is never overwritten by derived content;
- exact source identity and deduplication are reliable;
- all canonical mutations produce events and receipts;
- all derived indexes can be deleted and rebuilt;
- backup/restore and migration are proven on realistic datasets.

---

## 61. Phase 6 — Retrieval, Temporal Truth, and Context Compiler

### Deliver

- exact lookup;
- structured relational queries;
- benchmarked full-text search;
- versioned vector retrieval;
- graph expansion;
- temporal validity filtering;
- candidate fusion and reranking;
- source-authority and contradiction checks;
- evidence-coverage validation;
- permission-aware ContextPackage;
- ContextCapsule v1;
- cross-source citations;
- retrieval evaluation harness.

### Exit criteria

- exact retrieval is effectively perfect on the controlled corpus;
- citations support claims at the defined quality threshold;
- superseded facts do not outrank current truth without explicit historical intent;
- unauthorized records never appear;
- ContextPackages disclose source, freshness, omissions, and restrictions.

---

## 62. Phase 7 — Personal Vault and Obsidian Vertical Slice

### Deliver

- read-only Obsidian/filesystem connector;
- Markdown/link/alias/embed/canvas parsing;
- attachment mapping;
- file watcher;
- stable IDs and source bindings;
- exact and semantic search;
- citation-backed Ask experience;
- contradiction/staleness review;
- candidate memory proposals;
- audit and rollback;
- full export/restore.

### Exit criteria

- indexing does not modify user files;
- rescanning is idempotent;
- moves/renames preserve identity;
- answers cite exact passages;
- AI cannot directly mutate the vault;
- index loss is recoverable;
- Obsidian continues to work normally.

---

## 63. Phase 8 — Universal Ingestion and Connector Runtime

### Deliver

- manifest v2 parser;
- isolated connector runtime;
- secret broker;
- generic webhook;
- generic REST/OpenAPI connector;
- filesystem/dropbox connector;
- email ingestion;
- browser/share-extension contracts;
- event normalization;
- connector health/cursors/retries;
- source bindings and materialization UI;
- conformance suite;
- signing/package flow.

### First target connectors

1. Obsidian/filesystem.
2. One selected meeting tool.
3. A generic transcript/interview import.
4. Gmail/email ingestion.
5. Google Calendar.
6. GitHub.
7. Slack Relay.

### Exit criteria

- vendor-specific code does not enter canonical domain logic;
- duplicate delivery creates no duplicate canonical state;
- uninstall/reinstall is safe;
- connector credentials can be revoked independently;
- data flow, retention, and source authority are visible.

---

## 64. Phase 9 — Bidirectional MCP and A2A

### MCP deliverables

- local stdio server;
- remote authenticated transport;
- MCP client manager;
- connection profiles;
- resources, tools, prompts, progress, cancellation, elicitation as supported;
- ContextGrant/ToolGrant enforcement;
- proposal-only canonical writes;
- complete receipts;
- protocol-version adapters and conformance.

### A2A deliverables

- Agent Card registry;
- trust/approval workflow;
- task initiation and mapping;
- streaming/status updates;
- artifact ingestion;
- cancellation;
- input/auth-required states;
- short-lived credentials;
- external-agent conformance tests.

### Exit criteria

- at least two independent MCP clients retrieve governed context;
- MindVault successfully uses at least two external MCP servers;
- at least two independent agent runtimes accept and report WorkOrders;
- external protocols cannot bypass policy or direct-write rules;
- version incompatibilities fail clearly and safely.

---

## 65. Phase 10 — Workspace, Space, Identity, and Membership Foundation

### Deliver

- Workspace and organization boundaries;
- Personal Vault representation;
- shared Space creation;
- humans, devices, agents, apps, connectors as actors;
- memberships and roles;
- attribute/capability policy;
- invitations and guests;
- Space-specific retention and model rules;
- ContextGrant and ToolGrant UI;
- local/shared boundary enforcement;
- initial shared PostgreSQL service and local cache.

### Exit criteria

- private Vault leakage into shared Spaces is zero in adversarial tests;
- membership removal revokes access and active grants;
- shared-state changes sync correctly to offline clients;
- Space export and deletion semantics are documented and tested.

---

## 66. Phase 11 — Native Collaboration

### Deliver

- channels;
- DMs/group DMs;
- threads;
- mentions/reactions;
- attachments;
- unread/read state;
- search;
- guest participation;
- notification routing;
- message retention/edit/delete policies;
- agent presence;
- conversation-to-task/decision/document/proposal.

### Exit criteria

- realtime and offline/reconnect behavior is reliable;
- no duplicate or out-of-order user-visible events beyond defined semantics;
- message search respects current permissions and deletion policy;
- agent messages are clearly attributed;
- collaboration is accessible by keyboard and screen reader.

---

## 67. Phase 12 — Documents, Structured Knowledge, and Work Graph

### Deliver

- collaborative documents;
- comments and suggestions;
- structured collections/views;
- project/task/milestone/risk models;
- decision system and receipts;
- dependencies and verification;
- list/board/timeline/calendar/graph views;
- meeting-to-execution workflow;
- knowledge-debt review;
- Time Machine/work replay foundations.

### Exit criteria

- documents export to open formats;
- concurrent editing does not lose human work;
- tasks retain owner/delegate/reviewer semantics;
- decisions preserve evidence and supersession;
- work state can be reconstructed from receipts/events.

---

## 68. Phase 13 — Agent Control Plane

### Deliver

- agent registry and profiles;
- skills and compatibility;
- WorkOrder builder;
- ContextPackage/ToolGrant compilation;
- plan proposal and approvals;
- Run lifecycle and cards;
- budgets and resource accounting;
- progress, cancellation, pause, resume;
- artifacts and verification;
- multi-agent patterns;
- evaluation history;
- kill switch and suspension;
- DevX Runtime/Claude Code/Codex integration profiles.

### Exit criteria

- agents cannot exceed run grants;
- cancellation revokes credentials;
- every external effect is attributable;
- artifacts and verification link to the run;
- failed/partial work cannot appear as completed;
- human accountability remains explicit.

---

## 69. Phase 14 — Delegated Communication, Attention, and Automation

### Deliver

- Action Outbox;
- delegated-message identity;
- draft/approve/template/bounded-autonomy modes;
- internal/external recipient policies;
- schedules and triggers;
- deterministic and AI workflow steps;
- shadow mode and policy simulation;
- retries/idempotency/compensation;
- Attention Router;
- scheduled project reports;
- Slack/Teams/email/Discord delivery adapters;
- complete external receipts.

### Exit criteria

- no unauthorized external messages or actions;
- retries do not duplicate side effects;
- low-confidence reports escalate rather than auto-send;
- users can stop all automations immediately;
- shadow-mode evaluation precedes expanded autonomy;
- notification load is measured and controlled.

---

## 70. Phase 15 — Multi-Device Sync and Federation

### Deliver

- device identities;
- signed personal operation synchronization;
- encrypted blob sync;
- shared offline command queue;
- deterministic conflict resolution;
- member/device revocation;
- node capability negotiation;
- signed ContextCapsules;
- remote permission-aware query;
- selective event replication;
- federation trust policy;
- cross-organization guest/capsule workflow.

### Exit criteria

- convergence under loss, duplication, reordering, and offline edits;
- no silent conflict loss;
- revoked devices cannot continue syncing;
- capsules expire/revoke correctly;
- unauthorized federated queries return no data;
- restore and federation state remain consistent.

---

## 71. Phase 16 — Native Clients and Hardware Optimization

### Deliver

- stable native-client API contracts;
- premium macOS/iOS SwiftUI/AppKit client;
- Kotlin/Compose Android client;
- resource capability profiler;
- model registry/router;
- local compute backends;
- benchmarked C++ kernels only where warranted;
- battery/thermal policies;
- accessibility validation;
- cross-platform feature-parity matrix.

### Exit criteria

- native clients materially improve platform experience;
- no C++ component exists without benchmark justification and safe FFI;
- resource governor prevents harmful background behavior;
- local/cloud route is transparent;
- core functionality remains portable and exportable.

---

## 72. Phase 17 — Enterprise, Ecosystem, and Operations

### Deliver

- SSO/OIDC;
- SCIM;
- organization roles and policy templates;
- audit export;
- data residency controls;
- DLP and retention;
- legal-hold support where required by product direction;
- enterprise/self-hosted deployment;
- high availability and disaster recovery;
- official/community/private registries;
- extension verification and revocation;
- marketplace economics only after trust foundations;
- support, incident, and security-response procedures.

### Exit criteria

- enterprise controls do not weaken personal ownership or self-hosting;
- extensions have verifiable supply-chain metadata;
- incident response and key compromise drills succeed;
- hosted and self-hosted deployments pass the same core conformance suite.

---

## 73. Phase 18 — Alpha, Beta, and Public Release

### Private owner alpha

Use daily with:

- Obsidian;
- meeting capture;
- ChatGPT and Claude;
- Claude Code, Codex, Cursor;
- DevX projects;
- personal/career research;
- one shared project Space;
- scheduled internal reports.

### Trusted-team alpha

Add a small team and validate:

- collaboration;
- role/guest permissions;
- messages and work graph;
- agent delegation;
- connector behavior;
- notification load;
- shared backup/recovery.

### External private beta gates

- no unresolved critical data-loss/security bugs;
- successful migration from current public baseline;
- verified backup/restore;
- accessibility sign-off;
- performance across baseline hardware;
- privacy/security documentation;
- support/recovery runbooks;
- name and trademark clearance;
- signed installers and update path;
- export/deletion verification;
- conformance results published or available.

### Public release principle

Release breadth follows verified reliability. A smaller product with trustworthy evidence, migration, retrieval, agents, and Spaces is preferable to a huge unverified feature catalog.

---
# Part IX — First Vertical Slices and Acceptance Contracts

## 74. Vertical Slice A — Obsidian to Governed Context

### Objective

Prove the personal knowledge foundation before broad collaboration.

### Flow

1. Select an Obsidian vault or local Markdown workspace.
2. Index it without modifying source files.
3. Preserve exact file/attachment identity and provenance.
4. Retrieve by path, alias, title, ID, content, and semantic meaning.
5. Ask a question.
6. Return an answer with exact clickable source passages.
7. Identify contradiction, staleness, or superseded facts.
8. Propose a structured memory/claim update.
9. Show the evidence and diff.
10. Accept, edit, reject, or defer.
11. Record the decision and receipt.
12. Delete/rebuild indexes and demonstrate equivalent retrieval.
13. Export and restore the workspace.

### Acceptance criteria

| Criterion | Required result |
|---|---|
| Source files modified during indexing | Zero |
| Duplicate imports after repeated scan | Zero |
| Move/rename identity preservation | Verified |
| Exact known-item retrieval | Effectively perfect |
| Citation support | Every factual answer traceable |
| Direct AI canonical writes | Zero |
| Derived-index dependency | None for data survival |
| Unauthorized context | Zero |
| Offline capture/search/review | Functional |
| Backup/restore | Verified |
| Audit | Complete ingest/query/proposal/decision trail |

---

## 75. Vertical Slice B — External Meeting/Interview Tool to Any AI

### Objective

Prove universal ingestion, source authority, structured outcomes, MCP access, and approval.

### Flow

1. Install a meeting/transcript connector through a signed manifest.
2. Authenticate via API, webhook, email, file import, or local companion.
3. Receive the meeting event and deduplicate it.
4. Preserve the original artifact or authoritative source reference.
5. Normalize participants, transcript, metadata, and source segments.
6. Resolve people, organization, project, and Space.
7. Generate candidate summary, decisions, tasks, commitments, and follow-ups.
8. Human approves/corrects structured outcomes.
9. Canonical context updates with provenance.
10. ChatGPT, Claude, Codex, Cursor, DevX, or another MCP client retrieves approved context.
11. Raw transcript access follows its own sensitivity/retention policy.
12. Connector is removed and canonical approved outcomes remain intact.

### Acceptance criteria

| Criterion | Required result |
|---|---|
| Vendor-specific object in core domain | None |
| Duplicate event processing | None |
| Lost source provenance | None |
| Unattributed participant/content | Flagged, not silently guessed |
| Unsupported AI canonical change | None |
| MCP clients tested | At least two independent clients |
| Source authority | Explicit and inspectable |
| Connector uninstall | Safe |
| Retention/deletion behavior | Tested |
| Complete import and approval receipts | Required |

---

## 76. Vertical Slice C — Project Space to Agent Execution and Team Report

### Objective

Prove human-agent collaboration, structured work, external execution, verification, and reporting.

### Scenario

1. Create a project Space.
2. Add Hassan, one collaborator, Build Agent, Review Agent, and Project Reporter.
3. Create `#architecture` channel and a project.
4. Post: `@BuildAgent inspect the current relay system and propose the smallest migration for shared Spaces.`
5. Convert the request into a WorkOrder.
6. Hassan remains accountable owner; Build Agent is delegate.
7. Compile repository and architecture context.
8. Grant read access and limited execution through DevX Runtime/Claude Code/Codex.
9. Agent proposes a plan.
10. Hassan approves.
11. Agent executes and posts structured progress.
12. Review Agent independently checks implementation, tests, security, migration, and docs.
13. Build Agent returns patch/branch, tests, architecture notes, and risks.
14. Project Reporter prepares a verified update.
15. Hassan approves or policy authorizes internal delivery.
16. Update posts to the Space and optionally relays to Slack/email.
17. Accepted decisions/artifacts enter project memory.
18. Run and external-message receipts close the workflow.

### Acceptance criteria

| Requirement | Standard |
|---|---|
| Private-context leakage | Zero |
| Agent grant expansion | Impossible |
| Human accountability | Explicit throughout |
| Actor/principal attribution | Complete |
| Cancellation | Stops run and revokes credentials |
| Tool/action audit | Complete |
| Evidence | Tests and source artifacts required |
| External messaging | Cannot exceed policy/approval |
| Report basis | Verified project state, not chat inference alone |
| Knowledge update | Proposal-based and reversible |
| Failure/retry | Safe and idempotent |
| Notification duplicates | None |

---

## 77. Vertical Slice D — Delegated External Communication

### Objective

Prove safe on-behalf-of communication and Action Outbox behavior.

### Flow

1. A meeting creates an approved follow-up task.
2. Follow-up Agent drafts a message from selected evidence.
3. The Outbox shows actor, principal, recipients, data used, and exact payload.
4. Human edits and approves.
5. Secret broker sends through email or messaging connector.
6. Delivery receipt returns.
7. External reply is linked to the original action and project context.
8. Follow-up task updates only after confirmed send/delivery semantics.
9. Retry simulation proves no duplicate message.
10. Revocation simulation blocks a pending send.

### Acceptance criteria

- no raw secret reaches the agent;
- no send before authorization;
- exact content and recipients preserved in receipt;
- duplicate retry prevented;
- failure and partial delivery represented honestly;
- downstream reply linked by correlation ID;
- human can revoke future similar authority.

---

## 78. Vertical Slice E — Federated Query Across Personal and Shared Nodes

### Objective

Prove that the system can answer across distributed sources without centralizing all raw data.

### Question

> What commitments did we make to Organization X across meetings, email, project documents, shared messages, tasks, and private notes that I explicitly authorize for this query?

### Flow

1. User selects purpose and authorized personal/private scope.
2. Query planner identifies relevant nodes.
3. Each node receives a scoped subquery.
4. Nodes search locally and return ranked evidence plus freshness.
5. Local Trust Plane validates signatures, permissions, and source authority.
6. Results are deduplicated and reranked.
7. Contradictions and stale commitments are surfaced.
8. Context compiler produces a citation-backed answer.
9. No raw mailbox, vault, or complete transcript archive is copied centrally.
10. Query receipt records nodes, grants, evidence, and withheld sources.

### Acceptance criteria

- no node receives broader scope than required;
- no unauthorized private source appears;
- cross-node identity resolution is explainable;
- duplicate commitments merge without losing source links;
- stale/superseded state is visible;
- unavailable nodes are disclosed;
- answer remains reproducible from recorded evidence handles where retention permits.

---

# Part X — Evaluation, Benchmarks, and Release Gates

## 79. Evidence-First Evaluation Program

The product must be evaluated with the same discipline expected from a serious software buyer’s guide: official implementation evidence, reproducible tests, independent user validation, explicit uncertainty, and separation of implemented capability from marketing language.

### Evidence labels

| Label | Meaning |
|---|---|
| E1 | Independently reproduced in controlled test |
| E2 | Verified in production integration test |
| E3 | Implemented and covered by automated tests |
| E4 | Documented in code or design but not sufficiently validated |
| E5 | Prototype/experimental |
| C | Claim without sufficient verification |
| D | Disputed or conflicting evidence |
| U | Unknown |

No release note should call a capability production-ready when its strongest evidence is E4, E5, C, D, or U.

---

## 80. Golden Evaluation Corpus

Create a versioned synthetic and consented corpus containing:

- ordinary personal notes;
- project specifications;
- meetings and transcripts;
- messages and threads;
- emails;
- PDFs;
- images and OCR cases;
- source code;
- tables and spreadsheets;
- decisions and tasks;
- renamed/moved files;
- duplicates;
- contradictory and superseded claims;
- stale information;
- sensitive records;
- multilingual material;
- malformed documents;
- prompt-injection content;
- malicious extension payloads;
- permission boundaries;
- offline/concurrent edits;
- failed connector deliveries;
- external-action retries.

### Corpus governance

- no private real data in public test fixtures;
- synthetic identities and secrets;
- versioned expected results;
- mutation/fuzz variants;
- separate adversarial corpus;
- documented licenses and provenance;
- deterministic subset for CI;
- larger realistic corpus for nightly/release tests.

---

## 81. Retrieval and Context Metrics

| Metric | Purpose |
|---|---|
| Exact lookup precision | Known object retrieval correctness |
| Recall@K | Relevant-result coverage |
| MRR | Rank of first correct result |
| nDCG | Overall ranking quality |
| Entity-resolution precision/recall | Cross-source identity quality |
| Citation precision | Cited evidence supports the claim |
| Citation completeness | Important claims have sufficient support |
| Freshness accuracy | Current truth correctly selected |
| Historical accuracy | Correct state at requested time |
| Contradiction precision/recall | Detect real conflicts without excessive false alarms |
| Source-authority correctness | Right system treated as authoritative |
| Permission leakage | Unauthorized material returned |
| Context efficiency | Relevant supported information per token/byte |
| Context omission accuracy | Important authorized context not omitted |
| Abstention quality | Unsupported questions declined or qualified |
| Cross-node completeness | Relevant federated nodes/sources included |

### Critical targets

- unauthorized-context leakage: zero;
- exact ID/hash lookup: effectively perfect;
- unsupported canonical claim from an AI path: zero;
- citation validity on controlled factual corpus: at least the release threshold defined after baseline, with critical domains held to stricter gates;
- stale/superseded truth errors: tracked as a release-blocking class for high-confidence answers.

---

## 82. Ingestion and Connector Metrics

- duplicate-object rate;
- missed-event rate;
- schema-validation failure rate;
- source-binding correctness;
- participant/entity resolution;
- cursor recovery;
- webhook replay handling;
- rate-limit behavior;
- retry safety;
- latency from external event to normalized record;
- raw-payload retention compliance;
- connector health accuracy;
- uninstall/reinstall safety;
- external deletion/tombstone correctness;
- permission-scope correctness;
- source-authority conflict rate.

### Connector release gate

Every official connector must pass:

- deterministic fixtures;
- replay and duplication;
- partial failure;
- expired credential;
- rate limit;
- source deletion;
- API version change fixture;
- untrusted/malformed payload;
- migration from prior connector version;
- uninstall/reinstall;
- data export and revocation.

---

## 83. Agent and Automation Metrics

| Metric | Purpose |
|---|---|
| WorkOrder completion quality | Outcome against acceptance criteria |
| Plan approval/revision rate | Plan usefulness and calibration |
| Artifact acceptance rate | Downstream usefulness |
| Verification escape rate | Defects accepted as correct |
| Unauthorized-action count | Must be zero |
| Grant-violation attempts | Detection and prevention |
| Hallucinated-action/report rate | Accuracy of operational claims |
| Cost and duration predictability | Budget control |
| Cancellation latency | Kill-switch effectiveness |
| Recovery/resume success | Long-running robustness |
| Duplicate side-effect rate | Idempotency effectiveness |
| Human correction burden | Usability and trust |
| Escalation precision | Stops when it should without excessive friction |
| Agent-to-agent handoff loss | Context and artifact preservation |
| Shadow-to-production drift | Whether policy remains safe after activation |
| Scheduled-report factuality | Alignment to verified project state |

### Autonomy gate

A workflow may move from shadow mode to bounded autonomy only when:

- enough representative runs exist;
- false-positive/false-action risk is below the defined threshold;
- duplicate-side-effect rate is zero;
- permission behavior is proven;
- rollback/compensation is tested;
- notification burden is acceptable;
- owner explicitly approves the policy version.

---

## 84. Collaboration and Notification Metrics

- message delivery and ordering;
- offline queue recovery;
- unread-state correctness;
- thread consistency;
- attachment integrity;
- permission propagation latency;
- guest-expiry enforcement;
- search deletion/retention correctness;
- notification precision;
- duplicate notification rate;
- action-required miss rate;
- time to acknowledge;
- quiet-hours violations;
- user mute/defer effectiveness;
- project-report factual accuracy;
- collaboration task completion;
- channel-to-knowledge conversion quality.

---

## 85. Sync, Federation, and Recovery Metrics

- convergence after concurrent edits;
- silent data-loss count;
- conflict-detection precision;
- operation duplication/reordering tolerance;
- offline duration tolerance;
- device revocation latency;
- encrypted blob integrity;
- cross-node query latency;
- capsule expiration/revocation;
- signature verification;
- unavailable-node disclosure;
- selective-replication correctness;
- full backup restore;
- selective restore;
- point-in-time recovery;
- migration integrity;
- post-restore index equivalence.

Critical target: **zero silent data loss** in the supported synchronization and migration test matrix.

---

## 86. Security and Privacy Evaluation

### Required test families

- authentication bypass;
- privilege escalation;
- cross-Vault/Space leakage;
- confused-deputy attacks;
- context-grant overreach;
- tool-grant parameter bypass;
- prompt injection;
- indirect prompt injection through connectors;
- secret exfiltration;
- extension sandbox escape;
- network allowlist bypass;
- supply-chain tampering;
- replay attacks;
- malicious federation peer;
- unsafe model/provider routing;
- retention/deletion failure;
- log/audit leakage;
- insecure backup/recovery;
- cancellation race;
- duplicate external transaction;
- malicious file/archive ingestion;
- denial of service/resource exhaustion.

### Security gates

- critical/high findings resolved or explicitly blocked from release;
- encryption and key recovery tested;
- no raw secret in model prompts or agent-visible logs;
- all external actions attributable;
- revocation terminates active sessions/runs within defined SLO;
- SBOM and signatures attached to releases;
- extension permissions match observed behavior;
- telemetry and model-data use match declared settings;
- independent review before public multi-user or enterprise release.

---

## 87. Performance and Efficiency Metrics

### Workload levels

| Dimension | Levels |
|---|---|
| Canonical records | 10,000; 100,000; 1,000,000+ |
| Documents/files | Small personal; large professional; team archive |
| Artifacts | 1 GB; 10 GB; 100 GB+ |
| Concurrent users | 1; small team; organization target |
| Concurrent agents | 1; 5; 20; stress target |
| Connectors | 1; 10; 50+ |
| Offline duration | Minutes; days; weeks |

### Measurements

- cold and warm startup;
- initial and incremental ingest;
- indexing throughput;
- query median/p95/p99;
- ContextPackage compilation;
- idle and active memory;
- CPU/GPU/NPU use;
- disk footprint;
- battery/energy impact;
- sync throughput;
- event delivery latency;
- realtime fan-out;
- agent scheduling overhead;
- model cold/warm latency;
- C++ versus Rust/backend comparison;
- degradation under resource pressure;
- recovery after process/network failure.

### Performance publication rule

Every benchmark must state:

- exact hardware;
- OS/runtime versions;
- dataset/corpus;
- model and quantization;
- cache state;
- concurrency;
- sample size;
- median and tail latency;
- standard deviation/confidence where appropriate;
- power/energy method where claimed.

---

## 88. UX, Accessibility, and Human-Factors Evaluation

### Core task studies

- capture and retrieve a note;
- inspect evidence behind an answer;
- approve/reject an AI proposal;
- create a project Space;
- invite a guest safely;
- delegate work to an agent;
- understand why an agent is blocked;
- cancel a run;
- approve an external message;
- configure a connector;
- understand data flow/retention;
- resolve a sync conflict;
- restore lost content;
- export and leave the system.

### Measures

- task completion;
- time on task;
- error rate;
- recovery rate;
- comprehension of actor/principal;
- comprehension of local/cloud processing;
- comprehension of permissions;
- trust calibration;
- notification overload;
- proposal-review burden;
- accessibility issues;
- perceived control;
- post-task confidence.

### Accessibility gate

No critical workflow ships without keyboard completion and manual screen-reader verification on its supported platforms.

---

## 89. Release Gate Matrix

| Area | Blocking condition |
|---|---|
| Data | Any unresolved silent-loss path |
| Migration | Representative legacy fixture does not round-trip |
| Backup | Full restore not verified |
| Security | Unresolved critical/high issue affecting shipped scope |
| Permissions | Cross-boundary leakage or agent grant bypass |
| AI writes | Direct unsupported canonical write possible |
| External actions | Unauthorized or duplicate side effect possible |
| Retrieval | Critical source/citation failures above threshold |
| Sync | Unsupported conflict silently overwrites human work |
| Accessibility | Critical workflow not keyboard/screen-reader operable |
| Interoperability | Public contract lacks conformance/compatibility tests |
| Operations | No rollback, monitoring, or incident path |
| Privacy | Actual data flow contradicts product settings/documentation |
| UX | User cannot identify actor, source, action, or recovery path |

---
# Part XI — Ownership, Licensing, Naming, Business Model, and Governance

## 90. Ownership and Future Licensing Strategy

MindVault is Hassan’s product. The future codebase may be private, proprietary, dual-licensed, open-core, or selectively open according to product strategy.

### Immediate repository strategy

1. Preserve and tag the exact current public Apache-2.0 baseline.
2. Generate a Git bundle, source archive, dependency manifest, and SBOM.
3. Audit commit authorship, outside contributions, generated/copied material, and third-party licenses.
4. Decide whether to archive the current public repository, make it private where platform rules permit, or leave a legacy public mirror.
5. Create a private next-generation repository under the selected product/company organization.
6. Import only code with clear provenance and preserve required notices.
7. Establish contributor terms before accepting future outside contributions.
8. Record the license and provenance of every model, dataset, icon, font, design asset, dependency, and generated artifact.

### Recommended architecture of openness

| Component | Initial direction |
|---|---|
| Main end-user applications | Proprietary during product formation |
| Context/authority kernel | Proprietary initially |
| Hosted sync/collaboration services | Proprietary service |
| Public object schemas | Open specification |
| Connector/extension manifest | Open specification |
| Event and receipt formats | Open specification |
| ContextCapsule format | Open specification |
| SDKs | Permissive license where strategically useful |
| Reference connectors | Selected permissive open source |
| Conformance suite | Open |
| Import/export tools and formats | Open |
| MCP/A2A reference adapters | Open or partially open where useful |
| Community extensions | Publisher-selected approved licenses |

### Why this balance

- Users gain portability and ecosystem confidence.
- Developers can build without reverse-engineering.
- Self-hosting remains credible.
- The product’s differentiated implementation, UX, policy, synchronization, and orchestration can remain commercially protected.
- The moat becomes trust and execution quality, not hostage data.

### Important legal/operational principle

Changing future repository visibility or licensing does not erase rights already granted to recipients of earlier public versions. Preserve the legacy boundary and build future proprietary value through new code, trademarks, services, designs, data models, tests, migrations, and product quality.

Final licensing and trademark actions should receive qualified legal review before public commercial release.

---

## 91. Rename and Brand Architecture

A broader platform name is likely warranted because “MindVault” suggests only private memory or notes and is crowded in adjacent software naming.

### Naming requirements

The platform name should:

- communicate connection, context, continuity, sovereignty, or coordination;
- support personal and organizational use;
- work for humans and agents;
- remain broad enough for Vaults, Spaces, Work, Agents, Connect, Relay, and Trust;
- be easy to pronounce, spell, and remember;
- avoid sounding medical, financial, surveillance-oriented, or purely developer-focused;
- support product-family naming;
- clear trademark, company, app-store, domain, package, repository, and social-handle checks;
- avoid confusing similarity with existing knowledge, AI, security, or collaboration products.

### Naming process

| Stage | Output |
|---|---|
| Brand strategy | Category, promise, audience, tone, differentiation |
| Naming territories | Context, continuity, trust, connection, coordination, sovereignty |
| Candidate generation | Broad set, not a single improvised name |
| Linguistic filtering | Pronunciation, spelling, international meaning, abbreviation |
| Technical filtering | Domains, package registries, GitHub, app IDs, command names |
| Market filtering | Search, app stores, competitor/confusion analysis |
| Legal filtering | Trademark screening in relevant classes and markets |
| User testing | Recall, trust, category association, pronunciation |
| Product-family design | Platform, modules, SDK, file format, company name |
| Migration plan | Code, packages, schemas, apps, docs, domains, redirects |

### Interim terminology

| Scope | Working name |
|---|---|
| Platform | MindVault Next |
| Architectural category | Sovereign Context Fabric |
| Private module | Vault |
| Collaboration module | Spaces |
| Interoperability module | Connect |
| Agent system | Agents |
| Governance module | Trust |
| Experimental neural memory | Memory Lab |

“MindVault” may remain as the private Vault module or legacy codename even if the platform receives a broader name.

---

## 92. Business and Deployment Model

### Product editions

| Edition | Target | Capabilities |
|---|---|---|
| Personal Local | Individual owner | Local Vault, Obsidian/files, local search, agents, connectors, export |
| Personal Sync | Individual multi-device | Encrypted sync, mobile/native clients, hosted relay option |
| Team Spaces | Small teams/projects | Shared Spaces, collaboration, Work, Agents, managed/server deployment |
| Organization | Companies and institutions | SSO, SCIM, policy, retention, audit, data residency, private registries |
| Self-Hosted | Privacy/control-focused users | Owner-operated personal/team services |
| Managed Private | Organizations needing isolation | Dedicated managed environment and support |
| Developer Platform | Builders/agents/apps | APIs, SDKs, conformance, extension distribution |

### Revenue candidates

- managed encrypted synchronization;
- hosted team collaboration;
- organization administration/compliance;
- managed agent execution and resource governance;
- premium native applications/features;
- enterprise support and deployment;
- verified extension distribution or commercial marketplace;
- private connectors/integrations;
- advanced backup/disaster recovery;
- usage-based external model/compute brokerage where transparent;
- professional migration and implementation services.

### Prohibited business incentives

- selling personal data;
- training unrelated models on user content by default;
- disabling export to increase lock-in;
- hiding model or compute costs;
- manipulating notification volume for engagement;
- forcing cloud storage for local-only capabilities;
- charging to recover or export the user’s own canonical data;
- ranking extensions secretly based on commercial payments.

---

## 93. Governance and Change Control

### Architecture governance

- constitutional changes require a formal proposal and explicit approval;
- ADRs govern major implementation decisions;
- public schemas follow semantic/versioned compatibility rules;
- extension capabilities require security review;
- new external-action classes require policy and threat-model review;
- autonomy increases require shadow-mode evidence;
- new canonical stores require architecture approval;
- new languages/runtimes require responsibility and operational ownership;
- every deprecation has migration and sunset policy.

### Product governance

- feature requests map to user outcomes and product boundaries;
- parity work distinguishes foundational parity from superficial checklist parity;
- no module may silently duplicate another source of truth;
- new notifications require attention-impact review;
- new data collection requires privacy/data-flow review;
- experimental functions are visibly marked and removable;
- metrics cannot override human authority or user ownership.

---

# Part XII — Immediate Execution Program

## 94. First 30 Ordered Actions

1. Freeze unplanned feature additions.
2. Record branch, working tree, remotes, tags, submodules, and local data locations.
3. Create verified repository and data backups.
4. Tag the current public baseline.
5. Produce SBOM, dependency-license inventory, and secret scan.
6. Create private next-generation repository/workspace.
7. Approve the Product Constitution in this document.
8. Supersede the single-owner/no-shared-state assumption.
9. Approve Vault + Space + Context Node architecture.
10. Approve DevX boundary.
11. Complete the repository truth audit.
12. Produce feature-truth and production-call-path matrices.
13. Establish a golden evaluation corpus and baseline benchmarks.
14. Write the required ADRs.
15. Define typed domain schemas and compatibility layer.
16. Implement source authority, evidence, and receipt foundations.
17. Make encryption and safe data defaults mandatory.
18. Implement the Obsidian-to-governed-context vertical slice.
19. Benchmark FTS/vector/graph storage choices.
20. Build ContextPackage and ContextCapsule v1.
21. Implement universal connector manifest/runtime and generic ingestion.
22. Integrate one meeting/interview transcript source.
23. Upgrade MCP to bidirectional, versioned, governed operation.
24. Implement A2A adapter over WorkOrders/Runs.
25. Build Space/identity/membership foundation.
26. Implement one project Space with channel, document, task, and decision.
27. Execute the first external agent WorkOrder through DevX/Claude Code/Codex.
28. Build the Action Outbox and verified internal Project Reporter.
29. Validate the four initial vertical slices and release gates.
30. Begin formal naming/trademark evaluation before external beta.

---

## 95. Mandatory Documentation Set

### Constitution and product

- `PRODUCT_CONSTITUTION.md`
- `PRODUCT_BOUNDARIES.md`
- `INTEROPERABILITY_CONSTITUTION.md`
- `DATA_OWNERSHIP_CONTRACT.md`
- `PRIVACY_CONTRACT.md`
- `AI_AUTHORITY_MODEL.md`
- `AUTONOMY_POLICY.md`
- `NAMING_BRIEF.md`

### Current state and audit

- `CURRENT_STATE.md`
- `FEATURE_TRUTH_MATRIX.md`
- `ARCHITECTURE_MAP.md`
- `PRODUCTION_CALL_PATHS.md`
- `DATA_FLOW_MAP.md`
- `SECURITY_THREAT_MODEL.md`
- `PRIVACY_AND_EGRESS_MAP.md`
- `DEPENDENCY_LICENSE_SBOM_REPORT.md`
- `DATA_MIGRATION_AUDIT.md`
- `SYNC_AND_RECOVERY_AUDIT.md`
- `INTEROPERABILITY_AUDIT.md`
- `UI_UX_ACCESSIBILITY_AUDIT.md`
- `PERFORMANCE_BASELINE.md`
- `BLOCKER_REGISTER.md`

### Target architecture

- `V2_ARCHITECTURE.md`
- `DOMAIN_MODEL.md`
- `SOURCE_AUTHORITY_MODEL.md`
- `STORAGE_AND_PROJECTION_MODEL.md`
- `CONTEXT_COMPILER.md`
- `CONTEXT_GRANTS_AND_TOOL_GRANTS.md`
- `TRUST_LEDGER_AND_RECEIPTS.md`
- `SPACE_AND_COLLABORATION_MODEL.md`
- `WORK_GRAPH.md`
- `AGENT_CONTROL_PLANE.md`
- `AUTOMATION_AND_ACTION_OUTBOX.md`
- `CONNECTOR_EXTENSION_MODEL.md`
- `MCP_A2A_PROTOCOL_BOUNDARIES.md`
- `SYNC_AND_FEDERATION_MODEL.md`
- `HARDWARE_RUNTIME.md`
- `V2_MIGRATION_PLAN.md`

### Operations and quality

- `EVALUATION_PLAN.md`
- `RELEASE_GATES.md`
- `BACKUP_RESTORE_RUNBOOK.md`
- `INCIDENT_RESPONSE.md`
- `KEY_RECOVERY_AND_ROTATION.md`
- `EXTENSION_SECURITY_REVIEW.md`
- `CONNECTOR_CONFORMANCE.md`
- `ACCESSIBILITY_VALIDATION.md`
- `SELF_HOSTING_GUIDE.md`
- `DATA_EXPORT_AND_EXIT.md`

---

## 96. Prioritized ADR Register

| Priority | ADR |
|---:|---|
| 1 | Product constitution and scope |
| 2 | Vault/Space/Workspace/Node hierarchy |
| 3 | Typed canonical domain model |
| 4 | Source authority and materialization |
| 5 | Personal and shared canonical storage |
| 6 | Evidence, provenance, and Trust Ledger |
| 7 | ContextGrant and ToolGrant authority model |
| 8 | Proposal/approval/Action Outbox model |
| 9 | Retrieval and Context Compiler |
| 10 | Legacy migration and compatibility |
| 11 | Connector/extension architecture |
| 12 | Bidirectional MCP architecture |
| 13 | A2A adapter and external agent trust |
| 14 | Collaboration and Work Graph |
| 15 | Agent Control Plane |
| 16 | Sync, collaborative docs, and federation |
| 17 | Rust/C++ and hardware backends |
| 18 | Native/client strategy |
| 19 | Open/proprietary licensing boundaries |
| 20 | Naming/product-family migration |

---

## 97. First Coding-Agent Mission

```text
MISSION: MINDVAULT NEXT — BASELINE, TRUTH AUDIT, AND ARCHITECTURE GATE

You are working on a user-owned product repository currently known as MindVault.
The owner may rename it, make future development private, change the future
license strategy, commercialize it, preserve or replace any implementation,
and use different languages where evidence warrants.

Do not begin broad feature implementation.

NORTH STAR

Build a sovereign, interoperable context and coordination fabric where humans,
teams, applications, devices, AI models, and agents can share governed context,
communicate, perform work, preserve evidence, and act under explicit human
authority.

LOCKED PRODUCT LAWS

1. The user owns data, keys, policies, exports, and model choices.
2. Core personal operation remains local-first and exportable.
3. Private Vault context never enters a shared Space without an explicit grant.
4. Original evidence is distinct from derived knowledge.
5. AI never silently writes canonical state.
6. Context access and action authority are separate.
7. Every consequential action identifies actor, principal, authority, evidence,
   result, and recovery path.
8. Every index is derived and rebuildable.
9. External protocols are adapters, not the internal domain model.
10. No extension receives direct database access.
11. Every first-party capability has command, query, event, policy, provenance,
    audit, export, and conformance behavior.
12. Rust remains the systems/authority kernel unless an ADR changes a boundary.
13. C++ is introduced only after identical-workload benchmark evidence.
14. Agents are visibly non-human and have accountable human sponsors.
15. Human accountability remains when execution is delegated.
16. DevX Runtime remains the engineering execution authority when used.
17. Do not infer completion from file names, checklists, or documentation.
18. When identity, policy, provenance, or state is uncertain, fail closed.

PHASE A — PROTECT THE BASELINE

- Inspect branch, commit, working tree, remotes, tags, submodules, ignored files,
  local configuration, and persistent data.
- Preserve all uncommitted work.
- Produce a Git bundle/source archive and restore instructions.
- Record the exact current public-license boundary and commit.
- Inventory contributors, generated/copied code, dependencies, licenses,
  notices, and attribution.
- Generate an SBOM.
- Scan for secrets, credentials, private paths, and accidentally tracked data.
- Inventory every persistent store, key, cache, export, and migration.
- Reproduce the clean build, tests, packaging, and runtime.

PHASE B — RECONSTRUCT THE REAL SYSTEM

Audit:
- all crates and applications;
- production call paths;
- schemas and migrations;
- canonical and derived storage;
- exact/full-text/vector/graph retrieval;
- AI providers, local models, fallbacks, and data flows;
- memory capture/consolidation/retrieval/deletion;
- authentication, authorization, encryption, keys, and secrets;
- sync, conflict behavior, backups, and federation;
- relay, channels, messages, Slack/Discord/email adapters;
- agents, proposals, watcher behavior, and autonomous action paths;
- MCP, REST, gRPC, WebSocket, CLI, and plugin surfaces;
- WASM sandbox and extension permissions;
- Tauri/Svelte UI, accessibility, and design system;
- performance, reliability, packaging, releases, and documentation accuracy.

Classify every capability as:
- production-integrated;
- implemented but unverified;
- partial;
- prototype;
- scaffold;
- dead;
- documentation-only;
- missing;
- superseded.

PHASE C — PRODUCE THE ARCHITECTURE GATE

Create:
- PRODUCT_CONSTITUTION.md
- PRODUCT_BOUNDARIES.md
- INTEROPERABILITY_CONSTITUTION.md
- CURRENT_STATE.md
- FEATURE_TRUTH_MATRIX.md
- ARCHITECTURE_MAP.md
- PRODUCTION_CALL_PATHS.md
- DATA_FLOW_MAP.md
- SECURITY_THREAT_MODEL.md
- PRIVACY_AND_EGRESS_MAP.md
- DEPENDENCY_LICENSE_SBOM_REPORT.md
- DATA_MIGRATION_AUDIT.md
- SYNC_AND_RECOVERY_AUDIT.md
- INTEROPERABILITY_AUDIT.md
- UI_UX_ACCESSIBILITY_AUDIT.md
- PERFORMANCE_BASELINE.md
- KEEP_REFACTOR_MODULARIZE_REPLACE_RETIRE.md
- BLOCKER_REGISTER.md
- V2_ARCHITECTURE.md
- V2_MIGRATION_PLAN.md

Write evidence-backed ADRs for:
- Vault/Space/Workspace/Node hierarchy;
- typed domain model;
- source authority;
- canonical personal/shared stores;
- artifact storage and event ledger;
- FTS/vector/graph backends;
- context compiler;
- grants/policy/receipts;
- connector/extension runtime;
- MCP and A2A adapters;
- collaboration/work/agent boundaries;
- sync/federation;
- Rust/C++ boundary;
- client architecture;
- licensing/open-interface boundary.

MANDATORY BENCHMARKS

Build reproducible baselines for:
- exact lookup;
- full-text retrieval;
- vector retrieval;
- graph retrieval;
- hybrid ranking;
- citation validity;
- temporal/freshness behavior;
- indexing throughput;
- startup;
- memory/disk use;
- attachment extraction;
- sync/conflict recovery;
- migration;
- backup/restore;
- connector replay/idempotency;
- prompt-injection and permission leakage.

FIRST IMPLEMENTATION ONLY AFTER THE GATE

Obsidian/filesystem ingestion
→ immutable source preservation
→ exact and semantic retrieval
→ citation-backed answer
→ contradiction/staleness detection
→ proposal-based memory update
→ approval/rejection
→ audit, rollback, export, and restore.

STOP CONDITION

Do not start broad v2 implementation until the audit, baseline, ADRs, migration
plan, evaluation corpus, and first vertical-slice contract are complete and
internally consistent.
```

---

## 98. Definition of the First Major Milestone

The first major milestone is complete only when the system can demonstrate, through the actual production path:

1. a personal Obsidian/Markdown Vault remains untouched and human-readable;
2. evidence is ingested and identified exactly;
3. retrieval combines exact and semantic methods;
4. AI answers cite supporting passages;
5. conflicts and stale information are visible;
6. AI proposes rather than silently writes;
7. a human can accept/reject and inspect the receipt;
8. any authorized MCP client can retrieve a governed ContextPackage;
9. one external transcript/meeting connector imports through the universal contract;
10. one Agent WorkOrder executes through an external runtime and returns verified artifacts;
11. one internal status report is generated from verified project state;
12. one delegated external message is approved, sent, and receipted without leaking secrets;
13. the whole system can export, back up, restore, and rebuild derived indexes;
14. private-to-shared context boundaries survive adversarial tests.

This milestone establishes the architecture. Full Slack, Notion, automation, marketplace, enterprise, and federation breadth comes afterward.

---

# Part XIII — Final Product Summary

## 99. Final Architecture Formula


The product is not:

```text
MindVault + Slack clone + Notion clone + agents + random connectors
```

It is:

```text
Sovereign Personal Vaults
+ Governed Shared Spaces
+ Evidence and Temporal Knowledge
+ Structured Work and Decisions
+ Human and Agent Identity
+ Delegated Agent Runs
+ Universal MCP/A2A/API Interoperability
+ Open Connector and Extension Contracts
+ Federated Context Nodes
+ Governed External Actions
+ Trust Ledger and Human Authority
```

## 100. Final Product Statement

> **MindVault Next is a sovereign, interoperable human–AI workspace and context fabric. It connects private knowledge, shared collaboration, structured work, applications, devices, models, and agents through open contracts while preserving source authority, evidence, privacy, human accountability, and control.**

## 101. Final Strategic Principle

> **Any authorized human, AI, device, application, or extension should be able to connect and contribute—but nothing may bypass ownership, identity, context boundaries, permissions, provenance, source authority, or human control.**



---

# Part XIV — Research Expansion and Capability Constitution

## 102. Research Basis, Evidence Boundaries, and Repository Truth

### 102.1 What is directly supported by the current repository

The current repository is already materially ahead of the earlier sample-driven framing:

| Repository evidence | What it establishes | What it does **not** establish |
|---|---|---|
| `INTEROPERABILITY_CONSTITUTION.md` is ratified and defines MindVault as logically central and physically decentralized | Interoperability, source authority, grants, provenance, open contracts, portability, and federation already have constitutional priority | It does not prove the implementation satisfies every constitutional law |
| ADR 008 selects file-first Markdown for human documents, SQLite for structured state, and rebuildable search projections | Portability and dual canonical authority are explicit architectural decisions | It does not prove guarded writes, lossless round trips, backup, and conflict handling are complete |
| `IMPLEMENTATION_BACKLOG.md` contains 138 tracked obligations, 39 P0 items, and only 9 verified items at the last recount | The repository has an unusually honest execution ledger and explicit acceptance-command discipline | A large body of code or documentation is not equivalent to production readiness |
| The backlog identifies a live communication-to-knowledge promotion violation | Raw communication must be separated from candidate and canonical knowledge | Existing relay behavior must not be treated as the desired model |
| The governed agent graph has substantial schema and code but no executor; `start_run` executes nothing | Agent governance objects and tests exist as a foundation | The platform does not yet perform governed end-to-end agent execution |
| General CRDT editing, Kafka/microservice decomposition, Matrix/Solid adapters, and unrestricted workflow machinery are explicitly parked | The repo correctly resists premature infrastructure expansion | These subjects may be reopened later only by evidence-backed ADRs |

### 102.2 Source classes used for this expansion

| Evidence class | Examples | How it is used |
|---|---|---|
| Current MindVault repository | Constitution, ADRs, backlog, README, architecture contracts | Determines current truth, gaps, sequencing, and compatibility obligations |
| Primary standards | MCP, A2A, W3C PROV/SHACL/VC, FHIR, RO-Crate, LTI, OPC UA, Matrix, Solid, OpenTelemetry | Defines optional interoperability profiles and conformance ideas |
| Primary adjacent-system documentation | Temporal, OPA, Cedar, SPIFFE, Automerge, Yjs, OpenLineage | Supplies architecture patterns to evaluate, not dependencies to adopt blindly |
| Research systems | Ink & Switch local-first and malleable-software work | Informs product principles, gradual enrichment, local ownership, and composable tools |
| Architectural inference | Capability algebra, Domain Pack model, proof portfolio | New proposal derived from the repository and adjacent evidence; it must be validated by ADRs and implementation |

### 102.3 Honesty rule

The repository documentation was inspected as current source material, but the full workspace was not cloned, built, or benchmarked in this research pass. Repository statements are therefore classified as **documented current state** unless a backlog acceptance command is already recorded as verified. No new section below may convert a documented claim into a tested fact.

### 102.4 Research conclusion

The largest missing enhancement is not another list of use cases. It is a formal mechanism that allows the platform to absorb new domains safely:

\[
\boxed{
\text{Universal Capability Kernel}
+
\text{Versioned Domain Packs}
+
\text{Malleable Views and Workflows}
+
\text{Open Protocol Profiles}
+
\text{Conformance and Unknown-Domain Tests}
}
\]

---

## 103. The Floor-Not-Ceiling Constitutional Law

Add the following binding law to `INTEROPERABILITY_CONSTITUTION.md`:

> **Examples are floors, not ceilings.** Every named product comparison, connector, domain, workflow, vertical slice, user role, and interface in planning documents is a minimum conformance proof. It may not be interpreted as an exhaustive scope. A new domain should normally be expressible by composing existing universal capabilities with a versioned, permissioned Domain Pack. If a domain requires changes to the universal core, architecture review must determine whether (a) a genuinely universal primitive is missing, or (b) domain-specific leakage is being proposed.

### 103.1 Consequences

1. “Meeting,” “career,” “research,” “software project,” and “team chat” must not become privileged built-in assumptions.
2. The core may define `Artifact`, `Actor`, `Claim`, `Event`, `Task`, `Decision`, `Grant`, and `Run`; it must not define `PatientLabResult`, `InsuranceClaim`, `CandidateInterview`, or `RobotMission` as universal objects.
3. First-party domains use the same public Domain Pack contract available to third parties wherever security permits.
4. A new domain may contribute schemas, validation, workflows, policies, views, agents, connectors, and evaluation fixtures without direct canonical database access.
5. Product navigation may surface first-party packs prominently, but prominence does not grant special authority.
6. Every annual architecture review must include an **unknown-domain challenge**.
7. A pack that cannot be removed without corrupting the kernel is not a pack; it is leaked core coupling.

### 103.2 Core-change admission test

A proposed core primitive is admitted only when all are true:

| Question | Required answer |
|---|---|
| Is the primitive useful across at least three substantially different domain families? | Yes |
| Can it be expressed without vocabulary from one industry or vendor? | Yes |
| Does it have clear identity, authority, event, provenance, export, and lifecycle semantics? | Yes |
| Would forcing it into packs duplicate security- or consistency-critical logic? | Yes |
| Is the migration and reversal path documented? | Yes |
| Do conformance fixtures prove it does not weaken existing domains? | Yes |

If these conditions are not met, the capability remains in a Domain Pack or extension.

---

## 104. Capability-First Product Model

MindVault Next should be designed as a **capability operating system**, not a bundle of vertical applications.

### 104.1 Four levels

```text
Level 1 — Universal Kernel
Identity, authority, evidence, events, source authority, provenance,
state, storage, grants, execution, retrieval, export, recovery.

Level 2 — Platform Capabilities
Communication, documents, work, agents, workflows, views, devices,
resources, simulation, records, federation, observability.

Level 3 — Domain Packs
Health, research, education, career, legal, finance, engineering,
media, industrial, civic, family, and future unplanned domains.

Level 4 — User and Organization Compositions
Custom Spaces, dashboards, formulas, policies, workflows, agents,
mini-apps, and cross-domain operating systems.
```

### 104.2 Architectural test

A product capability is mature when it can be:

- invoked through the UI, API, CLI, SDK, automation, and agent boundary where applicable;
- permissioned independently;
- observed and audited;
- exported and restored;
- composed with other capabilities;
- supplied or extended by a pack;
- represented in multiple views;
- exercised locally where its data and policy permit;
- disabled without making unrelated data unreadable;
- tested through a public or internal conformance suite.

### 104.3 Capability graph rather than module checklist

Capabilities have explicit dependencies. For example:

```text
Delegated external communication
  requires Actor + Principal + Message + ContextGrant + ToolGrant
  + DurableWorkflow + SecretBroker + PolicyDecision + ActionReceipt.

Clinical coordination pack
  requires Evidence + Entity/Claim + TemporalValidity + Consent
  + RecordsLifecycle + DomainVocabulary + FHIRAdapter + ReviewWorkflow.

Industrial maintenance pack
  requires DeviceIdentity + Telemetry + Command + SafetyInterlock
  + TimeSeries + WorkOrder + Artifact + OPCUAAdapter + ResourceBudget.
```

The roadmap should track capability readiness and dependency closure, not only screens or end-user features.

---

## 105. Universal Capability Algebra

The following capability families define the intended breadth of the platform. They are not all immediate implementation commitments; they are the design vocabulary against which the core and Domain Pack boundaries must be tested.

### 105.1 Identity, authority, and trust capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| Actor identity | Human, agent, device, workload, application, service, organization | Every action has an authenticated actor type |
| Principal and representation | Principal, delegate, `on_behalf_of`, sponsor, custodian | The acting system never erases the accountable principal |
| Credential and attestation | Credential, issuer, holder, verifier, proof, revocation | A credential is evidence, not automatic authorization |
| Membership and role | Workspace, Space, membership, role, group | Membership is scoped and deny-by-default |
| Trust domain | Node, organization, issuer, federation trust | Trust is explicit, versioned, and revocable |
| Authentication | Passkey, OIDC, device identity, workload identity | Authentication does not imply broad authorization |
| Authorization | Principal–action–resource–context decision | Every consequential operation has a recorded decision |
| Consent | Purpose, data classes, duration, recipients, revocation | Consent is inspectable and revocable where legally/technically possible |
| Delegation | ContextGrant, ToolGrant, budget, expiry, constraints | Reading, acting, spending, and sharing remain separate grants |
| Approval | Request, reviewer, quorum, deadline, result | No action approves or broadens its own authority |

### 105.2 Context, knowledge, and evidence capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| Source registration | Source, source binding, external identity, authority | Every external object has a declared source of truth |
| Artifact preservation | File, blob, media, digest, version, location | Original evidence is immutable or version-preserved |
| Document | Markdown, block, attachment, link, version | Human-authored content remains portable |
| Entity | Stable identity, aliases, type assertions | Entity resolution is reversible and evidence-linked |
| Claim | Subject, predicate, value, validity, confidence, status | Derived claims never erase evidence or uncertainty |
| Relationship | Typed edge, direction, validity, evidence | Relationships are attributable and temporal when needed |
| Event | Actor, subject, time, causation, correlation | Every committed mutation emits a durable event |
| Decision | Alternatives, rationale, evidence, owner, outcome | Decisions survive chat and can be reviewed later |
| Memory lifecycle | Candidate, verified, disputed, superseded, archived | Captured content does not jump directly to canonical memory |
| Temporal truth | Observed, valid-from, valid-until, supersession | Historical and current truth coexist |
| Provenance | Entity, activity, agent, derivation, usage, generation | Every transformation can be traced across systems |
| Lineage | Inputs, process/run, outputs, versions | Data and artifacts retain transformation lineage |
| Attestation | Signed statement about process or artifact | Trust depends on verification, not presence alone |

### 105.3 Collaboration and human-work capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| Communication | Channel, thread, message, reaction, mention | Communication is not automatically canonical knowledge |
| Presence and awareness | Online state, cursor, activity, availability | Ephemeral awareness is not retained as durable truth by default |
| Document collaboration | Comments, suggestions, branches, merges | Merge semantics are selected by object type |
| Work | Project, task, milestone, dependency, risk, status | Human accountability remains explicit |
| Procedure | Checklist, SOP, playbook, runbook | Procedure versions and executions remain distinct |
| Queue and inbox | Item, priority, owner, due state, acknowledgment | Attention state is user-controlled and explainable |
| Meeting | Participants, source, agenda, decisions, actions | Raw transcript and approved outcomes remain separate |
| Review | Reviewer, rubric, evidence, verdict, revisions | Acceptance requires a named review contract |
| Handoff | Objective, state, context, artifacts, next action | Context transfer is explicit, bounded, and attributable |
| Notification | Trigger, urgency, recipient, channel, acknowledgment | Delivery does not imply action completion |

### 105.4 Execution, agent, and automation capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| Work order | Request, owner, delegate, contract, acceptance criteria | Delegation does not transfer accountability |
| Agent identity | Agent Card/profile, model, runtime, skills, sponsor | Agents remain visibly non-human actors |
| Run | Plan, state, grants, checkpoints, actions, result | Every execution has an inspectable lifecycle |
| Tool invocation | Tool, input schema, output, effect class | Tool access is separately granted and validated |
| Durable workflow | State history, timers, signals, retries, compensation | Long-running work survives interruption without duplicate effects |
| Human task | Assignment, form, deadline, escalation, decision | Waiting for humans is durable and auditable |
| Automation | Trigger, conditions, workflow, policy, schedule | Automation begins in simulation/shadow mode where risk warrants |
| External effect | Action outbox, delivery, receipt, compensation | Effects are idempotent or compensatable |
| Multi-agent coordination | Roles, dependency graph, arbitration, handoff | Disagreement is preserved rather than silently collapsed |
| Model routing | Model capability, privacy, quality, latency, cost | Provider selection is policy- and evidence-driven |
| Evaluation | Dataset, rubric, run, metric, regression | Agent/model promotion requires repeatable evidence |
| Sandbox | Runtime identity, filesystem/network/tool limits | A run cannot escape its declared isolation boundary |

### 105.5 Interoperability, distribution, and lifecycle capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| Schema registry | Schema, version, compatibility, owner, status | Unknown versions fail explicitly without discarding source data |
| Semantic profile | Vocabulary, mappings, validation, inference limits | External semantics do not become invisible core assumptions |
| Connector | Manifest, capabilities, cursor, health, revocation | Connectors never write directly to canonical storage |
| Extension | Package, signature, trust class, capabilities | Undeclared access fails closed |
| Materialization | Reference, mirror, projection, excerpt, replica, import | The least materialization needed is the default |
| Synchronization | Operation, version, conflict, cursor, tombstone | Offline work and convergence do not imply silent overwrite |
| Federation | Node identity, discovery, trust, query, replication | Federation preserves ownership and local policy |
| Context capsule | Purpose, authorized content, expiry, operations, signature | Sharing is bounded and inspectable |
| Import/export | Package, schema, identities, provenance, receipts | Exit preserves meaning, not only raw files |
| Records lifecycle | Retention, archive, hold, deletion, succession | Deletion, preservation, and legal hold are explicit state machines |
| Backup/recovery | Snapshot, journal, keys, restore proof | A backup is not trusted until restore is tested |
| Digital continuity | Custodian, succession, emergency access, legacy | Loss of an account or operator does not imply data loss |

### 105.6 Physical, economic, and analytical capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| Resource | CPU, GPU, memory, storage, network, API, model quota | Usage is metered against an explicit scope |
| Budget | Money, token, compute, time, carbon/energy, risk | No agent may exceed the budget attached to its grant |
| Cost and value | Allocation, unit cost, outcome, chargeback/showback | Cost is linked to work and outcomes, not only invoices |
| Transaction | Offer, order, payment, settlement, receipt | Financial effects require stronger policy and verification |
| Contract/obligation | Party, term, condition, duty, permission, prohibition | Natural-language text and executable interpretation remain distinguishable |
| Device | Identity, capabilities, firmware, state, owner | Device commands are effects with safety policy |
| Sensor/telemetry | Observation, unit, timestamp, quality, source | Telemetry quality and clock semantics are explicit |
| Actuator/command | Target, desired state, safety gate, acknowledgment | Physical commands require interlocks and fail-safe behavior |
| Location/environment | Geometry, coordinate reference, accuracy, jurisdiction | Location is sensitive, temporal, and precision-bounded |
| Time series | Metric, sample, interval, aggregation, retention | Downsampling never pretends to be original evidence |
| Simulation | Branch, assumptions, model, scenario, outputs | Simulated state is never confused with observed state |
| Digital twin | Physical/organizational subject, model, observations, commands | Twin fidelity and staleness are visible |
| Analytics | Query, cohort, transformation, privacy rule, result | Derived analytics retain lineage and disclosure limits |

### 105.7 Experience and malleability capabilities

| Capability | Universal objects and functions | Core invariant |
|---|---|---|
| View | Document, table, board, graph, map, timeline, canvas, inbox | A view is a projection, not a new hidden source of truth |
| Form | Schema-bound input, validation, policy, workflow | Forms use the same command and permission contracts as APIs |
| Formula | Inputs, expression, output, dependency graph | Computation is inspectable, versioned, and sandboxed |
| Query | Source scopes, filters, joins, ranking, result schema | Query authority is checked before execution |
| Template | Objects, views, policies, workflows, defaults | Templates create explicit objects, not opaque app state |
| Composition | Embed, transclude, bind, transform, route | Tools compose over governed data contracts |
| End-user programming | Declarative rules, formulas, scripts, extension hooks | Power grows gradually without bypassing security |
| Adaptive interface | Role, task, device, accessibility, context | Personalization is reversible and does not conceal authority |
| Explainability | Source, policy, withheld context, model/run details | The system explains why it showed, hid, or acted on information |

---

## 106. Domain Pack Architecture

A **Domain Pack** is the governed unit for adding a field of practice, profession, industry, or recurring operating model to MindVault without polluting the universal core.

### 106.1 Domain Pack contents

| Component | Required content |
|---|---|
| Identity | Stable pack ID, version, publisher, signature, trust class, license |
| Capability declaration | Universal capabilities consumed and extended |
| Semantic profile | Domain terms, types, relations, mappings, external vocabularies |
| Validation | JSON Schema, optional JSON-LD/RDF mappings, SHACL or equivalent constraints |
| Commands and queries | Typed operations exposed through governed APIs |
| Events | Versioned domain events and mapping to the universal event envelope |
| State machines | Valid statuses, transitions, invariants, timeout and cancellation behavior |
| Workflows | Durable procedures, human tasks, approvals, retries, compensation |
| Policies | Roles, grants, consent, segregation of duties, egress, retention, legal basis |
| Views | Forms, tables, documents, boards, timelines, maps, graphs, control rooms |
| Agents | Skills, allowed tools, output contracts, evals, escalation and risk class |
| Connectors | Source-authority mappings, materialization policies, cursors, transformations |
| Provenance profile | Evidence, lineage, attestation, signature, verification requirements |
| Records lifecycle | Retention, archival, legal hold, deletion, succession, export |
| Metrics and SLOs | Quality, latency, safety, cost, completeness, freshness |
| Fixtures | Synthetic and licensed real-world examples, adversarial cases, migrations |
| Conformance | Contract, security, accessibility, privacy, interoperability, restore tests |
| Localization | Terminology, units, locale, jurisdiction, accessibility needs |
| Upgrade/deprecation | Compatibility range, migration, rollback, end-of-life behavior |

### 106.2 Pack manifest sketch

```yaml
pack_version: 1
id: org.example.research-lab
version: 0.4.0
publisher: did:web:example.org
status: experimental
license: Apache-2.0

requires:
  kernel: ">=2.0 <3.0"
  capabilities:
    - evidence.artifacts
    - knowledge.claims
    - work.procedures
    - execution.durable_workflows
    - provenance.lineage
    - resources.budgets

semantic_profiles:
  - ro-crate-1.3
  - prov-o
  - custom:example-lab-v2

permissions:
  data_classes:
    - research_data
    - human_subject_data
  actions:
    - experiment.create
    - protocol.approve
    - instrument.command
  default: deny

workflows:
  - protocol_review
  - experiment_execution
  - data_quality_review
  - publication_release

views:
  - lab_notebook
  - experiment_timeline
  - sample_inventory
  - provenance_graph

agents:
  - literature_scout
  - reproducibility_auditor
  - statistical_reviewer

connectors:
  - local_instrument_dropbox
  - github
  - doi_crossref
  - ro_crate

conformance:
  suite: ./conformance
  fixtures: ./fixtures
  migrations: ./migrations
```

### 106.3 Pack isolation laws

1. Packs never obtain raw database handles.
2. Packs cannot create a hidden permission model.
3. Packs cannot emit unversioned events.
4. Packs cannot redefine core identity, provenance, grant, or receipt semantics.
5. Packs may specialize a core object but must preserve its exportable base representation.
6. Pack-to-pack dependencies are explicit and version constrained.
7. Cyclic pack dependencies are rejected unless a reviewed composition contract resolves them.
8. Removing a pack leaves source artifacts and portable data readable; derived projections may be rebuilt or retired.
9. Packs may be local-only, organization-private, public, or federated.
10. First-party packs pass the same conformance model as verified external packs.

### 106.4 Pack composition

Packs should compose through:

- shared universal objects;
- declared semantic mappings;
- event subscriptions;
- public commands and queries;
- policy composition;
- view embedding;
- workflow calls;
- Context Capsules;
- provenance and lineage references.

Example:

```text
Career Pack
  + Credential Pack
  + Meeting Pack
  + Networking Pack
  + Research Pack
  + Communication Pack
= governed career operating system
```

No combined product requires a new monolithic schema.

---

## 107. Domain Pack Lifecycle, Governance, and Marketplace

### 107.1 Lifecycle

```text
Idea
→ Capability mapping
→ Threat/privacy assessment
→ Experimental manifest
→ Synthetic fixture validation
→ Owner alpha
→ Domain expert review
→ Conformance candidate
→ Verified release
→ Monitored operation
→ Upgrade / deprecation / retirement
```

### 107.2 Trust classes

| Class | Meaning | Default authority |
|---|---|---|
| First-party core profile | Maintained with the platform | Still deny-by-default for sensitive actions |
| First-party domain pack | Official supported vertical | Capability-limited |
| Verified third-party | Identity, security, compatibility, and conformance reviewed | Declared grants only |
| Organization-private | Approved by an organization administrator | Organization policy |
| Community signed | Signed and inspectable but not independently reviewed | Narrow sandbox |
| Local development | Owner-controlled local package | Development profile only |
| Unverified | Unknown or incomplete provenance | Read-only or blocked |
| Revoked | Malicious, compromised, or incompatible | Disabled immediately |

### 107.3 Marketplace is optional, not sovereign

The product may offer discovery, reviews, security reports, compatibility data, and paid distribution, but users retain installation options through:

- local filesystem;
- Git repository;
- signed URL;
- organization registry;
- self-hosted registry;
- direct MCP/A2A endpoint;
- OCI-style package registry;
- official marketplace.

The marketplace may revoke trust labels; it may not make private ownership or self-hosted extension impossible.

### 107.4 Commercial model

Potential revenue may come from:

- managed hosting and encrypted relay;
- verified-pack review and signing;
- enterprise administration and compliance;
- premium native clients;
- managed model and agent execution;
- domain-specific first-party packs;
- support, migration, and professional services;
- marketplace revenue sharing;
- high-assurance deployment and audit tooling.

The platform must not monetize by trapping data or selling behavioral profiles.

---

## 108. Semantic Interoperability Plane

APIs alone do not produce interoperability. Two systems may exchange JSON while disagreeing about identity, units, validity, state, or meaning.

### 108.1 Required components

| Component | Responsibility |
|---|---|
| Public schema registry | Versioned structural contracts and compatibility |
| Vocabulary registry | Terms, owners, namespaces, labels, deprecations |
| Semantic profile | A coherent subset of types, properties, constraints, and mappings |
| Schema lens | Bidirectional transformation between external and internal representations |
| Validation engine | Structural, semantic, unit, cardinality, and cross-field checks |
| Identity resolver | Maps equivalent external identities without destructive merging |
| Unit and quantity service | Units, conversions, precision, uncertainty |
| Code-system service | External controlled vocabularies and versions |
| Mapping provenance | Records who mapped what, with which version and confidence |
| Compatibility negotiation | Exact, backward-compatible, lossy, unsupported |
| Quarantine | Preserves data that cannot yet be interpreted safely |

### 108.2 Standards posture

| Standard/pattern | Role in MindVault | Not the role |
|---|---|---|
| JSON Schema | Structural API and pack validation | Complete semantic model |
| JSON-LD | Optional linked-data serialization and mapping | Mandatory canonical storage |
| SHACL | Optional graph/profile constraints and dynamic UI hints | Universal runtime dependency for every object |
| W3C PROV | External provenance interchange profile | Replacement for the internal Trust Ledger |
| Verifiable Credentials | Portable credential/attestation profile | General authorization by itself |
| FHIR | Healthcare Domain Pack profile | Core health schema |
| RO-Crate | Research packaging/export profile | Canonical format for all projects |
| LTI | Education connector profile | General extension protocol |
| OPC UA companion models | Industrial semantic adapter/profile | Universal device model |
| C2PA | Media provenance profile | General artifact history format |
| OpenLineage | Data/ML job and dataset lineage profile | Complete work-order model |

### 108.3 Bidirectional schema lenses

Every important external mapping must declare:

- fields copied exactly;
- fields transformed;
- fields derived;
- fields dropped;
- precision lost;
- codes translated;
- default assumptions;
- unsupported extensions;
- reverse-mapping behavior;
- source and mapping version;
- validation results.

Lossy conversion must be visible. The original artifact is retained whenever policy permits.

### 108.4 Semantic conflict handling

Conflicts are classified as:

| Conflict | Example | Response |
|---|---|---|
| Identity conflict | Two systems use the same email for different records | Keep separate candidates and request resolution |
| Vocabulary conflict | “Completed” means approved in one system and merely submitted in another | Preserve source state and map to an explicit local state |
| Unit conflict | Pounds versus kilograms | Normalize only with units and precision recorded |
| Temporal conflict | Different effective dates | Preserve bitemporal assertions |
| Authority conflict | Calendar and project system disagree on due date | Apply declared source-authority policy |
| Legal/jurisdiction conflict | Retention rules differ by region | Select the stricter applicable policy or escalate |
| Model inference conflict | Agents extract incompatible claims | Mark disputed and preserve both evidence chains |

---

## 109. Malleable Workspace and Gradual Enrichment

To exceed Notion and Obsidian rather than clone them, MindVault should support a gentle slope from ordinary information to custom software.

### 109.1 Product law

> The user should be able to begin with freeform text, files, messages, or imported records and gradually enrich them into typed objects, calculations, views, workflows, agents, and mini-applications without a forced migration or separate developer platform.

### 109.2 Enrichment ladder

```text
Text or artifact
→ detected candidates
→ explicit properties and links
→ typed object
→ query or collection
→ formula and derived field
→ view
→ workflow and automation
→ agent skill
→ reusable template
→ Domain Pack or mini-app
```

Each transition is visible, reversible, and provenance-linked.

### 109.3 Universal view system

| View | Suitable for |
|---|---|
| Document | Narrative, specs, notes, reports |
| Table | Structured records and comparison |
| Board | State-based work |
| Timeline/Gantt | Temporal plans and dependencies |
| Calendar | Scheduled and recurring objects |
| Map | Geospatial objects and routes |
| Graph | Relationships, provenance, causality |
| Canvas | Spatial thinking and mixed media |
| Inbox/queue | Attention, triage, approvals |
| Control room | Live operations, agents, incidents, devices |
| Dashboard | Metrics and executive status |
| Form | Governed data capture |
| Chat/thread | Conversation and collaboration |
| Run console | Agent/workflow execution |
| Simulation | Assumptions, branches, outcomes |
| Report | Frozen, cited, exportable result |
| Mobile field view | Offline capture, checklists, device/location context |

Views are declarative projections over governed objects. They do not create hidden independent truth.

### 109.4 Formulas and computation

The formula system should support:

- typed inputs and outputs;
- dependency tracking;
- deterministic recalculation where possible;
- sandboxed custom functions;
- local and remote data functions under policy;
- unit-aware calculations;
- temporal and geospatial operations;
- uncertainty and confidence propagation;
- provenance from source fields to result;
- versioning and test fixtures;
- promotion of stable formulas into pack functions.

### 109.5 End-user programming tiers

| Tier | User experience | Safety model |
|---|---|---|
| 0. Templates | Choose and configure | Pre-verified |
| 1. Rules and formulas | Declarative builder | Schema-checked and sandboxed |
| 2. Visual workflows | Triggers, conditions, actions, approvals | Simulated before activation |
| 3. AI-assisted construction | Describe desired tool; system proposes objects/views/workflows | Diff, tests, and approval |
| 4. Script cells | TypeScript/Python/Rhai or evaluated language | Capability sandbox and resource limits |
| 5. Pack development | Full SDK, tests, manifests, migrations | Signing and conformance required |

### 109.6 AI-generated mini-apps

An AI may propose a mini-app, but the output must be a reviewable bundle of:

- schemas;
- views;
- formulas;
- workflows;
- permissions;
- sample fixtures;
- tests;
- migration/rollback behavior;
- external destinations;
- expected resource use.

“Generate an app” may not mean injecting opaque code into a privileged UI process.

---

## 110. Domain Universe and Expansion Matrix

The domains below are deliberately broader than the initial examples. They are candidate Domain Pack families and architectural test cases—not commitments to implement all of them at once.

| Domain family | Representative capabilities | Relevant external profiles or standards to evaluate | Distinct risks |
|---|---|---|---|
| Personal life operating system | Goals, routines, household records, purchases, travel, reminders, personal agents | iCalendar, CardDAV, receipts/export formats | Surveillance, over-retention, dependency on one assistant |
| Family and care coordination | Shared care plans, school, household tasks, emergency contacts, delegated access | FHIR where clinical, VCs for guardianship/credentials | Consent across dependents, sensitive location, role changes |
| Health and wellness | Records, symptoms, medications, appointments, evidence, care-team messaging | FHIR, DICOM adapters, consent profiles | Clinical safety, HIPAA/GDPR, false inference, emergency access |
| Education and learning | Courses, assignments, mastery, credentials, study agents, portfolios | LTI, QTI, Caliper, VCs | Student privacy, assessment integrity, institutional roles |
| Career and professional development | Evidence ledger, applications, networking, interviews, learning plan | HR-XML/ESCO/O*NET profiles, VCs | Misrepresentation, discrimination, employer policies |
| Research and science | Literature, hypotheses, protocols, datasets, experiments, analyses, publications | RO-Crate, PROV, DataCite, DOI, ORCID | Reproducibility, human-subject data, scientific integrity |
| Software engineering and DevOps | Repositories, issues, work orders, builds, tests, releases, incidents | Git, OpenLineage, SLSA, in-toto, SBOM standards | Supply-chain compromise, secret leakage, unsafe automation |
| Data and ML operations | Datasets, pipelines, models, evals, experiments, deployment, monitoring | OpenLineage, MLflow adapters, model cards | Dataset rights, model drift, lineage gaps, compute cost |
| Product and design | Research, roadmaps, specs, prototypes, decisions, experiments | Figma/design adapters, analytics schemas | Research consent, decision drift, duplicated truth |
| Creative and media | Assets, scripts, edits, rights, publishing, campaigns, generative workflows | C2PA, IPTC, XMP | Copyright, authenticity, rights and likeness |
| Legal and compliance | Matters, evidence, contracts, obligations, holds, review, filings | EDRM, LegalRuleML, e-signature profiles | Privilege, legal hold, jurisdiction, unauthorized practice |
| Finance, tax, and insurance | Accounts, budgets, transactions, filings, policies, claims | Open Banking, ISO 20022 adapters, consent profiles | Fraud, financial regulation, irreversible effects |
| Sales and customer operations | Accounts, opportunities, communications, support, renewals | CRM APIs, contact/identity profiles | Consent, spam, autonomous commitments |
| Human resources and workforce | Roles, hiring, onboarding, performance, learning, offboarding | SCIM, HR standards, VCs | Employment law, bias, high-sensitivity records |
| Procurement and supply chain | Vendors, requests, approvals, orders, inventory, shipments, risk | GS1, EDI, provenance/attestation profiles | Fraud, segregation of duties, third-party risk |
| Executive and board governance | Strategy, portfolio, decisions, risks, resolutions, confidential packs | Board/e-signature adapters | Fiduciary duties, confidentiality, retention |
| Incident and mission operations | Alerts, command roles, playbooks, situational awareness, after-action review | CAP, STIX/TAXII, NIMS-like profiles where relevant | Time-critical safety, stale state, authority confusion |
| Government and civic systems | Cases, services, permits, consultations, public records, interagency work | ActivityPub/Matrix bridges, VCs, records standards | Sovereignty, public records, accessibility, due process |
| Industrial, building, and IoT | Assets, telemetry, maintenance, commands, safety, digital twins | OPC UA, Matter, MQTT adapters | Physical harm, real-time guarantees, device compromise |
| Robotics and autonomous systems | Missions, maps, sensors, actions, human override, logs | ROS 2/DDS adapters | Safety certification, latency, command integrity |
| Travel and mobility | Itineraries, bookings, identity, disruptions, expenses, location context | iCalendar, travel APIs, VCs | Location privacy, changing authoritative sources |
| Communities and decentralized networks | Groups, moderation, federation, portable identity and content | Matrix, ActivityPub, Solid | Abuse, moderation, federation trust, key recovery |
| Cross-organization data spaces | Shared datasets, clean rooms, contracts, purpose-limited queries | Solid/UMA concepts, data-space connectors | Data leakage, policy mismatch, inference attacks |
| Digital legacy and continuity | Succession, emergency access, archives, estate instructions, custodians | VCs, archival packages | Death/incapacity verification, coercion, irreversible disclosure |
| AI-native ecosystem | Agent directories, skill markets, model routing, evals, compute markets | MCP, A2A, OpenTelemetry, VCs | Agent impersonation, runaway cost, unsafe delegation |
| Simulation and digital twins | Scenario branches, forecasts, organizational models, physical twins | Domain-specific model formats | False confidence, model mismatch, observed/simulated confusion |
| Privacy-preserving analytics | Cohorts, federated queries, differential privacy, clean rooms | Open standards as they mature | Re-identification, privacy-budget exhaustion |

The platform should support combinations of these domains because real life does not remain inside one vertical. A research project can involve health records, software, finance, contracts, meetings, physical instruments, and publication provenance simultaneously.


# Part XV — Domain Families as Architectural Stress Tests

## 111. Personal, Family, Care, and Life Continuity

This family tests whether the platform can serve an individual over years without reducing life to a task list or turning intimate context into an indiscriminate surveillance record.

### 111.1 Required domain objects

A Personal and Family pack may define:

- household;
- dependent;
- caregiver;
- emergency contact;
- routine;
- appointment;
- household asset;
- warranty;
- subscription;
- purchase;
- receipt;
- itinerary;
- reservation;
- preference;
- emergency plan;
- care plan;
- delegated authority;
- personal objective;
- journal entry;
- significant life event;
- location episode;
- consent directive;
- continuity instruction.

These are pack-level types. The kernel should only know that they are versioned objects with actors, evidence, policies, temporal validity, relationships, and lifecycle state.

### 111.2 Required workflows

The architecture must be able to support, without new kernel code:

- a household workflow that combines calendar events, purchases, repairs, warranties, reminders, and shared responsibilities;
- a care-coordination workflow where different relatives have different access to appointments, medications, documents, and messages;
- travel disruption handling that updates itinerary state, notifies selected people, proposes rebooking actions, and preserves receipts;
- emergency access with a tightly bounded break-glass policy, stronger authentication, explicit reason, automatic notification, and complete post-event review;
- long-lived personal records whose retention is measured in decades rather than project sprints;
- personal agents whose memory and communication authority differ by Space and life domain;
- gradual transfer of responsibility when a dependent becomes an adult or a caregiver relationship ends.

### 111.3 Unique architectural obligations

This family forces the platform to provide:

1. **Relationship-dependent authority.** Being a parent, spouse, caregiver, emergency contact, or household member does not imply universal access.
2. **Temporal roles.** Authority can start, expire, be suspended, or become invalid after a life event.
3. **Selective disclosure.** A user may share an appointment time without sharing the diagnosis, or a travel itinerary without sharing financial records.
4. **Compassionate failure modes.** Missed reminders, inaccessible recovery keys, or incorrect identity resolution can have serious consequences.
5. **Long-horizon portability.** Records must remain readable after vendors, models, devices, or the product itself change.
6. **Non-productivity states.** Grief, illness, disability, rest, and care cannot be modeled solely as optimization problems.
7. **Household multi-tenancy.** Shared and private objects coexist, with no presumption that one household member administers all others.

### 111.4 Architecture test

A Personal/Family pack passes only when a household can uninstall it, preserve ordinary documents and exported structured records, and remove its workflows without damaging the generic identity, evidence, event, policy, and relationship systems.

---

## 112. Health, Wellness, and Regulated Care

Health is not proposed as an initial commercial launch vertical. It is an adversarial architecture test because it combines high sensitivity, changing truth, professional roles, consent, regulated exchange, emergency access, evidence quality, and potentially safety-critical decisions.

### 112.1 Required distinctions

The platform must distinguish:

| Concept | Meaning |
|---|---|
| Personal wellness observation | User-entered or device-derived signal that is not a clinical diagnosis |
| Clinical source artifact | Authoritative imported record from a care system or clinician |
| Derived interpretation | Model or human interpretation that must retain its evidence and uncertainty |
| Care plan | Agreed or proposed sequence of actions with responsible parties |
| Medication record | Ordered, dispensed, reported, and currently taken states kept distinct |
| Consent | Purpose-, recipient-, data-, and time-bounded authorization |
| Emergency access | Exceptional access path with stronger controls and post-hoc accountability |
| Clinical recommendation | Regulated or professional judgment, not equivalent to general AI output |

### 112.2 Architecture obligations

A health-capable Domain Pack requires:

- field-level sensitivity and masking;
- purpose-of-use restrictions;
- consent receipts and revocation;
- data-minimization before model egress;
- jurisdiction-aware retention and deletion rules;
- patient, caregiver, clinician, institution, and service identities;
- source-system authority rather than blind overwrite;
- explicit separation of observed facts, reported facts, diagnoses, hypotheses, and recommendations;
- audit export suitable for regulated review;
- clinical terminology and exchange profiles behind adapters;
- immutable source records plus correctable derived views;
- safety escalation and abstention rules;
- human-review requirements for clinically consequential outputs;
- no inference of protected or sensitive characteristics merely because source data makes inference possible.

FHIR, DICOM, terminology systems, and jurisdiction-specific consent standards may be supported as profiles or connectors. They must not become the kernel's universal object model.

### 112.3 Why this matters beyond health

If the platform can correctly model medication history, consent revocation, emergency access, and conflicting clinical sources, it will have stronger primitives for legal matters, financial records, family care, personnel records, and confidential research.

### 112.4 Prohibited shortcuts

- No global `is_sensitive` boolean as the entire privacy model.
- No LLM-generated health fact may become canonical without an evidence and authority pathway.
- No emergency mode may silently broaden future ordinary access.
- No health connector may convert absence of a record into evidence of absence.
- No shared-space summary may expose more information than the least-privileged recipient is permitted to see.

---

## 113. Learning, Education, Career, and Verifiable Capability

This family treats learning and professional development as a lifelong evidence graph rather than a set of isolated courses, resumes, and certificates.

### 113.1 Domain capabilities

A pack may compose:

- learning objectives;
- prerequisite graphs;
- courses and curricula;
- assignments and assessments;
- practice history;
- mastery estimates;
- portfolios;
- evidence-backed skills;
- credentials;
- applications;
- opportunities;
- interviews;
- professional relationships;
- feedback;
- career decisions;
- role requirements;
- compensation and offer comparisons;
- continuing education and license renewal.

### 113.2 Architectural tests

The platform must support:

1. **Evidence-backed claims of capability.** A skill claim can point to coursework, projects, assessments, artifacts, endorsements, or verified credentials.
2. **Multiple interpretations.** One project may demonstrate different capabilities under different job or curriculum taxonomies.
3. **Issuer–holder–verifier flows.** Credentials can be received, stored, selectively disclosed, verified, expired, or revoked.
4. **Assessment integrity.** Practice assistance, permitted accommodations, and prohibited live assistance must be policy-distinct.
5. **Opportunity-specific context.** An interview agent can access approved career facts without receiving the user's unrelated private Vault.
6. **Bias and fairness review.** Ranking, matching, and recommendation systems expose reasons, confidence, missing evidence, and potential proxy effects.
7. **Portable professional identity.** Career history cannot be trapped inside one job-search product.
8. **Human-authored truth.** Resume and application claims remain traceable to an approved evidence ledger.

### 113.3 Relevant profiles

Domain Packs may map to learning-tool interoperability, assessment formats, occupational taxonomies, institutional identity, and Verifiable Credentials. These mappings should be optional profiles with round-trip tests and explicit loss reporting.

### 113.4 Expansion beyond job search

The same primitives support apprenticeships, professional licensing, medical continuing education, internal mobility, mentorship, succession planning, talent marketplaces, and agent-generated personalized curricula. Career tools are therefore one view over the wider capability and evidence graph—not the ceiling of the domain.

---

## 114. Research, Science, Laboratories, and Reproducible Knowledge

This family tests whether MindVault can preserve not just conclusions but the chain from question to evidence to method to result.

### 114.1 Core research objects

A Research Domain Pack may define:

- research question;
- hypothesis;
- preregistration;
- protocol;
- method;
- dataset;
- sample or specimen;
- instrument;
- run;
- notebook entry;
- transformation;
- analysis;
- figure;
- result;
- claim;
- uncertainty;
- review;
- publication;
- citation;
- replication;
- correction;
- retraction;
- ethics approval;
- data-use agreement;
- contributor role.

### 114.2 Required architecture

The platform must support:

- immutable raw artifacts plus reproducible derived artifacts;
- content-addressed datasets and scripts;
- method and environment capture;
- lineage from figure back to analysis, data, and source instrument;
- preregistered decision rules and deviations;
- contributor and institutional identities;
- embargoes and controlled-access data;
- evidence quality and uncertainty;
- negative results and failed replications;
- forked hypotheses and competing interpretations;
- citations that survive file relocation;
- export to research-object packages;
- integration with notebooks, repositories, data stores, instruments, and publication systems;
- agent runs that preserve prompts, tools, models, code, environment, and acceptance decisions without pretending that hidden reasoning is evidence.

RO-Crate, PROV, DOI/DataCite, ORCID, notebook formats, and domain repositories are useful profiles. The internal architecture should remain able to represent research that does not fit one scientific discipline.

### 114.3 Scientific integrity laws

1. A result may not overwrite its raw observation.
2. A correction may supersede a claim but must not erase the historical claim.
3. Generated or synthetic data must be visibly distinguishable from observed data.
4. A model's summary must not be cited as the source when the underlying paper or dataset is available.
5. Agent-generated analyses require executable artifacts or an explicit non-reproducible status.
6. A shared conclusion must expose contradictory and disconfirming evidence where material.

### 114.4 New product possibilities

Once the research model is sound, MindVault can support collaborative literature review, living systematic reviews, laboratory orchestration, evidence surveillance, grant management, peer-review workflows, reproducibility audits, multi-agent scientific councils, and long-term institutional memory.

---

## 115. Software Engineering, Data, Machine Learning, and Digital Production

This family connects project knowledge to executable work while preserving the separation between MindVault's coordination/context role and specialized execution runtimes such as DevX Runtime, Claude Code, Codex, CI systems, and external build platforms.

### 115.1 Domain objects

- repository;
- workspace;
- branch;
- commit;
- issue;
- worktree;
- build;
- test run;
- deployment;
- service;
- environment;
- incident;
- dependency;
- vulnerability;
- artifact;
- dataset;
- model;
- evaluation;
- experiment;
- prompt/program version;
- release;
- architecture decision;
- runbook;
- service-level objective;
- operational risk.

### 115.2 Required capabilities

- source, build, test, package, and deployment lineage;
- signed or attestable artifacts;
- repository-scoped context grants;
- secret brokering rather than secret disclosure to models;
- environment and dependency capture;
- durable long-running execution state;
- human review and verification gates;
- rollback and compensating actions;
- incident command and postmortem workflows;
- separation of proposed code, generated patch, reviewed change, merged change, deployed change, and observed production behavior;
- software bill of materials and vulnerability relationships;
- model and dataset cards;
- experiment tracking and comparison;
- compute, token, and cost budgets;
- project handoff capsules and historical decision reconstruction.

OpenLineage, SLSA, in-toto, SPDX/CycloneDX, OpenTelemetry, Git, CI APIs, model-card formats, and ML experiment systems are profiles and connectors. They should enrich the Work Graph and Trust Ledger without becoming a second execution engine hidden in the knowledge layer.

### 115.3 Product opportunities

Beyond coding-agent reporting, this enables release governance, software-supply-chain evidence, engineering onboarding, architecture conformance, operational readiness, data/ML lineage, evaluation governance, incident management, technical-debt and knowledge-debt tracking, and cross-repository program management.

---

## 116. Creative Work, Media, Publishing, and Rights

Creative work tests multi-modal artifacts, collaborative editing, attribution, licensing, authenticity, distribution, and reuse.

### 116.1 Domain objects

- concept;
- brief;
- script;
- scene;
- shot;
- take;
- image;
- audio track;
- model or source asset;
- edit;
- composition;
- version;
- campaign;
- license;
- consent/release;
- likeness right;
- contributor;
- publication;
- distribution channel;
- usage report;
- revenue share.

### 116.2 Architecture obligations

- very large binary artifacts stored externally or content-addressed;
- low-resolution proxies and searchable derivatives;
- time-coded annotations;
- branch/merge semantics for creative versions;
- source and transformation provenance;
- human, AI, and tool contribution attribution;
- rights and license constraints that travel with artifacts;
- model-training permission and usage restrictions;
- publication approval and embargo workflows;
- derivative-work relationships;
- external distribution receipts;
- authenticity manifests and edit history where supported;
- no assumption that a single linear document represents the creative object.

C2PA, XMP, IPTC, media-editing APIs, storage systems, and content-management platforms can be profiles and connectors. MindVault should expose provenance and rights as first-class context rather than merely attaching a license string to a file.

### 116.3 Expansion possibilities

This architecture supports film and media production, design systems, publishing, journalism, marketing campaigns, game development, architecture, music collaboration, creator businesses, and AI-assisted asset pipelines.

---

## 117. Legal, Compliance, Records, Governance, and Evidence

This family tests privilege, holds, jurisdiction, retention, review, evidentiary integrity, formal obligations, and decisions with durable consequences.

### 117.1 Required domain distinctions

| Object | Distinction that must be preserved |
|---|---|
| Source evidence | Original artifact and chain of custody |
| Assertion | What a person or system claims |
| Rule | External or internal normative requirement |
| Obligation | Required action, party, deadline, condition, and evidence of discharge |
| Permission | Allowed action under stated conditions |
| Prohibition | Disallowed action and exception paths |
| Matter | Scoped legal or compliance context |
| Privileged communication | Access and use restrictions independent of ordinary confidentiality |
| Hold | Suspension of ordinary deletion or mutation policy |
| Filing | Submitted version, destination, timestamp, and receipt |
| Decision | Authority, rationale, evidence, appeal or review path |

### 117.2 Architecture obligations

- immutable or tamper-evident evidence records;
- chain of custody;
- legal hold that overrides ordinary retention without silently changing the source object;
- jurisdiction- and matter-specific policy overlays;
- role separation and conflicts barriers;
- bulk review with defensible audit trails;
- redaction with preserved original and disclosure version;
- obligation extraction as proposals, not unquestioned truth;
- deadline calculation with source rules and human confirmation;
- signed approvals and filings;
- policy simulation and explanation;
- export suitable for discovery or external review;
- durable identity and timestamp evidence;
- no claim that the software is providing legal advice merely because it helps organize legal material.

EDRM concepts, LegalRuleML, e-signature systems, records-management standards, and jurisdiction-specific profiles may be implemented in packs. They should not force legal-specific terminology into every generic policy decision.

### 117.3 Wider value

These capabilities also strengthen procurement, healthcare, finance, HR, security, public administration, board governance, and any domain where data deletion, authority, and evidentiary history matter.

---

## 118. Finance, Commerce, Procurement, Insurance, and Contractual Exchange

This family tests irreversible external actions, segregation of duties, ledger reconciliation, consented data access, and value transfer.

### 118.1 Domain objects

- account;
- transaction;
- budget;
- commitment;
- invoice;
- purchase order;
- contract;
- entitlement;
- approval;
- payment instruction;
- receipt;
- refund;
- asset;
- liability;
- policy;
- claim;
- coverage;
- vendor;
- quote;
- bid;
- shipment;
- inventory item;
- tax record;
- filing;
- risk exposure.

### 118.2 Required controls

- separation between read authority, recommendation authority, commitment authority, and transfer authority;
- dual control or multi-party approval for configured actions;
- transaction-specific confirmation rather than permanent blanket permission;
- reconciliation between internal intent, external execution, and authoritative settlement state;
- immutable receipts and idempotency keys;
- spend, frequency, recipient, geography, and risk limits;
- fraud and anomaly escalation without automatic accusation;
- revocable data-access consent;
- vendor and contract obligation tracking;
- tax and accounting period boundaries;
- exchange-rate and valuation provenance;
- compensating actions where true rollback is impossible;
- simulation mode that can never be confused with a live transaction.

Open Banking consent patterns, payment and accounting APIs, procurement standards, ISO 20022 or EDI profiles, and insurer systems may connect through Domain Packs. Financial execution should remain disabled until the policy, secret-broker, reliable-effects, and receipt systems have passed dedicated safety tests.

---

## 119. Enterprise Work, Customer Operations, People Operations, and Organizational Memory

This family covers the broad operational territory usually split among Slack, Teams, Notion, Linear, Jira, CRM, HRIS, help desks, and workflow tools.

### 119.1 The product opportunity

MindVault can unify the context beneath those categories without forcing every team into one fixed application model. A Space can be rendered as:

- a conversation environment;
- project board;
- customer account room;
- case-management queue;
- hiring pipeline;
- onboarding program;
- service desk;
- executive portfolio;
- incident room;
- partner portal;
- knowledge base;
- agent operations center.

These are projections over shared identity, evidence, work, policy, communication, and automation primitives.

### 119.2 Architecture obligations

- organization, team, external guest, contractor, service, and agent identities;
- role and attribute-based authority;
- cross-Space references without unauthorized content leakage;
- configurable work states and schemas;
- communication retention and moderation;
- employee lifecycle and immediate revocation;
- external customer/partner partitions;
- service-level objectives and queues;
- handoffs and escalations;
- approval chains and delegation;
- structured commitments extracted from communication only through governed promotion;
- full-text and federated search with permission-aware snippets;
- administrative policy templates without destroying local team autonomy;
- import/export and coexistence with existing SaaS systems;
- analytics that preserve individual privacy and do not silently become worker surveillance.

### 119.3 Slack parity is a floor

Channels, threads, search, notifications, calls, bots, apps, and workflows are necessary collaboration capabilities, but the higher target is:

> conversation that can become evidence-backed work, decisions, knowledge, automation, and agent execution while retaining source, authority, and reversibility.

### 119.4 Notion parity is a floor

Documents, databases, templates, forms, wikis, and views are necessary, but the higher target is a malleable object-and-view system that can represent new domains without waiting for the core product team.

---

## 120. Industrial Systems, Devices, Buildings, Robotics, and the Physical World

This family exposes assumptions that are harmless in document software but dangerous when software can affect physical systems.

### 120.1 Required objects

- device;
- asset;
- sensor;
- actuator;
- controller;
- capability;
- telemetry stream;
- observation;
- command;
- command acknowledgment;
- maintenance event;
- calibration;
- location;
- topology;
- digital twin;
- operating envelope;
- alarm;
- safety interlock;
- mission;
- map;
- route;
- physical artifact;
- custody transfer.

### 120.2 Architectural obligations

1. **Observed state, estimated state, commanded state, and confirmed state are distinct.**
2. **Time quality matters.** Source clocks, delay, ordering, and uncertainty must be recorded.
3. **Commands require explicit capability and often local safety validation.**
4. **Loss of connectivity is normal.** Edge queues, local autonomy, and reconciliation are required.
5. **Real-time control does not belong in a general collaboration server.** MindVault coordinates and records; dedicated control systems retain safety-critical loops.
6. **Physical topology and geospatial context are first-class.**
7. **Maintenance and calibration history influence whether observations are trustworthy.**
8. **Simulation and live mode can never share an ambiguous action path.**
9. **Human override and emergency stop are architectural primitives where the domain requires them.**

Matter, OPC UA, MQTT, ROS 2/DDS, building-management systems, and industrial historians can be adapters or semantic profiles. The platform should not claim deterministic real-time guarantees unless a specialized runtime and hardware path proves them.

### 120.3 Expansion possibilities

This supports homes, laboratories, clinics, warehouses, factories, farms, buildings, fleets, field operations, drones, robotics, environmental monitoring, and assistive technology.

---

## 121. Civic, Government, Community, Crisis, and Cross-Organization Coordination

This family tests due process, public accountability, accessibility, federation, moderation, records law, multi-agency authority, and operation under crisis conditions.

### 121.1 Domain capabilities

- public consultation;
- service request;
- permit or license case;
- benefits or eligibility workflow;
- constituent relationship;
- public record;
- policy proposal;
- hearing;
- vote or resolution;
- grant;
- interagency task force;
- emergency incident;
- operational period;
- resource request;
- situation report;
- mutual-aid agreement;
- community moderation;
- federated group;
- public transparency portal.

### 121.2 Architecture obligations

- accessible interfaces and alternate participation modes;
- public/private/secret partitions with explicit declassification or disclosure paths;
- due-process states, appeal rights, and reasoned decisions;
- public-records retention and disclosure workflows;
- jurisdiction and organizational sovereignty;
- federated identity and trust agreements;
- operation during degraded connectivity;
- incident roles and command transitions;
- geospatial and temporal situation awareness;
- misinformation and source-quality handling without centralizing truth in one model;
- moderation actions with policy, evidence, appeal, and transparency;
- multilingual content and translation provenance;
- transparent automation and non-automated alternatives where required;
- no assumption that every participant can or should install one proprietary client.

Matrix, ActivityPub, public alerting, identity credentials, government data standards, and domain-specific incident frameworks may be supported through profiles and bridges. Federation should not be enabled merely as a marketing checkbox; it requires threat modeling, abuse handling, key rotation, replay protection, and operational governance.

---

## 122. Cross-Domain Composition Is the Real Product Test

The most valuable workflows cross domain boundaries. The architecture must support composition such as:

- clinical research combining consent, health data, laboratories, software, statistics, publication, and audit;
- a startup combining founder knowledge, product design, software, customer conversations, hiring, finance, contracts, and investor governance;
- a family-care situation combining health, travel, scheduling, documents, finance, emergency authority, and communication;
- disaster response combining sensors, maps, agencies, volunteers, logistics, public communication, and after-action review;
- an acquisition combining diligence, legal holds, finance, software, personnel, security, and integration planning;
- a scientific instrument program combining procurement, device telemetry, calibration, experiments, data lineage, maintenance, and research outputs;
- a media campaign combining creative provenance, rights, approvals, audience feedback, budgets, distribution, and brand policy;
- an educational program combining credentials, content, assessment, mentorship, accessibility, and employment outcomes.

These examples are still only stress tests. The architectural rule is:

> A cross-domain workflow should be composable from Domain Packs, policies, schemas, views, connectors, and workflows without merging the packs into a new monolithic core.

### 122.1 Composition invariants

1. Every object retains the pack and schema version that defines it.
2. Shared kernel identities and evidence can link objects across packs.
3. One pack may reference another pack's public schema but may not bypass its authority rules.
4. Policy conflicts fail closed or enter explicit review.
5. Uninstalling one pack leaves preserved exportable records and understandable dependency diagnostics.
6. Search, context compilation, and agent access remain permission-aware across packs.
7. A cross-domain automation has one correlated run and receipt chain even when it spans many systems.
8. The UI explains which pack, source, and policy control each field or action.


# Part XVI — Additional Platform Planes Required by the Expanded Scope

## 123. Durable Workflow and Long-Running Coordination Plane

The current product vision includes schedules, agents, approvals, external actions, reports, and work that may remain paused for days or months. A normal request/response service and an in-memory job queue are insufficient.

### 123.1 Required workflow semantics

The platform requires an owned `DurableWorkflow` contract with:

- stable workflow identity;
- versioned workflow definition;
- append-only execution history;
- deterministic or explicitly recorded decision replay;
- durable timers;
- external signals;
- human approval tasks;
- child workflows;
- retries with bounded backoff;
- idempotency keys;
- cancellation;
- pause and resume;
- deadlines and timeouts;
- compensation for irreversible effects;
- checkpointing;
- activity heartbeats;
- failure classification;
- migration between workflow versions;
- policy evaluation at every consequential boundary;
- correlation with Agent Runs, Work Orders, events, and receipts.

### 123.2 Workflow versus agent

| Concept | Responsibility |
|---|---|
| Workflow | Durable coordination, state, timers, approvals, retries, and effects |
| Agent | Judgment, interpretation, planning, generation, and tool selection within granted bounds |
| Activity | Deterministic or bounded execution step |
| Human task | Explicit request for review, information, or approval |
| External effect | Action performed through the reliable-effects and secret-broker path |
| Receipt | Evidence that an action was authorized and what actually happened |

An agent may participate in a workflow. A workflow must not depend on an LLM remembering its previous state.

### 123.3 Build-versus-adopt posture

Temporal and adjacent durable-execution systems provide valuable patterns: event history, durable timers, signals, human-in-the-loop waiting, retries, and audit. MindVault should:

1. define its own domain-facing workflow contracts;
2. implement a minimal durable engine sufficient for initial use cases or adapt an external engine behind an interface;
3. benchmark operational complexity, recovery, portability, and embedded/self-hosted fit;
4. avoid exposing vendor-specific workflow IDs and semantics as canonical Domain Pack contracts;
5. support export/replay independently of the chosen engine.

### 123.4 Admission test for external engines

An external workflow engine may be adopted only if it passes:

- local and self-hosted deployment requirements;
- offline or degraded-mode strategy;
- data-location and encryption requirements;
- human-task and approval integration;
- workflow-history export;
- deterministic migration story;
- bounded operational complexity for a single-user node;
- multi-tenant and organization-scale path;
- license and commercial-use review;
- failure injection and restore tests.

### 123.5 Required first workflows

The first proof set should include:

- wait for human approval for seven days without consuming model compute;
- survive process and machine restarts;
- retry an idempotent external action;
- compensate after a partially successful multi-system operation;
- migrate a running workflow to a compatible definition version;
- suspend immediately after a principal, connector, or policy is revoked;
- produce one complete timeline from request through final receipt.

---

## 124. Policy, Consent, Purpose, and Delegation Plane

Role-based access alone is not sufficient for personal context, regulated data, agents, external sharing, or decentralized collaboration.

### 124.1 Policy decision model

Every protected query or action should be evaluated against:

```text
principal
actor
on_behalf_of
resource
space
purpose
requested_action
context
source_authority
sensitivity
jurisdiction
time
risk
consent
contractual_constraints
retention_state
workflow_state
agent_run
external_destination
```

The result is not merely `allow` or `deny`. It may include:

- allow;
- deny;
- allow with redaction;
- allow with reduced scope;
- require step-up authentication;
- require one or more approvers;
- require purpose confirmation;
- require a different execution environment;
- local-only;
- no model training;
- no retention;
- no onward sharing;
- rate, cost, recipient, or time limit;
- log at enhanced detail;
- expire after a stated condition;
- return an explanation safe for the requesting actor.

### 124.2 Consent object

A first-class `ConsentGrant` should record:

- grantor and capacity;
- recipient or class of recipients;
- data categories or exact resources;
- permitted purposes;
- permitted actions;
- excluded uses;
- start and expiry;
- jurisdiction or governing policy;
- whether onward delegation is permitted;
- revocation method;
- proof of notice and acceptance;
- source document or interaction;
- version;
- status;
- related receipts.

Consent is not equivalent to every other authorization. It may be one required input among law, contract, organizational policy, safety rules, and user preference.

### 124.3 Delegation chains

The platform must represent:

```text
Human principal
  → delegates limited authority to Agent A
  → Agent A requests Tool B
  → Tool B calls Connector C
  → Connector C performs an external action
```

Every hop must be attributable, scope-preserving, revocable, and unable to amplify authority.

### 124.4 Policy engine architecture

Create a `PolicyBackend` trait with an owned input/output contract. Evaluate:

- an internal deterministic rule engine for embedded/local deployments;
- Cedar-like principal/action/resource/context policies;
- OPA/Rego for broad policy-as-code and enterprise integration;
- signed policy bundles;
- pack-contributed policy templates;
- organization overrides;
- user-level restrictions that organizations may not silently weaken where product law protects the user.

No external policy language should define the canonical authorization data model.

### 124.5 Policy simulation

Before activating a policy or automation, users and administrators should be able to ask:

- who gains access;
- who loses access;
- which active workflows will pause;
- which connectors become invalid;
- which agents can act;
- which external destinations become reachable;
- which records become retained or deletable;
- whether the change conflicts with a higher-priority rule.

Simulation results must be testable against fixtures and stored with the policy change.

---

## 125. Identity, Workload Identity, Credentials, and Attestation Plane

Interoperability requires more than human login. The platform must identify people, devices, applications, agents, connectors, organizations, services, and transient workloads.

### 125.1 Actor taxonomy

| Actor type | Examples | Identity requirements |
|---|---|---|
| Human | owner, teammate, guest, reviewer | account, verified methods, roles, recovery |
| Device | laptop, phone, recorder, server | device key, attestation where available, revocation |
| Application | Obsidian plugin, meeting app, external client | OAuth client or signed application identity |
| Connector | Gmail, GitHub, industrial adapter | scoped service identity and source binding |
| Agent | research agent, build agent, external A2A agent | sponsor, Agent Card, version, skill and policy profile |
| Workload | one sandboxed run or worker process | short-lived identity tied to one run and grants |
| Organization | company, laboratory, institution, household | authority domain, policy and federation metadata |
| Credential issuer/verifier | school, employer, authority | trust framework, keys, status and revocation |

### 125.2 Short-lived workload identity

Agents and activities should not share long-lived service credentials. A SPIFFE-like model is worth evaluating for:

- workload identity documents;
- short-lived certificates or tokens;
- automatic rotation;
- trust-domain federation;
- binding identity to runtime and workload state;
- immediate revocation;
- workload-to-workload authentication.

MindVault should preserve its own `ActorIdentity` and `RunIdentity` contracts and adapt to SPIFFE or platform identity systems where appropriate.

### 125.3 Verifiable credentials

Support a credential profile for issuer–holder–verifier exchanges:

- receive and store credentials;
- verify issuer and status;
- present selected claims;
- preserve user consent and purpose;
- record disclosure receipts;
- handle expiration, suspension, and revocation;
- avoid equating a credential with universal truth outside its stated context.

Credentials can represent education, professional licenses, organizational roles, device certification, training completion, authorization, membership, or test results.

### 125.4 Attestation

For high-risk actions, identity may need evidence about the executing environment:

- signed application version;
- approved extension hash;
- sandbox configuration;
- device security state;
- model and tool version;
- policy bundle version;
- container or build provenance;
- location or network constraints where lawful and necessary.

Attestation is evidence for a policy decision, not a magical guarantee of safety.

---

## 126. Provenance, Lineage, Authenticity, and Attestation Profiles

MindVault's Trust Ledger should remain the canonical internal action and derivation history. Interoperability requires mappings to adjacent provenance systems rather than one overloaded universal export.

### 126.1 Internal provenance primitives

The internal model should minimally distinguish:

- **Entity:** a document, dataset, artifact, claim, model, message, or physical/digital object;
- **Activity:** ingestion, transformation, review, execution, publication, decision, or external action;
- **Agent:** human, software, organization, or AI responsible for or involved in an activity;
- **Derivation:** how one entity was produced from others;
- **Attribution:** responsibility or contribution;
- **Delegation:** acting on behalf of another principal;
- **Source authority:** which system controls the authoritative state;
- **Evidence:** support for a claim or decision;
- **Receipt:** signed or tamper-evident record of authorization and result.

### 126.2 Profile mappings

| Profile | Use |
|---|---|
| W3C PROV | General cross-system provenance for entities, activities, and agents |
| OpenLineage | Data jobs, runs, datasets, and extensible facets |
| SLSA / in-toto | Software build and supply-chain provenance and authorized steps |
| C2PA | Media origin, transformations, and content credentials |
| RO-Crate | Research objects, contextual entities, workflows, and packaged metadata |
| Domain audit formats | Regulated or sector-specific export |

These profiles should be generated from the Trust Ledger and domain records. They should not require duplicate, independently mutable provenance stores.

### 126.3 Provenance quality levels

| Level | Meaning |
|---|---|
| P0 — Unknown | Imported without reliable source information |
| P1 — Declared | Source reports its own provenance but is not independently verified |
| P2 — Linked | Source and transformation chain are recorded |
| P3 — Reproducible | Inputs, environment, method, and output can be reproduced |
| P4 — Attested | Relevant actors or systems cryptographically attest the chain |
| P5 — Independently verified | Independent verification or reconciliation supports the chain |

The UI must avoid presenting a P1 claim as equivalent to a P4 or P5 artifact.

### 126.4 Lineage queries

The system should answer:

- Where did this field, summary, chart, recommendation, or message come from?
- Which version of the source and model produced it?
- Which human accepted it?
- What downstream objects depend on a now-corrected source?
- Which external recipients received an earlier version?
- Can this result be reproduced?
- Which rights, consent, or retention conditions travel with it?

---

## 127. Object-Specific Collaboration and Synchronization Semantics

A single global CRDT is not an architecture. Different object classes require different consistency, conflict, authority, and recovery behavior.

### 127.1 Update-model taxonomy

| Object class | Preferred update model | Examples |
|---|---|---|
| Immutable source artifact | Content-addressed append | imported PDF, recording, evidence blob |
| Transactional record | Optimistic concurrency/transaction | task state, membership, approval |
| Append-only event | Durable log with idempotency | audit event, external receipt |
| Human document | Versioned text/block model; CRDT only when needed | notes, collaborative docs |
| Derived index | Rebuildable projection | FTS, vectors, graph projection |
| External authoritative object | Source binding and sync cursor | GitHub PR, calendar event |
| Safety-critical command | Exclusive authority plus acknowledgment | device command, payment instruction |
| Workflow | Event-sourced durable state machine | approvals, agent runs |
| Presence/awareness | Ephemeral best-effort state | cursor, typing, online status |
| Aggregate analytics | Recomputed or reconciled materialization | dashboards, reports |

### 127.2 CRDT admission criteria

CRDTs should be introduced for an object class only when:

- concurrent offline editing is a real measured requirement;
- user-visible merge behavior is understandable;
- metadata and permission changes cannot be smuggled through the document merge path;
- storage and history growth are bounded;
- schema migrations are testable;
- deletion and legal hold semantics are defined;
- encryption and key rotation are compatible;
- export to ordinary human-readable formats remains possible;
- conflict-free convergence does not hide semantic conflict.

Yjs, Automerge, or other libraries may be evaluated behind an owned document collaboration interface. The platform should preserve the existing decision to defer CRDT adoption until a measured need and benchmark exist.

### 127.3 Conflict classes

The system should distinguish:

- byte/text conflict;
- structural schema conflict;
- source-authority conflict;
- policy conflict;
- temporal truth conflict;
- duplicate identity conflict;
- semantic contradiction;
- external-effect conflict;
- human ownership conflict.

A merge algorithm can solve only some of these. Others require review, authority resolution, or a new decision.

---

## 128. Resource, Cost, Energy, and Economic Governance Plane

An open agent platform can create unbounded model cost, compute use, storage growth, API charges, human-review burden, notification load, or external commitments. Resource governance must therefore be a core cross-cutting plane.

### 128.1 Resource dimensions

- model tokens and calls;
- CPU, GPU, NPU, memory, and runtime duration;
- device battery and thermal impact;
- network transfer;
- storage and index growth;
- third-party API credits;
- money spent or committed;
- human approval and review time;
- notification and attention budget;
- privacy budget for aggregate analytics;
- rate of external messages or actions;
- domain risk budget;
- carbon/energy estimates where measurable and useful.

### 128.2 Budget hierarchy

```text
Owner or organization budget
  → Space budget
    → Project or workflow budget
      → Agent budget
        → Run and activity budget
```

A lower level may narrow but not silently expand a higher-level limit.

### 128.3 Resource policy examples

- use local embedding model while on battery;
- cloud synthesis permitted only after user-approved redaction;
- stop a research run after a configured spend or marginal-evidence threshold;
- require approval before using a premium model;
- prevent one automation from generating more than N external messages;
- reserve compute for foreground work;
- archive cold derived indexes before deleting canonical evidence;
- showback cost by project, agent, model, and outcome;
- suspend an agent whose error rate or cost per accepted artifact exceeds a threshold.

### 128.4 Value linkage

FinOps-style allocation is insufficient if it tracks cost but not value. The platform should link resource use to:

- accepted artifact;
- completed task;
- verified decision;
- avoided incident;
- research evidence gained;
- human time saved;
- revenue or strategic outcome where the user chooses to record it;
- rejected or abandoned output.

The goal is not to pretend all value is monetary. It is to make resource tradeoffs observable and governable.

### 128.5 Agent economy—future, gated

The same primitives can later support:

- internal agent service catalogs;
- budgets and quotas;
- paid external agents;
- escrow or milestone release;
- reputation based on verified outcomes;
- service-level commitments;
- dispute and refund workflows.

No open agent marketplace should launch before identity, provenance, sandboxing, policy, billing integrity, and dispute handling are mature.

---

## 129. Device, Edge, Time-Series, Geospatial, and Physical-State Plane

General-purpose documents and relational tables are insufficient for continuous telemetry and physical systems.

### 129.1 Required data forms

- high-volume time series;
- sampled observations with quality and uncertainty;
- geospatial features and trajectories;
- topology graphs;
- device capability descriptions;
- command and acknowledgment streams;
- event windows;
- media streams or external references;
- calibration and maintenance intervals;
- edge summaries and anomaly proposals;
- synchronized and unsynchronized clocks.

### 129.2 Storage and processing posture

The canonical platform should not copy all telemetry into SQLite. Use:

- source-authoritative references;
- external time-series or object stores;
- bounded local caches;
- downsampled summaries;
- event extraction;
- hash-linked evidence windows;
- pack-specific query adapters;
- retention tiers;
- edge processing before central synchronization.

### 129.3 Safety boundaries

- Read telemetry and issue commands through separate capabilities.
- Commands require exact target, expected state, deadline, and acknowledgment policy.
- High-risk commands require local interlocks outside the general agent runtime.
- Agent recommendations cannot bypass certified control logic.
- The platform must expose stale or uncertain state prominently.
- Offline edge behavior must be explicitly defined rather than treated as failure.

### 129.4 Device digital twin

A `DigitalTwin` is a pack-defined projection that may combine:

- authoritative configuration;
- observed state;
- estimated state;
- maintenance history;
- simulation state;
- predicted state;
- command history;
- evidence and uncertainty.

The UI and APIs must never conflate those categories.

---

## 130. Records Lifecycle, Archival, Legal Hold, Continuity, and Digital Legacy Plane

The existing plan addresses backup and retention, but the expanded product needs a unified lifecycle model spanning personal, organizational, regulated, and post-user continuity.

### 130.1 Lifecycle states

```text
Active
→ Inactive
→ Archived
→ Scheduled for deletion
→ Held
→ Restored
→ Exported/transferred
→ Deleted/tombstoned
```

A record may also be:

- superseded;
- disputed;
- revoked;
- quarantined;
- inaccessible pending key recovery;
- retained by an external source;
- subject to multiple conflicting policies.

### 130.2 Retention policy inputs

- user choice;
- Space or organization policy;
- source-system policy;
- contract;
- consent;
- jurisdiction;
- legal hold;
- credential status;
- safety requirement;
- archival value;
- dependency by downstream objects;
- key availability;
- pack-specific policy.

### 130.3 Digital continuity and legacy

A private personal platform must prepare for:

- device loss;
- incapacity;
- death;
- organizational dissolution;
- provider shutdown;
- abandoned accounts;
- key loss;
- designated custodians;
- staged release of selected records;
- transfer of projects or intellectual property;
- destruction of records the user explicitly did not want inherited.

### 130.4 Continuity controls

- encrypted recovery packages;
- multi-party or threshold recovery options;
- designated successors with narrow scopes;
- waiting periods and challenge mechanisms;
- evidence requirements for activation;
- independent notification channels;
- reversible dry-run and review;
- full audit;
- jurisdiction-specific legal review before offering formal estate functions.

Digital legacy is not merely a feature. It is a test of whether the product genuinely delivers user ownership and longevity.

---

## 131. Simulation, Scenario Branching, Forecasting, and Digital-Twin Reasoning Plane

The platform should help users reason about possible futures without confusing simulations with observed truth.

### 131.1 State categories

| State | Meaning |
|---|---|
| Observed | Supported by a source observation |
| Reported | Asserted by a person or system |
| Derived | Computed or inferred from other state |
| Assumed | Explicit scenario input |
| Simulated | Produced inside a model or branch |
| Predicted | Forecast with uncertainty and horizon |
| Intended | Desired target or plan |
| Commanded | Requested external state |
| Confirmed | External source reports result |

### 131.2 Scenario branch

A scenario should record:

- parent state and snapshot time;
- changed assumptions;
- model and version;
- constraints;
- random seed where applicable;
- data inputs;
- uncertainty;
- outputs;
- comparisons;
- human interpretations;
- actions that would be required to make the scenario real.

### 131.3 Uses

The same architecture can support:

- product and business planning;
- budget scenarios;
- project critical paths;
- policy impact;
- clinical or scientific hypotheses under appropriate controls;
- incident tabletop exercises;
- supply-chain disruption;
- travel alternatives;
- resource allocation;
- physical digital twins;
- agent-plan comparison.

### 131.4 Safety laws

1. Simulated state never updates live authoritative state directly.
2. A scenario output must expose assumptions and uncertainty.
3. A user can compare branches and merge only explicit selected decisions.
4. Agents may recommend actions from a scenario but require ordinary authority and effect controls to execute them.
5. The UI labels simulated, predicted, and observed information at every view where confusion is possible.

---

## 132. Observability, Evaluation, Quality, and Operational Evidence Plane

Every important action must be observable without turning private content into uncontrolled telemetry.

### 132.1 Observability layers

| Layer | Examples |
|---|---|
| System | latency, errors, resource use, queue depth, storage health |
| Workflow | state transitions, retries, waits, compensation |
| Agent | model/tool calls, grants, cost, duration, output status |
| Retrieval | query plan, sources, ranking, permission filtering |
| Connector | cursor, rate limits, failures, source freshness |
| User experience | task success, correction burden, accessibility failures |
| Security | authorization decisions, anomalous behavior, revocation |
| Quality | citation validity, hallucination, duplicate memory, stale retrieval |
| Value | accepted results, time saved, project outcomes where recorded |

### 132.2 OpenTelemetry posture

Use OpenTelemetry-compatible traces, metrics, and logs where appropriate, including emerging generative-AI semantic conventions, but:

- preserve MindVault's canonical correlation IDs and receipt model;
- redact or tokenize sensitive content;
- support local-only observability;
- distinguish operational telemetry from user knowledge;
- make external export opt-in and destination-controlled;
- define retention independently from canonical audit history.

### 132.3 Evaluation registry

Each model, agent, connector, pack, and workflow should have:

- evaluation suites;
- fixtures;
- versioned results;
- hardware/runtime context;
- known limitations;
- regression thresholds;
- security/adversarial tests;
- acceptance authority;
- last verified date;
- evidence links.

### 132.4 Continuous evidence

A feature is not “complete” because its code exists. It remains verified only while its acceptance commands, evidence, dependencies, and operational targets continue to pass. This extends the repository's existing evidence-gated backlog philosophy to Domain Packs and ecosystem components.

---

## 133. Federated Query, Data Spaces, and Privacy-Preserving Computation Plane

Logical centrality does not require physical centralization. Some data should remain on the source node while authorized questions travel to it.

### 133.1 Federated query modes

- metadata discovery;
- exact object retrieval;
- full-text subquery;
- semantic subquery;
- structured aggregate;
- policy-limited cohort query;
- remote model execution;
- signed result package;
- time-bounded interactive session;
- one-time Context Capsule.

### 133.2 Query-to-data workflow

```text
Requester
→ proposes query and purpose
→ policy/consent evaluation
→ remote node executes locally
→ node applies minimum necessary result policy
→ result is signed with source and freshness
→ requester validates and combines results
→ receipt records the exchange
```

### 133.3 Privacy-preserving options

Evaluate, by domain and maturity:

- aggregation thresholds;
- differential privacy;
- secure enclaves or confidential computing;
- secure multi-party computation;
- homomorphic techniques for narrow operations;
- federated learning;
- clean-room execution;
- synthetic or de-identified data;
- local embeddings and remote vector references.

No technique should be marketed as “anonymous” without an explicit threat model and re-identification analysis.

### 133.4 Data-space contracts

A cross-organization Space may require:

- purpose and permitted-use terms;
- data categories;
- contributor and consumer identities;
- query and export limits;
- retention;
- model-training restrictions;
- derived-output rights;
- audit and inspection rights;
- revocation and termination;
- breach or misuse handling;
- jurisdiction and dispute process.

The policy and consent plane should enforce machine-readable parts while preserving the human-readable agreement as evidence.

---

## 134. Search, Retrieval, Reasoning, and Context Compilation Evolution

The existing exact/full-text/vector/graph/temporal model should expand into a federated, multimodal, domain-aware retrieval system while keeping canonical truth separate from indexes.

### 134.1 Retrieval sources

- exact identifiers and hashes;
- structured relational queries;
- full-text indexes;
- semantic/vector indexes;
- graph traversal;
- temporal intervals and event history;
- geospatial queries;
- time-series windows;
- media transcripts and embeddings;
- remote/federated nodes;
- external authoritative sources;
- pack-defined specialist indexes;
- live operational state.

### 134.2 Query plan

The query planner should consider:

- user intent;
- requested output type;
- actor permissions;
- source authority;
- freshness requirement;
- domain pack;
- data location;
- sensitivity and egress;
- cost and latency budget;
- offline availability;
- evidence quality;
- temporal validity;
- contradiction likelihood;
- expected answer coverage.

### 134.3 Pack-contributed retrieval

A Domain Pack may contribute:

- query expansions;
- field mappings;
- domain terminology;
- rerankers;
- validation rules;
- citation renderers;
- answer templates;
- abstention conditions.

It may not:

- bypass permissions;
- silently send content to an external model;
- mark an inference canonical;
- suppress contradictory authoritative evidence;
- redefine global source authority;
- hide query cost or destinations.

### 134.4 Context package evolution

A `ContextPackage` should support:

- task objective;
- source excerpts and structured facts;
- pack and schema metadata;
- evidence quality;
- validity intervals;
- contradictions and uncertainty;
- action and redistribution restrictions;
- token/size budget;
- references that can be resolved later;
- omitted-context explanation;
- freshness deadline;
- cache key;
- requested output contract;
- verification requirements.

### 134.5 Retrieval quality beyond recall

Measure:

- source authority accuracy;
- temporal accuracy;
- citation support;
- contradiction coverage;
- permission leakage;
- context efficiency;
- diversity of evidence;
- stale result rate;
- domain-specific utility;
- robustness to prompt injection and poisoned sources;
- federated-result trust and freshness.

---

## 135. Moderation, Safety, Abuse, and Community Governance Plane

Once the product supports shared Spaces, federation, marketplaces, external agents, and public communities, security controls alone are insufficient.

### 135.1 Required governance objects

- community or Space policy;
- moderator role;
- report;
- flagged content;
- moderation action;
- appeal;
- evidence bundle;
- transparency notice;
- rate limit;
- quarantine;
- blocked actor or node;
- reputation signal;
- federation trust decision;
- safety incident.

### 135.2 Governance principles

1. Moderation policy is explicit and versioned.
2. Automated detection produces proposals or bounded actions according to risk.
3. Consequential moderation has evidence and appeal paths.
4. Private content is not broadly scanned merely because community abuse exists elsewhere.
5. Federation permits local communities to choose trust policies.
6. Reputation never replaces identity, evidence, or due process.
7. Safety controls cannot become hidden employee or citizen surveillance.
8. Child safety, harassment, fraud, impersonation, malware, and non-consensual content require dedicated threat models.
9. Extension and agent marketplaces require reporting, revocation, and emergency disable paths.
10. The product team must define what it will not host or facilitate before public federation.

### 135.3 Community governance possibilities

Domain Packs may implement:

- member proposals;
- voting or consent processes;
- delegated moderation;
- rotating roles;
- transparent budgets;
- community archives;
- federated group rules.

These are optional governance profiles, not one mandatory political model imposed by the kernel.


# Part XVII — Malleable Product Experience and Ecosystem Composition

## 136. Universal Object, View, and Interaction System

A platform that supports unknown future domains cannot ship a bespoke page for every object type. It requires a common view grammar that Domain Packs and users can safely compose.

### 136.1 Universal view primitives

The view system should support:

- document;
- record detail;
- table;
- board;
- list;
- timeline;
- calendar;
- graph;
- map;
- gallery;
- hierarchy/tree;
- matrix;
- dashboard;
- inbox/queue;
- conversation;
- form;
- report;
- comparison;
- diff;
- provenance trace;
- workflow/run view;
- simulation branch view;
- media review;
- time-series chart;
- custom composition.

These views should bind to schema-aware queries rather than proprietary duplicated data stores.

### 136.2 Field and component grammar

A declarative UI manifest should be able to express:

- scalar, rich-text, code, media, relation, location, temporal, credential, money, unit, and uncertainty fields;
- validation and required states;
- edit/read/hidden conditions;
- permission-sensitive rendering;
- provenance and source badges;
- confidence and status;
- actions and approval requirements;
- computed values;
- responsive layout;
- keyboard interactions;
- screen-reader semantics;
- print/export representation;
- empty/loading/error/offline/conflict states.

### 136.3 Native quality without semantic fragmentation

The same semantic view definition may be rendered differently by:

- SwiftUI/AppKit;
- Kotlin/Compose;
- Svelte/Tauri/web;
- CLI;
- accessibility or voice interfaces;
- external third-party clients.

A native client can use platform-native controls and navigation while honoring the same object contracts, policy, actions, events, and accessibility requirements.

### 136.4 Personal and team customization

Users should be able to:

- save filtered views;
- change grouping and sorting;
- create formulas;
- define dashboards;
- compose object types;
- add fields through pack-supported schema extensions;
- create commands and workflows;
- publish templates to a Space;
- fork a template;
- compare and merge template changes;
- revert to a known version.

Customization should not require users to fork the product source code, while expert users retain an SDK and code escape hatch.

### 136.5 Accessibility as a schema concern

A Domain Pack is not conformant unless its views provide:

- semantic labels;
- focus order;
- keyboard operation;
- text alternatives;
- status announcements;
- non-color state differentiation;
- reduced motion behavior;
- scalable text;
- error identification and correction;
- accessible approval and diff review;
- locale-aware dates, units, and number formats.

Accessibility metadata belongs in the view contract and conformance suite, not only in final manual testing.

---

## 137. Domain Pack Studio and the Gentle Slope from User to Creator

The product should provide a progression from using a template to creating a complete governed Domain Pack.

### 137.1 Creation surfaces

| Surface | Audience | Output |
|---|---|---|
| Template gallery | ordinary user | configured Space and views |
| Schema builder | power user | fields, relations, validation |
| View composer | power user/designer | reusable interfaces |
| Formula editor | analyst | derived fields and metrics |
| Workflow builder | operator | triggers, conditions, approvals, actions |
| Policy designer | administrator | scoped policy templates and simulations |
| Agent designer | expert user | skills, instructions, grants, evals, budgets |
| Pack SDK | developer | signed package, migrations, tests, connectors |
| AI Pack Copilot | all tiers | proposed pack changes as reviewable artifacts |

### 137.2 Pack creation lifecycle

```text
Need identified
→ start from template or blank pack
→ define objects and source authority
→ define views and commands
→ define workflows and agents
→ define permissions, retention, and egress
→ generate fixtures and tests
→ simulate policies and migrations
→ run conformance suite
→ sign and publish locally/Space/registry
→ observe use and collect explicit feedback
→ version, migrate, deprecate, or fork
```

### 137.3 AI-assisted pack creation

A user might say:

> Create a system for managing a multi-site laboratory, with instruments, calibration, experiments, sample custody, maintenance, project budgets, and weekly reports.

The AI may propose:

- pack dependencies;
- schemas;
- source-authority assignments;
- views;
- forms;
- workflows;
- policies;
- agents;
- connectors;
- test data;
- evaluation criteria;
- migration and uninstall behavior.

The user sees a structured diff and simulation. The AI cannot activate external actions, broaden access, or install unreviewed code as a side effect of the conversation.

### 137.4 Forking and communal creation

A Domain Pack can be:

- private;
- shared within a household or organization;
- published to a registry;
- forked;
- extended by another pack;
- localized;
- specialized for a jurisdiction or profession;
- compared and merged.

The ecosystem should preserve authorship, license, dependencies, security review, compatibility, and upgrade lineage.

### 137.5 Pack quality signals

- conformance status;
- automated security scan;
- permission footprint;
- network destinations;
- data classes;
- maintenance activity;
- compatibility matrix;
- migration quality;
- accessibility score;
- performance/resource profile;
- reproducible evaluation results;
- verified publisher identity;
- incident and revocation history;
- user-reported fit by context—not a single gamified popularity score.

---

## 138. Communication, Attention, Presence, and Social Context Beyond Slack

Communication should be treated as one input and output of coordinated work, not the place where all organizational truth disappears.

### 138.1 Communication surfaces

- channel;
- direct message;
- group message;
- thread;
- announcement;
- comment on an object;
- decision discussion;
- run discussion;
- meeting or voice room;
- external relay;
- public/federated post;
- structured update;
- approval request;
- incident alert;
- digest;
- asynchronous video/audio message.

### 138.2 Message classes

Every message should be classifiable as:

- conversational;
- informational;
- action requested;
- decision proposed;
- approval required;
- status update;
- alert;
- external commitment;
- automated system event;
- agent progress;
- public statement;
- confidential/privileged;
- ephemeral.

The classification can be user-selected, source-provided, rule-derived, or agent-proposed. It must never silently change the legal or canonical meaning of the content.

### 138.3 Communication-to-work promotion

A message can be promoted into:

- task;
- decision;
- risk;
- issue;
- document;
- claim;
- meeting agenda item;
- workflow;
- agent Work Order;
- follow-up;
- durable memory candidate.

Promotion creates a linked object with source, proposer, approver, and status. It does not mutate raw chat into canonical knowledge automatically.

### 138.4 Universal inbox and attention contract

The unified Inbox should combine:

- mentions;
- direct messages;
- approvals;
- agent requests;
- blocked workflows;
- assigned work;
- scheduled reviews;
- connector failures;
- policy conflicts;
- important external changes;
- security alerts;
- expiring credentials or consent;
- knowledge-quality issues.

Each item should state:

- why it reached the user;
- urgency and due time;
- required action;
- source and principal;
- related Space and workflow;
- what will happen if ignored;
- whether an agent can help;
- how to mute, delegate, schedule, or change the policy.

### 138.5 Attention budgets

Users and organizations can set:

- quiet hours;
- channel and Space priorities;
- maximum automated notifications;
- escalation rules;
- batching and digest frequency;
- interruption eligibility;
- preferred device and modality;
- accessibility accommodations;
- agent office hours;
- emergency exceptions.

### 138.6 Presence and availability

Presence must be privacy-preserving and purpose-specific. Distinguish:

- online;
- actively editing;
- available for interruption;
- focus mode;
- on call;
- agent available;
- agent running;
- agent waiting for input;
- stale/offline device.

Do not derive employee performance metrics from presence without explicit, lawful, and transparent policy.

---

## 139. Agent Collaborator Experience Beyond the Bot Metaphor

Agents should be understandable as delegated collaborators with scopes, evidence, and operational state—not animated chat personalities that obscure risk.

### 139.1 Agent roster

The Agents surface should show:

- identity and sponsor;
- purpose and skills;
- model/runtime/version;
- connected tools;
- allowed Spaces and data classes;
- action authority;
- current and scheduled Runs;
- cost and resource use;
- evaluation record;
- reliability and correction burden;
- outstanding approvals;
- known limitations;
- incident/suspension history;
- last configuration change.

### 139.2 Agent office hours and service contracts

An agent can publish:

- available skills;
- accepted input schemas;
- output schemas;
- response expectations;
- cost/compute profile;
- data requirements;
- escalation behavior;
- human reviewer requirements;
- maintenance owner;
- jurisdiction or environment limitations.

### 139.3 Handoffs

Support:

- human to agent;
- agent to human;
- agent to agent;
- team to another team;
- local agent to external A2A agent;
- automated workflow to incident commander.

A handoff capsule includes objective, current state, evidence, unresolved questions, permissions, budget, deadline, and expected artifact.

### 139.4 Multi-agent councils

Users may compose roles such as:

- proposer;
- verifier;
- red team;
- security reviewer;
- domain expert;
- cost reviewer;
- synthesizer;
- final human decision maker.

Council rules must preserve disagreements and independent evidence. A synthesis agent may summarize but cannot erase dissenting findings or present consensus that did not occur.

### 139.5 Agent memory scopes

An agent may have:

- no persistent memory;
- Run-only memory;
- project memory;
- Space-scoped memory;
- personal-agent memory;
- explicitly approved learning from accepted corrections.

Agent memory is separate from canonical human knowledge and can be inspected, reset, exported, or revoked.

### 139.6 Trust calibration

Do not show a single opaque “agent trust score.” Present evidence such as:

- task class;
- evaluation set;
- success rate;
- verified artifact rate;
- hallucination or unsupported-claim rate;
- external-action error rate;
- cost per accepted outcome;
- age of evaluation;
- model and tool version;
- conditions under which the evidence was measured.

---

## 140. Cross-Domain Composition Studio and Reference Solutions

The platform should offer solution blueprints that demonstrate composition without turning those blueprints into permanent core assumptions.

### 140.1 Blueprint anatomy

A blueprint contains:

- required and optional Domain Packs;
- schema mappings;
- Spaces and roles;
- source-authority assignments;
- connectors;
- views;
- workflows;
- agents;
- policies and consent;
- reports;
- evaluation fixtures;
- migration/uninstall behavior;
- deployment profile.

### 140.2 Reference solution portfolio

The initial research portfolio should include diverse blueprints such as:

| Reference solution | Architectural stress |
|---|---|
| Personal knowledge and life operations | local-first, longevity, household roles, privacy |
| Human–agent software project | repositories, agent execution, verification, releases |
| Research laboratory | instruments, samples, lineage, reproducibility, budgets |
| Regulated care coordination sandbox | consent, field-level sensitivity, changing authority |
| Cross-organization due-diligence room | federation, contracts, legal hold, selective disclosure |
| Community and public working group | moderation, federation, accessibility, public records |
| Media production pipeline | large artifacts, rights, provenance, approvals |
| Incident command exercise | degraded connectivity, roles, geospatial state, escalation |
| Learning and credential portfolio | evidence-backed skills, assessment integrity, credentials |
| Household/device orchestration lab | edge nodes, telemetry, commands, safety boundaries |
| Agent service marketplace simulation | identity, budgets, reputation, dispute, escrow |
| Digital legacy and continuity exercise | recovery, successor access, staged disclosure |

These are not product-market commitments. They are reference implementations used to expose hidden coupling and missing primitives.

### 140.3 Blueprint portability

A blueprint should be exportable as:

- pack dependency lockfile;
- schemas and policies;
- views and workflow definitions;
- connector manifests without secrets;
- fixture data;
- evaluation results;
- human-readable deployment guide;
- migration and removal plan.

### 140.4 Solution acceptance rule

A reference solution passes only when:

- its domain-specific behavior resides in packs and configuration;
- the core is unchanged except for a broadly reusable primitive accepted through the core admission test;
- removal leaves no orphaned authority or inaccessible canonical data;
- another client can render and operate its public contracts;
- all consequential effects produce receipts;
- all AI writes remain governed;
- backup, restore, and export succeed.


# Part XVIII — Proof Portfolio, Roadmap, and Governance of Expansion

## 141. Replace Sample Vertical Slices with a Capability Proof Portfolio

The earlier plan used a small set of vertical slices to prove interoperability. Those remain valuable, but they are insufficient as the governing validation strategy. The expanded platform requires a portfolio of orthogonal proofs that exercise different failure modes.

### 141.1 Proof families

| ID | Proof | What it validates | Minimum result |
|---|---|---|---|
| P-A | File-first sovereign Vault | portability, stable identity, exact/semantic retrieval, guarded writes | Obsidian/Markdown round-trip with no silent source modification |
| P-B | Generic connector | manifest, OAuth/webhook/file modes, idempotency, source authority | same normalized object imported through two independent connector implementations |
| P-C | Shared Space collaboration | identity, membership, isolation, communication, work promotion | two humans and one agent collaborate with zero cross-Space leakage |
| P-D | Durable agent execution | Work Order, Run, workflow history, grants, pause/resume, artifact | process restart and seven-day approval wait do not lose state |
| P-E | Reliable external action | outbox/inbox, secret broker, idempotency, receipt, compensation | one message/action delivered exactly once across crash boundaries |
| P-F | Consent and regulated disclosure | field-level policy, purpose, revocation, redaction | recipient receives minimum necessary data; revocation blocks future use |
| P-G | Federated query | remote policy, query-to-data, signed results, source freshness | answer cites authorized results from at least three independent nodes |
| P-H | Device and edge | telemetry, stale state, offline buffering, safe command boundary | device reconnect reconciles without duplicate command or false live state |
| P-I | Research reproducibility | raw/derived separation, methods, lineage, package export | figure/result can be traced and reproduced from frozen inputs |
| P-J | Software supply chain | repository context, build/test/deploy provenance, attestation | accepted release links source, build, tests, SBOM, artifact, and reviewer |
| P-K | Credential and selective disclosure | issuer/holder/verifier, status, limited presentation | user proves one claim without exposing unrelated credential fields |
| P-L | Resource governance | hierarchical budgets, provider routing, stop conditions, value | runaway agent stops before exceeding cost/time/action limits |
| P-M | Scenario branch | observed/simulated separation, branch comparison, merge decision | simulation cannot mutate live state; selected decision is explicitly promoted |
| P-N | Records and continuity | retention, hold, archive, restore, successor access | deletion is blocked by hold; recovery/transfer is auditable and scoped |
| P-O | Moderation/federation abuse | reporting, quarantine, appeal, node trust, revocation | malicious extension/node can be isolated without corrupting local records |
| P-P | Unknown-domain challenge | pack architecture and malleability | useful new domain built without kernel change or governance bypass |
| P-Q | Alternate client | no UI-only feature law | independent client completes a core workflow through public contracts |
| P-R | Pack lifecycle | install, migrate, compose, disable, uninstall, export | pack removal leaves readable data and complete dependency diagnostics |

### 141.2 Cross-proof invariants

Every proof must verify:

- source authority;
- actor and principal attribution;
- explicit grants;
- separate read and action authority;
- provenance;
- policy decision;
- no direct extension database access;
- no direct AI canonical write;
- durable event and receipt;
- failure and recovery behavior;
- export and restore;
- accessibility of human approval surfaces;
- resource accounting;
- versioned conformance evidence.

### 141.3 Proof maturity

| State | Meaning |
|---|---|
| Designed | Contract and threat model exist |
| Fixture-ready | Synthetic/adversarial fixtures exist |
| Implemented | Code path exists |
| Locally verified | Acceptance commands pass on declared environment |
| Failure-injected | Crash, retry, revocation, and malformed-input tests pass |
| Independently reproduced | Another client/node/implementation reproduces result |
| Operationally observed | Real owner-alpha use confirms the workflow |
| Release-gated | Required continuously for release |

No proof is “done” merely because a demo succeeded once.

### 141.4 Reference pack assignment

Each proof should be exercised by at least one Domain Pack, and every reference Domain Pack should exercise multiple proofs. This prevents a synthetic test harness from drifting away from real product composition.

---

## 142. Unknown-Domain Challenge

The strongest test of “examples are a floor” is whether the architecture can support a domain not anticipated by the core team.

### 142.1 Quarterly challenge

At least once per quarter during architecture development:

1. choose three domains not represented in the previous quarter;
2. include one primarily human/document domain;
3. include one regulated or policy-heavy domain;
4. include one physical, scientific, or high-volume domain;
5. provide a short external domain brief to a pack team that did not design the kernel;
6. require a usable workflow built through packs, public APIs, connectors, views, and policies;
7. prohibit kernel changes during the first implementation attempt;
8. record every workaround, missing primitive, and attempted bypass;
9. run conformance, export, uninstall, and alternate-client tests;
10. review whether any proposed core change is genuinely domain-general.

### 142.2 Core-change admission test

A missing capability may enter the kernel only when all are true:

- it is required by at least two materially different domains;
- it cannot be safely expressed by a pack, schema, workflow, policy, view, connector, or adapter;
- its semantics can be stated without domain terminology;
- it preserves the product constitution;
- it has migration, export, security, privacy, and conformance contracts;
- it does not force all deployments to pay its complexity cost;
- a reversal or deprecation path exists;
- the architecture council records accepted and rejected alternatives.

### 142.3 Challenge metrics

| Metric | Target direction |
|---|---|
| Pack-only implementation rate | increase over time |
| Kernel files touched | zero unless core admission accepted |
| Time to first useful workflow | decrease over time |
| Schema/view/workflow reuse | increase |
| Policy bypasses | zero |
| Direct database access | zero |
| Unexportable data | zero |
| Uninstall data loss | zero |
| Alternate-client completion | pass |
| Accessibility critical failures | zero |
| Resource-budget violations | zero |
| Domain expert correction burden | measured and decreasing |

### 142.4 Failure is valuable

A failed unknown-domain challenge should not be disguised as a successful demo. It should produce:

- missing-primitive report;
- workaround inventory;
- security and policy risks;
- UX gaps;
- domain misconceptions;
- performance bottlenecks;
- recommendation: pack improvement, public contract change, new optional service, or core ADR.

---

## 143. Dual-Track Execution Plan: Finish Repository Truth While Designing Expansion

The expanded architecture must not become an excuse to abandon the repository's current P0 truth, safety, and execution gates.

### 143.1 Current repository reality

The current authoritative backlog already states that:

- the product's feature truth is evidence-gated rather than inferred from file presence;
- shared `Actor`, `Workspace`, `Space`, and `Membership` schemas and authorization matrices are not yet implemented;
- `WorkOrder`, `AgentRun`, and `Artifact` state machines for collaborative execution remain unimplemented;
- reliable external effects still lack persisted adapter bindings, durable idempotency, and crash-boundary proof;
- the live relay currently promotes every allowed message into the knowledge graph, violating the communication-to-knowledge boundary;
- the governed agent graph has extensive schema and tests but no executor—`start_run` executes nothing;
- broad CRDT editing, Kafka/microservice decomposition, and Matrix/Solid integrations are deliberately parked pending evidence.

The expansion plan therefore creates four coordinated tracks rather than one enormous feature branch.

### 143.2 Track A — Truth, safety, and executable foundation

Follow the repository's current suggested order unless a new evidence-backed ADR changes it:

1. Correct false or contradictory documentation and progress claims.
2. Repair dead schema, dead fixtures, and vacuous conformance tests.
3. Complete public grant admission and core identity/context/action-grant contracts.
4. Give the inbox/outbox and reliable-effects path a durable runtime.
5. Implement the governed Agent Run executor.
6. Retire superseded plan schemas.
7. Implement guarded writes behind their unlock gates.
8. Keep federation disabled until all declared threat-model gates are verified.
9. Complete the existing end-to-end interoperability slices last.

Add the communication-to-knowledge promotion boundary as an immediate, independently startable P0 because it is a live contamination risk.

### 143.3 Track B — Expansion contracts, no broad feature code

In parallel, produce and ratify:

- capability constitution;
- Domain Pack model;
- semantic profile and schema-lens model;
- object-specific collaboration model;
- durable workflow contract;
- policy and consent model;
- identity/credential/attestation model;
- provenance profiles;
- resource governance model;
- device/edge model;
- records/continuity model;
- simulation/branch model;
- malleable view model;
- pack conformance and unknown-domain challenge.

Track B may create schemas, fixtures, prototypes, and benchmarks. It may not ship broad domain feature code into the core before Track A gates are satisfied.

### 143.4 Track C — Reference Domain Packs

After the minimum extension, policy, workflow, and view contracts stabilize, implement small reference packs in this order:

1. **Personal Knowledge Pack** — exercises file-first Vault, evidence, views, and guarded memory.
2. **Project and Agent Operations Pack** — exercises Spaces, Work Orders, Runs, reports, and software connectors.
3. **Research Object Pack** — exercises provenance, lineage, reproducibility, and external schemas.
4. **Consent and Sensitive Records Sandbox Pack** — exercises field-level policy, consent, redaction, retention, and audit with synthetic data only.
5. **Device Lab Pack** — exercises telemetry, stale state, offline buffering, and safe command proposals against simulated devices.
6. **Community/Federation Sandbox Pack** — exercises moderation, federation, portable identity, and node revocation.

Each pack is deliberately narrow but must pass lifecycle and alternate-client tests.

### 143.5 Track D — Productization and ecosystem

Only after Tracks A–C establish reliable contracts:

- build the Domain Pack Studio;
- publish SDKs and conformance tools;
- enable trusted third-party packs;
- add registries and signing;
- broaden native clients;
- offer hosted/shared nodes;
- add enterprise policy and provisioning;
- pursue public federation;
- evaluate marketplaces and economic exchange.

### 143.6 Program increments

| Increment | Outcome |
|---|---|
| I0 — Truth | claims, docs, tests, schemas, and live behavior agree |
| I1 — Authority | identities, grants, source authority, policy, and promotion boundary work |
| I2 — Durable effects | inbox/outbox, idempotency, receipts, recovery, secret broker |
| I3 — Executable agents | one governed Run executes and returns verified artifact |
| I4 — Pack kernel | schemas, manifests, lifecycle, views, workflows, conformance |
| I5 — Personal composition | Obsidian/Vault plus user-created views and workflows |
| I6 — Shared composition | Spaces, communication, Work Graph, agent reporting |
| I7 — Cross-system | generic connectors, MCP, A2A, external effects |
| I8 — Cross-domain | reference packs and unknown-domain challenge |
| I9 — Federation and continuity | multi-node query/sync, records lifecycle, recovery |
| I10 — Ecosystem | pack studio, registry, third-party developers, enterprise controls |

### 143.7 Parallelism rules

Work may proceed in parallel only when:

- authority and source-of-truth boundaries are explicit;
- migration ownership is clear;
- acceptance tests do not depend on unratified semantics;
- no team builds a second competing implementation of the same primitive;
- integration points are contract-first;
- merge order and rollback are documented.

---

## 144. Required Architecture Documents, ADRs, Backlog Groups, and Conformance Suites

### 144.1 New governing documents

Create:

1. `CAPABILITY_CONSTITUTION.md`
2. `DOMAIN_PACK_MODEL.md`
3. `DOMAIN_PACK_LIFECYCLE.md`
4. `SEMANTIC_INTEROPERABILITY_MODEL.md`
5. `SCHEMA_LENS_AND_MAPPING_MODEL.md`
6. `DURABLE_WORKFLOW_MODEL.md`
7. `POLICY_CONSENT_AND_PURPOSE_MODEL.md`
8. `IDENTITY_CREDENTIAL_ATTESTATION_MODEL.md`
9. `PROVENANCE_AND_LINEAGE_PROFILES.md`
10. `OBJECT_COLLABORATION_AND_CONSISTENCY_MODEL.md`
11. `RESOURCE_AND_ECONOMIC_GOVERNANCE.md`
12. `DEVICE_EDGE_AND_PHYSICAL_SAFETY_MODEL.md`
13. `RECORDS_CONTINUITY_AND_DIGITAL_LEGACY.md`
14. `SIMULATION_AND_BRANCHING_MODEL.md`
15. `MALLEABLE_VIEW_AND_FORMULA_MODEL.md`
16. `DOMAIN_PACK_SECURITY_MODEL.md`
17. `DOMAIN_PACK_CONFORMANCE.md`
18. `UNKNOWN_DOMAIN_CHALLENGE.md`
19. `REFERENCE_SOLUTION_PORTFOLIO.md`
20. `ECOSYSTEM_TRUST_AND_REGISTRY_MODEL.md`
21. `MODERATION_AND_FEDERATION_GOVERNANCE.md`
22. `OBSERVABILITY_AND_EVALUATION_MODEL.md`

### 144.2 Proposed ADR sequence

| ADR | Decision |
|---:|---|
| 013 | Capability algebra and Domain Packs as the expansion boundary |
| 014 | Public schema registry, semantic profiles, and schema lenses |
| 015 | Durable workflow contract and engine boundary |
| 016 | Policy backend, consent, purpose, and delegation model |
| 017 | Actor, workload identity, credentials, and attestation |
| 018 | Provenance/lineage internal model and export profiles |
| 019 | Object-specific collaboration and CRDT admission |
| 020 | Hierarchical resource and economic governance |
| 021 | Malleable view, formula, and end-user programming system |
| 022 | Device/edge data and physical-action safety boundary |
| 023 | Records lifecycle, legal hold, recovery, and digital continuity |
| 024 | Simulation/observed-state separation and scenario branching |
| 025 | Domain Pack lifecycle, signing, migration, and uninstall |
| 026 | Federated query and data-space contracts |
| 027 | Moderation, abuse, and federation governance |

Each ADR must include current repo impact, rejected alternatives, security/privacy implications, migration, reversal, benchmarks, and acceptance tests.

### 144.3 New backlog groups

Add evidence-gated groups to `IMPLEMENTATION_BACKLOG.md` only after the corresponding governing ADR is accepted:

| Prefix | Group |
|---|---|
| CAP | capability algebra and core admission |
| PACK | Domain Pack runtime, manifest, dependencies, lifecycle |
| SEM | schemas, mappings, profiles, validation |
| FLOW | durable workflow and human tasks |
| POL | policy, consent, purpose, and simulation |
| IDA | workload identity, credentials, attestation |
| PROV | provenance, lineage, authenticity profiles |
| CONS | object-specific consistency and collaboration |
| RES | resource, cost, energy, and value governance |
| EDGE | devices, telemetry, geospatial, physical safety |
| REC | records lifecycle, hold, archive, continuity |
| SIM | scenario branches and simulated-state separation |
| VIEW | universal views, formulas, mini-apps, accessibility |
| MOD | moderation, abuse, reputation, federation trust |
| EVAL | proof portfolio and unknown-domain challenges |
| ECOS | SDK, signing, registry, third-party conformance |

Every item needs priority, status, mandate, governing authority, blockers, files, acceptance command, and evidence—matching the repository's existing evidence discipline.

### 144.4 Conformance suites

Create separate suites for:

- pack manifest and dependency resolution;
- schema compatibility and migration;
- source authority and sync;
- permissions and policy;
- consent and revocation;
- workflow durability;
- agent execution and grant preservation;
- external effects and receipts;
- extension sandboxing;
- view accessibility;
- formula determinism and sandboxing;
- resource budgets;
- provenance export;
- pack uninstall and data preservation;
- alternate clients;
- federation and malicious nodes;
- device command safety;
- records hold and recovery;
- simulation/live separation;
- unknown-domain challenge.

### 144.5 Fixture strategy

Maintain:

- minimal fixtures for unit tests;
- golden cross-pack fixture Vault;
- adversarial sensitive-data fixture;
- malformed and malicious connector events;
- long-running workflow histories;
- large-scale synthetic corpus;
- multi-language and accessibility fixtures;
- offline/reconnect and clock-skew fixtures;
- legal hold and deletion-conflict fixtures;
- simulated devices;
- federated node lab;
- external protocol compatibility matrix.

No real medical, financial, employment, or other sensitive user data should be required to validate the early architecture.

---

## 145. Expanded Master Direction and Immediate Ordered Actions

### 145.1 Final product definition

MindVault—under a future cleared platform name—should become:

> **A sovereign, malleable, interoperable context and coordination fabric in which humans, teams, applications, devices, models, and agents can preserve evidence, compose new tools, collaborate, execute durable work, and act across decentralized systems under explicit authority.**

### 145.2 The complete platform formula

\[
\boxed{
\begin{aligned}
&\text{Sovereign Personal Vaults}\\
+&\text{Governed Shared Spaces}\\
+&\text{Typed Evidence and Temporal Knowledge}\\
+&\text{Universal Capability Algebra}\\
+&\text{Composable Domain Packs}\\
+&\text{Malleable Views and End-User Programming}\\
+&\text{Durable Workflows and Human Tasks}\\
+&\text{Human, Device, Application, and Agent Identity}\\
+&\text{Policy, Consent, Purpose, and Delegation}\\
+&\text{Structured Work and Agent Runs}\\
+&\text{Bidirectional MCP and A2A Interoperability}\\
+&\text{Open Connectors, Extensions, and Alternate Clients}\\
+&\text{Federated Context Nodes and Query-to-Data}\\
+&\text{Provenance, Lineage, Attestation, and Trust Ledger}\\
+&\text{Resource, Cost, Energy, and Attention Governance}\\
+&\text{Records Continuity, Recovery, and Digital Legacy}\\
+&\text{Scenario Branching and Observed/Simulated Separation}\\
+&\text{Device, Edge, Geospatial, and Physical-Safety Boundaries}\\
+&\text{Moderation, Abuse Handling, and Community Governance}\\
+&\text{Human Authority, Portability, and Reversibility}
\end{aligned}}
\]

### 145.3 Core product laws added by this expansion

1. Examples and initial vertical slices are minimum proofs, never scope ceilings.
2. New domains enter through Domain Packs before core changes are considered.
3. The kernel contains only domain-general authority, evidence, execution, lifecycle, and interoperability primitives.
4. Users can reshape views, schemas, formulas, workflows, and agents through a gentle slope from template to code.
5. Every domain-specific standard is an optional profile or adapter unless a separate ADR proves it belongs in the core.
6. A feature is incomplete until public contracts, events, permissions, provenance, export, recovery, and conformance exist.
7. Different objects use different collaboration and consistency models.
8. Long-running agent and human work requires durable workflow state.
9. Read permission, action authority, consent, purpose, and delegated authority are distinct.
10. Resource, money, energy, attention, and risk budgets are enforceable—not dashboard-only analytics.
11. Observed, reported, derived, assumed, simulated, predicted, intended, commanded, and confirmed states remain distinguishable.
12. Physical actions require a stricter boundary than ordinary software writes.
13. Communication does not become knowledge without governed promotion.
14. Provenance quality is graded and visible.
15. The platform supports logical centrality without mandatory physical centralization.
16. Digital continuity includes recovery, provider exit, archival, and user succession.
17. Federation and marketplaces require governance, abuse handling, and revocation—not only protocol compatibility.
18. Every quarter, the unknown-domain challenge tests whether the architecture remains genuinely open-ended.

### 145.4 Immediate ordered actions

#### Protection and truth

1. Preserve the current public commit, repository bundle, data fixtures, license boundary, and SBOM.
2. Reconcile all documentation and progress claims with executable evidence.
3. Assign owners and evidence paths to every P0 obligation.
4. Fix the live communication-to-knowledge promotion violation.
5. Repair dead fixtures, dead schema paths, and vacuous tests.

#### Current foundation

6. Complete public grant admission and identity/context/action-grant contracts.
7. Implement durable inbox/outbox and reliable external effects.
8. Add crash-boundary and duplicate-delivery tests.
9. Implement the governed Agent Run executor.
10. Retire or freeze superseded planning schemas.
11. Complete guarded-write gates.
12. Keep federation and high-risk external action disabled until all threat-model gates pass.

#### Expansion governance

13. Ratify `CAPABILITY_CONSTITUTION.md`.
14. Ratify the Domain Pack and lifecycle models.
15. Ratify semantic profiles and schema lenses.
16. Ratify durable workflow semantics.
17. Ratify policy/consent/delegation semantics.
18. Ratify identity/credential/attestation semantics.
19. Ratify provenance and lineage mappings.
20. Ratify object-specific consistency semantics.
21. Ratify resource and economic governance.
22. Ratify device/edge safety boundaries.
23. Ratify records continuity and digital legacy.
24. Ratify simulation/live-state separation.
25. Ratify malleable view and formula contracts.

#### Minimal platform implementation

26. Build the pack manifest parser and dependency resolver.
27. Build the schema registry and validator.
28. Build pack-scoped migrations and rollback.
29. Build the public command/query/event contract for pack objects.
30. Build the declarative view runtime.
31. Build formula sandbox and dependency graph.
32. Build durable human-task/workflow primitives.
33. Build policy simulation and explainable decisions.
34. Build resource budget enforcement.
35. Build pack install/disable/uninstall/export conformance.
36. Build the first independent alternate client or CLI path.

#### Reference proofs

37. Complete File-first Vault proof.
38. Complete generic meeting/capture connector proof.
39. Complete Shared Space collaboration proof.
40. Complete durable agent execution proof.
41. Complete reliable external-action proof.
42. Complete synthetic consent/sensitive-record proof.
43. Complete federated query proof.
44. Complete simulated device/edge proof.
45. Complete research reproducibility proof.
46. Complete software supply-chain proof.
47. Complete resource-budget proof.
48. Complete records-continuity proof.
49. Complete the first unknown-domain challenge.

#### Product and ecosystem

50. Build Domain Pack Studio after pack contracts stabilize.
51. Publish Rust, TypeScript, and Python SDKs plus conformance tools.
52. Build signing, trust classes, registries, and revocation.
53. Add native Swift and Kotlin renderers for the universal view contract.
54. Expand reference solutions only after core proof coverage remains green.
55. Begin formal product naming, trademark, domain, package, and app-store clearance before public beta.

### 145.5 Decisions explicitly deferred

Do not automatically begin:

- a global CRDT document rewrite;
- Kafka or broad microservice decomposition;
- an unrestricted workflow engine;
- public federation;
- financial transactions;
- medical decision support;
- safety-critical device control;
- an open agent marketplace;
- autonomous external messaging without bounded authority;
- a universal central copy of all connected data;
- a proprietary schema that replaces domain standards;
- a feature-for-feature office suite clone.

Each requires its own evidence, threat model, architecture decision, and release gate.

### 145.6 Success criteria for the expanded plan

The architecture is succeeding when:

- a new domain can be represented without a core fork;
- users can adapt it without becoming full-time programmers;
- independent clients and agents can interoperate through governed contracts;
- sources retain authority and provenance;
- private context remains private across Spaces, nodes, packs, and agents;
- long-running work survives interruption and revocation;
- external actions are attributable and reliably receipted;
- models and vendors can be replaced;
- data remains readable and exportable;
- resource and attention use stay bounded;
- physical, regulated, and high-risk domains can impose stricter policies without burdening ordinary personal use;
- failed or disputed conclusions remain inspectable;
- the system can outlive individual applications, devices, organizations, and model providers.

### 145.7 Research-derived design influences

The expansion drew on adjacent primary-source concepts while preserving MindVault-owned internal contracts:

- local-first software for ownership, offline use, privacy, and longevity;
- malleable software and gradual enrichment for user agency and composable tools;
- durable workflow patterns for timers, signals, retries, human approval, and audit;
- policy-as-code and principal/action/resource/context models;
- workload identity and trust-domain federation;
- W3C provenance and Verifiable Credentials;
- research-object packaging and lineage;
- software supply-chain and media authenticity attestations;
- healthcare, education, industrial, robotics, finance, records, and civic interoperability profiles;
- open decentralized communication and user-controlled data nodes;
- cost/value governance and OpenTelemetry-compatible observability;
- current MCP and A2A protocol evolution.

These are research inputs and optional compatibility targets. They do not silently become product dependencies.

### 145.8 Final governing sentence

> **Any authorized human, organization, AI, device, application, or extension should be able to contribute to or act through the system, and users should be able to reshape the system for domains not yet imagined—but no capability may bypass ownership, source authority, context boundaries, policy, consent, provenance, resource limits, reversibility, or human accountability.**

