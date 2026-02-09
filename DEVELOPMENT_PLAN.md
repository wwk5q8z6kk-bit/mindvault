# MindVault Sovereign Intelligence System: Enhanced Comprehensive Development Plan

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
- Single admin owner; no shared multi-user logins.
- All delegated access uses OAuth client credentials or API keys scoped by permission templates.
- OAuth client secrets live only in the Sovereign Keychain; tokens are minted locally as access keys.
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
- **P1** Owner profile linkage (relay contact sync done; federation identity propagation pending).
- **P1** Delegation auth hardening (OAuth client lifecycle, audit trails, tests, and docs).
- **P1** Adapter hardening (email stabilization + Slack/Discord adapters on the same contract).
- **P1** Proposal inbox UX hardening (diff clarity, undo/revoke, provenance surfacing).
- **P1** MCP scoping tests + audit linkage.
- **P2** Device sync conflict resolution + recovery UX.
- **P2** Multi-modal extraction quality (PDF/image/audio edge cases).
- **P3** Performance profiling on large vaults and packaging/onboarding polish.

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

### 4) Master Program Board
- [~] `P0` Keep sovereign defaults strict (offline-first, local embeddings, explicit opt-ins)
- [~] `P1` Owner profile as canonical identity source (API/UI + relay sync implemented; federation linkage pending)
- [~] `P1` Delegation auth (OAuth client credentials + API key lifecycle; audit/tests pending)
- [x] `P1` Email adapter end-to-end (IMAP inbound + SMTP outbound + attachment ingestion)
- [~] `P1` Email adapter stabilization (compile clean, test matrix, failure-path hardening)
- [ ] `P1` Exchange inbox hardening (proposal review UX, diff clarity, undo/revoke)
- [ ] `P1` MCP hardening (tool scoping tests, permission templates, audit linkage)
- [ ] `P2` Additional relay adapters (Slack, Discord) on same adapter contract
- [ ] `P2` Device sync hardening (conflict strategy + recovery flow)
- [ ] `P2` Multi-modal extraction quality passes (pdf/image/audio edge cases)
- [ ] `P3` Performance passes on large local vaults
- [ ] `P3` Packaging and install polish

### 5) Adapter Strategy (Shared Contract)
- adapter interface requirements:
- inbound ingest path -> relay message -> vault registration
- outbound delivery path <- relay outbound queue
- contact/channel resolution with deterministic mapping
- idempotency and cursor state persistence
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
- [~] Owner profile API + UI (persisted profile; relay sync done, federation linkage pending)
- [x] Email adapter: IMAP inbound, SMTP outbound, attachment ingest, threading
- [~] Email adapter stabilization: compile clean, test matrix, and failure-path hardening
- [ ] Slack adapter (webhook outbound + bot inbound)
- [ ] Discord adapter (webhook outbound + bot inbound)
- [~] Delegation auth (OAuth client credentials + API key lifecycle; audit/tests pending)

### Phase 4 — Ecosystem & Polish
- [x] Plugin framework + management UI
- [ ] Performance profiling + large-vault benchmarks
- [ ] Onboarding/docs + migration polish
- [ ] Community module support and marketplace considerations

## Implementation Status (as of Feb 2026)
Note: “Complete” here means code is present in the repository. Operational validation is tracked in the A→Z Program Map and gap register above.

### Architecture Summary

| Crate | Purpose | Status |
|-------|---------|--------|
| `mv-core` | Models, traits, error types | Complete |
| `mv-storage` | SQLite + LanceDB + Tantivy + Keychain storage | Complete |
| `mv-engine` | Orchestrator: 13 subsystems | Complete |
| `mv-server` | REST (90+ endpoints) + gRPC + WebSocket + UDS | Complete |
| `mv-mcp` | MCP server (stdio transport, scoped access) | Complete |
| `mv-plugin` | WASM plugin framework (hooks, manifest, registry) | Complete (wired to REST) |
| `mv-cli` | CLI with 15+ subcommands | Complete |

### Engine Subsystems (all wired on `MindVaultEngine`)

| Subsystem | Module | Status | REST Routes |
|-----------|--------|--------|-------------|
| Ingest Pipeline | `ingest.rs` | Complete | /api/v1/nodes |
| Recall Pipeline | `recall.rs` | Complete | /api/v1/recall, /api/v1/search |
| Keychain Engine | `keychain.rs` | Complete | 23 endpoints |
| Proactive Engine | `proactive.rs` | Complete | /api/v1/proactive/* |
| Enrichment Pipeline | `enrichment.rs` | Complete | Spawned at startup |
| Reflection Engine | `reflection.rs` | Complete | /api/v1/agent/feedback |
| Autonomy Gate | `autonomy.rs` | Complete | /api/v1/autonomy/* |
| Relay Engine | `relay.rs` | Complete | /api/v1/relay/* |
| MultiModal Pipeline | `multimodal/` | Complete (audio/image/pdf) | /api/v1/multimodal/status |
| Sync Engine | `sync/` | Complete | /api/v1/sync/* |
| Federation Engine | `federation.rs` | Complete (REST transport) | /api/v1/federation/* |
| Metrics Collector | `metrics_collector.rs` | Complete | /api/v1/metrics/* |
| Intent Executor | `intent_executor.rs` | Complete | On-demand via apply_intent() |

### Storage Traits Implemented (SqliteNodeStore)

NodeStore, AgenticStore, ExchangeStore, SafeguardStore, FeedbackStore, AutonomyStore, RelayStore, KeychainStore

### Migrations

| # | File | Status |
|---|------|--------|
| 001 | Core schema | Applied |
| 002 | Keychain | Applied |
| 003 | Agentic (intents, insights, chronicle) | Applied |
| 004 | Exchange (proposals) | Applied |
| 005 | Relay safeguards | Ready |
| 006 | Feedback (agent feedback, confidence overrides) | Ready |
| 007 | Autonomy (rules, action log) | Ready |
| 008 | Relay (contacts, channels, messages) | Ready |
| 009 | Keychain security (brute-force, HMAC) | Ready |
| 010 | Owner profile | Ready |

### Frontend Components (SvelteKit + Tauri)

| Component | Status | Purpose |
|-----------|--------|---------|
| AgentStream.svelte | Complete | Live agent activity WebSocket consumer |
| AiBriefingWidget.svelte | Complete | Daily briefing display |
| AttachmentsPanel.svelte | Complete | File management with preview |
| DueTasksWidget.svelte | Complete | Due/overdue task widget |
| FavoritesBar.svelte | Complete | Pinned items bar |
| HabitsWidget.svelte | Complete | Habit tracking |
| InsightsDashboard.svelte | Complete | AI-generated insights |
| IntentInbox.svelte | Complete | Autonomous suggestion queue |
| RecentNotesWidget.svelte | Complete | Recent notes list |
| TheChronicle.svelte | Complete | Agent reasoning log |
| WhatsNextWidget.svelte | Complete | AI task prioritization |
| Stats page (/stats) | Complete | Productivity dashboard |
| Selection store | Complete | Multi-select infrastructure |

---

## Remaining Work

### Phase 3: Interoperability & Enhancements
**Status:** Near-complete (profile linkage + delegation auth hardening + adapters remaining)

Backend:
- [~] Profile API (GET/PUT) with persisted storage (contact linkage pending)
- [~] Delegation auth: OAuth client credentials + API key lifecycle endpoints (audit/tests/docs pending)
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
- [ ] Slack adapter (webhook outbound + bot inbound)
- [ ] Discord adapter (webhook outbound + bot inbound)

Frontend:
- [~] Profile settings UI (owner identity, signature, default contact info)
- [~] Access keys + OAuth clients UI (API-aligned; testing pending)
- [x] Build relay chat page (/relay) with contacts, channels, messages
- [x] Build autonomy settings UI (/autonomy)
- [x] Build federation peers management UI (/federation)
- [x] Build sync status/export/import page (/sync)
- [x] Build provenance/metrics dashboard (/provenance)

### Phase 4: Ecosystem & Polish
**Status:** In progress

- [x] Define plugin framework (hooks, manifest, registry)
- [x] Wire plugin registry to REST endpoints
- [x] Build plugin management UI (/plugins)
- [ ] Performance profiling + optimization pass (10K/100K/1M node benchmarks)
- [ ] Documentation, onboarding, and migration polish
- [ ] Community module support and marketplace considerations

## Conclusion
MindVault is a sovereign personal intelligence framework: a private canonical vault enhanced by natural communication relay and proactive local agents. It respects cognitive clarity, human control, and privacy while harnessing agentic potential through bounded, owner-mediated interactions. Build the sovereign foundation and relay integration first to establish trust and daily utility, then layer proactive intelligence for deeper differentiation. This is a system designed for long-term personal augmentation in a privacy-first world.
