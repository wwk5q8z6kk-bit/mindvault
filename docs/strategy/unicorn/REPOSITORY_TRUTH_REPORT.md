# Repository Truth Report

**Date:** 2026-07-31  
**Baseline:** `3de2371` on branch `strategy/unicorn-2026-07-31`  
**Method:** Live backlog + source inspection + reproduced tests (`cargo test -p mv-engine -- promotion_boundary agent_run_executor`)

## Executive verdict

MindVault today is a **local-first knowledge and interoperability kernel with a newly verified internal Agent Run executor**, not yet a collaborative Spaces product, not yet a live external-effects platform, and not yet a production federation system. Documentation and session memory lag recent P0 verifications.

## Executable products

| Product | Path | Status | Evidence |
|---|---|---|---|
| REST/gRPC/UDS server | `crates/mv-server` | Production-path core | E3/E4 — extensive tests; ports 9470 / 50051 / UDS |
| Engine | `crates/mv-engine` | Production-path core | E3 |
| Storage (SQLite + Lance) | `crates/mv-storage` | Canonical store | E3 |
| CLI | `crates/mv-cli` | Present | E4 |
| MCP crate | `crates/mv-mcp` | Partial adapter | E4 — grant-aware server profile still gated (PROTO/EXT backlog) |
| Frontend (SvelteKit/Tauri) | `frontend/` | Partial UX | E4/E5 |
| Plugins | `crates/mv-plugin` | Sandbox intended | E4 — gateway-only enforcement incomplete |
| Web admin | `web/` | Secondary | E4 |

## P0 claim verification (plan claims → live truth)

| Claim | Verdict | Status | Evidence |
|---|---|---|---|
| Shared Actor/Workspace/Space/Membership auth incomplete | **Confirmed incomplete** | SPACE-001 `not_started` | E4 schema/docs; no verified matrix |
| WorkOrder/AgentRun/Artifact machines incomplete | **Partially false** | SPACE-002 still open; AGENT-001 **verified** | E1 executor test pass 2026-07-31; full collaborative SM still incomplete |
| Reliable external effects incomplete | **Confirmed** | SPACE-003, IK-005, IK-006 `not_started` | E4 local ack publisher only |
| Relay auto-promotes messages to knowledge | **Falsified (fixed)** | SPACE-004 **verified** | E1: 3/3 `promotion_boundary` tests ok |
| Agent graph has no executor | **Falsified (fixed)** | AGENT-001 **verified** | E1: `agent_run_executor_drives_run_to_terminal_with_artifact` ok |
| Dead schemas / vacuous tests | **Confirmed risk** | AGENT-002/003, WS-001..005, WS-014 | E4/D — plans retirement WIP; workspace tables without R/W |
| Guarded writes gated | **Confirmed gated** | WS-006..WS-013 | E4 |
| Broad CRDT/Kafka/Matrix/Solid deferred | **Confirmed** | PARK-* | E4 |
| Federation production gated (FED-000) | **Was false; fixed this session** | FED-000 now fail-closed 501 unless `MINDVAULT_FEDERATION_ENABLED` | E1 `federation_disabled_by_default` |

## What actually works end-to-end (highest confidence)

1. Local node CRUD with grant admission on selected mutating node routes (IK-001/002 verified).
2. Authority grant issue/suspend/revoke admin APIs (IK-001b).
3. Identity registry vs local-system fallback (IK-003).
4. Outbox claim/complete with **LocalAckPublisher** (IK-004); live HTTP publisher absent (IK-005).
5. Relay store-as-communication + explicit promotion/retraction (SPACE-004).
6. Work Order admit → lease → **internal Engine execute_run** → artifact → low-risk gates → completed (AGENT-001). Scope: `ExecutorKind::Engine`, `RiskTier::Low` only.

## Architectural shape (truth)

- **Canonical:** SQLite nodes, messages, grants, work orders, outbox/inbox tables.
- **Derived:** FTS, vector (Lance), graph projections — intended rebuildable.
- **Authority plane:** Action envelopes + grants on some paths; Trust Ledger (IK-020) not verified.
- **Collaboration plane:** Spaces incomplete (SPACE-001/007/008).
- **Agent plane:** Internal executor exists; external executors / isolation PARTIAL (AGENT-004).
- **Interop plane:** MCP/A2A adapters incomplete vs constitution.

## False confidence sources

1. Stale `AGENTS.md` still claiming SPACE-004 live and missing executor.
2. `DEVELOPMENT_PLAN.md` historical completeness language (partially corrected via DOC-*).
3. Workspace migration tables without readers/writers (WS-001..005).
4. Vacuous conformance declaration (AGENT-003).
5. Plans endpoints vs Work Orders dual model during AGENT-002 WIP.
6. UI surfaces implying capabilities not on production path.
7. Federation code presence while FED-000 keeps production disabled.

## Strategic implication

The unfair head start is **local-first evidence + grants + WorkOrder/Run/artifact path**, not Spaces chat parity or federation. The wedge must use the agent execution + provenance path before collaborative Surfaces or live MCP hosting become the product story.
