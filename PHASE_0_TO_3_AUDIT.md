# Phase 0–3 Audit Index

Status: **audit package complete; v2 implementation prohibited by open gates**

Evidence commit: `dcd7dbdb8c87ca01e22777f79e8f196b993fca23`

Audit date: 2026-07-25

Scope note: this package audits `dcd7dbd` and the dirty-worktree state captured from it. During
final validation, the original checkout had independently moved to `feat/product-evolution-session`
at `ef19da3`; that later lineage is not covered and is a new P0 reconciliation gate.

## Decision summary

MindVault has substantial working subsystems, but the repository is not a safe foundation for
v2 implementation yet. The decisive blockers are preservation/restore proof, original-evidence
semantics, canonical-versus-derived consistency, proposal enforcement, desktop authentication,
secret handling, licensing/authorship, unresolved vulnerability closure, migration ledger gaps,
and a red public baseline.

The recommended direction is an incremental, reversible consolidation:

1. human-readable source files and immutable evidence bytes remain user-owned inputs;
2. SQLite is the authority for identities, immutable evidence metadata, claims, versions,
   proposals, policy, audit, and projection state;
3. FTS, vector, and graph structures are derived projections with rebuild contracts;
4. AI emits evidence-bound typed proposals and cannot hold the canonical-write capability;
5. the first v2 slice is the approval-gated Obsidian/filesystem evidence loop defined in
   [FIRST_V2_VERTICAL_SLICE.md](FIRST_V2_VERTICAL_SLICE.md).

## Read in this order

### Phase 0 — protection

- [PHASE0_PROTECTION_BASELINE.md](PHASE0_PROTECTION_BASELINE.md)
- [BACKUP_AND_RESTORE_PROCEDURE.md](BACKUP_AND_RESTORE_PROCEDURE.md)
- [LEGACY_BUILD_AND_VALIDATION_RUNBOOK.md](LEGACY_BUILD_AND_VALIDATION_RUNBOOK.md)
- [audit/README.md](audit/README.md) and [audit/SBOM.cdx.json](audit/SBOM.cdx.json)

### Phase 1 — product constitution

- [PRODUCT_CONSTITUTION.md](PRODUCT_CONSTITUTION.md)
- [PRODUCT_BOUNDARIES.md](PRODUCT_BOUNDARIES.md)
- [DATA_OWNERSHIP.md](DATA_OWNERSHIP.md)
- [AI_AUTHORITY_MODEL.md](AI_AUTHORITY_MODEL.md)
- [PRIVACY_CONTRACT.md](PRIVACY_CONTRACT.md)
- [NAMING_BRIEF.md](NAMING_BRIEF.md)

### Phase 2 — repository truth

- [CURRENT_STATE.md](CURRENT_STATE.md)
- [FEATURE_TRUTH_MATRIX.md](FEATURE_TRUTH_MATRIX.md)
- [ARCHITECTURE_MAP.md](ARCHITECTURE_MAP.md)
- [DATA_FLOW_MAP.md](DATA_FLOW_MAP.md)
- [SECURITY_THREAT_MODEL.md](SECURITY_THREAT_MODEL.md)
- [VULNERABILITY_REACHABILITY_AND_DISPOSITION.md](VULNERABILITY_REACHABILITY_AND_DISPOSITION.md)
- [PRIVACY_DATA_FLOW.md](PRIVACY_DATA_FLOW.md)
- [DEPENDENCY_AND_LICENSE_AUDIT.md](DEPENDENCY_AND_LICENSE_AUDIT.md)
- [DATA_AND_MIGRATION_AUDIT.md](DATA_AND_MIGRATION_AUDIT.md)
- [UI_UX_ACCESSIBILITY_AUDIT.md](UI_UX_ACCESSIBILITY_AUDIT.md)
- [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md)
- [KEEP_REFACTOR_MODULARIZE_RETIRE.md](KEEP_REFACTOR_MODULARIZE_RETIRE.md)

### Phase 3 — decisions and migration

- [V2_ARCHITECTURE.md](V2_ARCHITECTURE.md)
- [V2_MIGRATION_PLAN.md](V2_MIGRATION_PLAN.md)
- [docs/adr-v2/README.md](docs/adr-v2/README.md)
- [BLOCKER_REGISTER.md](BLOCKER_REGISTER.md)
- [FIRST_V2_VERTICAL_SLICE.md](FIRST_V2_VERTICAL_SLICE.md)
- [benchmarks/phase0/README.md](benchmarks/phase0/README.md)

## Stop gate

Do not implement the v2 slice until every P0 blocker in
[BLOCKER_REGISTER.md](BLOCKER_REGISTER.md) has an owner, verified exit evidence, and an accepted
disposition; every ADR acceptance criterion needed by the slice is met; the backup has restored
to a fresh root; and the benchmark adapters required for a before/after comparison are executable.

This audit changes documentation and QA infrastructure only. It does not change product source,
schema, runtime configuration, or user data.
