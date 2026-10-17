# MindVault Docs

- `MINDVAULT_NEXT_MASTER_PLAN.md`: **Superseding** MindVault Next product,
  architecture, interoperability, collaboration, agent, security, domain
  expansion, competitive landscape, and execution plan. Subordinate to
  `../INTEROPERABILITY_CONSTITUTION.md` for binding interop law; execution
  status only in `../IMPLEMENTATION_BACKLOG.md`.
- `research/`: Source research for the master plan expansion
  (`adjacent-domain-research-register-2026-07-31.md`,
  `research-expansion-delta-2026-07-31.md`,
  `master-enhancement-plan-2026-07-26-baseline.md`).


- `../INTEROPERABILITY_CONSTITUTION.md`: Highest-priority architectural law.
- `adr/011-sovereign-interoperability-fabric.md`: Binding decision for the
  sovereign, interoperable context fabric.
- `architecture/interoperability-baseline.md`: Current implementation mapped to
  the target fabric, Phase 0 gate, and first kernel status.
- `architecture/interoperability-kernel-v1.md`: Executable contract for stable
  URIs, governed registries, grants, the versioned event envelope, atomic
  mutation/outbox, durable dispatch, action receipts, and replay.
- `architecture/ACTION_RECEIPT_MODEL.md`: Durable delivery leases, immutable
  publication-attempt evidence, retries, and terminal delivery semantics.
- `architecture/AUTHORITY_GRANT_MODEL.md`: Context Grants and Tool
  Grants — issue, enforce, suspend, revoke, and resume contracts.
- `architecture/CONSUMER_INBOX_MODEL.md`: Durable event admission, application
  leases and receipts, local ordering, and terminal checkpoints.
- `architecture/SOURCE_AUTHORITY_MODEL.md`: Authority, materialization, mutation,
  conflict, and deletion contract.
- `architecture/CONTEXT_NODE_MODEL.md`: Node identity, capabilities, trust, and
  lifecycle.
- `architecture/EXTENSION_SECURITY_MODEL.md`: Extension permissions, isolation,
  revocation, and UI trust.
- `architecture/WORK_ORDER_MODEL.md`: Work Orders, node contracts, typed edge
  taxonomy, run lifecycle, risk-scaled gate hierarchy, and enforced budgets.
- `architecture/EXECUTION_ISOLATION_MODEL.md`: The five isolation dimensions,
  write-lease semantics, and which boundaries are enforced versus declared.
- `architecture/AGENT_EXECUTION_GRAPH_ASSESSMENT.md`: Assessment of the external
  graph-engineering standard — what was adopted, what MindVault already
  specifies more strictly, and what was rejected with the law it would violate.
- Work Orders API: `/api/v1/work-orders` (runs, gates, artifacts, artifact
  content, export/restore; see REST OpenAPI). Operator surface: `/work-orders`.
- `architecture/PROTOCOL_BOUNDARIES.md`: MCP, A2A, HTTP, event, identity, and
  provenance boundaries.
- `architecture/FEDERATION_THREAT_MODEL.md`: Federation abuse cases, trust
  boundaries, and production release gates.
- `architecture/DATA_PORTABILITY_CONTRACT.md`: Export, import, round-trip, and
  exit guarantees.
- `onboarding.md`: First-run setup and verification steps.
- `architecture/system-overview.md`: High-level MindVault architecture diagram and data flow.
- `architecture/knowledge-workspace.md`: File-first Library storage, identity,
  migration, and staged delivery contract.
- `architecture/knowledge-workspace-document-contract.md`: Portable paths,
  byte-preserving Markdown, links, history, trash, and conflict semantics.
- `architecture/knowledge-workspace-manifest-v1.sql`: Executable design
  contract for the managed workspace manifest; not an active migration.
- `architecture/knowledge-workspace-baseline.md`: Repository-grounded audit of
  reusable foundations and blockers for a canonical document workspace.
- `architecture/knowledge-workspace-migration.md`: Reversible Notes migration,
  workspace safety boundary, and implementation unlock checklist.
- `architecture/collaborative-spaces-baseline.md`: Repository-grounded audit of
  relay, work, agent, adapter, proposal, policy, and audit foundations for
  Collaborative Spaces.
- `architecture/components.md`: Component boundaries and invariants (legacy RLM notes).
- `architecture/invariants.md`: Runtime invariants (legacy RLM notes).
- `adr/README.md`: ADR index (architecture decisions).
- `plugin-development.md`: WASM plugin authoring and lifecycle.
- `review-report.md`: Source-linked codebase re-review (Feb 2026).
- `p0-p1-execution-checklist.md`: Living checklist for P0/P1 hardening work.
- API docs: `/api/docs` (Swagger UI) and `/api/openapi.json` (OpenAPI JSON).
- Sharing API: `/api/v1/shares` and `/public/shares/{token}` (see REST OpenAPI).
- Google Calendar sync: `/api/v1/calendar/google/*` endpoints (see onboarding).
- AI sidecar proxy: `/api/v1/ai/*` endpoints (see README for config).
- Comments API: `/api/v1/nodes/{id}/comments` (resolve/delete subroutes).
- MCP marketplace: `/api/v1/mcp/connectors` registry endpoints.
- Meeting notes AI: `POST /api/v1/assist/transform` with `mode=meeting`.
- `security.md`: Security model and keychain concepts.
- `performance.md`: Performance considerations and tuning.
