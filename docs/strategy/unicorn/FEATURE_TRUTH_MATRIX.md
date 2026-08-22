# Feature Truth Matrix

**Date:** 2026-07-31 · Baseline `3de2371`

| Capability | Evidence | User value today | Commercial relevance | Disposition |
|---|---|---|---|---|
| Local node vault / SQLite | E3 | High for personal capture | High foundation | Keep / harden |
| Search FTS + vector | E3/E4 | Medium | Medium | Harden |
| Grant admission (nodes) | E3 | Low UX visibility | Critical wedge trust | Harden / extend |
| Action envelopes | E3 | Indirect | Critical | Harden |
| Outbox local ack | E3 | None external | Foundation | Harden → IK-005 |
| Live external publisher | U/C absent | None | Required for effects wedge | Build (gated) |
| Inbox consumer | U absent | None | Required multi-node | Build (gated) |
| Trust Ledger | E4 design | None | Differentiator | Build after envelopes |
| Relay messaging | E3 (boundary fixed) | Medium | Medium | Keep; defer Slack parity |
| Message→knowledge promotion | E1/E3 | Prevents pollution | Trust | Keep |
| WorkOrder state machine | E3 partial | Medium | High wedge | Harden |
| AgentRun internal executor | E1/E3 | Medium (engine-only) | **Primary leverage** | Harden / productize |
| HTTP `.../execute` trusted-work E2E | E1 | High for wedge demo | **Wedge proof** | Keep |
| WATW `trusted_work_completed` + `mv trusted-work demo` | E1 | High for measurement | **North-star interim** | Keep; acceptance UI later |
| External agent executors | E4 gated | Low | Expansion | Defer |
| Spaces membership auth | E4 incomplete | Low | Platform later | Foundation after wedge |
| Plans schema | E5 retiring | Confusion | Debt | Retire (AGENT-002) |
| MCP server/host | E4 | Low | Ecosystem | Adapter after wedge |
| A2A | E4/U | None | Later | Defer |
| Federation | E1 kill-switch; E5 transport | None safe | Later | Keep FED-000; FED-001+ open |
| Frontend notes UX | E4/E5 | Medium | Distribution | Support wedge demo only |
| Domain Packs runtime | E4/E5 | Low | Platform | Spec Track B |
| CRDT collab editing | PARK | None | Not now | Park |
| Guarded workspace writes | E4 gated | Low | Continuity | Unlock per WS gates |

Legend: E1 runtime repro · E2 prod integration · E3 meaningful tests · E4 documented/code insufficient · E5 prototype · C claim · D disputed · U unknown
