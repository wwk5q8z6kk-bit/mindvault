# Current Security and Authority Gaps

**Date:** 2026-07-31

## Critical gaps (ordered)

1. **Incomplete command-admission coverage** — node routes covered; not all mutating surfaces (IK-017/018).
2. **No live authenticated external effect path** — LocalAck only (IK-005, SPACE-003).
3. **No Trust Ledger** as mandatory durable store (IK-020).
4. **Spaces membership authorization missing** (SPACE-001) — shared collaboration unsafe to ship.
5. **Agent isolation PARTIAL** (AGENT-004) — external executors gated.
6. **Federation threat model unsatisfied** — FED-000 kill-switch now returns 501 by default; FED-001..010 still open. Do not treat experimental transport as production-safe.
7. **Plugin/MCP confused-deputy risks** until gateway-only + grant split enforced.
8. **OIDC / multi-actor audience binding** incomplete (IK-019).

## Strengths already present

- Fail-closed grant admission on enforced node create path (E3).
- Action envelope required fields (E3).
- Promotion boundary prevents chat→knowledge pollution (E1).
- Low-risk engine runs produce artifacts with provenance digest (E1).
- Federation production disabled until gates pass (correct).

## Wedge implication

Ship value proofs that stay inside **local Engine + Low risk + explicit human gates**. Do not claim multi-tenant Spaces security or live tool execution until listed gaps close.
