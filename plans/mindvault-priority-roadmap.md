# MindVault Priority Roadmap

Last updated: 2026-02-07

## Current Status

- MV-001: JWT + shared-token authentication - Done
- MV-002: RBAC + namespace-scoped authorization - Done
- MV-003: Verification hardening (`verify_all.sh`, smoke path, CI gate) - Done
- MV-004: Input validation caps across REST/gRPC - Done
- MV-005: Rate limiting + namespace quotas - Done
- MV-006: Structured audit logs + stats-tab audit viewer - Done
- MV-007: Backup and export/import safety checks - Done
- MV-011: AI auto-tagging on ingest/update - Done
- MV-012: Rich writing workspace (Markdown + WYSIWYG + AI completion assist) - Done
- MV-013: Daily notes templates + midnight scheduler + auto-linking - Done
- MV-017: Attachment pipeline (upload/list/download/delete) - Done
- MV-032: Wiki-link auto backlinking (`[[...]]` -> `references`) - Done
- MV-033: Template system (CRUD + packs + instantiate + version history API) - Done
- MV-034: AI semantic wiki-link suggestions in rich editor - Done
- MV-040: Markdown/WYSIWYG @mention semantic suggestions + insertion - Done
- MV-035: AI transform actions in rich editor (summarize/action-items/refine) - Done
- MV-036: Saved search workbench (persisted query presets + run API + web controls) - Done
- MV-037: Rich editor grounded auto-suggest mode (auto toggle + provenance metadata) - Done
- MV-038: Node version history (non-template snapshots + restore) - Done
- MV-039: Permission templates + access keys (assistant governance) - Done

## Research-Driven Additions (2026)

| Category | Feature | Priority | Status |
|---|---|---|---|
| Interop | MCP connector marketplace (Notion/GitHub/Calendar connectors) | High | Planned |
| Collaboration | Shared live components (Loop-style task/table blocks in notes) | High | Planned |
| Interop | JSON Canvas import/export for whiteboards and graph boards | High | In Progress |
| AI | Database autofill assistant for typed properties (tasks, people, projects) | High | Planned |
| Publishing | Lightweight publish/share mode for selected knowledge spaces | Medium | Planned |
| Linking | Mention + URL backlink extraction for linked references | Medium | Done |
| Workspace | Edgeless canvas (docs + whiteboards + databases in one surface) | High | Planned |
| AI | Cross-app connectors for AI search/Q&A (MCP-style) | High | Planned |
| AI | Auto-fill database fields with summaries/insights (entity extraction) | High | Planned |
| AI | Diagram/flowchart generation from notes (Mermaid/Canvas) | Medium | Planned |
| Knowledge Model | Object galleries + typed templates (object-first capture) | Medium | Planned |
| Language | Inline translation + multilingual summaries | Medium | Planned |
| Capture | Web clipper + source metadata capture (URL, author, highlights) | High | Planned |
| Knowledge Model | Object-first notes (typed objects with reusable properties) | High | Planned |
| Planning | Time-blocking assistant (task-to-calendar auto-suggestions) | High | Planned |
| AI | Meeting transcript + action extraction pipeline | High | Planned |
| Graph | Perspective lenses (project/person/time filtered graph views) | Medium | Planned |
| Retrieval | Context packs (saved semantic search bundles for projects) | Medium | Planned |
| Automation | Agent workflows with approval gates (research -> draft -> task) | Medium | Planned |
| Privacy | Local encrypted sync profiles (personal/work vault separation) | Medium | Planned |
| Focus | Calm mode (workload pressure signal + reduced-notification view) | Medium | Planned |
| Extensibility | Command palette macros + plugin hooks (Wasm/Lua) | Medium | Planned |
| Planner | Dependency-aware timeline planner (critical-path + slack hints) | High | Planned |
| Capture | Global quick-capture with system hotkeys + notification actions | High | In Progress |
| Interop | JSON Canvas round-trip + board-to-note sync | High | In Progress |
| Retrieval | Qdrant payload-facet search presets (saved filter packs) | Medium | Planned |
| Resilience | Snapshot/restore pipeline for vectors + metadata indexes | Medium | Planned |
| Workflow | Note/task "buttons" for one-click automations (repeatable actions) | Medium | Planned |
| Sync | Optional CRDT collaboration layer for low-conflict co-editing | Medium | Planned |
| AI Ops | Retrieval quality evaluation harness + relevance regression suite | Medium | Planned |
| Knowledge | Unified object schemas (person/project/meeting) with rollup views | High | Planned |
| Focus | Workload balancing assistant (auto-reschedule overload windows) | Medium | Planned |
| AI Routing | Mixture-of-model routing for assist endpoints (link/transform/search) | High | Planned |
| Planning | Dependency-aware schedule auto-shift (blocked task propagation) | High | Planned |
| Knowledge Model | Database-style note views (typed fields, formulas, rollups) | High | Planned |
| Memory | Context compression + semantic cache for long-running projects | Medium | Planned |
| Search | Hybrid query presets (vector + keyword + graph weight profiles) | Medium | Planned |
| Knowledge Graph | Block-level transclusion and reference embeds across notes/canvas cards | High | Planned |
| Query UX | Live query blocks inside notes (saved filters rendered inline) | High | Planned |
| Documents | PDF deep-link annotations + OCR citation backlinks | High | Planned |
| Calendar | Bi-directional calendar planning assistant (note/task/event triage) | High | Planned |
| AI Assist | Meeting-note ingestion with action extraction + task/project linking | High | Planned |
| Automation | MCP-backed tool automation recipes (capture, summarize, schedule) | Medium | Planned |
| UX | Global URI/deep-link protocol for note/task/event jumps across apps | Medium | Planned |
| Governance | Vault data lineage view for AI-generated transformations | Medium | Planned |

## Phase 1 (Fast Security Wins)

### MV-004: Input validation caps
- Status: Done
- Scope:
  - Enforce max payload size for node content/title/tags.
  - Reject invalid namespace/tag formats early.
- Effort: 1-2 days

### MV-005: Rate limiting and quotas
- Status: Done
- Scope:
  - Per-auth-identity request throttling.
  - Namespace storage quotas (node-count guard).
- Effort: 2-4 days

### MV-006: Structured audit logs
- Status: Done
- Scope:
  - Record subject, role, namespace, action, resource ID, status. (Done)
  - Add config toggles and log sink format. (Done)
  - Stats-tab audit viewer with filters (`subject`, `action`, `since`, `limit`). (Done)
  - Stats-tab incremental pagination (`offset` + "Load More") for long audit trails. (Done)
- Effort: 2-3 days

## Phase 2 (Data Safety + Operations)

### MV-007: Backup and export/import
- Status: Done
- Scope:
  - REST export/import endpoints with relationship transfer. (Done)
  - Stats-tab export/import controls in web UI. (Done)
  - CLI backup snapshots + restore flow. (Done)
  - Restore archive preflight validation (entry types + path traversal guard + non-empty archive check). (Done)
- Effort: 3-5 days

### MV-008: Metrics and observability
- Status: Pending
- Scope:
  - Request latency/error metrics.
  - Storage/index health counters.
- Effort: 3-5 days

## Phase 3 (Platform Expansion)

### MV-009: Encryption at rest
- Status: Pending
- Scope:
  - Encrypt SQLite and local vector/index artifacts.
  - Key management and migration plan.
- Effort: 1-2 weeks

### MV-010: Local embeddings + provider abstraction
- Status: Pending
- Scope:
  - Local model provider path.
  - Runtime provider selection and fallback.
- Effort: 1-2 weeks

### MV-011: AI auto-tagging on ingest/update
- Status: Done
- Scope:
  - Feature-flagged lexical + similar-node tag enrichment.
  - Config/env knobs for safe rollout and tuning.
  - Unit + integration coverage for enabled/disabled behavior.
- Effort: 1-2 days

### MV-012: Rich writing workspace + AI completion assist
- Status: Done
- Scope:
  - Web admin modal now supports WYSIWYG, Markdown, and split editing.
  - Bidirectional conversion between HTML and Markdown in-browser.
  - Retrieval-assisted writing suggestions via REST completion endpoint.
- Effort: 1-2 days

## Phase 4 (2026 Product Expansion)

### MV-013: Daily notes templates and auto-linking
- Priority: High
- Status: Done
- Scope:
  - Scheduled daily note generation with template variables. (Done, includes UTC-midnight scheduler)
  - Auto-link tasks/events created on the same day. (Done for native `task`/`event` plus tag-triggered candidates)
- ROI: High (creates structured journaling + planning habit loop)

### MV-014: Recurring tasks/reminders with natural-language scheduling
- Priority: High
- Status: In Progress
- Scope:
  - Task entities with recurrence rules and completion roll-forward. (Native `task`/`event` kinds + scheduler-backed rollforward added)
  - Natural language date parsing and reminder notification dispatch.
- ROI: High (core personal productivity utility)

### MV-015: Calendar workspace (day/week/month) + external sync
- Priority: High
- Status: In Progress
- Scope:
  - Native calendar views and event editing. (Done: REST `GET /api/v1/calendar/items` + `GET /api/v1/calendar/ical` + `POST /api/v1/calendar/ical/import`, web day/week/month workspace with range navigation, iCal import/export, and event quick-capture)
  - iCal import/export and optional Google/Notion calendar sync connectors.
- ROI: High (ties tasks and projects to time)

### MV-016: Projects with hierarchical tasks, Kanban, and milestones
- Priority: High
- Scope:
  - Project entity graph, unlimited subtasks, dependencies, milestone rollups.
  - Kanban and Gantt-oriented views for delivery tracking.
- ROI: High (bridges PKM + execution management)

### MV-017: Attachment pipeline (files, OCR, and extraction)
- Priority: High
- Status: Done
- Scope:
  - Upload/list/download/delete APIs with metadata indexing. (Done)
  - Text-attachment extraction + indexing into node search metadata. (Done)
  - Optional local OCR/PDF extraction adapters (`tesseract`/`pdftotext`) with graceful fallback. (Done)
  - Searchable chunk extraction metadata (`attachment_text_chunks`) with attachment preview surfaced in API/web list views. (Done)
  - Attachment chunk inspector endpoint (`GET /api/v1/files/{node_id}/{attachment_id}/chunks`) + web “View Text” viewer. (Done)
  - Attachment extraction retry endpoint (`POST /api/v1/files/{node_id}/{attachment_id}/reindex`) + web “Reindex” action. (Done)
  - Attachment triage endpoint (`GET /api/v1/files/{node_id}/paged`) with server-side filters/sort/pagination/status facets + web controls. (Done)
  - Failed-only batch reindex endpoint (`POST /api/v1/files/{node_id}/reindex-failed`) + web batch action. (Done)
  - Guarded bulk-delete endpoint (`POST /api/v1/files/{node_id}/delete-filtered`) with dry-run + explicit confirmation count. (Done)
  - Audio transcription for attachments via local Whisper with inline previews. (Done)
  - Remaining: first-class OCR pipeline (native/embedded models).
- ROI: High (critical for real-world knowledge capture)

### MV-033: Templates system (reusable structures + instantiation)
- Priority: High
- Status: Done
- Scope:
  - Template CRUD entry points for reusable note/task structures. (Done: create/list/delete + duplicate + instantiate)
  - Variable placeholders (`{{variable}}`) with runtime substitution. (Done)
  - Template-to-instance lineage tracking (`derived_from` + metadata). (Done)
  - Web template manager UX (filter, create, instantiate, variable form rendering). (Done)
  - Template version history + restore flows (API + web). (Done)
  - Version diff preview before restore (API detail endpoint + web modal + field-level change matrix). (Done)
  - Curated template packs with idempotent install and overwrite support (API + web). (Done)
- ROI: High (reduces repetitive authoring and improves consistency)

### MV-032: Wiki-link auto backlinking and reference sync
- Priority: High
- Status: Done
- Scope:
  - Parse wiki-style links (`[[Title]]`, `[[Title|alias]]`, `[[Title#Heading]]`) from node content.
  - Parse markdown reference links (`[label](Title)` and `[label](Title#Heading)`) plus bare/source URLs for backlink resolution.
  - Parse mention references (`@slug`, `@"Title"`, `@'Title'`) for fast people/project cross-linking.
  - Resolve targets namespace-locally by title or UUID.
  - Resolve URL references against node `source` metadata within namespace.
  - Create/update/remove auto-managed `references` relationships on node create/update.
  - Editor modal relationship context inspector for incoming/outgoing links while editing.
  - Relationship inspector now surfaces auto-reference provenance source (`wikilink`, `markdown_link`, `mention`, `source_url`) for auto-managed edges.
  - Click-through related-node navigation from relationship context cards. (Done)
  - Optional config flags for enablement and scan bounds.
- ROI: High (turns plain notes into navigable networked thought with zero manual graph work)

### MV-034: AI semantic wiki-link suggestions in rich editor
- Priority: High
- Status: Done
- Scope:
  - New `POST /api/v1/assist/links` endpoint for retrieval-ranked wiki-link target suggestions.
  - Markdown editor `[[...]]` context detection with semantic suggestion panel.
  - Markdown editor `@...` / `@"Title"` mention context detection using the same semantic suggestion service.
  - WYSIWYG editor `@...` mention context detection and insertion with shared semantic ranking.
  - Keyboard navigation (`Up`/`Down` + `Tab`/`Enter`) and confidence badges for faster selection.
  - Heading deep-link suggestions (`[[Title#Heading]]`) and alias-preserving insertion (`[[Title#Heading|alias]]`) plus node-exclusion support when editing.
  - Mention insertion with token-aware formatting (`@slug`, `@"Title"`, `@'Title'`) and safe fallback slugification.
  - Context preview snippets for heading and title suggestions to reduce mis-linking.
- ROI: High (faster networked-thought authoring and stronger backlink graph quality)

### MV-035: AI transform actions in rich editor
- Priority: High
- Status: Done
- Scope:
  - New `POST /api/v1/assist/transform` endpoint with `summarize`, `action_items`, and `refine` modes.
  - Retrieval-grounded transforms that reuse vault context from semantic recall.
  - Editor controls to inject transformed markdown blocks directly into active notes.
  - Targeting modes for appending sections or replacing selected markdown with safe async fallback.
- ROI: High (turns drafting into guided execution with low-friction AI edits)

### MV-036: Saved search workbench
- Priority: High
- Status: Done
- Scope:
  - New REST lifecycle for persisted search presets: `GET/POST /api/v1/search/saved`, `PUT/DELETE /api/v1/search/saved/{id}`, and `POST /api/v1/search/saved/{id}/run`.
  - Saved query metadata supports strategy, result limit, namespace scope, kind/tag filters, and score/importance thresholds.
  - Search-tab UI controls for save/load/run/update/delete workflows with active preset highlighting and advanced filter inputs.
  - Optional saved-search fields (`description`, `target_namespace`, `min_score`, `min_importance`) support explicit clear semantics via nullable updates.
  - Namespace-scoped authorization for target namespace filters to prevent cross-scope leakage.
- ROI: High (cuts repeated query friction and improves retrieval workflow throughput)

### MV-041: Saved views (list/kanban/calendar presets)
- Priority: High
- Status: Done
- Scope:
  - Persisted saved views with filters, sort, group_by, and query payloads. (Done)
  - REST lifecycle (`GET/POST/PATCH/DELETE /api/v1/saved_views`) with validation. (Done)
  - Frontend selector to apply saved views across list/kanban/calendar. (Done)
  - Saved view navigation across task/kanban/calendar views. (Done)
- ROI: High (fast context switching and reusable workflow presets)

### MV-037: Rich editor grounded auto-suggest mode
- Priority: High
- Status: Done
- Scope:
  - Added explicit auto-suggest toggle in editor actions to control background AI suggestion polling.
  - Suggestion chips now show grounding provenance (`source_nodes`, retrieval strategy) for better trust and traceability.
  - Suggestion provenance now includes top source-node chips with one-click `[[...]]` citation insertion in the editor.
  - Extended notes workspace rich editor with grounded suggestion/source chips and semantic link recommendation chips (namespace-scoped, current-note exclusion-aware).
  - Added repeated-request dedupe window to reduce unnecessary assist calls while typing unchanged text.
  - Fixed suggestion insertion reliability bug in web editor flow.
- ROI: High (higher writing velocity with lower cognitive overhead and clearer AI provenance)

### MV-038: Node version history (non-template)
- Priority: High
- Status: Done
- Scope:
  - Added REST lifecycle for non-template node history:
    - `GET /api/v1/nodes/{id}/versions`
    - `GET /api/v1/nodes/{id}/versions/{version_id}`
    - `POST /api/v1/nodes/{id}/versions/{version_id}/restore`
  - Non-template node updates now snapshot authored fields/metadata into rolling `node_versions`.
  - Restore flow snapshots current state before applying historical data to keep rollback safety.
  - Web editor modal now renders node history with diff preview and one-click restore actions.
- ROI: High (reduces accidental data loss and enables safer iterative writing workflows)

### MV-018: Visual knowledge canvas (JSON Canvas + Mermaid)
- Priority: Medium
- Scope:
  - Import/export JSON Canvas format for interoperability.
  - Render Mermaid flowcharts/Gantt within notes/projects.
- ROI: Medium-High (better systems thinking and communication)

### MV-019: Semantic memory upgrade (Qdrant option + payload filters)
- Priority: Medium
- Scope:
  - Optional Qdrant backend for large-scale semantic retrieval.
  - Payload-aware filtering and hybrid query pipelines.
- ROI: Medium-High (scales better for larger vaults)

### MV-020: Collaboration primitives (presence, comments, shared spaces)
- Priority: Medium
- Scope:
  - Workspace sharing, role-bound namespaces, inline comments.
  - Realtime updates over WebSocket and change feed replay.
- ROI: Medium (multi-user and AI-agent collaboration readiness)

### MV-021: Quick capture and global shortcuts
- Priority: Medium
- Status: In Progress
- Scope:
  - Desktop quick-capture entry point and global hotkeys. (Done: Tauri global `Cmd/Ctrl+Shift+N` opens task capture; `Cmd/Ctrl+Shift+M/L/V` open note/link/voice capture; `Cmd/Ctrl+Shift+I/D` route capture directly to Inbox/Daily note; shortcuts restore and focus app window.)
  - Capture target routing in quick capture (`Default`, `Inbox`, `Daily note`) with auto-tag and daily-link behavior. (Done)
  - Command palette quick-capture actions dispatch mode+target events for parity with desktop shortcut routing. (Done)
  - Reminder notification click actions can trigger prefilled quick capture with configurable target (`Inbox`, `Daily note`, `Planned`, `Review`, or disabled). (Done)
  - Additional system-level target presets beyond inbox/daily (`Planned`, `Review`) with task-status routing. (Done)

### MV-022: Voice notes and transcription
- Priority: High
- Scope:
  - Record audio notes with whisper-rs for transcription.
  - Integrate with daily notes and quick capture.
- ROI: High (enables hands-free capture)

### MV-023: Multimodal capture (screenshots, screen recordings)
- Priority: Medium
- Scope:
  - Screenshot capture and annotation.
  - Screen recording integration.
- ROI: Medium (enhances visual note-taking)

### MV-024: AI-powered task prioritization and scheduling
- Priority: High
- Status: In Progress (focus planner heuristic + REST + web admin)
- Scope:
  - AI analyzes tasks for priority based on deadlines, dependencies, user patterns.
  - Suggests optimal scheduling. (Pending)
- ROI: High (automates productivity workflows)

### MV-025: Habit tracking with streaks and analytics
- Priority: Medium
- Scope:
  - Track habits with streaks, calendars.
  - Analytics dashboards for progress.
- ROI: Medium (builds on goal tracking)

### MV-026: Goal setting with SMART goals and progress tracking
- Priority: High
- Scope:
  - Define SMART goals, break into tasks.
  - Progress visualization and AI coaching.
- ROI: High (core life management)

### MV-027: Advanced natural language search
- Priority: High
- Scope:
  - Conversational queries like "notes about AI from last month".
  - Enhanced semantic understanding.
- ROI: High (improves discoverability)

### MV-028: Collaborative features (sharing, comments)
- Priority: Medium
- Scope:
  - Share notes/projects, add comments.
  - Real-time collaboration.
- ROI: Medium (enables team use)

### MV-029: Plugin system for extensibility
- Priority: Low
- Scope:
  - Wasm/Lua plugins for custom functionality.
- ROI: Low-Medium (future extensibility)

### MV-030: Mobile app companion
- Priority: Medium
- Scope:
  - Tauri-based mobile app for sync and capture.
- ROI: Medium (ubiquitous access)

### MV-031: External integrations (calendar, email)
- Priority: Medium
- Scope:
  - Sync with Google Calendar, import emails.
- ROI: Medium (reduces silos)
  - Route captured snippets to inbox namespace for triage.
- ROI: Medium (dramatically reduces capture friction)

### MV-022: Insight dashboards and weekly AI review
- Priority: Medium
- Scope:
  - Goal/task completion analytics and streak trends.
  - AI-generated weekly brief with blockers and recommendations.
- ROI: Medium (drives continuous behavior improvement)

## Execution Policy

- Keep `cargo fmt`, strict `clippy`, `cargo test`, and `./scripts/verify_all.sh` green on every milestone.
- Land changes in small vertical slices with transport parity tests (REST + gRPC) for security-sensitive features.
