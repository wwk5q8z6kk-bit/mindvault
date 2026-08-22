# Production Path Map

**Date:** 2026-07-31

## Primary production path (server)

`mv-cli` / process start → `mv-server` → auth middleware → REST/gRPC handlers → `MindVaultEngine` → `UnifiedStore` / SQLite (+ Lance indexes)

## High-leverage production paths for wedge

### A. Governed node mutation
Client → `POST/PUT/DELETE /api/v1/nodes` → grant admission / action envelope → engine → SQLite commit → (optional) outbox event → LocalAckPublisher

### B. Agent work execution (wedge core)
Client → Work Order APIs (`rest/work_orders.rs`) → admit run → acquire leases → `execute_run` (`agent_run_executor.rs`) → digested artifact + gate results → Completed

**Limits:** Engine executor, Low risk tier only; no live provider calls.

### C. Communication boundary
Relay send/receive → message tables only → optional `promote_relay_message` with provenance → retract without rewriting source

### D. Not production

| Path | Why |
|---|---|
| Live HTTP/Slack/email publisher | IK-005 not_started |
| Inbox domain consumer | IK-006 not_started |
| Production federation | FED-000 |
| Stage-2 guarded workspace writes | WS-010 unlock gates |
| Plans execution | Superseded; retiring |
| External ExecutorKind | AGENT-004 isolation incomplete |

## Prototype / secondary paths

- Frontend SvelteKit surfaces
- Plugin sandbox (not gateway-complete)
- MCP tools (not full Context/Tool grant split)
- Web static admin
