## Learned User Preferences

- Prefer full in-repo master product specs over condensed summaries that only link out to existing ADRs.
- Treat discussed verticals (notes/Obsidian, meetings, interview tools, Slack-like Spaces, coding agents, scheduled reports) as conformance floors and proof slices—not the product ceiling; expand via adjacent-domain and competitor research.
- Use evidence-first planning: do not equate code, schemas, routes, or documentation presence with verified implementation.
- Do not copy stealth or live interview-copilot behaviors; treat that category as competitive/ethical risk context only.
- Finish P0 foundation work (truth/docs, identity and grants, reliable effects, agent executor, communication-to-knowledge promotion) before broad domain vertical implementation.
- When implementing from an attached Cursor plan, follow the plan as specified and do not edit the plan file itself.

## Learned Workspace Facts

- Product direction is a sovereign interoperable context fabric: Private Personal Vaults plus Governed Shared Spaces, superseding the older single-owner no-shared-state assumption.
- `docs/MINDVAULT_NEXT_MASTER_PLAN.md` is the superseding product and architecture plan; `INTEROPERABILITY_CONSTITUTION.md` is binding interop law; `IMPLEMENTATION_BACKLOG.md` is the sole execution-status authority.
- Domain Packs carry domain-specific schemas, workflows, policies, and connectors; the kernel stays a domain-agnostic capability algebra.
- SPACE-004 is verified (2026-07-31): relay messages stay out of the knowledge graph unless explicitly promoted; see `cargo test -p mv-engine -- promotion_boundary`.
- Internal Agent Run executor is verified (AGENT-001, 2026-07-31) for `ExecutorKind::Engine` + `RiskTier::Low` via `execute_run` and HTTP `POST .../runs/:id/execute`.
- AGENT-002/003 and WS-001/003/014 are verified (2026-07-31, `72e48b4`): plans schema retired, conformance counts enforceable, workspace event journal + conflicts list route + fixture consumers landed.
- SPACE-001 is verified (2026-07-31): CollabWorkspace/Space/Membership schema + `space_authorization_matrix` default-deny and cross-Space isolation (`cargo test -- space_authorization_matrix`).
- SPACE-002 is verified (2026-08-22): `work_order_agent_run` covers SM/retry rules and self-approve / own-G5 / broaden refusals. Still unfinished ahead of vertical packs: SPACE-006 DocumentWorkspace rename, IK-006 inbox consumer, SPACE-003 reliable adapter effects, Trust Ledger (IK-020), and external executor isolation (AGENT-004).
- Federation HTTP routes are fail-closed by default (FED-000 / `MINDVAULT_FEDERATION_ENABLED`); FED-001..010 threat gates remain open — do not treat experimental transport as production-safe.
- Broad CRDT editing, Kafka-scale decomposition, Matrix federation, and Solid integration stay parked until measured requirements justify them.
- Dual-track execution is intended: Track A finishes foundation truth, safety, and execution; Track B formalizes Domain Pack, workflow, and policy contracts without shipping vertical features yet.
- Extensions are fail-closed and must not access canonical storage directly; context grants and tool grants remain separate.
- MCP `2026-07-28` is the current protocol version to target via versioned adapters, without folding MCP transport details into MindVault’s internal object model.
- Wedge demo CLI: `mv trusted-work demo`; interim north-star counter `trusted_work_completed` (WATW proxy).
