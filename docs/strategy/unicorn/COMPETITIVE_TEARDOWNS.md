# Competitive Teardowns (focused)

**Date:** 2026-07-31 · Depth: decision-grade, not encyclopedic

## Obsidian
- **JTBD:** Local Markdown knowledge base.
- **Strength:** File ownership, plugins, offline, graph.
- **Weakness:** Weak multiplayer authority; AI bolted on; no governed agent runs.
- **Exploit:** Become the intelligence/authority substrate while remaining Markdown-friendly.
- **Copy risk:** High for notes UX; low for grants/runs/receipts.

## Notion
- **JTBD:** Collaborative docs/databases.
- **Strength:** Templates, team UX, distribution.
- **Weakness:** Cloud-centric; provenance/agent authority shallow relative to MindVault constitution.
- **Exploit:** Local-first + evidence + agent work for teams that cannot put crown jewels in Notion AI.

## Slack / Teams
- **JTBD:** Real-time communication.
- **Strength:** Distribution, habit, integrations.
- **Weakness:** Chat≠knowledge; poor durable work/agent accountability.
- **Exploit:** Promotion boundary + work orders; do not clone chat.

## Linear
- **JTBD:** Issue execution for eng teams.
- **Strength:** Speed, taste, workflow gravity.
- **Weakness:** Not a context fabric; agents secondary.
- **Exploit:** Evidence-linked agent runs that issue trackers lack.

## LiteLLM / Portkey / Obot-class control planes
- **JTBD:** Govern model/tool calls inline.
- **Strength:** Fast enterprise buy for AI security/FinOps.
- **Weakness:** Thin on personal vault, temporal knowledge, WorkOrder semantics, human authoring.
- **Exploit:** Be the system of record for context and work; integrate gateways rather than replace every proxy feature.
- **Incumbent response:** Add audit dashboards; hard to absorb local-first ownership + evidence graph quickly.

## Glean-class enterprise search
- **JTBD:** Find company knowledge.
- **Strength:** Connectors, ranking, enterprise sales.
- **Weakness:** Search ≠ authority to act; limited agent work machine.
- **Exploit:** Compile context *and* execute governed work with receipts.

## Failed / cautionary patterns (I + historical)
- AI memory apps that cannot prove source authority → novelty churn.
- PKM tools that chase every feature → no wedge, no revenue.
- Agent wrappers without durable identity/grants → security rejection.

---

## 2026-08 Feature-Level Refresh

**Date:** 2026-08-21 · Depth: shipped features and UX patterns, with dates and sources.
Positioning verdicts above stand; this section is about what competitors actually
shipped in the last ~6 months and what is worth stealing at the feature level.

### Notion — Custom Agents + External Agents
- **2026-02-24 — Notion 3.3 "Custom Agents":** autonomous agents on schedules/triggers (Slack, mail, DB changes), plain-language builder, model picker (Claude/GPT/Gemini/Grok), and — critically — **per-agent granular permissions that do not inherit the triggering user's permissions**, with every run logged and changes reversible. Business/Enterprise only. <https://www.notion.com/releases/2026-02-24>
- **2026-05-04 — usage-based Notion Credits** ($10/1,000 credits) with per-agent credit limits, a workspace-wide agent dashboard, pause controls, and proactive spend alerts. >1M agents created during the two-month beta. <https://www.notion.com/blog/what-we-learned-during-the-custom-agents-beta>
- **2026-07-01 — Notion 3.6 "External Agents":** Claude and Cursor become first-class assignable teammates on shared boards — @-mention them, drag a card into a `Ready for Agent` column, watch the run inline with an audit trail of every page touched. Plus an External Agent API and "Workers" hosted runtime. <https://www.notion.com/releases/2026-07-01>
- **Magic moment:** dragging a card into `Ready for Agent` and watching a teammate-with-its-own-permissions execute, logged.
- **Stealable:** the agent-as-teammate profile page (own permissions, own activity feed); trigger-by-status-column; run log presentation. **Not stealable:** credits meter, cloud-only custody.

### Obsidian + AI plugin ecosystem
- **Copilot for Obsidian v4.0.0 (2026):** full rewrite — Claude Code, Codex, and opencode run *natively inside the vault*; browser-style tabs for parallel agent sessions; **Project Mode with per-project `AGENTS.md`**; multi-agent collaboration (`@claude @codex` in one prompt, Plus tier); cross-agent skills (web search, PDF, YouTube, X capture). <https://github.com/logancyang/obsidian-copilot/releases/tag/4.0.0>
- **Smart Connections v4+ (2026):** core plugin simplified to zero-setup local semantic search; power features moved to Pro (Smart Chat Pro, Connections Pro, Obsidian Bases similarity columns). <https://www.shadow.do/blog/best-ai-plugins-for-obsidian-2026>
- **Magic moment:** frontier coding agents writing back into your Markdown with wikilinks and canvases — the vault as agent workspace.
- **Stealable:** per-project agent instruction files; configure-a-skill-once-enable-for-every-agent. **Gap we exploit:** zero governance — BYOK cloud calls, no grants, no receipts, no run evidence.

### ChatGPT — memory, Projects, Scheduled Tasks
- **2026 — auto-updating memory** replaces the stale saved-memories model (memory summary review UI; 2x memory capacity for Plus/Pro). <https://help.openai.com/en/articles/6825453-chatgpt-release-note>
- **Project-only memory:** per-project isolation, switchable after creation, *mandatory* for shared projects — project content never leaks into outside memory. <https://help.openai.com/en/articles/10169521>
- **2026-06-17 — Pulse retired.** The inferred-proactive morning brief (~9 months old) was killed and folded into **explicit, user-configured Scheduled Tasks** (daily briefs, web monitoring, condition-based notify; 5–15 active-task caps by tier). <https://prowlo.com/blog/chatgpt-pulse-shut-down>, <https://www.narracomm.com/chatgpt-adds-scheduled-tasks-feature-sunsets-pulse/>
- **Magic moment:** "it just remembers." **But the real lesson is inverted:** inferred proactive feeds failed on relevance; explicit user-defined tasks won. This validates explicit WorkOrders over ambient inference.
- **Stealable:** project-scoped memory toggle UX; user-visible memory review/forget. **Not stealable:** opaque auto-memory writes becoming canon.

### Claude — memory, Projects, Cowork
- **2026-03 — memory for all users incl. free tier**, project-scoped memory spaces, incognito chats, view/edit/delete in Settings. <https://claude.com/blog/memory>
- **2026 — memory v2:** individual categorized entries Claude reads/updates during chats (replacing the daily summary); **monthly recap under Settings → Reflect** (beta); memory import/export. <https://support.claude.com/en/articles/12138966-release-notes>
- **Cowork (2026, research preview → paid plans):** Claude Code's agentic architecture repackaged for non-coding knowledge work — sandboxed local folder access, cloud sessions steerable from phone, scheduled tasks, projects with their own files/instructions/memory. Launch was Max-tier macOS-only [exact date unverified]; now web/mobile beta. <https://venturebeat.com/technology/anthropic-launches-cowork-a-claude-desktop-agent-that-works-in-your-files-no>, <https://claude.com/product/cowork/>
- **Magic moment:** hand Claude a goal, steer from your phone, come back to finished work *in your files*.
- **Stealable:** categorized memory entries with user review; incognito = "off-the-record" mode; folder-scoped agent sandbox (≈ our file-scope handles, EXT-005).

### Glean — agent platform
- **2026 (incl. June Product Drop):** Agent Builder (natural language + manual), Agent Library, **Task Tool for dynamic sub-agent delegation**, A2A protocol for calling external agents, **governed service credentials so agents act under their own identity**, Plan & Execute with adaptive planning, **named immutable checkpoints for agent drafts**, debug/trace views, MCP server support. <https://docs.glean.com/agents/how-agents-work>, <https://www.glean.com/product-drop/june-2026>
- **Magic moment:** checkpoint a draft agent, trace every step, certify it for the org.
- **Stealable:** immutable run checkpoints + step-trace view; agent certification workflow. **Behind us:** no local-first, no user-owned evidence, search-DNA not work-DNA.

### Linear — agents as workspace members
- **Linear for Agents (2025→2026):** agents are full workspace members; **"Delegate issues, but not accountability" — the human remains primary assignee, the agent is added as contributor**; triage rules auto-delegate; My Issues monitors agent work and flags "needs your input." <https://linear.app/agents>
- **Cursor background agents (2025-08-21, matured since):** assign an issue → agent creates branch + draft PR; comment to steer; seamless handoff to IDE. <https://linear.app/changelog/2025-08-21-cursor-agent>
- **Magic moment:** delegate a well-formed issue, get a PR — and accountability never transfers.
- **Stealable:** the assignee/contributor split, verbatim; the agent-needs-input inbox.

### Granola — biggest threat this cycle
- **2026-03-25 — $125M Series C at $1.5B valuation** (Index, Kleiner Perkins; $192M total), with an explicit repositioning as **"the enterprise context layer for AI agents."** <https://worktechjournal.com/granola-series-c-spaces-api-mcp-team-notes/>, <https://www.tbpndigest.com/story/2026-03-30/granola-raises-125m-series-c-at-15b-valuation-to-become-the-enterprise-context-layer-for-ai-agents>
- **Shipped:** Spaces (team workspaces, folders, granular access controls), Personal + Enterprise APIs, expanded MCP (folder + shared team content), admin dashboards, SSO/SCIM, and **retention controls spanning 24-hour auto-delete to permanent** (a litigation-risk answer). Agent features that act on notes announced "within a year."
- **Magic moment:** botless, zero-friction capture → queryable team memory, spread by word of mouth.
- **Why this is the threat:** they own the *context inlet* our SLICE-002 proof chain needs, they have distribution we lack, and "context layer for agents" is one receipts-and-grants step away from our wedge. They are cloud-only with no authority model — today.

### Limitless / Rewind — cautionary tale
- **Acquired by Meta; Pendant sales ended 2025-12-05; Rewind capture disabled 2025-12-19;** existing customers supported "throughout 2026" only. <https://www.limitless.ai/>, <https://github.com/api-evangelist/limitless>
- **Lessons:** ambient-capture hardware standalone is a dead end; Consent Mode was the real differentiator; cloud custody made the product acquirable and killable. Local-first capture (screenpipe) inherited the category.

### screenpipe — local-first ambient capture
- **2026:** source-available 24/7 screen+audio capture (Mac/Win/Linux), local Whisper transcription, full REST API + MCP server, "pipes" agent plugin system, 16k+ GitHub stars, $400 lifetime / free self-host. <https://github.com/screenpipe/screenpipe>
- **Stealable:** don't build capture — ingest screenpipe/Granola-class sources through the governed connector contract (SRC-001, SRC-007) so ambient context lands as *evidence with provenance*, not surveillance.

### Mem.ai
- **2026:** Google/Outlook **calendar integration with auto-attached source cards** on meeting notes; Claude connector (MCP); model selector (Claude/Gemini/GPT); betas: **Meeting Briefings** (pre-meeting context pulled from your notes) and **Heads Up Live** (real-time relevant-note surfacing against the live transcript). <https://get.mem.ai/blog/product-update-roundup-calendar-integration-claude-connector-referral-program-plus-experimental-features-for-pro-users>
- **Magic moment:** walk into a meeting with the briefing already assembled from your own notes.
- **Stealable:** calendar-triggered pre-meeting briefing into daily notes; Heads-Up-style proactive recall. Weak moat: cloud-only, no Android, thin integrations.

### Capacities
- **2026:** API 2.0 (full programmatic CRUD); **AI Chat Connectors 2.0 (MCP)** — external AI (ChatGPT/Claude/Cursor) can create pages/tasks/custom objects, update properties, append content; concurrent AI chats. <https://capacities.io/whats-new/release-66>
- **Stealable:** MCP *write* verbs — but our version must be grant-scoped and receipted (PROTO-001, PROTO-002), which theirs is not.

### Tana
- **2026 pivot: the meeting layer owns the roadmap** — botless capture of Zoom/Teams/Meet via system audio, pre-meeting hub card, Today page as home surface, live AI digest; **AI agents turn conversations into filed work (Linear/Jira/GitHub/Slack/HubSpot) that lands as Proposals you review before anything changes**; MCP server; re-run extraction de-dupes instead of spawning copies. <https://tana.inc/blog/best-ai-note-taking-apps-2026>, <https://sparkpulse.io/compare/capacities-vs-tana>
- **Magic moment:** conversation → reviewed proposal → filed ticket; nothing changes without approval.
- **Stealable:** the **Proposals inbox** — the human gate as a first-class review queue. This is the closest any PKM has come to our accept-step; they still lack grants, receipts, and local custody.

### Reflect
- **2026:** E2E-encrypted daily notes; baked-in AI ("ask my notes," transcription, outlines); ~140ms capture latency; public API in limited beta. Deliberately single-player: AI as thought partner, no team model. <https://pickuma.com/for-pm/reflect-vs-tana-vs-capacities-engineers-2026/>
- **Stealable:** capture latency treated as a headline feature. Otherwise confirms single-player PKM is not our wedge.

### Heptabase
- **2026-05 — Agent Mode:** AI edits cards, creates whiteboard connections, manages layouts and tag properties; **Heptabase CLI lets Claude Code/Codex/Cursor read and edit the knowledge base from the terminal**, with official CLI skills. <https://wiki.heptabase.com/newsletters/2026-05-29>
- **2026-08-04 — v1.102.0:** **active view as AI context** (selected whiteboard objects, current PDF page); **failed agent runs preserve partial output and offer retry**. <https://whataidoineed.com/tool/heptabase>
- **Stealable:** partial-output preservation + retry on failed runs; CLI-as-agent-surface with shipped skills; active-selection-as-context.

### MyMind
- **2026:** first API for private integrations; AI tagging/OCR search; Substack-notes capture. Strictly private, no sharing — the anti-team extreme. <https://mymind.com/new-upgrades-to-your-mind>
- Nothing to steal beyond clipper polish; confirms privacy-only positioning has no team wedge.

### NotebookLM → Gemini Notebook
- **2026-06-08 — Gemini 3.5 + Antigravity rebuild:** every notebook gets a **secure cloud computer** (code execution, 100+ skills), **agentic source discovery** (chat builds your source library via Google Search), **visible reasoning steps**, 12+ export formats (PPTX/XLSX/PDF). Jan 2026: saved chat history + 1M-token context. <https://blog.google/innovation-and-ai/products/notebooklm/better-research-notebooklm/>, <https://9to5google.com/2026/06/08/notebooklm-gemini-3-5-antigravity/>
- **Stealable:** visible reasoning as consumer-grade "show your work." Our receipt graph is strictly stronger — match their presentation quality.

### Agent audit-trail ecosystem — the moat is being validated
- **Regulatory tailwind:** EU AI Act high-risk provisions reach **full enforcement August 2026** (Art. 12: ≥6-month logs, traceability, named-human attribution); COSO genAI internal-controls guidance (Feb 2026); SEC SOX AI enforcement group (Mar 2026); OCC/Fed/FDIC SR 26-2 (Apr 2026). <https://www.kognitos.com/blog/ai-audit-trail-requirements-2026-checklist/>
- **Open-source provable-memory startups emerged:** Hakuya (per-tenant SHA-256 hash-chained belief logs, GDPR cryptographic erasure) and agent-receipts (Ed25519-signed JSON receipts, offline verification, no cloud). <https://hakuya.ai/>, <https://github.com/webaesbyamin/agent-receipts>
- **Read:** the market is converging on our thesis. None of these productize receipts into a *work system with human gates* — but the window is open, not permanent.

### Prioritized steal-list

| # | Pattern (source) | MindVault surface | Backlog ID | Effort | Wedge-fit | Why |
|---|---|---|---|---|---|---|
| 1 | "Delegate, not accountability": human stays primary assignee, agent is contributor (Linear) | agent runs | AGENT-001, SPACE-002 | S | high | Proven language + model for WorkOrder requestor vs. agent executor; zero new architecture |
| 2 | Proposals inbox: agent-produced changes queue for human review before anything lands (Tana) | agent runs / receipts | SPACE-002, SPACE-005 | M | high | The human gate as a first-class review surface — this *is* the wedge's accept step |
| 3 | Per-agent permission profile, not inherited from triggerer + admin dashboard (Notion) | agent runs | IK-001, IK-011, IK-012 | M | high | Grants exist; steal the teammate-profile presentation and workspace-wide agent overview |
| 4 | Immutable run checkpoints + step-trace view (Glean) | agent runs / receipts | SPACE-005, AGENT-006 | M | high | Checkpoint/restore atop the receipt graph; AGENT-006's missing reader is literally the trace view |
| 5 | Failed runs preserve partial output + one-click retry (Heptabase) | agent runs | AGENT-001, SPACE-002 | S | high | Cheap, obvious, and nobody with receipts does it yet |
| 6 | Botless meeting capture → evidence pipeline (Granola, Tana) | capture | SRC-001, SLICE-002 | L | high | SLICE-002 *is* "external-meeting-to-team-report proof chain"; capture is the wedge's missing inlet |
| 7 | Calendar-triggered pre-meeting briefing from vault (Mem) | recall / daily notes | none | M | med-high | Proactive recall is a magic-moment amplifier; daily notes already exist as the landing surface |
| 8 | Project-scoped context isolation toggle (ChatGPT/Claude project memory) | recall | SPACE-001, IK-001 | M | high | "Project-only memory" = grant-scoped context; users now expect this UX by default |
| 9 | MCP write verbs for external agents, grant-scoped + receipted (Capacities, Tana) | MCP | PROTO-001, PROTO-002 | M | med-high | Everyone ships read; write-with-grants-and-receipts is the differentiated version |
| 10 | mv-cli as coding-agent surface + shipped agent skills (Heptabase CLI, Copilot v4) | CLI | none | S | high | Claude Code/Codex are the wedge user's daily driver; make mv-cli + skills the governed path in |
| 11 | Per-run model picker + effort/budget controls (Notion, Glean model hub) | agent runs | SPACE-005 | S | med-high | Budgets already mandated; local models are a cost story cloud competitors can't match |
| 12 | User-visible memory review: categorized entries, edit/forget, monthly recap (Claude) | recall | none | S | med | Auditable memory builds trust and matches the evidence ethos |
| 13 | Retention spectrum control incl. 24h auto-delete (Granola) | capture / receipts | WS-008, PARK-003 | S | med | The litigation-risk answer design partners will ask about |
| 14 | Active selection (nodes, PDF pages) as run/chat context (Heptabase v1.102) | recall / search | none | S | med | Cheap UX win: "use what I'm looking at" as explicit run context |
| 15 | Visible-reasoning "show your work" presentation (NotebookLM) | receipts / search | IK-021 | S | med | Our receipts are stronger; their presentation is better — close the gap |

### Do-NOT-steal list

- **Metered agent credits as the value metric (Notion Credits)** — selling model calls is the anti-value; we sell accepted, evidence-linked work. Cloud-only custody comes free with it — also no.
- **Inferred proactive feeds (ChatGPT Pulse)** — OpenAI killed it in June 2026 on relevance failure; explicit user-configured tasks won. No ambient "we noticed you'd like this" surfaces without an explicit, user-defined task behind them.
- **Ungoverned multi-agent vault writes (Copilot v4, Heptabase Agent Mode)** — capability without grants, receipts, or review. Exceed it with governance; never copy the write path.
- **Always-on capture without consent (pre-Consent-Mode Limitless; the Recall pattern)** — surveillance posture violates the constitution; any capture must be opt-in, consent-signaled, and locally sealed.
- **Cloud-only auto-memory profiles (ChatGPT/Claude memory defaults)** — model-inferred facts silently becoming canon with no provenance. MindVault's line: no canon without evidence.
- **Capture hardware (Pendant)** — Meta acqui-killed Limitless inside a year; hardware is a dependency, not a moat. Ingest, don't build.
- **Engagement-bait (mymind Serendipity feeds, streaks, social graphs)** — explicitly anti-value; optimizes time-in-app, not accepted work.
- **Chat as the primary interface (everyone above)** — chat is table stakes and commodity; the work/evidence surface is the differentiator. Chat may exist; it must never be where work becomes canon.

### Brutal honesty: behind vs. ahead

**Where MindVault is behind:**
- **Execution exists nowhere.** AGENT-001: `start_run` executes nothing — while Notion (>1M agents in beta), Glean, and Linear ship autonomous agents to millions of users. This is the gap; everything else is noise.
- **No capture inlet.** Granola, Tana, and Mem all ship botless meeting capture; MindVault has none, and SLICE-002 (the wedge's own proof chain) is unverified without it.
- **No review UX.** No proposal inbox, no run trace view, no checkpoints — AGENT-006: `work_order_history` has a writer and no reader.
- **No proactive recall.** Nothing like Mem's pre-meeting briefings or Heads Up Live.
- **Distribution.** Granola has word-of-mouth compounding and $192M; Notion has the installed base; MindVault has zero design partners named in this repo.

**Where competitors are behind MindVault:**
- **Cryptographic receipts / evidence graph:** nobody in PKM or productivity has one. The emergent audit-trail startups (Hakuya, agent-receipts) validate the primitive but are infrastructure, not a work system.
- **Local-first + governance combined:** every governed-agent player is cloud-only; every local player is ungoverned. The intersection is still empty.
- **Regulatory timing:** EU AI Act enforcement (this month) turns receipts from a nice-to-have into a compliance purchase trigger — and no competitor can retrofit local-first custody quickly.
- **Explicit-work primitive:** Pulse's death and the rise of scheduled/explicit tasks across ChatGPT, Notion, and Linear confirm the industry is converging on user-defined, auditable work units — the WorkOrder shape — without the authority model underneath.
