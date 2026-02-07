# Research-Driven Enhancement Analysis

Last updated: 2026-02-06

## Objective

Apply security and reliability enhancements grounded in established API standards and guidance, then map findings to concrete MindVault implementation work.

## Research Inputs

1. OWASP API Security Top 10 (2023)
- Link: https://owasp.org/API-Security/editions/2023/en/0x11-t10/
- Relevance: prioritizes API resource abuse controls and authorization hardening.

2. OWASP API4:2023 - Unrestricted Resource Consumption
- Link: https://owasp.org/API-Security/editions/2023/en/0xa4-unrestricted-resource-consumption/
- Relevance: explicit guidance to enforce request limits and resource quotas.

3. RFC 6585 (HTTP Additional Status Codes)
- Link: https://www.rfc-editor.org/rfc/rfc6585
- Relevance: standard semantics for `429 Too Many Requests` and optional `Retry-After` response header.

4. gRPC Status Codes
- Link: https://grpc.io/docs/guides/status-codes/
- Relevance: recommends `RESOURCE_EXHAUSTED` for quota/rate-limit conditions.

## Findings Applied to MindVault

### 1) Request-rate controls should be first-class

Research signal:
- OWASP API4 highlights unrestricted request volume as a primary abuse path.
- RFC 6585 defines the interoperable HTTP response pattern when limits are exceeded.

Implemented:
- Per-auth-identity in-memory rate limiting across REST, gRPC, and WebSocket handshake.
- REST now returns `429` with a `Retry-After` header and JSON body metadata.
- gRPC interceptor returns `RESOURCE_EXHAUSTED` with retry context.

### 2) Storage growth controls should exist at write boundaries

Research signal:
- OWASP API4 emphasizes server-side enforcement of resource consumption constraints.

Implemented:
- Namespace node-count quotas enforced in write paths (`store_node`, namespace-changing `update_node`) for REST and gRPC.
- Quota breaches return `429` (REST) and `RESOURCE_EXHAUSTED` (gRPC).

### 3) Transport parity is required for security consistency

Research signal:
- Best-practice API security posture is weakened when controls are uneven across interfaces.

Implemented:
- Same rate-limit core reused by REST middleware, gRPC interceptor, and WebSocket upgrade path.
- WebSocket change stream now applies namespace visibility filtering by auth scope.

## Configuration Surface Added

- `MINDVAULT_RATE_LIMIT_REQUESTS` (default `120`; set `0` to disable)
- `MINDVAULT_RATE_LIMIT_WINDOW_SECS` (default `60`)
- `MINDVAULT_NAMESPACE_NODE_QUOTA` (optional max nodes per namespace)

## Residual Gaps (Next Research-Backed Targets)

1. Structured audit logs (MV-006)
- Why: necessary for incident triage and compliance traceability.

2. Observability metrics (MV-008)
- Why: rate-limit/quota tuning requires request and saturation telemetry.

3. Data-at-rest encryption (MV-009)
- Why: closes the largest remaining confidentiality gap for local sensitive knowledge stores.

## Status Impact

- MV-005 (Rate limiting + quotas): Completed.
- Security posture: improved against request flood and unbounded namespace growth attacks.

## AI Enhancement Addendum (2026-02-06)

### Implemented: AI-assisted auto-tagging on ingest/update

- Feature: `KnowledgeVaultIndexNoteEmbeddingAutoTagger` in engine ingest pipeline.
- Method:
  - lexical candidate extraction from title/content
  - similar-node tag transfer via Tantivy search seeds
  - deterministic merge with user-provided tags
- Safety:
  - feature-flagged (`auto_tagging_enabled`)
  - bounded output (`max_generated_tags`, `max_total_tags`)
  - graceful degradation (ingest never fails if enrichment fails)

### Research rationale

1. Hybrid retrieval signals remain robust for production retrieval workflows
- Tantivy documents BM25 scoring support directly in core query components (`Bm25Weight`), matching Lucene-style lexical ranking:
  - https://docs.rs/tantivy/latest/tantivy/query/index.html
  - https://docs.rs/crate/tantivy/latest/source/README.md
- BM25 + semantic augmentation is supported in current retrieval literature (e.g., BMX):
  - https://arxiv.org/abs/2408.06643

2. Resource-safe rollout is required for AI features
- OWASP API4 recommends server-side constraints on interaction frequency and resource consumption:
  - https://owasp.org/API-Security/editions/2023/en/0xa4-unrestricted-resource-consumption/

3. Rate-limit signaling should remain protocol-correct while adding AI capabilities
- HTTP `429` and `Retry-After` semantics:
  - https://www.rfc-editor.org/rfc/rfc6585.html
  - https://www.rfc-editor.org/rfc/rfc9110
- gRPC quota semantics via `RESOURCE_EXHAUSTED`:
  - https://grpc.io/docs/guides/status-codes/

## Product Research Addendum (2026-02-06)

### Additional must-have second-brain features identified

1. Daily note and journaling automation
- Research signal: Obsidian daily notes and templates are a core retention workflow.
- Sources:
  - https://help.obsidian.md/Plugins/Daily+notes
  - https://help.obsidian.md/Web+clipper/Use+templates

2. Backlinks and graph-native navigation
- Research signal: graph/backlink navigation remains a defining differentiator for PKM tools.
- Sources:
  - https://help.obsidian.md/plugins/backlinks
  - https://help.obsidian.md/plugins/graph

3. Calendar-native planning and timeline views
- Research signal: calendar tooling is now table-stakes for personal/project planning.
- Sources:
  - https://www.notion.com/help/guides/build-a-content-calendar-in-notion
  - https://www.notion.com/help/guides/show-and-hide-notion-database-properties

4. Rich block editor + Markdown interoperability
- Research signal: modern editors require bidirectional HTML/Markdown with extension ecosystems.
- Sources:
  - https://tiptap.dev/docs/editor/markdown
  - https://prosemirror.net/

5. Visual thinking layer (canvas + diagrams)
- Research signal: node canvases and embedded diagrams are common in advanced PKM workflows.
- Sources:
  - https://www.jsoncanvas.org/
  - https://mermaid.js.org/intro/

6. Large-scale semantic memory backend option
- Research signal: payload-aware vector databases simplify scalable semantic retrieval.
- Sources:
  - https://qdrant.tech/documentation/
  - https://docs.rs/qdrant-client/latest/qdrant_client/

7. Recurring task and reminder ergonomics
- Research signal: natural recurring date workflows drive daily usability.
- Sources:
  - https://www.todoist.com/help/articles/introduction-to-recurring-due-dates-YUYVJJAV

8. Desktop notifications and quick capture
- Research signal: low-friction capture and reminders rely on native OS integrations.
- Sources:
  - https://v2.tauri.app/plugin/notification/

### Applied in this cycle

- Added rich writing workspace (WYSIWYG + Markdown + split view) to web admin.
- Added retrieval-assisted writing completion endpoint:
  - `POST /api/v1/assist/completion`
- Added editor AI suggestion UI integration consuming completion API.

### Strategic recommendation

- Execute Phase 4 roadmap in this order:
  1. Daily notes + recurring reminders
  2. Calendar and project hierarchy
  3. Attachment/OCR pipeline
  4. Canvas/diagram layer
  5. Scalable semantic backend option
