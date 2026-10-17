## Learned User Preferences

- Prefer full in-repo master product specs over condensed summaries that only link out to existing ADRs.
- Treat discussed verticals (notes/Obsidian, meetings, interview tools, Slack-like Spaces, coding agents, scheduled reports) as conformance floors and proof slices—not the product ceiling; expand via adjacent-domain and competitor research.
- Use evidence-first planning: do not equate code, schemas, routes, or documentation presence with verified implementation.
- Do not copy stealth or live interview-copilot behaviors; treat that category as competitive/ethical risk context only.
- Finish P0 foundation work (truth/docs, identity and grants, reliable effects, agent executor, communication-to-knowledge promotion) before broad domain vertical implementation.
- When implementing from an attached Cursor plan, follow the plan as specified and do not edit the plan file itself.
- Keep MindVault work scoped to this repository/product; do not conflate with other products or workspaces.
- Prefer shipping the ratified Trusted Agent Work wedge and design-partner proof over broad competitor feature-chasing.

## Learned Workspace Facts

- Product direction is a sovereign interoperable context fabric: Private Personal Vaults plus Governed Shared Spaces, superseding the older single-owner no-shared-state assumption.
- `docs/MINDVAULT_NEXT_MASTER_PLAN.md` is the superseding product and architecture plan; `INTEROPERABILITY_CONSTITUTION.md` is binding interop law; `IMPLEMENTATION_BACKLOG.md` is the sole execution-status authority.
- Domain Packs carry domain-specific schemas, workflows, policies, and connectors; the kernel stays a domain-agnostic capability algebra.
- Primary wedge is Trusted Agent Work for AI-native teams (`docs/strategy/unicorn/PRIMARY_WEDGE_DECISION.md`); demo `mv trusted-work demo`; interim north-star `trusted_work_completed` (WATW); explicitly not MCP-gateway, Slack-clone, federation-first, or Domain Pack marketplace-first.
- SPACE-004 is verified (2026-07-31): relay messages stay out of the knowledge graph unless explicitly promoted; see `cargo test -p mv-engine -- promotion_boundary`.
- Internal Agent Run executor is verified (AGENT-001, 2026-07-31) for `ExecutorKind::Engine` + `RiskTier::Low` via `execute_run` and HTTP `POST .../runs/:id/execute`.
- AGENT-002/003 and WS-001/003/014 are verified (2026-07-31, `72e48b4`): plans schema retired, conformance counts enforceable, workspace event journal + conflicts list route + fixture consumers landed.
- SPACE-001 is verified (2026-07-31): CollabWorkspace/Space/Membership schema + `space_authorization_matrix` default-deny and cross-Space isolation (`cargo test -- space_authorization_matrix`).
- IK-005 is verified (`HttpOutboxPublisher`; `MINDVAULT_OUTBOX_HTTP_URL` + bearer token). SPACE-003 is verified (2026-08-22, `ec07c00`): durable adapter bindings, unique provider delivery IDs, poll cursor held on partial failure (`adapter_reliable_effects` 3/3). IK-007 is verified (2026-08-22, `182517a`): `redrive_consumer_dead_letter` admits a caused event without rewriting receipt/checkpoint. IK-016 is verified (2026-08-22, `8fe9580`): `reconcile_projections_once` recovers FTS/vector/graph after crash between commit and projection. SPACE-002 is in_progress (2026-08-22): SM/retry, self-approve, own-G5, broaden, and Space-scoped admit membership are tested; unified action envelope (IK-020) remains. Still unfinished ahead of vertical packs: SPACE-006 DocumentWorkspace rename, Trust Ledger (IK-020), and external executor isolation (AGENT-004).
- Federation HTTP routes are fail-closed by default (FED-000 / `MINDVAULT_FEDERATION_ENABLED`); FED-001..010 threat gates remain open — do not treat experimental transport as production-safe.
- Dual-track execution is intended: Track A finishes foundation truth, safety, and execution; Track B formalizes Domain Pack, workflow, and policy contracts without shipping vertical features yet. Broad CRDT editing, Kafka-scale decomposition, Matrix federation, and Solid integration stay parked until measured requirements justify them.
- Extensions are fail-closed and must not access canonical storage directly; context grants and tool grants remain separate. MCP `2026-07-28` is the current protocol version to target via versioned adapters, without folding MCP transport details into MindVault’s internal object model.
