# Wedge Value Proof Results

**Date:** 2026-07-31  
**Wedge:** Trusted Agent Work (T01)  
**Status:** HTTP production-path proof **passing**

## Evidence

| Layer | Command | Result |
|---|---|---|
| Engine executor + WATW | `cargo test -p mv-engine --lib -- agent_run_executor -- --test-threads=1` | ok (`trusted_work_completed`) |
| SPACE-002 refusals | `cargo test -p mv-engine --lib -- work_order_agent_run -- --test-threads=1` | ok (self-approve, own G5, broaden grant, not-a-plan-step) |
| HTTP wedge | `cargo test -p mv-server --test work_order_conformance -- wedge_value_proof -- --test-threads=1` | ok |
| FED-000 default-off | `cargo test -p mv-server --test federation_e2e -- federation_disabled_by_default -- --test-threads=1` | ok |
| CLI | `cargo build -p mv-cli` + `mv trusted-work demo` | builds; demo path wired |

## Magic moment exercised

Local Context Node → Tool Grant → Work Order admit → start run → execute → completed artifact with digest + G0/G2 evidence on the public HTTP surface.

## Gaps (honest)

- WATW counter is process-lifetime (not durable analytics).
- SPACE-002: Space-scoped admit membership landed; unified action envelope still open (IK-020).
- Design-partner retention / H1 interviews not started.
