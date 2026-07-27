# MindVault Sovereign Intelligence System: Enhanced Comprehensive Development Plan

> **Architecture direction notice (2026-07-26):** The Interoperability
> Constitution and ADR 011 are the highest-priority target architecture. ADR
> 010 defines Personal Vault and Collaborative Space semantics within that
> logically central, physically decentralized context fabric. They supersede
> this plan's platform-wide "single owner / no shared state," automatic
> registration, and application-centric integration assumptions. Until those
> contracts are implemented, the plan below describes the current product
> baseline, not the complete target topology.
>
> **Interoperability implementation status (2026-07-26):** Phase 0 contracts
> are ratified. The implemented Phase 1 kernel slices now provide canonical
> `mindvault://` identities, a versioned internal event envelope, atomic,
> principal-scoped idempotent node creation with a durable outbox, immutable
> public schema registration with fail-closed event admission, and governed
> Source Binding registration/rebinding, revisioned Context Node descriptors
> and capability manifests, internal purpose-bound Context/Tool Grants, and a
> transport-neutral outbox lease/retry/dead-letter lifecycle with immutable,
> attributable action receipts, plus a durable consumer inbox with exclusive
> application leases, immutable application receipts, and terminal local
> stream checkpoints.
> **Governed agent execution graph (2026-07-27):** ADR 012 is accepted and the
> `WorkOrder` / `AgentRun` / `Artifact` layer declared by
> `docs/architecture/PROTOCOL_BOUNDARIES.md` now exists. Migration 038 adds
> trigger-enforced Work Orders, node contracts, typed edges, exclusive write
> leases, immutable gate evidence, and digest-verified artifacts. Declared write
> scope resolves against an effective Tool Grant at admission (gate G0), so an
> over-scoped contract never becomes schedulable. Conflict edges are derived
> from intersecting write scope rather than authored. Approval is unbounded and
> holds no leases. Budget decrement, run transition, and event emission share
> one immediate transaction.
>
> Separately, the autonomy gate is now structurally non-bypassable:
> `IntentExecutor::execute` requires an `EffectAdmission`, a token with no
> public constructor. Previously the gate was caller-invoked and `apply_intent`
> did not call it, so that path executed effectful work unchecked.
>
> This governs internal runs only. It does not run an external dispatcher,
> contact a provider, execute a third-party agent, or authorize any external
> side effect. The `plans` schema (024) and its endpoints are superseded.
>
> This is a foundation, not completion of public grant admission/enforcement,
> live provider publishing or subscribing, governed dead-letter redrive,
> remote issuer sequence-gap detection, connector governance, public registry
> transports, or federation. Local checkpoints prove disposition order after
> admission; they do not prove a remote stream was gap-free. A `published`
> receipt proves destination acknowledgement only, not end-to-end exactly-once
> application.

**Sovereign Core + Modular Exchange with Integrated Communication Relay and Agentic Enhancements**

**Scope:** A unified sovereign, local-first personal intelligence system combining a canonical second brain vault, a natural communication relay network for deferral-based human interactions, and proactive agentic capabilities—all under exclusive owner control with absolute privacy and modularity. No vendor lock-in; all components use open protocols and swappable modules.

## Executive Summary

MindVault is a sovereign personal intelligence system: a private, encrypted vault serving as the single canonical second brain, augmented by a natural communication relay and proactive local AI agents. It behaves like familiar messaging and voice applications while injecting high-confidence context from the vault only when explicitly permitted, automatically registering every interaction to enrich the knowledge base over time, and enabling tightly bounded, permissioned exchanges with other people and agents. Everything runs offline on-device using local small language models (SLMs), preserving complete data sovereignty.

Key integrated elements:

- Natural Relay System for text and voice communication with deferral, high-threshold autonomy, and passive vault growth.
- Proactive agentic layer for monitoring, suggestion, reflection, semantic insight, and multi-modal processing.
- Strict emphasis on owner-mediated approvals, provenance, and granular control.

The design delivers a familiar communication experience that becomes progressively smarter through natural use, while keeping full control, transparency, and privacy in the hands of the user.

**Overall Design Philosophy:** High alignment across sovereignty, functionality, security, usability, maintainability, and innovation.

**Project Focus:** Privacy-focused individuals and those seeking deep personal augmentation via local agents and mediated communication. Excels in personal use cases; not designed for multi-user or enterprise shared-state scenarios.

## Vision

Create a communication network that feels and functions like everyday messaging and voice tools, powered by a sovereign personal vault acting as a second brain. It enables natural human exchanges (text or voice), injects precise vault context when permitted and accurate, registers every interaction to continuously improve utility, and supports proactive agentic facilitation—all while preserving absolute user control. The system defaults to safe deferral and direct relay over any form of automation, turning MindVault into a trusted digital cognitive extension.

## Objectives

1. Deliver natural, familiar communication (channels, DMs, voice notes) with optional intelligent context injection.
2. Grow the second brain vault automatically by registering all interactions.
3. Enforce strict, user-configurable autonomy: act autonomously only at very high confidence and in explicitly allowed domains or with specific contacts.
4. Support bounded, permissioned collaboration via controlled query access.
5. Minimize friction from poor framing, context mismatch, and repetition while preserving human judgment and adding proactive insight.

## Aims

- Relay messages and voice notes directly by default.
- Defer intelligently when context is missing or autonomy restricted: present original content with preview and prompt for next action.
- Register every exchange to accumulate canonical, searchable knowledge naturally.
- Allow deep customization of autonomy scope, thresholds, and reach.
- Facilitate daily life, projects, and scheduling through relevant, queryable vault knowledge—always with human confirmation unless high-precision autonomy is explicitly enabled.

## System Principles

1. Human-led by default — relay content directly unless vault context is permitted and highly accurate.
2. Safe deferral for unknowns — show preview and prompt action when no precise match exists or autonomy is off.
3. Opt-in, high-threshold autonomy — autonomous responses only above very high confidence, only in allowed domains/contacts, and fully disableable.
4. Automatic knowledge registration — every interaction becomes part of the vault without manual effort.
5. Granular user control — define access, autonomy levels, quiet hours, export options exactly as desired.
6. Interoperable and familiar — enhance existing chat/voice surfaces rather than replace them.
7. Explainable and reviewable — injected context shows sources, confidence, and is always reversible.
8. Single admin owner — all delegated access is issued and revoked by the owner via OAuth/API credentials.

## Core Architecture: Sovereign Vault Foundation

**Details**: Encrypted local database with hybrid retrieval (vectors + full-text + graph), rich node metadata, automatic backlinks, and provenance tracking. All external input (human or agent) arrives as proposals in a dedicated Exchange Inbox layer; only the owner can review and accept. The Natural Relay System integrates as a deferral and registration mechanism for communication.

**Core Design Tenets**:

- Single-owner, canonical vault — no shared state.
- Owner-mediated exchanges — proposals only; no external writes.
- Local-first execution — zero cloud dependency for core functionality.
- Policy-driven governance — explicit rules for autonomy, access, and sharing.
- Modularity — crates and feature flags for swappability.

## Sovereign Second Brain Vault

- Private, encrypted repository for projects, schedules, conversations, preferences.
- Passive growth: logs messages, voice notes, transcriptions, replies, summaries.
- Query surface: answers only when confidence is very high and permission granted.
- Customization engine: rules such as domain-specific autonomy, contact-specific access, global disable, adjustable thresholds.
- Learning mode toggle: optional prompts to suggest tags after key exchanges.
- Export/import: JSON and open formats for rules, preferences, and selected vault items.
- Conflict detection: flags contradictory updates for owner confirmation.
- Data lifecycle controls: retention policies, secure deletion, encrypted backups, and integrity checks.

## Communication Network Layer (Natural Relay System)

- Supports channels, direct messages, group chats — text and voice notes.
- Default relay flow:
  1. Sender transmits message or voice note.
  2. Delivered to recipient's vault for context check.
  3. High-confidence match + permitted autonomy -> clean answer/summary with confirm/edit step.
  4. No match / low confidence / autonomy off -> notify with short preview; prompt to play, reply, ignore, etc.
  5. Recipient action completes the exchange.
  6. Full thread (original + reply + summary) registered in both vaults.
- Voice handling: transcription and summary on demand; playback user-initiated unless high-confidence autonomy active.
- Multi-device sync: primary device receives prompt; registrations propagate instantly.
- Offline-first: queue incoming items with expiration; present in arrival/urgency order on reconnect.
- Quick actions on deferral: emoji replies (acknowledge & ignore, block sender, snooze).
- Context injection transparency: show sources and confidence; always editable before sending.

## Autonomy & Precision Controls

- Confidence gate: autonomous reply only at very high certainty (user-adjustable).
- Toggles: global, domain-specific (projects, scheduling, personal), contact-specific.
- Policy engine: conditional rules by time, channel, and context; explicit approval required for new scopes.
- Per-contact presets: quick templates (e.g., "Family -- schedule only", "Work team -- relay only", "Everyone else -- full deferral").
- Snooze/quiet hours: temporary muting globally or per contact.
- Fallback: always defer to direct relay + user prompt when in doubt.

## Second Brain Facilitation

- Scheduling: vault lookup if permitted and precise, otherwise defer to calendar relay.
- Projects: relay query; register reply for future reference.
- Life: recurring reminders and follow-ups emerge from registered history.
- Customization depth: fine-tune access, learning behavior, organization preferences.
- Optional digest: summary of recent registrations, autonomous actions, and queries.

## Collaboration & Interoperability

- Bounded sharing: owner approves exact query scopes (e.g., "schedule only", "project status only").
- Plug into existing surfaces: Slack, Discord, email, voice apps — acts as intelligent relay layer.
- Easy onboarding: begins as plain messaging/voice; intelligence enabled gradually via configuration.
- Delegation model: AI assistant managers authenticate via OAuth or API keys; no shared-state multi-user accounts.

## Delegation & Identity Model

- Single admin owner; no shared multi-user logins or shared-state accounts.
- Delegated actors are AI assistant managers only; access is via OAuth client credentials or scoped API keys.
- Built-in local OAuth2 server mints tokens; client secrets live only in the Sovereign Keychain.
- Keychain is the single secret store for OAuth clients, adapters, and API keys (no duplicate secret stores).
- Owner profile is the canonical identity for adapters (email, relay, federation).

## Key Safeguards

- No guessing — deferral is the safe default.
- Full audit trail: logged queries, relays, autonomous replies, registrations; tamper-evident and visible per-contact/domain views.
- Instant revocation: disable any rule or access immediately.
- Privacy-first: vault contents never leave device without explicit, rule-based permission.
- Explainability: show sources and confidence for any injected context.
- Retention controls: secure deletion and data lifecycle policies for sensitive content.
- Rate limiting & abuse prevention: caps per sender.
- Registration visibility: preview of saved content + short undo window.
- Ignore/block sender: one-tap action on deferral prompts.

## High-Priority Enhancements: Proactive Agentic Intelligence

1. **Proactive Local Agent Monitoring & Suggestion Engine**
   Lightweight watcher (local SLM) scans vault changes and proposes actions/insights via Inbox.

2. **Self-Improving / Reflection Loop for Agents**
   Logs accept/reject feedback to personalize future proposals (local fine-tuning).

3. **Semantic Insight Engine**
   Advanced hybrid retrieval + local LLM for conceptual links and insight proposals.

4. **Multi-Modal Local Processing**
   Native embedding and reasoning over images, audio, PDFs (CLIP, Whisper-local, etc.).

## Medium/Lower-Priority Enhancements

- Federated / shadow protocol compatibility (read-only across approved vaults).
- Privacy-preserving fine-tuning hooks (offline on vault data).
- Audit-grade provenance visualization (dashboards and influence graphs).

## Key Architectural Components (Implementation Focus)

1. **Vault Core** — SQLite + LanceDB + Tantivy; encrypted; rich nodes; hybrid retrieval.
2. **Exchange Layer** — Inbox for proposals; modular primitives (reminders, artifacts, queries, context handoffs); extended for relay deferrals.
3. **Protocols** — MCP (primary agent protocol) plus REST/gRPC/WebSocket/UDS for clients; additional protocols deferred until demand.
4. **Interfaces** — REST/gRPC/WebSocket/CLI; Tauri desktop/mobile; visual inbox with diff previews.
5. **Security & Observability** — Encryption at rest; provenance logging; rate limiting; metrics.
6. **Policy & Governance Layer** — Rule engine, consent logs, and review workflows.

## Competitive Positioning

MindVault stands apart by combining absolute sovereignty, native agent interop, local-first privacy, high modularity, and zero lock-in risk—especially in an era moving toward agentic personal knowledge systems.

## Implementation Approach

- Build thin vertical slices that include UX, policy, and storage together.
- Default to offline/local behavior; networked integrations are explicit opt-ins.
- Gate new agentic behaviors behind feature flags and permission templates.
- Maintain backward-compatible migrations with export-first safety.
- Validate relay, deferral, and proposal flows with repeatable test scenarios.

## Reality Check & Gap Register (Feb 2026)

This section is the factual anchor for execution: what is verified in the codebase, what claims require calibration, and the highest-leverage gaps to close next.

### Verified in the codebase (evidence)

- Encryption at rest (AES-256-GCM + Argon2id) — `crates/mv-storage/src/crypto.rs`
- OpenAPI + Swagger UI — `crates/mv-server/src/openapi.rs`
- Prometheus metrics + audit logging — `crates/mv-server/src/metrics.rs`, `crates/mv-server/src/audit.rs`
- Local embeddings (FastEmbed) + hybrid search — `crates/mv-storage/src/vector.rs`, `crates/mv-engine/src/engine.rs`
- gRPC + REST + WebSocket + UDS transports — `crates/mv-server/src/grpc.rs`, `crates/mv-server/src/lib.rs`
- Permission system + rate limiting — `crates/mv-server/src/auth.rs`, `crates/mv-server/src/limits.rs`
- Node versioning + export/import — `crates/mv-server/src/rest/node_versions.rs`, `crates/mv-server/src/rest/sync.rs`
- Obsidian/Markdown import — `crates/mv-engine/src/import/obsidian.rs`, `crates/mv-cli/src/commands/import.rs`
- MCP server (stdio) + tool/resource surfacing — `crates/mv-mcp/`
- LLM provider abstraction (OpenAI-compatible + fallback) — `crates/mv-engine/src/llm.rs`
- Email adapter (IMAP inbound + SMTP outbound) — `crates/mv-server/src/email.rs`
- Owner profile persistence (migration + API + UI) — `migrations/010_profile.sql`, `crates/mv-server/src/rest/profile.rs`, `frontend/src/routes/settings/profile/+page.svelte`
- Local OAuth2 client-credentials server + Keychain-backed secrets — `crates/mv-server/src/rest/oauth.rs`, `crates/mv-engine/src/keychain.rs`, `frontend/src/routes/settings/profiles/+page.svelte`
- Owner profile identity propagation (relay contact sync + email adapter sourcing) — `crates/mv-engine/src/engine.rs`, `crates/mv-server/src/email.rs`

### Claims that must be calibrated

- AI features are LLM-backed only when a provider is configured and reachable; otherwise they fall back to heuristics. UI and docs must surface provider status clearly.
- “Autonomy” is high-threshold and policy-gated; any new automation must default to deferral unless explicitly enabled.

### Active gaps (prioritized)

- ~~**P1** Owner profile linkage~~ — **DONE**: relay contact sync + federation identity endpoint + handshake auto-discovery + HMAC-SHA256 request signing (20 tests).
- ~~**P1** Delegation auth hardening~~ — **DONE**: 19 tests covering constant-time eq, credential lifecycle, admin gating, basic auth, RFC3339 parsing, revocation.
- ~~**P1** Adapter hardening~~ — **DONE**: 57 adapter tests (Discord 15, Slack 11, Email 18, Poll orchestration 24), Mutex deadlock fix, cursor persistence.
- ~~**P1** Proposal inbox UX hardening~~ — **DONE**: 16 exchange tests (diff preview, glob matching, error mapping, sender resolution, batch size).
- ~~**P1** MCP scoping tests + audit linkage~~ — **DONE**: 66 MCP tests (auth scoping 35, tools 8, resources 8, server 15).
- ~~**P2** Device sync conflict resolution + recovery UX~~ — **DONE**: vector-clock conflict detection + import summaries + conflict review UX.
- ~~**P2** Multi-modal extraction quality (PDF/image/audio edge cases)~~ — **DONE**: 58 tests (23 image, 14 audio, 9 PDF, 12 pipeline/util); command timeout protection, file size limits (256 MB), WebP dimension parsing, JPEG bounds hardening, silent failure fixes.
- ~~**P3** Performance profiling~~ — **DONE**: Criterion benchmarks (SQLite 6, Engine 4, Vector 6); baseline: insert 268µs, get 70µs, list 988µs, vector search 149ms.
- ~~**P3** Packaging and onboarding polish~~ — **DONE**: Cargo workspace metadata (description, repository, categories, keywords), Tauri metadata cleanup, cross-platform release workflow (GitHub Actions: linux-x86_64, linux-aarch64, darwin-x86_64, darwin-aarch64 + SHA256 checksums), install.sh one-liner, CLI crate packaging metadata.
- ~~**P3** Public share links~~ — **DONE**: REST + storage support + web admin UI + tests + docs + Svelte/Tauri UI + public viewer.

---

## A→Z Program Map (Living Task List)

Legend: **[ ]** planned, **[~]** in progress, **[x]** done.

### Autonomous Delivery Protocol (A→Z)

This section is the operational source of truth for day-to-day execution.

### 1) Workflow States

- `Backlog`: approved objective, not started
- `Ready`: scoped with acceptance criteria and test surface
- `In Progress`: implementation active with task checklist
- `Review`: implementation complete, tests passing, docs updated
- `Done`: verified in product and reflected in this tracker

### 2) Change Control & Safety Rules

- Schema changes require migrations plus explicit rollback notes.
- All code, schema, and plan updates are reflected in this tracker; phase exits are summarized with dated notes.
- Secrets never written to disk; OAuth client secrets live only in the Sovereign Keychain.
- External adapters must include rate limits, retry policies, and audit logging.
- Risky behaviors ship disabled by default behind config/feature flags.
- Each phase includes explicit exit criteria and a rollback path.

### 3) Quality Gates Per Item

- Definition of Ready:
  - API/UX contract is explicit
  - storage impact identified
  - security/privacy impact identified
- Definition of Done:
  - implementation complete
  - tests added or updated
  - observability hooks present (logs/metrics/audit where relevant)
  - user-facing docs updated
- Phase progression is sequential; the next phase does not start until the previous exit criteria are met and verified.

### 4) Master Program Board

- [x] `P0` Keep sovereign defaults strict (offline-first, local embeddings, explicit opt-ins)
- [x] `P1` Owner profile as canonical identity source (API/UI + relay sync + federation identity + handshake + HMAC signing; 20 tests)
- [x] `P1` Delegation auth (OAuth client credentials + API key lifecycle; 19 tests, audit trails)
- [x] `P1` Email adapter end-to-end (IMAP inbound + SMTP outbound + attachment ingestion)
- [x] `P1` Email adapter stabilization (compile clean, Debug derive, test matrix, failure-path hardening)
- [x] `P1` Exchange inbox hardening (proposal review UX, undo/batch, diff clarity/provenance; 16 tests)
- [x] `P1` MCP hardening (tool scoping tests, permission templates, audit linkage; 66 tests)
- [x] `P2` Additional relay adapters (Slack, Discord) on same adapter contract
- [x] `P2` Device sync hardening (conflict strategy + recovery flow)
- [x] `P2` Multi-modal extraction quality passes (58 tests; timeout protection, file size limits, WebP parsing, JPEG hardening)
- [x] `P3` Performance passes on large local vaults (SQLite/Engine/Vector benchmarks baselined; Criterion reports in target/criterion/)
- [x] `P3` Packaging and install polish (Cargo metadata, Tauri metadata, cross-platform release workflow, install.sh, SHA256 checksums)
- [x] `P1` Governed agent execution graph (ADR 012): Work Orders, Agent Runs,
  Artifacts with grant-bound write scope, lease-based conflict control, and
  risk-scaled gates. Contracts, migration 038, core types, storage, engine
  admission, the non-bypassable autonomy gate, REST command+query surfaces with
  OpenAPI registration, portable export and restore, read-only extension access,
  the operator surface at `/work-orders`, and a conformance suite covering all
  ten items of the Feature Completeness Contract (12 core, 13 storage, 11
  engine, 4 integration, 13 conformance, 10 frontend tests).

  **Runs are startable, not executable.** `POST
  /api/v1/work-orders/{id}/nodes/{node_id}/runs` is the only path that creates
  an Agent Run. It spends a budgeted attempt, records the run and its event
  atomically, and takes write leases only when the autonomy gate admits;
  otherwise the run parks for the owner before taking any lease. It executes
  nothing — no dispatcher, provider, or third-party agent is invoked. Until this
  landed, an `AgentRun` could only be created from Rust, so every downstream
  route had no subject it could act on.

  **Operator surface is live, and degrades to polling by design.**
  `AgentRunTransitioned` and `AgentRunGateRecorded` are emitted on the agent
  WebSocket and consumed by `/work-orders`, which coalesces bursts into one
  detail reload and raises a toast when a run parks for approval.

  A namespace-scoped session receives none of these events and must poll: the
  execution graph is not namespace-partitioned, so a scoped token — which may
  belong to a delegate rather than the owner, per System Principle 8 — would
  otherwise be handed vault-wide governance signal it did not ask for. The rule
  is `AgentNotification::deliverable_to`, pinned by
  `execution_graph_events_are_withheld_from_namespace_scoped_sessions` so it
  cannot be undone by accident. Refresh stays available for those clients.

  **What this does not prove.** No external dispatcher, no provider contact, no
  third-party agent execution, and no outbound side effect is authorized by this
  layer — see the non-claims in `docs/adr/012-governed-agent-execution-graph.md`.
  Public grant admission and enforcement remain prerequisites, not parallel
  work: a Work Order cannot authorize anything outbound until they land.
  Restore deliberately does not carry authority between vaults; a restored
  contract must be re-authorized locally before any new run can pass G0.

### 5) Adapter Strategy (Shared Contract)

- adapter interface requirements:
- inbound ingest path -> relay message -> vault registration
- outbound delivery path <- relay outbound queue
- contact/channel resolution with deterministic mapping
- idempotency and cursor state persistence (store + poll loop integration done)
- per-adapter rate-limit and retry policy
- email is reference implementation; Slack/Discord follow the same contract

### Phase 0 — Core Vault Foundation

- [x] Unified storage (SQLite + LanceDB + Tantivy) with encryption-ready path
- [x] Ingest + recall pipelines with hybrid search + graph traversal
- [x] Relay core (contacts, channels, messages) with audit trails

### Phase 1 — Sovereignty Lock

- [x] Local embeddings as default with offline-first guarantees
- [x] Keychain integration and encryption-at-rest controls
- [x] Export/import + backup/restore workflows

### Phase 2 — Agent Interop & Proposals

- [x] MCP server with scoped access and tool/resource surfacing
- [x] Proposal node primitive with accept/reject/extract flow
- [x] Autonomy + reflection + feedback loops

### Phase 3 — Relay & Adapters

- [x] Owner profile config (display name, primary email, signature)
- [x] Owner profile API + UI (persisted profile; relay sync + federation identity + handshake auto-discovery + HMAC-SHA256 signing)
- [x] Google Calendar sync (OAuth refresh + sync endpoints + scheduler)
- [x] Email adapter: IMAP inbound, SMTP outbound, attachment ingest, threading
- [x] Email adapter stabilization: compile clean, Debug derive, test matrix, failure-path hardening, Mutex deadlock fix
- [x] Slack adapter (webhook outbound + bot inbound + unit/integration tests, 11 tests)
- [x] Discord adapter (webhook outbound + bot inbound + unit/integration tests, 15 tests)
- [x] Adapter registry + poll orchestration (run_poll_cycle, AdapterPollScheduler, 24 tests)
- [x] Delegation auth (OAuth client credentials + API key lifecycle; 19 tests, audit trails)

### Phase 4 — Ecosystem & Polish

- [x] Plugin framework + management UI
- [x] Performance profiling + large-vault benchmarks (SQLite/Engine/Vector benchmarks baselined)
- [x] Onboarding/docs + migration polish (migration runner with schema_version inserts for all 25 migrations, docs + ADRs)
- [x] AI sidecar proxy endpoints (optional local OpenAI-compatible bridge)
- [x] Public share links (backend + web admin UI + Svelte/Tauri UI + public viewer)
- [x] Community module support and marketplace considerations (manifest: repository/license/homepage/checksum/min_version/keywords; SHA-256 verification on install; 5 new tests)

## Implementation Status (as of Feb 2026)

> **Superseded for execution tracking (2026-07-27).** `IMPLEMENTATION_BACKLOG.md`
> is the authoritative record of remaining work. The checklists in this section
> and in the A→Z Program Map predate the Interoperability Constitution and mark
> items complete under the weaker definition below. Do not plan from them.

Note: “Complete” in the tables below means **only that code is present in the
repository**. It does not mean the capability is governed, reachable,
conformance-tested, or safe to enable. That weaker definition is why several
rows read "Complete" for subsystems whose own contracts still gate them — most
visibly Federation, which `docs/architecture/FEDERATION_THREAT_MODEL.md:22`
requires to stay disabled in production until its release gates pass.

A capability is complete only when it satisfies the ten-point feature
completeness contract in `INTEROPERABILITY_CONSTITUTION.md:112-128` and its
backlog item carries a verification command whose output was observed.
Operational validation is tracked in `IMPLEMENTATION_BACKLOG.md`.

### Architecture Summary

| Crate        | Purpose                                           | Status                   |
| ------------ | ------------------------------------------------- | ------------------------ |
| `mv-core`    | Models, traits, error types                       | Complete                 |
| `mv-storage` | SQLite + LanceDB + Tantivy + Keychain storage     | Complete                 |
| `mv-engine`  | Orchestrator: 13 subsystems                       | Complete                 |
| `mv-server`  | REST (90+ endpoints) + gRPC + WebSocket + UDS     | Complete                 |
| `mv-mcp`     | MCP server (stdio transport, scoped access)       | Complete                 |
| `mv-plugin`  | WASM plugin framework (hooks, manifest, registry) | Complete (wired to REST) |
| `mv-cli`     | CLI with 15+ subcommands                          | Complete                 |

### Engine Subsystems (all wired on `MindVaultEngine`)

| Subsystem           | Module                 | Status                     | REST Routes                    |
| ------------------- | ---------------------- | -------------------------- | ------------------------------ |
| Ingest Pipeline     | `ingest.rs`            | Complete                   | /api/v1/nodes                  |
| Recall Pipeline     | `recall.rs`            | Complete                   | /api/v1/recall, /api/v1/search |
| Keychain Engine     | `keychain.rs`          | Complete                   | 23 endpoints                   |
| Proactive Engine    | `proactive.rs`         | Complete                   | /api/v1/proactive/\*           |
| Enrichment Pipeline | `enrichment.rs`        | Complete                   | Spawned at startup             |
| Reflection Engine   | `reflection.rs`        | Complete                   | /api/v1/agent/feedback         |
| Autonomy Gate       | `autonomy.rs`          | Complete                   | /api/v1/autonomy/\*            |
| Relay Engine        | `relay.rs`             | Complete                   | /api/v1/relay/\*               |
| MultiModal Pipeline | `multimodal/`          | Complete (audio/image/pdf) | /api/v1/multimodal/status      |
| Sync Engine         | `sync/`                | Complete                   | /api/v1/sync/\*                |
| Federation Engine   | `federation.rs`        | Complete (REST transport)  | /api/v1/federation/\*          |
| Metrics Collector   | `metrics_collector.rs` | Complete                   | /api/v1/metrics/\*             |
| Intent Executor     | `intent_executor.rs`   | Complete                   | On-demand via apply_intent()   |

### Storage Traits Implemented (SqliteNodeStore)

NodeStore, AgenticStore, ExchangeStore, SafeguardStore, FeedbackStore, AutonomyStore, RelayStore, ShareStore, KeychainStore

### Migrations

| #   | File                                            | Status  |
| --- | ----------------------------------------------- | ------- |
| 001 | Core schema                                     | Applied |
| 002 | Keychain                                        | Applied |
| 003 | Agentic (intents, insights, chronicle)          | Applied |
| 004 | Exchange (proposals)                            | Applied |
| 005 | Relay safeguards                                | Ready   |
| 006 | Feedback (agent feedback, confidence overrides) | Ready   |
| 007 | Autonomy (rules, action log)                    | Ready   |
| 008 | Relay (contacts, channels, messages)            | Ready   |
| 009 | Keychain security (brute-force, HMAC)           | Ready   |
| 010 | Owner profile                                   | Ready   |
| 011 | Consumer profiles                               | Ready   |
| 012 | Access policies                                 | Ready   |
| 013 | Proxy audit                                     | Ready   |
| 014 | Sync conflicts                                  | Ready   |
| 015 | Contact identity                                | Ready   |
| 016 | Approval queue                                  | Ready   |
| 017 | Shamir key splits                               | Ready   |
| 018 | Breach alert dedup                              | Ready   |
| 019 | Metadata encryption                             | Ready   |
| 020 | Credential ACLs                                 | Ready   |
| 021 | Shamir rotation                                 | Ready   |
| 022 | Adapter poll state                              | Ready   |
| 023 | Conversations                                   | Ready   |
| 024 | Plans (superseded by 038 per ADR 012)           | Ready   |
| 025 | Public shares                                   | Ready   |
| 026 | Node comments                                   | Ready   |
| 027 | MCP connectors                                  | Ready   |
| 028 | Sealed node payloads                            | Ready   |
| 029 | Key epoch re-encryption                         | Ready   |
| 030 | Conversation turn sources                       | Ready   |
| 031 | Knowledge workspace manifest                    | Ready   |
| 032 | Interoperability kernel (outbox, local identity) | Ready  |
| 033 | Governed interoperability registries            | Ready   |
| 034 | Context Node registry                           | Ready   |
| 035 | Authority grants                                | Ready   |
| 036 | Outbox dispatch and action receipts             | Ready   |
| 037 | Consumer inbox and checkpoints                  | Ready   |
| 038 | Work Orders and Agent Runs                      | Ready   |

### Frontend Components (SvelteKit + Tauri)

| Component                | Status   | Purpose                                |
| ------------------------ | -------- | -------------------------------------- |
| AgentStream.svelte       | Complete | Live agent activity WebSocket consumer |
| AiBriefingWidget.svelte  | Complete | Daily briefing display                 |
| AttachmentsPanel.svelte  | Complete | File management with preview           |
| DueTasksWidget.svelte    | Complete | Due/overdue task widget                |
| FavoritesBar.svelte      | Complete | Pinned items bar                       |
| HabitsWidget.svelte      | Complete | Habit tracking                         |
| InsightsDashboard.svelte | Complete | AI-generated insights                  |
| IntentInbox.svelte       | Complete | Autonomous suggestion queue            |
| RecentNotesWidget.svelte | Complete | Recent notes list                      |
| TheChronicle.svelte      | Complete | Agent reasoning log                    |
| WhatsNextWidget.svelte   | Complete | AI task prioritization                 |
| Stats page (/stats)      | Complete | Productivity dashboard                 |
| Selection store          | Complete | Multi-select infrastructure            |

---

## Remaining Work

> **This section is historical. See `IMPLEMENTATION_BACKLOG.md` for remaining work.**
>
> "Interoperability" below means the pre-constitution sense — MCP stdio, relay
> adapters, and peer federation — not the governed context fabric defined by
> `INTEROPERABILITY_CONSTITUTION.md` and ADR 011. The two are different things
> that share a word. Retained for provenance.

### Phase 3: Interoperability & Enhancements (pre-constitution scope)

**Status:** Code complete, governance incomplete. The federation rows below are
code-present but **must not be read as production-ready**:
`docs/architecture/FEDERATION_THREAT_MODEL.md:22` requires production federation
to remain disabled until its ten release gates pass, and peers currently live in
process memory. Tracked as `FED-000`..`FED-012` in `IMPLEMENTATION_BACKLOG.md`.

Backend:

- [x] Profile API (GET/PUT) with persisted storage (contact linkage + federation identity + handshake + HMAC signing)
- [x] Delegation auth: OAuth client credentials + API key lifecycle endpoints (19 tests, audit trails)
- [x] Implement MCP server (tools/resources) with scoped access
- [x] Build relay engine (contacts, channels, messages, blocking)
- [x] Build autonomy gate (rules, quiet hours, rate limiting)
- [x] Build reflection engine (feedback, confidence adjustment)
- [x] Build sync engine (vector clocks, snapshot export/import)
- [x] Build federation engine (peer management)
- [x] Build metrics collector
- [x] Build encrypted backup/restore
- [x] Add migration tools (Markdown/Obsidian/CSV) with dry-run
- [x] Implement multi-modal audio backend (Whisper transcription)
- [x] Implement multi-modal image backend (metadata extraction + dimension parsing)
- [x] Implement multi-modal PDF text extraction (pdftotext + OCR)
- [x] Complete federation query transport (REST-based, parallel peer queries)
- [x] Email adapter (IMAP inbound + SMTP outbound + attachment ingest)
- [x] Slack adapter (webhook outbound + bot inbound + unit/integration tests)
- [x] Discord adapter (webhook outbound + bot inbound + unit/integration tests)

Frontend:

- [x] Profile settings UI (owner identity, signature, default contact info)
- [x] Access keys + OAuth clients UI (API-aligned; 19 tests)
- [x] Build relay chat page (/relay) with contacts, channels, messages
- [x] Build autonomy settings UI (/autonomy)
- [x] Build federation peers management UI (/federation + handshake)
- [x] Build sync status/export/import page (/sync + conflict recovery)
- [x] Build provenance/metrics dashboard (/provenance)

### Phase 4: Ecosystem & Polish

**Status:** Complete

- [x] Define plugin framework (hooks, manifest, registry)
- [x] Wire plugin registry to REST endpoints
- [x] Build plugin management UI (/plugins)
- [x] Performance profiling + optimization pass (Criterion benchmarks: SQLite 6, Engine 4, Vector 6; baseline established)
- [x] Documentation, onboarding, and migration polish (migration runner with schema_version inserts for all 25 migrations; docs: onboarding, plugin-development, CONTRIBUTING, 7 ADRs, architecture)
- [x] Community module support (manifest community fields: repository/license/homepage/checksum/min_version/keywords; SHA-256 checksum verification on install; 5 new tests, 16 total plugin tests)
- [x] Public share links: read-only public viewer for `/public/shares/:token`

## Conclusion

MindVault is a sovereign personal intelligence framework: a private canonical vault enhanced by natural communication relay and proactive local agents. It respects cognitive clarity, human control, and privacy while harnessing agentic potential through bounded, owner-mediated interactions. Build the sovereign foundation and relay integration first to establish trust and daily utility, then layer proactive intelligence for deeper differentiation. This is a system designed for long-term personal augmentation in a privacy-first world.
