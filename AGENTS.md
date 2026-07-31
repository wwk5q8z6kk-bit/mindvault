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
- SPACE-004 is a live P0: the relay currently inserts every allowed message into the knowledge graph and violates the communication-to-knowledge promotion boundary.
- Shared Actor/Workspace/Space/Membership authorization, collaborative WorkOrder/AgentRun/Artifact state machines, durable inbox/outbox effects, and an Agent Run executor remain unfinished ahead of vertical packs.
- Broad CRDT editing, Kafka-scale decomposition, Matrix federation, and Solid integration stay parked until measured requirements justify them.
- Dual-track execution is intended: Track A finishes foundation truth, safety, and execution; Track B formalizes Domain Pack, workflow, and policy contracts without shipping vertical features yet.
- Extensions are fail-closed and must not access canonical storage directly; context grants and tool grants remain separate.
- MCP `2026-07-28` is the current protocol version to target via versioned adapters, without folding MCP transport details into MindVault’s internal object model.
