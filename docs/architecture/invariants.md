# Proprietary RLM Core - Platform Invariants

These invariants are non-negotiable constraints for the core runtime. If any invariant is violated, the run is invalid by definition.

1. **Recursion Boundedness:** Every run must have enforced `max_depth`, `max_children_per_node`, and `max_total_steps` limits.
2. **Policy Non-Escalation:** Child runs may only receive equal or narrower permissions than their parent policy scope.
3. **Context Snapshot Immutability:** A run executes against immutable context snapshot IDs; mutation requires creating a new snapshot/version.
4. **Deterministic Merge Semantics:** Parent aggregation behavior must be deterministic for identical inputs and constraints.
5. **Budget Enforcement:** Token/time/cost budgets are hard limits; on breach, downstream execution is cancelled deterministically.
6. **Full Provenance:** Every derived artifact must link to source context references, operation metadata, and producing run/task IDs.
7. **Replayable Audit Trail:** Run timelines and parent-child relationships must be reconstructible from persisted trace events.
8. **Capability-Scoped Execution:** REPL/tool execution must run with explicit capability tokens and least privilege.
9. **Egress Guardrails:** External network/data egress is deny-by-default and policy-gated per task.
10. **Typed Failure Outcomes:** Runtime failures must emit typed error classes (policy, budget, tool, provider, validation, internal).
11. **Provider-Neutral Core Contracts:** Core domain types cannot depend on vendor-specific SDK types or error enums.
12. **Idempotent Retry Semantics:** Retry of a failed task with identical inputs must preserve correctness and not duplicate committed side effects.
13. **Schema Versioning Discipline:** Run/task/context/audit event payloads require explicit schema versions and compatibility policy.
14. **Observable Critical Path:** Each run must expose critical-path timing and bottleneck attribution for operator debugging.

## Acceptance rule
Changes that conflict with these invariants require an explicit ADR and architecture review.
