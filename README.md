# MindVault

MindVault is a local-first knowledge memory system for humans and AI agents.

## Components

- `mv-core`: shared domain models and traits
- `mv-storage`: SQLite node store + LanceDB vector store + embedders
- `mv-index`: Tantivy full-text index + hybrid ranking utilities
- `mv-graph`: relationship storage and traversal
- `mv-engine`: ingest and recall orchestration
- `mv-server`: REST, gRPC, WebSocket transports
- `mv-cli`: command-line client and server launcher
- `web/`: web-based administrative interface
- `connectors/*`: external integrations (MCP/OpenClaw)

## Quick Start

```bash
cargo build --workspace
cargo test --workspace

# Start server (foreground)
cargo run -p mv-cli -- server start --foreground
```

By default, the server binds to `127.0.0.1`:

- REST: `http://127.0.0.1:9470`
- gRPC: `127.0.0.1:50051`

## Configuration

Default config is in `config/default.toml`.

CLI uses `~/.mindvault/config.toml` by default:

```bash
mv config show
mv config set server.rest_port 9480
mv config set server.cors_allowed_origins http://localhost:3000,http://localhost:5173
mv config validate
```

Environment overrides are supported, including:

- `MINDVAULT_REST_PORT`
- `MINDVAULT_GRPC_PORT`
- `MINDVAULT_BIND_HOST`
- `MINDVAULT_DATA_DIR`
- `MINDVAULT_EMBEDDING_MODEL`
- `MINDVAULT_AI_AUTO_TAGGING_ENABLED`
- `MINDVAULT_AI_AUTO_TAGGING_MAX_GENERATED_TAGS`
- `MINDVAULT_AI_AUTO_TAGGING_MAX_TOTAL_TAGS`
- `MINDVAULT_AI_AUTO_TAGGING_SIMILARITY_SEED_LIMIT`
- `MINDVAULT_AI_AUTO_TAGGING_MIN_TOKEN_LENGTH`
- `MINDVAULT_DAILY_NOTES_ENABLED`
- `MINDVAULT_DAILY_NOTES_MIDNIGHT_SCHEDULER_ENABLED`
- `MINDVAULT_DAILY_NOTES_NAMESPACE`
- `MINDVAULT_DAILY_NOTES_TITLE_TEMPLATE`
- `MINDVAULT_DAILY_NOTES_CONTENT_TEMPLATE`
- `MINDVAULT_DAILY_NOTES_DEFAULT_IMPORTANCE`
- `MINDVAULT_RECURRENCE_ENABLED`
- `MINDVAULT_RECURRENCE_SCHEDULER_INTERVAL_SECS`
- `MINDVAULT_RECURRENCE_MAX_INSTANCES_PER_TEMPLATE`

### Owner Profile

The owner profile is used for outbound relay identity (email display name, signature, and default sender):

```toml
[profile]
display_name = "MindVault Owner"
primary_email = "me@example.com"
timezone = "UTC"
signature = "— Sent from MindVault"
```

Environment overrides:

- `MINDVAULT_PROFILE_DISPLAY_NAME`
- `MINDVAULT_PROFILE_PRIMARY_EMAIL`
- `MINDVAULT_PROFILE_TIMEZONE`
- `MINDVAULT_PROFILE_SIGNATURE`

### Email Adapter (IMAP + SMTP)

Enable email relay by configuring IMAP (inbound) and SMTP (outbound). Passwords are stored via the secret store:

```toml
[email]
enabled = true
namespace = "default"
poll_interval_secs = 120
max_fetch = 20
max_attachment_bytes = 5242880
mark_seen = false
imap_host = "imap.example.com"
imap_port = 993
imap_username = "me@example.com"
imap_folder = "INBOX"
imap_starttls = true
smtp_host = "smtp.example.com"
smtp_port = 587
smtp_username = "me@example.com"
smtp_from = "me@example.com"
smtp_starttls = true
```

Secrets:

```bash
mv secret set MINDVAULT_EMAIL_IMAP_PASSWORD
mv secret set MINDVAULT_EMAIL_SMTP_PASSWORD
```

Environment overrides:

- `MINDVAULT_EMAIL_ENABLED`
- `MINDVAULT_EMAIL_NAMESPACE`
- `MINDVAULT_EMAIL_POLL_INTERVAL_SECS`
- `MINDVAULT_EMAIL_MAX_FETCH`
- `MINDVAULT_EMAIL_MAX_ATTACHMENT_BYTES`
- `MINDVAULT_EMAIL_MARK_SEEN`
- `MINDVAULT_EMAIL_IMAP_HOST`
- `MINDVAULT_EMAIL_IMAP_PORT`
- `MINDVAULT_EMAIL_IMAP_USERNAME`
- `MINDVAULT_EMAIL_IMAP_FOLDER`
- `MINDVAULT_EMAIL_IMAP_STARTTLS`
- `MINDVAULT_EMAIL_SMTP_HOST`
- `MINDVAULT_EMAIL_SMTP_PORT`
- `MINDVAULT_EMAIL_SMTP_USERNAME`
- `MINDVAULT_EMAIL_SMTP_FROM`
- `MINDVAULT_EMAIL_SMTP_STARTTLS`

To send email via the relay UI/API, create a relay contact with
`vault_address = "mailto:recipient@example.com"` (or set the contact
`public_key` to the email address). Outbound relay messages on that direct
channel will be delivered via SMTP when `[email] enabled = true`.

Inbound relay messages can optionally create reply proposals in the Exchange
Inbox (action `relay.reply`) or auto-replies when autonomy rules allow it.

### Embedding Providers

MindVault supports:

1. `local_fastembed` (local ONNX inference; compile-time optional)
2. `openai` (remote API; requires `OPENAI_API_KEY`)

To run with local embeddings enabled, build/run with:

```bash
cargo run -p mv-cli --features local-embeddings -- server start --foreground
```

If ONNX Runtime is not discoverable by your dynamic loader, set `ORT_DYLIB_PATH` to the runtime library path before starting MindVault.

Recommended local models:

- `bge-small-en-v1.5`
- `all-minilm-l6-v2`

If `local_fastembed` is enabled with an OpenAI-style model name (e.g. `text-embedding-3-small`),
MindVault auto-falls back to `bge-small-en-v1.5`.

The default config uses `local_fastembed`. If the binary is built without `local-embeddings`, selecting `local_fastembed` falls back to no-op embeddings with a startup warning.

## Offline-First Mode

MindVault stores data locally and binds to `127.0.0.1` by default. To keep deployments fully offline:

- Set `[embedding] provider = "local_fastembed"` and build with `--features local-embeddings`.
- Avoid configuring external endpoints (OpenAI-compatible base URLs, audit webhooks).
- Keep services bound to localhost unless you explicitly need LAN access.

Encryption at rest is optional; enable it with `MINDVAULT_ENCRYPTION_ENABLED=true` or `[encryption] enabled = true`.
Audit logging is optional; enable it with `MINDVAULT_AUDIT_ENABLED=true`.

## Data Lifecycle Controls

MindVault includes data lifecycle tuning in `[lifecycle]` (see `config/default.toml`) for decay and consolidation behavior. For keychain data, lifecycle runs can be triggered via `POST /api/v1/keychain/lifecycle/run` (admin-only).

## Authentication

MindVault supports two auth modes:

1. Shared bearer token via `MINDVAULT_AUTH_TOKEN`.
2. HS256 JWT bearer tokens via `MINDVAULT_JWT_SECRET`.
3. OAuth2 client-credentials (built-in, local) backed by the Sovereign Keychain.

If both are set, either token type is accepted.

### Roles and Namespace Scope

MindVault enforces role and namespace scope across REST, gRPC, and WebSocket:

- Roles:
  - `admin`: read/write across all namespaces
  - `write`: read/write within allowed namespace scope
  - `read`: read-only within allowed namespace scope
- Shared token role/scope (optional):
  - `MINDVAULT_AUTH_ROLE` (`admin`, `write`, `read`)
  - `MINDVAULT_AUTH_NAMESPACE` (restricts non-admin access to one namespace)
- JWT claims (optional):
  - `role` (`admin`, `write`, `read`; defaults to `write` when omitted)
  - `namespace` (restricts non-admin access to one namespace)
- Permission templates + access keys:
  - `permission_templates` define tiers (`view`, `edit`, `action`, `admin`) and scopes.
  - `access_keys` map to templates and can be created/revoked via REST + admin UI.
  - Default templates: **Owner** (admin, full access) and **Assistant** (action, namespace `assistant`).
- Resource protection (optional):
  - `MINDVAULT_RATE_LIMIT_REQUESTS` (default `120`; set `0` to disable)
  - `MINDVAULT_RATE_LIMIT_WINDOW_SECS` (default `60`)
  - `MINDVAULT_NAMESPACE_NODE_QUOTA` (max nodes per namespace)

For shared token mode, requests must send:

```http
Authorization: Bearer <token>
```

Auth is enforced on REST, WebSocket, and gRPC when enabled.
When limits are exceeded, APIs return `429` (REST/WebSocket) or `RESOURCE_EXHAUSTED` (gRPC).

### OAuth2 (Client Credentials)

MindVault includes a local OAuth2 client‑credentials flow for delegated AI managers.
Client secrets are stored in the Sovereign Keychain; the vault must be unsealed to issue tokens.

- Create client (admin-only): `POST /api/v1/oauth/clients`
- Token endpoint: `POST /api/v1/oauth/token`

Tokens returned are standard Bearer tokens (internally backed by access keys with expiry).

## Runtime Diagnostics

Embedding provider diagnostics are available via:

- `GET /api/v1/diagnostics/embedding`
- `mv server status` (prints configured/effective provider, model, dimensions, fallback reason)

## AI Auto-Tagging

MindVault can self-enrich node tags during ingest/update using a hybrid strategy:

1. lexical keyword extraction from title/content
2. semantic tag transfer from similar existing nodes (FTS seeds)

This is controlled via `[ai]` config and matching `MINDVAULT_AI_*` environment variables.

Suggested rollout:

1. Enable in one namespace first (`MINDVAULT_AI_AUTO_TAGGING_ENABLED=true`).
2. Monitor generated tags through existing node APIs.
3. Tune max tag counts and min token length for your dataset.

## Auto Backlinking

MindVault can auto-create and maintain `references` relationships from content references in node body text:

- Wiki links: `[[Target Title]]`, `[[Target Title|alias]]`, `[[Target Title#Heading]]`
- Markdown links: `[label](Target Title)` and `[label](Target Title#Heading)`
- Mentions: `@TargetSlug`, `@"Target Title"`, `@'Target Title'`
- Source URL references: bare `https://...` links or markdown URL targets, matched against node `source` URLs
- Relationships are created from source node -> referenced node using `kind=references`
- Auto-generated reference edges are reconciled on update (new links added, stale links removed)
- Matching is namespace-scoped and title-based (case/whitespace normalized), with UUID targets also supported

This is controlled via `[linking]` config and matching `MINDVAULT_LINKING_*` environment variables.

## LLM Providers (Optional)

MindVault uses OpenAI-compatible chat completion APIs for assist/briefing features. Configure `[llm]` in `config.toml` (disabled by default):

```toml
[llm]
enabled = true
base_url = "http://localhost:11434/v1"
model = "llama3.2"
```

For remote providers, set `base_url` to the API endpoint and supply `MINDVAULT_LLM_API_KEY` (or `OPENAI_API_KEY`). If no provider is available, MindVault falls back to heuristic outputs.

## Watcher Agent (Proactive Monitoring)

MindVault ships with a local watcher agent that scans recent vault activity, detects intents, and generates insight proposals:

- Configure in `[watcher]` (see `config/default.toml`).
- Default behavior: enabled, runs every 300s, scans the last 24h with a 50-node cap.
- Status endpoint: `GET /api/v1/agent/watcher/status`.
- Disable with `[watcher] enabled = false` if you want a fully manual workflow.

## Autonomy Rules (Policy Engine)

Autonomy rules control when agentic actions can auto-apply vs defer:

- Manage rules: `GET/POST /api/v1/autonomy/rules`, `GET/PUT/DELETE /api/v1/autonomy/rules/{id}`.
- Evaluate decisions: `POST /api/v1/autonomy/evaluate`.
- Rule types: `global`, `domain`, `contact`, `tag` with cascading priority.
- Controls: confidence threshold, quiet hours, max actions per hour, allow/deny intent types.

Feedback is captured through the reflection endpoints:

- `POST /api/v1/agent/feedback` (record apply/dismiss)
- `GET /api/v1/agent/reflection/stats`
- `POST /api/v1/agent/reflection/calibrate`

## MCP Server (Agent Interop)

MindVault ships an MCP (Model Context Protocol) server over stdio for local agent access:

- Start: `mindvault mcp --access-key <token>`
- Env: `MINDVAULT_MCP_ACCESS_KEY` (scoped access key from permission templates)
- Read-only fallback: `--allow-unscoped` or `MINDVAULT_MCP_ALLOW_UNSCOPED=true`

MCP tools are scoped to the access key’s namespace/tags/kinds. Write operations always submit proposals for owner approval (no direct writes).

## AI Writing Assist

MindVault provides retrieval-assisted completion and linking assistance for note editing:

- REST: `POST /api/v1/assist/completion`
- Request: `{ "text": "...", "limit": 4, "namespace": "default" }`
- Response: ranked sentence suggestions plus grounding metadata (`sources`, `source_nodes`, `strategy`)
- REST: `POST /api/v1/assist/autocomplete`
- Request: `{ "text": "partial line", "limit": 5, "namespace": "default" }`
- Response: ranked inline completions tuned for prefix matching
- REST: `POST /api/v1/assist/links`
- Request: `{ "text": "topic prompt", "limit": 6, "namespace": "default", "exclude_node_id": "<optional current node UUID>" }`
- Response: ranked wiki-link targets (`title`, optional `heading`, optional `preview`, `node_id`, `namespace`, `reason`) from semantic recall
- REST: `POST /api/v1/assist/transform`
- Request: `{ "text": "...", "mode": "summarize|action_items|refine", "limit": 4, "namespace": "default" }`
- Response: transformed markdown block for quick insertion into the editor
- Web editor supports `[[...]]` inline semantic suggestions with arrow-key selection and alias-preserving insertion (`[[Title|alias]]` and `[[Title#Heading|alias]]`)
- Transform UI can either append an AI section or replace selected Markdown text
- Notes workspace rich editor also renders grounding/source chips and semantic link chips for one-click `[[...]]` insertion

## Quick Capture (Desktop + Web)

MindVault supports fast capture flows from anywhere in the app, with desktop-global hotkey support in Tauri builds:

- Shortcut: `Cmd/Ctrl + Shift + N`
- Mode shortcuts (desktop Tauri app): `Cmd/Ctrl+Shift+M` (note), `Cmd/Ctrl+Shift+L` (link), `Cmd/Ctrl+Shift+V` (voice)
- Target shortcuts (desktop Tauri app): `Cmd/Ctrl+Shift+I` (task -> Inbox), `Cmd/Ctrl+Shift+D` (note -> Daily note), `Cmd/Ctrl+Shift+P` (task -> Planned), `Cmd/Ctrl+Shift+R` (task -> Review)
- Custom preset shortcuts: `Cmd/Ctrl+Shift+1..5` for user-defined capture presets configured in Settings.
- Desktop (Tauri): works as a global system hotkey, restores/focuses the app window, and opens quick capture.
- Browser/web: works while the app tab is focused.
- Capture modes: task, note, link, and voice (when enabled).
- Capture target routing in modal: `Default`, `Inbox` (task status inbox + auto-tag), `Daily note` (auto-link to today's daily note), `Planned` (task status planned), `Review` (task status review).
- Task reminders can open prefilled quick capture on notification click (configurable in Settings: Inbox, Daily note, Planned, Review, or disabled).
- Quick Capture Presets in Settings allow reusable mode/target/prefill bundles with optional shortcut assignments.
- Command Palette (`Cmd/Ctrl+K`) now includes enabled quick-capture presets as direct actions.
- Smart Inbox includes `AI Triage` to suggest priority/status updates for inbox tasks and apply top recommendations in one click.
- Inbox AI triage behavior is configurable in Settings (`auto-run on open` and default `Apply Top N` size).
- Command Palette includes `AI: Triage Inbox` and `AI: Apply Top Inbox Triage` for keyboard-first triage workflows.
- Command Palette also supports query action `triage top N` (e.g. `triage top 5`) for one-off batch triage size.

## Web Editor

The admin web editor now uses TipTap (ProseMirror) with Markdown as the canonical storage format:

- WYSIWYG, Markdown, and split modes with bidirectional sync
- Task lists, tables, links, headings, blockquotes, code blocks
- AI autocomplete, completion suggestions, and semantic wiki-linking remain available
- AI suggestions are context-grounded with visible provenance (`source_nodes`, retrieval strategy) and source citation insertion

## Attachments & Previews

MindVault supports file attachments with inline previews and local-first extraction:

- Upload and manage attachments per node with searchable metadata.
- Inline preview downloads via `?inline=true` for images, PDFs, audio, and video.
- OCR/PDF text extraction with optional local tools (`pdftotext`, `tesseract`) and graceful fallbacks.
- Audio transcription via local Whisper with per-attachment chunk viewer and search preview.
- Embed attachments into notes (image Markdown, audio/video HTML with link fallback).
- Main app includes an attachments panel in Notes and Tasks, plus a global Media Library at `/media` for vault-wide browsing.

Optional environment variables:

- `MINDVAULT_ATTACHMENT_PDFTOTEXT_BIN` (path to `pdftotext`)
- `MINDVAULT_ATTACHMENT_TESSERACT_BIN` (path to `tesseract`)
- `MINDVAULT_WHISPER_BIN` (path to `whisper` CLI)
- `MINDVAULT_WHISPER_MODEL` (model name, e.g. `base`)
- `MINDVAULT_WHISPER_LANGUAGE` (language hint, optional)

## Canvas (JSON Canvas)

MindVault’s canvas can import/export JSON Canvas (`.canvas`) files for basic round-trip interoperability:

- Export current node cards as JSON Canvas text nodes.
- Import JSON Canvas to re-apply node positions (id match first, title fallback).

## Daily Notes

MindVault supports template-driven daily notes with idempotent ensure behavior:

- `GET /api/v1/daily-notes` (query: `namespace`, `date`, `limit`, `offset`)
- `POST /api/v1/daily-notes/ensure` (body: `namespace`, `date`)

Daily-note auto-linking is also enabled in the engine for task/event-style nodes:

- Triggered by native node kinds `task` and `event`
- Backward-compatible tag trigger still supported: `task`, `tasks`, `todo`, `to-do`, `event`, `events`, `meeting`, `reminder`
- Automatically creates (or reuses) the daily note for that node's creation date in the same namespace
- Creates a `contains` relationship from daily note -> node (deduplicated on repeated updates)
- Server startup ensures today's daily note in the configured daily-notes namespace
- Optional UTC-midnight background scheduler can ensure each new day's note without restart

Daily notes are configured via `[daily_notes]` in `config/default.toml`:

- `enabled`
- `midnight_scheduler_enabled`
- `namespace`
- `title_template`
- `content_template`
- `default_importance`

## Goals & Habits Workspace

MindVault now includes a dedicated goals and habits workspace (`/goals`) for long-term planning:

- Goals are stored as `kind=project` nodes with structured metadata (`goal_status`, `goal_progress`, `goal_target_date`, `goal_priority`).
- Habits are stored as `kind=project` nodes with structured metadata (`habit_frequency`, `habit_target_per_period`, `habit_checkins`, streak stats).
- The web app supports creating, editing, completing, and deleting goals/habits with local cache fallback.
- Habit streaks are computed client-side from normalized date check-ins and persisted back to node metadata.
- AI weekly review in the goals workspace summarizes momentum, consistency, blockers, and next actions.

## Web Clipper Workspace

MindVault includes a clipper-first reading list workspace (`/bookmarks`) for web capture:

- Capture clips with URL, title, excerpt, and tags.
- URL dedupe is applied on save (normalized URL comparison) through `POST /api/v1/clips/import`.
- When dedupe occurs, `/bookmarks` now provides conflict actions: focus the existing clip or create a linked note without duplicating the bookmark.
- Browser handoff is supported via query params (`/bookmarks?url=...&title=...&text=...&tags=...`).
- AI helpers can suggest tags and summarize clip excerpts locally through existing assist endpoints.
- Optional one-click note creation from a captured clip now happens server-side and creates a `references` relationship from note -> bookmark.

Clip import API:

- `POST /api/v1/clips/import` with JSON body:
  - `url` (required)
  - `title` (optional)
  - `excerpt` (optional; aliases `text`/`selection`)
  - `tags` (optional list)
  - `namespace` (optional)
  - `clip_source` (optional, default `manual`)
  - `dedupe` (optional, default `true`)
  - `create_note` (optional, default `false`)
- Response: `{ bookmark, created, note? }`
  - `created=true` when a new bookmark is stored
  - `note` included when `create_note=true`
- `POST /api/v1/clips/enrich` with JSON body:
  - `url` (required)
  - `html` (optional; if omitted MindVault fetches the URL and extracts metadata)
- Response: `{ normalized_url, title?, description?, site_name?, content_preview?, estimated_reading_minutes?, suggested_tags, fetched, content_type? }`
  - used by the `/bookmarks` workspace `Fetch metadata` action before saving clips

## Browser Clipper Extension (MV3)

MindVault ships a real browser extension in `extensions/mindvault-clipper`:

- Popup capture for current tab + selection.
- Popup save toggles for dedupe and optional linked-note creation.
- Context-menu capture (`Save selection to MindVault`).
- Keyboard capture command (`Ctrl+Shift+Y` / `Cmd+Shift+Y`).
- Direct ingest to local API (`/api/v1/clips/import`).
- Optional enrichment call (`/api/v1/clips/enrich`) before save, configurable in extension options.
- Fallback to `/bookmarks` handoff when API is offline.

Install in Chrome/Edge:

1. Open extensions page (`chrome://extensions` or `edge://extensions`).
2. Enable Developer Mode.
3. Click `Load unpacked`.
4. Select `extensions/mindvault-clipper`.

## Recurring Tasks

MindVault supports recurring task templates using node metadata for `kind=task`:

- `task_recurrence`: object with `frequency` (`daily|weekly|monthly`), `interval`, optional `count`, optional `until`, optional `enabled`
- `task_due_at`: RFC3339 timestamp for due-at scheduling context
- `event_start_at`: optional RFC3339 timestamp for `kind=event` calendar start time (falls back to `task_due_at` if omitted)
- `event_end_at`: optional RFC3339 timestamp for `kind=event` calendar end time

During recurrence rollforward:

- due task instances are materialized as `kind=task` nodes with `recurring_instance=true`
- each instance is linked to its template via `derived_from`
- each instance is linked to the day note through existing daily auto-link behavior

Recurrence scheduler config (`[recurrence]`):

- `enabled`
- `scheduler_interval_secs`
- `max_instances_per_template`

Due-task retrieval API:

- `GET /api/v1/tasks/due` with query params:
  - `namespace`
  - `before` (RFC3339; defaults to current time when omitted)
  - `limit`
  - `include_completed` (`true|false`)
- `POST /api/v1/tasks/prioritize` with JSON body:
  - `namespace` (optional)
  - `limit` (max 200)
  - `include_completed` (`true|false`)
  - `include_without_due` (`true|false`)
  - `persist` (`true|false`, requires write permission and stores rank metadata)
  - `now` (optional RFC3339 scoring anchor)
- `POST /api/v1/tasks/{id}/complete` to mark a task as completed
- `POST /api/v1/tasks/{id}/reopen` to mark a task as active again

### Focus Planner (Task Prioritization)

MindVault can generate a ranked focus list using deterministic heuristics (importance + due date + status + effort).

Optional task metadata hints:
- `task_priority` or `priority` (1-5; lower is higher priority)
- `task_status` or `status` (`in_progress`, `planned`, `review`, `waiting`, `blocked`, `inbox`)
- `task_estimate_minutes`, `task_estimate_min`, or `estimate_min` (numeric)

When `persist=true`, the task metadata gets an `ai_priority` object with `score`, `rank`, `reason`, `generated_at`, and `algorithm`.

Calendar retrieval API:

- `GET /api/v1/calendar/items` with query params:
  - `namespace`
  - `view` (`day|week|month`, defaults to `week`)
  - `anchor` (RFC3339 datetime anchor for view windows; defaults to now)
  - `start` + `end` (optional RFC3339 custom range, must be provided together)
  - `limit`
  - `include_completed` (`true|false`, tasks only)
- `GET /api/v1/calendar/ical` with the same query params:
  - returns `text/calendar` (`.ics`) for external calendar import/subscription workflows
- `POST /api/v1/calendar/ical/import` (`multipart/form-data`):
  - `file`: `.ics` payload (max 2 MB)
  - `namespace` (optional target namespace)
  - `overwrite_existing` (`true|false`; updates existing nodes matched by `X-MINDVAULT-NODE-ID` or `UID`)
  - `default_kind` (`task|event`, default `event`) for events without kind hints

Saved search API:

- `GET /api/v1/search/saved` with query params:
  - `namespace` (optional storage namespace filter)
  - `limit`
  - `offset`
- `POST /api/v1/search/saved` with JSON body:
  - `name` (required)
  - `query` (required)
  - `search_type` (`hybrid|vector|fulltext|graph`, optional)
  - `limit` (optional, max 200)
  - `namespace` (optional storage namespace)
  - `target_namespace` (optional query scope namespace, auth-scoped)
  - `kinds` (optional list of node kinds)
  - `tags` (optional list of tag filters)
  - `min_score` (optional 0.0-1.0 threshold)
  - `min_importance` (optional 0.0-1.0 threshold)
- `PUT /api/v1/search/saved/{id}` partially updates a saved search definition
  - nullable fields (`description`, `target_namespace`, `min_score`, `min_importance`) can be cleared by sending `null`
- `DELETE /api/v1/search/saved/{id}` deletes the saved search node
- `POST /api/v1/search/saved/{id}/run` executes the saved query and returns ranked results

Saved views API:

- `GET /api/v1/saved_views` with query params:
  - `namespace`
  - `limit`
  - `offset`
- `POST /api/v1/saved_views` with JSON body:
  - `name` (required)
  - `view_type` (`list|kanban|calendar`)
  - `filters` (object)
  - `sort` (optional `{ field, direction }`)
  - `group_by` (optional string)
  - `query` (optional string)
  - `namespace` (optional storage namespace)
- `PATCH /api/v1/saved_views/{id}` partially updates a saved view
- `DELETE /api/v1/saved_views/{id}` deletes the saved view node

Template API:

- `GET /api/v1/template-packs` lists curated built-in template packs
- `POST /api/v1/template-packs/{pack_id}/install` with JSON body:
  - `namespace` (optional target namespace; defaults to `default`)
  - `overwrite_existing` (`true|false`; updates templates matched by `template_key`)
  - `additional_tags` (optional tags applied to all installed/updated templates)
- `GET /api/v1/templates` with query params:
  - `namespace`
  - `kind` (filters by template target kind)
  - `limit`
  - `offset`
- `POST /api/v1/templates` with JSON body:
  - same core fields as node create (`kind`, `content`, `title`, `source`, `namespace`, `tags`, `importance`, `metadata`)
  - optional `template_key`
  - optional `template_variables`
- `PATCH /api/v1/templates/{id}` updates template content and metadata (template-only)
- `DELETE /api/v1/templates/{id}` deletes a template node (rejects non-template nodes)
- `POST /api/v1/templates/{id}/apply` with JSON body:
  - `target_node_id` (optional)
  - `target_kind` (optional override)
  - `overwrite` (optional, default false for merge-fill)
- `POST /api/v1/templates/{id}/duplicate` with JSON body:
  - `namespace` (optional target namespace; defaults to source template namespace)
  - `title` (optional title override; defaults to `<source title> (copy)` when source has a title)
  - `tags` (optional additional template tags)
  - `template_key` (optional key override; empty string clears copied key)
  - `metadata` (optional metadata overlay)
- `GET /api/v1/templates/{id}/versions` lists saved template versions (newest first)
- `GET /api/v1/templates/{id}/versions/{version_id}` returns version detail, field-level change summary, and line-diff summary against current template
- `POST /api/v1/templates/{id}/versions/{version_id}/restore` restores a historical version
- `POST /api/v1/templates/{id}/instantiate` with JSON body:
  - `namespace` (optional override)
  - `title` (optional override)
  - `tags` (optional additional tags)
  - `values` (placeholder substitution map for `{{variable}}` tokens)
  - `metadata` (optional metadata overlay)
- Templates are stored as `kind=template` with `template_target_kind` metadata for instantiation.
- Instantiation adds `instantiated_from_template_id` and `instantiated_at` metadata and creates a `derived_from` relationship from template to instance.
- Template updates keep rolling version history in metadata (`template_versions`) with restore support.

Node version history API (non-template nodes):

- `GET /api/v1/nodes/{id}/versions` lists saved node versions (newest first)
- `GET /api/v1/nodes/{id}/versions/{version_id}` returns version detail, field-level change summary, and line-diff summary against current node
- `POST /api/v1/nodes/{id}/versions/{version_id}/restore` restores a historical version and snapshots current state first
- Node updates now maintain rolling metadata-based history (`node_versions`) for non-template nodes.

Import/export API:

- `GET /api/v1/export` with query params:
  - `namespace` (optional scope filter)
  - `include_relationships` (`true|false`, default `true`)
- `POST /api/v1/import` with JSON body:
  - `nodes`
  - `relationships` (optional)
  - `namespace_override` (optional)
  - `overwrite_existing` (`true|false`)
  - `include_relationships` (`true|false`)

File upload API:

- `POST /api/v1/files/upload` (`multipart/form-data`)
  - `node_id`: target node UUID
  - `file`: binary file payload (max 10 MB)
  - stores file under `<data_dir>/blobs/<node_id>/...`
  - appends attachment metadata to node `metadata.attachments`
  - extracts searchable text for text-like files (`.txt`, `.md`, `.json`, `.csv`, source files, etc.)
  - attempts PDF extraction via local `pdftotext` and image OCR via local `tesseract` when available
  - persists attachment-level text chunks (`metadata.attachment_text_chunks`) for search preview and future retrieval flows
  - updates node-level `attachment_search_text` metadata used by full-text indexing
- `GET /api/v1/files/{node_id}` lists attachment metadata and download URLs for a node (including extraction status/chars and optional chunk preview metadata when available)
- `GET /api/v1/files/{node_id}/paged` returns filtered/sorted paginated attachment results with status facets (`q`, `status`, `failed_only`, `sort`, `limit`, `offset`)
- `POST /api/v1/files/{node_id}/reindex-failed` batch reindexes failed attachments and returns per-item outcomes
- `POST /api/v1/files/{node_id}/delete-filtered` performs guarded bulk delete for current filters (`dry_run` preview + `confirmed_count` execution)
- `GET /api/v1/files/{node_id}/{attachment_id}/chunks` returns paginated indexed text chunks for an attachment (`limit`, `offset`)
- `POST /api/v1/files/{node_id}/{attachment_id}/reindex` re-runs extraction/OCR against the stored attachment and refreshes attachment search metadata
- `GET /api/v1/files/{node_id}/{attachment_id}` downloads attachment content (namespace-scoped auth enforced)
- `DELETE /api/v1/files/{node_id}/{attachment_id}` removes attachment metadata and deletes the stored file when present

Optional extraction tool env overrides:

- `MINDVAULT_ATTACHMENT_PDFTOTEXT_BIN` (default: `pdftotext`)
- `MINDVAULT_ATTACHMENT_TESSERACT_BIN` (default: `tesseract`)

## Development Checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Demo Runbook

For fast live validation and demo flow, use `RUNBOOK.md` and run:

```bash
./scripts/smoke_test.sh
```

For full local pre-submit validation:

```bash
./scripts/verify_all.sh
```
