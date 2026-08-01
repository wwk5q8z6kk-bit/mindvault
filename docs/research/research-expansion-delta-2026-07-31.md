# MindVault Next — Capability-First Research Expansion

**Status:** Binding amendment to the master enhancement plan  
**Date:** July 31, 2026  
**Applies to:** Sections 0–101 of the July 26 master plan and the current `wwk5q8z6kk-bit/mindvault` repository  
**Owner:** Hassan El Jesr  

> This amendment makes one correction explicit: the examples already described—Obsidian, meetings, interview tools, Slack-style collaboration, coding agents, team reports, career workflows, and federated queries—are minimum conformance proofs. They are not the platform's scope ceiling. The platform must be designed from reusable capabilities and governed Domain Packs so that unplanned personal, organizational, scientific, regulated, industrial, civic, and physical-world domains can be added without corrupting the universal kernel.

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

