# MindVault Implementation Backlog

- **Status:** Authoritative execution tracker
- **Created:** 2026-07-26 · **Last recount:** 2026-07-31 (UX group appended; recount dashboard when verifying)
- **Branch:** `feat/product-evolution-session`
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`, then `docs/adr/011-sovereign-interoperability-fabric.md`, then `docs/adr/012-governed-agent-execution-graph.md`
- **Governing product plan:** `docs/MINDVAULT_NEXT_MASTER_PLAN.md` (product
  direction and roadmap narrative; does not replace this file for status)
- **Supersedes for execution tracking:** `DEVELOPMENT_PLAN.md` sections
  "Master Program Board", "Phase 0-4", and "Remaining Work"

## What this file is

Every explicitly deferred, gated, or blocked work item stated by the ratified
constitution, ADRs 008-012, and the supporting architecture contracts,
in one place, each traceable to the sentence that mandates it.

## Definition of done

> An item is done when its named **acceptance command passes**.

`DEVELOPMENT_PLAN.md:351` states *"'Complete' here means code is present in the
repository."* **That definition is rejected for this file.** It is the direct
cause of `DEVELOPMENT_PLAN.md:452` (`[x] Build federation engine`) coexisting
with `FEDERATION_THREAT_MODEL.md:22` ("Production federation must remain
disabled until the high-priority release gates in this document are satisfied").

An item whose acceptance test does not exist yet is normal and honest. The item
*is* "write test X and make it pass."

## Status vocabulary

| Status | Meaning | Self-assertable? |
|---|---|---|
| `not_started` | No code or test written | yes |
| `in_progress` | Work begun, acceptance command does not pass | yes |
| `blocked` | Cannot proceed until `blocked_by` items are `verified` | yes |
| `verified` | Acceptance command passed; `evidence` names command + commit + date | **no** — evidence required |

There is no `complete`. There are no section-level status rollups: the only
aggregate is a derived count in the dashboard, labeled derived.

## Item schema

```
#### <ID> — <title>
- **priority:** P0|P1|P2|P3 — <how the priority was derived>
- **status:** not_started|in_progress|blocked|verified
- **mandate:** `path:line` — "<quoted anchor>"
- **governing_authority:** <Law N | ADR 0NN §clause | feature-completeness §N>
- **blocked_by:** <IDs> (declared|inferred) | none
- **files:** <files that must change>
- **acceptance:** <exact command + assertion>
- **evidence:** —
```

- **mandate** carries `file:line` *and* a short quote. All cited docs are
  uncommitted; line numbers will drift, the quote will not.
- **governing_authority** may be a constitutional law (1-15,
  `INTEROPERABILITY_CONSTITUTION.md:44-77`), an ADR clause, or
  `feature-completeness §N` (`INTEROPERABILITY_CONSTITUTION.md:116-125`).
  Never blank. Never invented.
- **blocked_by** is `declared` when a doc states the ordering, `inferred`
  otherwise.

## Priority derivation

| | Meaning |
|---|---|
| **P0** | A cited doc names it a release blocker or the immediate next slice |
| **P1** | A cited doc names it a release gate for a specific capability |
| **P2** | A required disposition with no stated blocking relationship |
| **P3** | Explicitly deferred by a cited doc |

Spines reused verbatim: ADR 011:116-127; `FEDERATION_THREAT_MODEL.md:198-207`
(TM priorities as written); `interoperability-kernel-v1.md:282-285`;
`knowledge-workspace-migration.md:278-283`.

## Rules that keep this file honest

1. **IDs are append-only.** Never renumbered, never reused. Withdrawn items keep
   their ID with `status: withdrawn` and a reason.
2. **No section-level status rollups.** Aggregates are derived counts only.
3. **`verified` requires evidence.** No command + commit + date → not verified.
4. **No orphan gates.** A commit adding a "remains gated / not yet / release
   gate / is still required" sentence to any file under `docs/architecture/` or
   `docs/adr/` must add a backlog item in the same commit.
5. **Backlog beats prose.** If a document claims a capability is complete while
   an unverified item here cites it, the backlog wins and a `DOC-` item is filed.

## Citation anchoring caveat

Every `file:line` below points at content that is **uncommitted** on
`feat/product-evolution-session`. This file must land in the same commit as that
work, or its citations are unanchored from the first day.

## Dashboard (derived — recount, do not trust)

| Group | Items | P0 | verified |
|---|---|---|---|
| DOC — documentation honesty | 9 | 5 | 8 |
| PROG — program governance | 4 | 3 | 0 |
| IK — interoperability kernel | 25 | 9 | 7 |
| EXT — extension runtime & gateway | 9 | 0 | 0 |
| PROTO — protocol adapters | 9 | 0 | 0 |
| SRC — source authority & connectors | 8 | 0 | 0 |
| FED — federation | 13 | 10 | 0 |
| PORT — portability & conformance | 7 | 0 | 0 |
| SLICE — end-to-end vertical slice | 3 | 0 | 0 |
| WS — knowledge workspace | 22 | 6 | 0 |
| SPACE — collaborative spaces | 8 | 4 | 1 |
| PARK — explicitly deferred | 6 | 0 | 0 |
| AGENT — governed agent execution | 8 | 1 | 1 |
| HYG — engineering hygiene | 6 | 1 | 3 |
| UX — product experience (see `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md`) | 12 | 6 | 0 |
| **Total** | **150** | **45** | **9** |

---

# A. DOC — documentation corrections

Cheap, and they stop false-completeness claims from propagating into planning.
Do these first.

#### DOC-001 — Retract `DEVELOPMENT_PLAN.md` "Phase 3: Complete" and `[x] Build federation engine`
- **priority:** P0 — the claim directly contradicts a ratified release gate
- **status:** verified
- **mandate:** `FEDERATION_THREAT_MODEL.md:22` — "Production federation must remain disabled until the high-priority release gates"; contradicted by `DEVELOPMENT_PLAN.md:441` ("**Status:** Complete") and `DEVELOPMENT_PLAN.md:452` ("[x] Build federation engine (peer management)")
- **governing_authority:** Law 13 (`INTEROPERABILITY_CONSTITUTION.md:73`)
- **blocked_by:** none
- **files:** `DEVELOPMENT_PLAN.md` (437-462)
- **acceptance:** `grep -n "Status:\*\* Complete" DEVELOPMENT_PLAN.md` returns nothing in the Remaining Work section; federation line reads `[~] Federation engine — experimental transport only; production disabled per FEDERATION_THREAT_MODEL.md:22`
- **evidence:** Remaining Work Phase 3/4 no longer claim federation Complete; federation checklist uses `[~]` gated wording (2026-07-31, commit )

#### DOC-002 — Extend the `DEVELOPMENT_PLAN.md` migrations table from 025 to 037
- **priority:** P0 — schema inventory is used to plan migrations; 12 rows missing
- **status:** verified
- **mandate:** `DEVELOPMENT_PLAN.md:391-415` ends at "| 025 | Public shares | Ready |"; `migrations/` contains 026-037
- **governing_authority:** feature-completeness §1 (`INTEROPERABILITY_CONSTITUTION.md:116`)
- **blocked_by:** none
- **files:** `DEVELOPMENT_PLAN.md` (391-415, and 344 "all 25 migrations")
- **acceptance:** `ls migrations/*.sql | wc -l` equals the row count of the migrations table; rows exist for 026 node_comments, 027 mcp_connectors, 028 sealed_node_payloads, 029 key_epoch_reencryption, 030 conversation_turn_sources, 031 knowledge_workspace_manifest, 032 interoperability_kernel, 033 governed_interoperability_registries, 034 context_node_registry, 035 authority_grants, 036 outbox_dispatch_and_action_receipts, 037 consumer_inbox_checkpoints
- **evidence:** migrations table has 39 rows matching `ls migrations/*.sql | wc -l` including 026-039 (2026-07-31, commit )

#### DOC-003 — Replace `DEVELOPMENT_PLAN.md:351`'s definition of "Complete"
- **priority:** P0 — this sentence is the root cause of DOC-001 and DOC-009
- **status:** verified
- **mandate:** `DEVELOPMENT_PLAN.md:351` — "'Complete' here means code is present in the repository"
- **governing_authority:** feature-completeness §10 (`INTEROPERABILITY_CONSTITUTION.md:125`) — "automated contract and conformance tests"
- **blocked_by:** none
- **files:** `DEVELOPMENT_PLAN.md` (349-351)
- **acceptance:** line 351 states that completeness for any capability governed by the constitution means the feature-completeness contract is satisfied and points to `IMPLEMENTATION_BACKLOG.md`; the phrase "code is present in the repository" no longer appears
- **evidence:** removed "code is present in the repository"; completeness points to constitution + IMPLEMENTATION_BACKLOG.md (2026-07-31, commit )

#### DOC-004 — Register the four unregistered contracts in the constitution
- **priority:** P0 — unregistered contracts are invisible to anyone following the constitution
- **status:** verified
- **mandate:** `INTEROPERABILITY_CONSTITUTION.md:171-177` lists 7 supporting contracts; missing `ACTION_RECEIPT_MODEL.md`, `AUTHORITY_GRANT_MODEL.md`, `CONSUMER_INBOX_MODEL.md`, `interoperability-kernel-v1.md`
- **governing_authority:** `INTEROPERABILITY_CONSTITUTION.md:169` "Supporting contracts"
- **blocked_by:** none
- **files:** `INTEROPERABILITY_CONSTITUTION.md` (169-177)
- **acceptance:** all 11 files under `docs/architecture/` referenced by ADR 011 or the kernel doc appear in the list; `for f in $(grep -o 'docs/architecture/[A-Za-z_-]*\.md' INTEROPERABILITY_CONSTITUTION.md); do test -f "$f"; done` passes
- **evidence:** constitution Supporting contracts list includes ACTION_RECEIPT, AUTHORITY_GRANT, CONSUMER_INBOX, interoperability-kernel-v1; linked architecture paths exist (2026-07-31, commit )

#### DOC-005 — Add `AUTHORITY_GRANT_MODEL.md` to `docs/README.md`
- **priority:** P1 — the doc index registers 12 of 13 new contracts
- **status:** verified
- **mandate:** `docs/README.md` contains entries for ACTION_RECEIPT, CONSUMER_INBOX, SOURCE_AUTHORITY, CONTEXT_NODE, EXTENSION_SECURITY, PROTOCOL_BOUNDARIES, FEDERATION_THREAT, DATA_PORTABILITY but no `AUTHORITY_GRANT_MODEL` line
- **governing_authority:** ADR 011 §Decision — "Context Grants and Tool Grants" (`docs/adr/011-sovereign-interoperability-fabric.md:53`)
- **blocked_by:** none
- **files:** `docs/README.md`
- **acceptance:** `grep -c AUTHORITY_GRANT_MODEL docs/README.md` ≥ 1
- **evidence:** `grep -c AUTHORITY_GRANT_MODEL docs/README.md` = 1 (2026-07-31, commit )

#### DOC-006 — Register `IMPLEMENTATION_BACKLOG.md` in the constitution
- **priority:** P0 — without this the backlog is an orphan and repeats the drift it exists to prevent
- **status:** verified
- **mandate:** `INTEROPERABILITY_CONSTITUTION.md:169-177` "Supporting contracts"
- **governing_authority:** `INTEROPERABILITY_CONSTITUTION.md:28-37` priority and conflict rule
- **blocked_by:** none
- **files:** `INTEROPERABILITY_CONSTITUTION.md`
- **acceptance:** `grep -c IMPLEMENTATION_BACKLOG INTEROPERABILITY_CONSTITUTION.md` ≥ 1
- **evidence:** `grep -c IMPLEMENTATION_BACKLOG INTEROPERABILITY_CONSTITUTION.md` = 2 (2026-07-31, commit )

#### DOC-007 — Point `DEVELOPMENT_PLAN.md` at the backlog for execution tracking
- **priority:** P1 — readers currently land on the all-`[x]` board first
- **status:** verified
- **mandate:** `DEVELOPMENT_PLAN.md:3-10` already carries the supersession notice but names no replacement tracker
- **governing_authority:** `INTEROPERABILITY_CONSTITUTION.md:33-37`
- **blocked_by:** DOC-006 (inferred)
- **files:** `DEVELOPMENT_PLAN.md` (3-29), `DEVELOPMENT_PLAN.md:285` Master Program Board header
- **acceptance:** the notice block names `IMPLEMENTATION_BACKLOG.md` as the execution tracker; the Master Program Board carries a "pre-constitution, historical" header
- **evidence:** notice names IMPLEMENTATION_BACKLOG.md; Master Program Board header marked pre-constitution, historical (2026-07-31, commit )

#### DOC-008 — Reconcile ADR 010 status `Proposed` with the constitution calling it authoritative
- **priority:** P2 — derived: no doc declares this blocking, but it is an unresolved contradiction
- **status:** not_started
- **mandate:** `INTEROPERABILITY_CONSTITUTION.md:39` — "ADR 010 remains authoritative for Personal Vaults and Collaborative Spaces"; `docs/adr/010-personal-vault-and-collaborative-spaces.md:3` — "**Status:** Proposed"
- **governing_authority:** ADR 010 §Acceptance gate (`docs/adr/010-personal-vault-and-collaborative-spaces.md:255-271`)
- **blocked_by:** SPACE-001..SPACE-007 (declared — the 7 acceptance-gate conditions)
- **files:** `docs/adr/010-personal-vault-and-collaborative-spaces.md`, `INTEROPERABILITY_CONSTITUTION.md:39`
- **acceptance:** either ADR 010 is Accepted with all 7 gate conditions `verified`, or the constitution says "ADR 010 (Proposed) is the intended authority"
- **evidence:** —

#### DOC-009 — Correct the `DEVELOPMENT_PLAN.md:379` Federation Engine status row
- **priority:** P1 — a subsystem table row asserting "Complete (REST transport)" for a subsystem that must be disabled
- **status:** verified
- **mandate:** `DEVELOPMENT_PLAN.md:379` — "| Federation Engine | `federation.rs` | Complete (REST transport) |"; contradicted by `FEDERATION_THREAT_MODEL.md:11` — "an experimental transport, not a production security boundary"
- **governing_authority:** Law 13 (`INTEROPERABILITY_CONSTITUTION.md:73`)
- **blocked_by:** none
- **files:** `DEVELOPMENT_PLAN.md` (367-390)
- **acceptance:** the row reads `Experimental — production disabled (FEDERATION_THREAT_MODEL.md:22)`
- **evidence:** Federation Engine row reads Experimental — production disabled (FEDERATION_THREAT_MODEL.md:22) (2026-07-31, commit )

---

# B. PROG — program governance

#### PROG-001 — Assign durable owners for eight kernel domains
- **priority:** P0 — listed as an active obligation of the closed Phase 0 gate
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:116-117` — "assign durable owners for schemas, grants, events, outbox delivery, receipts, connector runtime, and conformance"
- **governing_authority:** ADR 011 §Implementation order step 1 (`docs/adr/011-sovereign-interoperability-fabric.md:118`)
- **blocked_by:** none
- **files:** `IMPLEMENTATION_BACKLOG.md` (owners column), `docs/architecture/interoperability-baseline.md`
- **acceptance:** each of the 8 domains has a named owner recorded in this file; no group has an unowned P0
- **evidence:** —

#### PROG-002 — Migration and rollback plan for seven existing subsystems
- **priority:** P0 — active Phase 0 obligation; blocks every migration item
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:118-120` — "define migrations and rollback for existing federation peers, OAuth consumers, MCP connector records, plugins, adapters, relay identities, and exported objects"
- **governing_authority:** ADR 011 §Costs — "require migration rather than relabeling" (`docs/adr/011-sovereign-interoperability-fabric.md:112-113`)
- **blocked_by:** none
- **files:** new `docs/architecture/KERNEL_MIGRATION_PLAN.md`; `migrations/`
- **acceptance:** the plan names, per subsystem, the source table/struct, the target kernel contract, a forward migration, a rollback, and a test; `cargo test --workspace -- migration_rollback` passes
- **evidence:** —

#### PROG-003 — Select the first federation deployment profile and validate its threat-model assumptions
- **priority:** P0 — active Phase 0 obligation; gates the whole FED group
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:121-123` — "select the first federation deployment profile and validate its threat-model assumptions"
- **governing_authority:** ADR 011 §Implementation order step 6 (`docs/adr/011-sovereign-interoperability-fabric.md:123`)
- **blocked_by:** none
- **files:** `docs/architecture/FEDERATION_THREAT_MODEL.md` (42-53 deployment assumptions)
- **acceptance:** a profile document states tenancy, exposure, TLS termination, egress control, and identity anchor, and each of the 10 assumptions at `FEDERATION_THREAT_MODEL.md:42-51` is marked holds / does-not-hold with evidence
- **evidence:** —

#### PROG-004 — Add a CI check enforcing the no-orphan-gate rule
- **priority:** P2 — derived: the mechanism that keeps this file from becoming the next all-`[x]` board
- **status:** not_started
- **mandate:** this file, §"Rules that keep this file honest" rule 4
- **governing_authority:** feature-completeness §10 (`INTEROPERABILITY_CONSTITUTION.md:125`)
- **blocked_by:** DOC-006 (inferred)
- **files:** `.github/workflows/`, `scripts/`
- **acceptance:** a CI job fails when a diff adds "remains gated", "release gate", "is still required", "does not yet", or "not yet" to `docs/architecture/**` or `docs/adr/**` without touching `IMPLEMENTATION_BACKLOG.md`
- **evidence:** —

---

# C. IK — interoperability kernel

ADR 011 §Implementation order step 2. `interoperability-kernel-v1.md:15-19`
states the implemented slice "is a kernel foundation, not completion of the full
interoperability kernel."

#### IK-001 — Wire the grant resolver into public command admission
- **priority:** P0 — named as the immediate next slice
- **status:** **verified** (2026-07-27, node-create command path)
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:283-284` — "The next gated kernel slice should wire action-envelope and grant admission ahead of any live provider publisher"
- **governing_authority:** Law 7 (`INTEROPERABILITY_CONSTITUTION.md:60`), Law 8 (`:62`)
- **blocked_by:** none
- **files:** `crates/mv-core/src/model/interoperability.rs`, `crates/mv-engine/src/engine/interoperability_ops.rs`, `crates/mv-server/src/rest/interoperability.rs`, `crates/mv-server/src/state.rs`, `crates/mv-server/src/rest.rs`, `config/default.toml`
- **acceptance:** `cargo test -p mv-core -p mv-engine` and `cargo test -p mv-server --tests -- --test-threads=1`
- **evidence:** Shipped in four commits behind `MINDVAULT_COMMAND_ADMISSION_MODE` (`off` default / `observe` / `enforce`). `auth.rs` and `traits.rs` were **not** modified: `AuthContext` has 21 struct literals across five crates and extending it would have broken all of them for no functional gain, so `CommandIdentity` carries the identity instead. Admission is additive and sits after the idempotent-replay lookup and before the quota check. Denials return 403 with a stable `command_admission_denied` code and never the bounded reason, which would be a probing oracle. Observed: 86 mv-core + 393 mv-engine + 249 mv-server lib + 33 api_integration + 2 federation_e2e + 9 transport_parity + 14 work_order_conformance passed; clippy exit 0; fmt clean.
- **scope note:** covers `POST /api/v1/nodes` only. Other mutating endpoints are `IK-002`/`IK-017`.

#### IK-001a — Governed command for local Context Node registration
- **priority:** P0 — a hard prerequisite for running `enforce` on a real vault
- **status:** **verified** (2026-07-27)
- **mandate:** `crates/mv-storage/src/sqlite.rs` — `commit_authority_grant_with_event` refuses a grant whose governing node has no Active registered descriptor
- **governing_authority:** feature-completeness §2 (`INTEROPERABILITY_CONSTITUTION.md:116`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/engine/interoperability_ops.rs`, `crates/mv-server/src/rest/interoperability.rs`, `crates/mv-server/src/rest.rs`, `crates/mv-server/src/openapi.rs`
- **acceptance:** a fresh vault can register its local Context Node through a governed command, so a Tool Grant can then be issued
- **evidence:** `POST/GET /api/v1/context-nodes/local` (admin write / authenticated read). Engine command is idempotent and registers the self-governed node directly as `active` per `CONTEXT_NODE_MODEL.md`. `LocalContextNodeRegistration.newly_registered` comes from the idempotent commit (not a TOCTOU pre-check). Observed: 7/7 `engine::interoperability_ops` + `local_context_node_registers_idempotently_over_http`.

#### IK-001b — Admin-only grant issuance and lifecycle endpoints
- **priority:** P0 — the second prerequisite for `enforce`
- **status:** **verified** (2026-07-27)
- **mandate:** `docs/architecture/AUTHORITY_GRANT_MODEL.md` — "public grant APIs ... remain later integration gates"
- **governing_authority:** Law 2 (`INTEROPERABILITY_CONSTITUTION.md:46`), feature-completeness §2-§3
- **blocked_by:** IK-001a
- **files:** `crates/mv-engine/src/engine/interoperability_ops.rs`, `crates/mv-server/src/rest/interoperability.rs`, `crates/mv-server/src/rest.rs`, `crates/mv-server/src/openapi.rs`, `docs/adr/013-admin-authority-grant-apis.md`
- **acceptance:** an owner can issue, suspend and revoke a Tool Grant over the API; lifting the deferral is recorded in an ADR note rather than done silently
- **evidence:** Admin `POST/GET /api/v1/authority-grants` plus suspend/revoke/resume. Grantor is the local owner principal; grantee accepts a URI or `grantee_subject` matching command-admission derivation. Defaults target the local node with Tool/`command`/durable retention so enforce can admit creates. ADR 013 records the deferral lift. Observed: 10/10 `engine::interoperability_ops` (issue/idempotent/suspend-revoke) + `authority_grant_lifecycle_enables_enforced_node_create`.

#### IK-001c — Durable command-admission decisions
- **priority:** P1 — makes denials durable and is the first brick of the Trust Ledger
- **status:** **verified** (2026-07-27)
- **mandate:** `docs/architecture/ACTION_RECEIPT_MODEL.md` § "Command admission does not write an action receipt"
- **governing_authority:** Law 15 (`INTEROPERABILITY_CONSTITUTION.md:77`), ADR 010:149-152
- **blocked_by:** none
- **files:** `migrations/039_command_admission_decisions.sql`, `crates/mv-core/src/model/interoperability.rs`, `crates/mv-core/src/traits.rs`, `crates/mv-storage/src/sqlite.rs`, `crates/mv-engine/src/engine/interoperability_ops.rs`
- **acceptance:** a denied command leaves an immutable, idempotent record keyed on `(principal_uri, idempotency_key)`
- **evidence:** Append-only `interoperability_command_admission_decisions` with immutability triggers. `resolve_command_admission` persists admitted and denied decisions fail-closed. Idempotent replay on `(principal, idempotency_key)` with digest/decision conflict → `IdempotencyConflict`. Observed: engine tests for durable denial, conflict, and admitted persistence.

#### IK-002 — Versioned action envelope on every public command
- **priority:** P0 — named as the immediate next slice alongside IK-001
- **status:** **verified** (2026-07-27)
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:283` — "wire action-envelope and grant admission"; `ACTION_RECEIPT_MODEL.md:63-64` — "Public action-envelope admission must resolve an effective Tool Grant and policy decision before any live publisher is enabled"
- **governing_authority:** Law 8 (`:62`); ADR 010 §Action envelope (`docs/adr/010-personal-vault-and-collaborative-spaces.md:135-148`); ADR 014
- **blocked_by:** IK-001 (declared)
- **files:** `crates/mv-core/src/model/interoperability.rs`, `crates/mv-server/src/rest/interoperability.rs`, `crates/mv-server/src/rest.rs`, `docs/adr/014-versioned-action-envelope.md`
- **acceptance:** `cargo test -p mv-core -- action_envelope` — an envelope missing action ID, correlation ID, principal, acting actor, resource, operation, grant IDs, or policy decision is rejected; mutating node commands construct one under observe/enforce
- **evidence:** `ActionEnvelope` / `NewActionEnvelope` with fail-closed `try_new`/`from_admission`. `admit_command` returns the envelope when admission is active. `POST/PUT/DELETE /api/v1/nodes` construct it; create embeds `action_envelope` beside admission metadata. Observed: `cargo test -p mv-core -- action_envelope` + server admission observe/enforce tests. Space/work-order/budget ADR fields remain deferred (IK-020).

#### IK-003 — Governed identity registry replacing the local-system fallback
- **priority:** P0 — an explicitly labelled transition identity underneath all attribution
- **status:** verified
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:158-159` — "The current local-system fallback remains a transition identity until the governed identity registry is implemented"
- **governing_authority:** Law 5 (`:54`); ADR 010 §Actors (`docs/adr/010-personal-vault-and-collaborative-spaces.md:82-101`)
- **blocked_by:** none
- **files:** `crates/mv-core/src/model/interoperability.rs`, `migrations/040_identity_registry.sql`, `crates/mv-core/src/traits.rs`, `crates/mv-storage/src/sqlite.rs`, `crates/mv-server/src/auth.rs`, `crates/mv-server/src/rest/interoperability.rs`
- **acceptance:** `cargo test -p mv-storage -- identity_registry` — principal URIs resolve through versioned identity records with actor kind (`human`/`agent`/`service`/`integration`); the local-system fallback is removed or gated behind an explicit dev flag
- **evidence:** `cargo test -p mv-storage -- identity_registry` → 2/2 ok; commit `49fb8f7`; 2026-07-31. Migration 040 + `IdentityRecord`/`ActorKind`; `MINDVAULT_ALLOW_LOCAL_SYSTEM_IDENTITY` gates missing-subject fallback; `system_admin` uses explicit `local-system` subject.

#### IK-004 — Outbox dispatcher worker
- **priority:** P0 — the outbox has no runtime; events accumulate as pending forever
- **status:** **verified** (2026-07-27)
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:199-201` — "it does not run a background worker or implement an HTTP, Slack, email, MCP, or A2A publisher"; `ACTION_RECEIPT_MODEL.md:61` — "This slice does not run a dispatcher"
- **governing_authority:** Law 4 (`:52`)
- **blocked_by:** IK-001, IK-002 (declared — "ahead of any live provider publisher")
- **files:** `crates/mv-engine/src/engine/outbox_dispatch.rs`, `crates/mv-server/src/outbox_dispatch.rs`, `crates/mv-server/src/lib.rs`
- **acceptance:** `cargo test -p mv-engine -- outbox_dispatcher` — the worker claims under a lease, honours `next_attempt_at`, advances attempt counters exactly once on reclaim, and writes exactly one immutable receipt per completion
- **evidence:** `OutboxPublisher` trait + `LocalAckPublisher`; `dispatch_outbox_once` claims/publishes/completes via storage APIs only. Env-gated server spawn (`MINDVAULT_OUTBOX_DISPATCH_ENABLED`, default off). Observed: claim+one receipt, `next_attempt_at` gate, reclaim attempt+1, idle second tick. Live HTTP publisher remains IK-005.

#### IK-005 — Authenticated live transport publisher (first destination)
- **priority:** P0 — named gated work
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:17-19` — "authenticated live transport publishers ... remain later gated work"
- **governing_authority:** Law 4 (`:52`), Law 5 (`:54`)
- **blocked_by:** IK-004 (declared)
- **files:** `crates/mv-engine/src/engine/outbox_dispatch.rs`, `crates/mv-server/src/rest.rs`
- **acceptance:** `cargo test -p mv-engine -- publisher_http` — a destination acknowledgement produces a `published` receipt; a transient failure produces `retry_scheduled` with a future `next_attempt_at`; a terminal failure produces `dead_lettered`
- **evidence:** —

#### IK-006 — Consumer transport listener and domain handler
- **priority:** P0 — the inbox has no runtime
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:223-225` — "It does not run a transport listener, invoke a domain projection, infer an issuer sequence, or requeue a dead letter"; `CONSUMER_INBOX_MODEL.md:58-59` — "does not verify remote signatures, run a background consumer"
- **governing_authority:** Law 4 (`:52`), Law 14 (`:75`)
- **blocked_by:** IK-002 (declared — "a policy-authorized action envelope", `CONSUMER_INBOX_MODEL.md:61`)
- **files:** new `crates/mv-engine/src/engine/inbox_consumer.rs`, `crates/mv-server/src/lib.rs`
- **acceptance:** `cargo test -p mv-engine -- inbox_consumer` — admission, exclusive claim, domain application, receipt, and checkpoint advance in order; retry does not advance the checkpoint
- **evidence:** —

#### IK-007 — Governed dead-letter redrive command
- **priority:** P1 — explicitly requires its own governed command
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:227-228` — "Dead-letter redrive requires a separate governed command so a prior terminal receipt and checkpoint are never rewritten"
- **governing_authority:** Law 4 (`:52`)
- **blocked_by:** IK-004, IK-006 (declared)
- **files:** `crates/mv-core/src/traits.rs`, `crates/mv-storage/src/sqlite.rs`, `crates/mv-server/src/rest.rs`
- **acceptance:** `cargo test -p mv-storage -- dead_letter_redrive` — redrive creates a new event with causation to the dead letter; the original receipt and checkpoint are byte-identical afterwards
- **evidence:** —

#### IK-008 — Schema lifecycle commands and replacement migration guard
- **priority:** P1 — transitions deliberately blocked pending this contract
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:75-76` — "Lifecycle transitions are deliberately blocked until they receive their own governed command/event contract and replacement migration guard"
- **governing_authority:** Law 14 (`:75`)
- **blocked_by:** none
- **files:** `migrations/033_governed_interoperability_registries.sql` successor, `crates/mv-core/src/model/interoperability.rs`, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-storage -- schema_lifecycle` — a schema can be deprecated/withdrawn only through the governed command with an emitted event; withdrawing a schema with live outbox references fails closed
- **evidence:** —

#### IK-009 — Public registry transports (schemas, bindings, Context Nodes)
- **priority:** P1 — release gate before the end-to-end proof
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:139-141` — "public registry and grant transports, admission enforcement, and conformance fixtures remain required before the end-to-end proof"; `interoperability-kernel-v1.md:17` — "public registry APIs"
- **governing_authority:** Law 2 (`:46`); feature-completeness §2-3 (`:117-118`)
- **blocked_by:** IK-001 (declared)
- **files:** new `crates/mv-server/src/rest/interoperability.rs`, `crates/mv-server/src/rest.rs`, `crates/mv-server/src/openapi.rs`
- **acceptance:** `cargo test -p mv-server -- registry_api` — schema, Source Binding, and Context Node registration/query are reachable over REST with grant admission; the routes appear in generated OpenAPI. *(Verified today: zero `/api/v1/schemas|source-bindings|context-nodes|grants` routes exist.)*
- **evidence:** —

#### IK-010 — Public grant transport
- **priority:** P1 — named a later integration gate
- **status:** not_started
- **mandate:** `AUTHORITY_GRANT_MODEL.md:96-97` — "public grant APIs, and signatures remain later integration gates"
- **governing_authority:** Law 7 (`:60`), Law 8 (`:62`)
- **blocked_by:** IK-001, IK-009 (inferred)
- **files:** `crates/mv-server/src/rest/interoperability.rs`, `crates/mv-server/src/openapi.rs`
- **acceptance:** `cargo test -p mv-server -- grant_api` — issue, list, delegate, suspend, revoke over REST; a delegation attempt that widens targets, capabilities, validity, sensitivity, retention, or depth is rejected
- **evidence:** —

#### IK-011 — Grant retention-ceiling enforcement at use time
- **priority:** P1 — named a later integration gate
- **status:** not_started
- **mandate:** `AUTHORITY_GRANT_MODEL.md:96` — "Retention enforcement, action-envelope admission, external tool execution ... remain later integration gates"
- **governing_authority:** Law 7 (`:60`)
- **blocked_by:** IK-001 (declared)
- **files:** `crates/mv-core/src/model/interoperability.rs`, `crates/mv-server/src/auth.rs`
- **acceptance:** `cargo test -p mv-core -- retention_ceiling` — a request whose retention class exceeds the grant ceiling is denied and the denial is recorded
- **evidence:** —

#### IK-012 — Grant signatures
- **priority:** P1 — named a later integration gate
- **status:** not_started
- **mandate:** `AUTHORITY_GRANT_MODEL.md:96-97` — "public grant APIs, and signatures remain later integration gates"
- **governing_authority:** Law 13 (`:73`)
- **blocked_by:** IK-010 (inferred)
- **files:** `crates/mv-core/src/model/interoperability.rs`, `crates/mv-engine/src/keychain.rs`
- **acceptance:** `cargo test -p mv-core -- grant_signature` — a grant round-trips through sign/verify; a tampered target set fails verification
- **evidence:** —

#### IK-013 — Remote signature verification
- **priority:** P1 — named gated work in four contracts
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:18` — "remote signature verification ... remain later gated work"; `CONSUMER_INBOX_MODEL.md:58`; `ACTION_RECEIPT_MODEL.md:61`; `CONTEXT_NODE_MODEL.md:191`
- **governing_authority:** Law 13 (`:73`), Law 14 (`:75`)
- **blocked_by:** FED-002 (declared — key proof must exist first)
- **files:** `crates/mv-core/src/model/interoperability.rs`, `crates/mv-engine/src/federation.rs`
- **acceptance:** `cargo test -p mv-engine -- remote_signature` — an envelope signed by an untrusted or rotated-out key is rejected before admission
- **evidence:** —

#### IK-014 — Authenticated inbox transport admission
- **priority:** P1 — release gate before the end-to-end proof
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:137-139` — "authenticated inbox transport admission, outbox transport publication, connector lifecycle ... remain required"
- **governing_authority:** Law 5 (`:54`), Law 14 (`:75`)
- **blocked_by:** IK-006, IK-013 (declared)
- **files:** `crates/mv-server/src/rest.rs`, `crates/mv-engine/src/engine/inbox_consumer.rs`
- **acceptance:** `cargo test -p mv-server -- inbox_admission` — unauthenticated or unsigned inbound envelopes are rejected and recorded; authenticated ones admit exactly once
- **evidence:** —

#### IK-015 — Remote issuer sequence-gap detection
- **priority:** P1 — a stated non-claim of the current checkpoint contract
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:144-145` — "Consumer checkpoints use local admission sequence numbers and therefore cannot detect gaps in a remote issuer's stream"; `interoperability-kernel-v1.md:224` — "infer an issuer sequence"
- **governing_authority:** Law 14 (`:75`)
- **blocked_by:** IK-014 (inferred)
- **files:** `migrations/037_consumer_inbox_checkpoints.sql` successor, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-storage -- issuer_sequence_gap` — a missing issuer sequence number raises an observable gap rather than silently advancing
- **evidence:** —

#### IK-016 — Durable projection checkpoint and recovery worker
- **priority:** P1 — projection recovery is explicitly not complete
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:240-242` — "a durable projection checkpoint/worker is still required before projection recovery can be considered complete"
- **governing_authority:** Law 4 (`:52`)
- **blocked_by:** IK-004 (inferred)
- **files:** `crates/mv-engine/src/engine/mod.rs`, `crates/mv-engine/src/enrichment.rs`, new migration
- **acceptance:** `cargo test -p mv-engine -- projection_recovery` — killing the process between canonical commit and projection leaves a pending outbox event that the worker later reconciles; FTS/vector/graph converge
- **evidence:** —

#### IK-017 — Event coverage for all committed mutations
- **priority:** P1 — Law 4 applies to every mutation; only node-create is covered
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:46` — "Add authenticated live publishers, CloudEvents/AsyncAPI mappings, and coverage for all mutations"; `interoperability-kernel-v1.md:43-44` — "The first event type is `dev.mindvault.knowledge.node.created.v1`"
- **governing_authority:** Law 4 (`:52`)
- **blocked_by:** IK-008 (inferred — each new event type needs a registered schema)
- **files:** `crates/mv-storage/src/sqlite.rs`, `crates/mv-engine/src/engine/node_ops.rs`, `crates/mv-engine/src/engine/graph_ops.rs`
- **acceptance:** `cargo test -p mv-server -- mutation_event_coverage` — an inventory test enumerates mutating REST routes and asserts each commits an outbox event in the same transaction
- **evidence:** —

#### IK-018 — One governed command/query contract with transport parity policy
- **priority:** P2 — required disposition, no stated blocking relationship
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:40` — "Define one governed command/query contract and transport parity policy"
- **governing_authority:** Law 2 (`:46`); `PROTOCOL_BOUNDARIES.md:102-103` — "internal gRPC does not silently expose broader authority"
- **blocked_by:** IK-002 (inferred)
- **files:** `crates/mv-server/src/rest.rs`, `crates/mv-server/src/grpc.rs`, `crates/mv-server/src/websocket.rs`, `crates/mv-server/src/openapi.rs`
- **acceptance:** `cargo test -p mv-server -- transport_parity` — no gRPC or WebSocket operation exposes authority absent from OpenAPI
- **evidence:** —

#### IK-019 — OIDC and distinct actor/device/agent identities with audience binding
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:41` — "Add OIDC, distinct actor/device/agent identities, short-lived grants, and audience binding"
- **governing_authority:** Law 8 (`:62`); `PROTOCOL_BOUNDARIES.md:116-126`
- **blocked_by:** IK-003 (inferred)
- **files:** `crates/mv-server/src/auth.rs`, `crates/mv-server/src/rest/oauth.rs`
- **acceptance:** `cargo test -p mv-server -- oidc_identity` — tokens carry actor kind and audience; a token for one audience is rejected at another
- **evidence:** —

#### IK-020 — Mandatory durable Trust Ledger
- **priority:** P2 — required disposition; request audit is telemetry, not a ledger
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:49` — "Keep as telemetry; add mandatory Trust Ledger with principal/actor/grants/effects"; ADR 010:150-152 — "none is a substitute until it carries the complete envelope and is durable by default"
- **governing_authority:** feature-completeness §7 (`:122`)
- **blocked_by:** IK-002 (declared — the ledger stores action envelopes)
- **files:** new migration, `crates/mv-server/src/audit.rs`, `crates/mv-core/src/model/`
- **acceptance:** `cargo test -p mv-server -- trust_ledger` — append-only, survives restart, cannot be disabled by configuration, and every action envelope produces one record
- **evidence:** —

#### IK-021 — Receipt boundary across all remaining effectful actions
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:48` — "Require the same receipt boundary across remaining effectful actions"
- **governing_authority:** feature-completeness §6-7 (`:121-122`)
- **blocked_by:** IK-020 (inferred)
- **files:** `crates/mv-server/src/rest/`, `crates/mv-engine/src/adapters/`
- **acceptance:** `cargo test -p mv-server -- effectful_receipt_coverage` — an inventory test enumerates externally effectful operations and asserts each produces an immutable receipt
- **evidence:** —

#### IK-022 — Protocol translation layer
- **priority:** P2 — named gated work; no stated ordering against IK-001
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-kernel-v1.md:18-19` — "and protocol translation remain later gated work"; `:57-59` — "Those mappings are not part of this slice"
- **governing_authority:** Law 9 (`:64`)
- **blocked_by:** IK-002 (inferred)
- **files:** new `crates/mv-core/src/protocol/`
- **acceptance:** `cargo test -p mv-core -- protocol_translation` — a round trip through each supported mapping preserves every security-relevant envelope field; a lossy mapping surfaces translation loss rather than dropping fields
- **evidence:** —

---

# D. EXT — extension runtime and universal gateway

ADR 011 §Implementation order step 3. `EXTENSION_SECURITY_MODEL.md:94-95`:
"The existing WASM permission gate and fuel metering are reusable foundations,
not proof of this complete boundary."

#### EXT-001 — Signed versioned extension manifest (23 required fields)
- **priority:** P1 — release gate for extension execution
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:35-64` — the manifest field list; `:65` — "Unknown security-relevant fields or versions fail closed"
- **governing_authority:** Law 15 (`:77`), Law 10 (`:66`)
- **blocked_by:** none
- **files:** `crates/mv-plugin/src/manifest.rs`
- **acceptance:** `cargo test -p mv-plugin -- manifest_v2` — a manifest missing any of the 23 fields fails closed; an unknown security-relevant field fails closed; signature binds manifest to artifact digest
- **evidence:** —

#### EXT-002 — Eight-step installation lifecycle with receipts
- **priority:** P1 — release gate
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:66-79` — steps 1-8; `:78` — "Record install, upgrade, permission change, disable, and removal receipts"
- **governing_authority:** Law 10 (`:66`); feature-completeness §7 (`:122`)
- **blocked_by:** EXT-001 (declared)
- **files:** `crates/mv-plugin/src/registry.rs`, `crates/mv-server/src/rest/plugins.rs`
- **acceptance:** `cargo test -p mv-plugin -- install_lifecycle` — an update adding permissions requires new approval; rollback preserves the previous signed package
- **evidence:** —

#### EXT-003 — Secret broker (no raw secrets to extensions or models)
- **priority:** P1 — release gate
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:90-91` — "Secrets are resolved by a broker for a specific destination and operation; raw secret values are not exposed to models"
- **governing_authority:** Law 3 (`:49`), Law 15 (`:77`)
- **blocked_by:** EXT-001 (inferred)
- **files:** `crates/mv-plugin/src/sandbox.rs`, `crates/mv-engine/src/keychain.rs`
- **acceptance:** `cargo test -p mv-plugin -- secret_broker` — an extension receives a handle, never a value; a handle for destination A cannot be used against destination B
- **evidence:** —

#### EXT-004 — Destination-, method-, protocol-, and data-class-scoped network grants
- **priority:** P1 — release gate; TM-009 mitigation
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:92` — "Network access is destination-, method-, protocol-, and data-class scoped"; `FEDERATION_THREAT_MODEL.md:206` (TM-009) — "Manifest network access is broad"
- **governing_authority:** Law 15 (`:77`)
- **blocked_by:** EXT-001 (declared)
- **files:** `crates/mv-plugin/src/sandbox.rs`, `crates/mv-plugin/src/wasm_plugin.rs`
- **acceptance:** `cargo test -p mv-plugin -- network_grant` — an undeclared destination is denied and recorded; a declared destination with an undeclared data class is denied
- **evidence:** —

#### EXT-005 — File-scope handles instead of ambient host paths
- **priority:** P1 — release gate
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:93` — "File access uses explicit handles or mounted scopes, not ambient host paths"
- **governing_authority:** Law 15 (`:77`); ADR 009 gate 3 (`docs/adr/009-workspace-identity-and-mutation-boundary.md:220`)
- **blocked_by:** EXT-001 (inferred)
- **files:** `crates/mv-plugin/src/sandbox.rs`
- **acceptance:** `cargo test -p mv-plugin -- file_scope` — a path outside a mounted scope is denied; traversal and symlink escape are denied
- **evidence:** —

#### EXT-006 — Complete runtime resource budgets
- **priority:** P1 — TM-009 gap: "no explicit wall-clock or memory cap observed"
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:94` — "CPU, memory, wall time, output size, concurrency, and action rate are bounded"; `FEDERATION_THREAT_MODEL.md:206`
- **governing_authority:** Law 15 (`:77`)
- **blocked_by:** none
- **files:** `crates/mv-plugin/src/wasm_plugin.rs`
- **acceptance:** `cargo test -p mv-plugin -- resource_budget` — each of the six budgets is independently enforced with a test that trips it
- **evidence:** —

#### EXT-007 — Extensions use only the gateway (no direct canonical DB access)
- **priority:** P1 — Law 3, stated twice
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:85-88` — "Extensions never connect directly to canonical databases. All reads use the query gateway and a Context Grant. All mutations and effects use the command bus, Tool Grant, policy decision, transactional outbox, and action envelope"
- **governing_authority:** Law 3 (`:49`)
- **blocked_by:** IK-001, IK-002 (declared)
- **files:** `crates/mv-plugin/src/sandbox.rs`, `crates/mv-engine/src/engine/mod.rs`
- **acceptance:** `cargo test -p mv-plugin -- gateway_only` — no host call reaches `SqliteNodeStore` without traversing grant admission; a mutation from an extension produces an outbox event and a receipt
- **evidence:** —

#### EXT-008 — Prompt-injection containment: five-way content labeling
- **priority:** P1 — TM-004 and TM-009 mitigation
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:112-122` — the five labels; `:120-122` — "Tool descriptions and model output remain untrusted inputs to authorization"
- **governing_authority:** Law 15 (`:77`)
- **blocked_by:** none
- **files:** `crates/mv-core/src/model/`, `crates/mv-engine/src/llm.rs`, `crates/mv-mcp/src/tools.rs`
- **acceptance:** `cargo test -p mv-engine -- prompt_injection_containment` — imported content carries one of the five labels; content labeled `untrusted embedded instructions` cannot influence an authorization decision
- **evidence:** —

#### EXT-009 — Extension security test suite (7 categories)
- **priority:** P1 — release gate for the whole group
- **status:** not_started
- **mandate:** `EXTENSION_SECURITY_MODEL.md:136-144` — the seven required test categories
- **governing_authority:** feature-completeness §10 (`:125`)
- **blocked_by:** EXT-001..EXT-008 (declared)
- **files:** `crates/mv-plugin/tests/`
- **acceptance:** `cargo test -p mv-plugin` — all seven categories present: manifest/unknown-field fail-closed; signature/digest/downgrade/escalation; network/file/secret/context/action/resource isolation; prompt-injection and confused-deputy fixtures; crash/retry/idempotency at inbox/command/outbox/receipt; uninstall/reinstall without corruption; audit correlation to provider receipt
- **evidence:** —

---

# E. PROTO — protocol adapters

ADR 011 §Implementation order step 4.

#### PROTO-001 — MCP server role: governed server profiles
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:39` — "Add governed server profiles plus client/host manager; retain proposal-only mutation"; `PROTOCOL_BOUNDARIES.md:56-62`
- **governing_authority:** Law 9 (`:64`), Law 7 (`:60`)
- **blocked_by:** IK-001 (declared — "Read access compiles a Context Grant")
- **files:** `crates/mv-mcp/src/server.rs`, `crates/mv-mcp/src/auth.rs`
- **acceptance:** `cargo test -p mv-mcp -- server_profile` — resource reads compile a Context Grant; mutation-capable tools require a separate Tool Grant and emit proposals
- **evidence:** —

#### PROTO-002 — MCP client/host role with pinned identity and egress policy
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:66-76` — six per-server requirements; `:74-76` — "Connecting a server does not grant it access to the caller's context, and listing a tool does not authorize invocation"
- **governing_authority:** Law 9 (`:64`), Law 15 (`:77`)
- **blocked_by:** FED-001 (declared — endpoint policy), EXT-004 (inferred)
- **files:** new `crates/mv-mcp/src/host.rs`, `migrations/027_mcp_connectors.sql` successor
- **acceptance:** `cargo test -p mv-mcp -- host_manager` — each external server is a Context Node with pinned identity, negotiated capabilities, destination restrictions, per-call approval policy, revocation, and health
- **evidence:** —

#### PROTO-003 — A2A adapter with the nine documented mappings
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:82-99` — the mapping table; `:97-99` — "A2A metadata cannot widen grants or bypass policy, and remote task identity is bound through a Source Binding"
- **governing_authority:** Law 9 (`:64`)
- **blocked_by:** SPACE-002 (declared — WorkOrder/AgentRun must exist first)
- **files:** new `crates/mv-engine/src/adapters/a2a.rs`
- **acceptance:** `cargo test -p mv-engine -- a2a_mapping` — all nine mappings round-trip; an Agent Card claiming broader capability than the grant does not widen it
- **evidence:** —

#### PROTO-004 — CloudEvents profile with required MindVault extension attributes
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:106-109` — "MindVault requires additional extension attributes or signed data for actor, principal, Space, sensitivity, causation, correlation, provenance, retention, and trust"; `interoperability-baseline.md:28-29` — "still do not share ... CloudEvents profile"
- **governing_authority:** Law 9 (`:64`), Law 5 (`:54`)
- **blocked_by:** IK-022 (inferred)
- **files:** `crates/mv-core/src/protocol/cloudevents.rs`
- **acceptance:** `cargo test -p mv-core -- cloudevents_profile` — every internal envelope field maps or is explicitly declared as translation loss; no security-relevant field is silently dropped
- **evidence:** —

#### PROTO-005 — AsyncAPI subscription contract
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:104-105` — "AsyncAPI describes supported subscriptions, channels, delivery guarantees, and consumer responsibilities"
- **governing_authority:** Law 9 (`:64`)
- **blocked_by:** PROTO-004 (inferred)
- **files:** new `docs/api/asyncapi.yaml`, `crates/mv-server/src/websocket.rs`
- **acceptance:** the AsyncAPI document validates and enumerates every channel the WebSocket transport actually serves; a channel absent from it is not served
- **evidence:** —

#### PROTO-006 — SCIM provisioning adapter
- **priority:** P2 — protocol map entry with no current implementation
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:27` — "Enterprise provisioning | SCIM | User/group lifecycle adapter"; `:116-118` — "SCIM provisions organization users and groups"
- **governing_authority:** Law 9 (`:64`)
- **blocked_by:** IK-003, SPACE-001 (inferred)
- **files:** new `crates/mv-server/src/rest/scim.rs`
- **acceptance:** `cargo test -p mv-server -- scim` — user/group lifecycle maps to Actors and Space memberships without directly defining grants
- **evidence:** —

#### PROTO-007 — W3C PROV provenance mapping
- **priority:** P2 — required for the portability contract
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:130-138` — Entity/Activity/Agent mapping; `:136-138` — "Export must not discard principal/acting-actor separation, grants, approvals, source versions, or provider receipts"; `DATA_PORTABILITY_CONTRACT.md:78` — "Provenance includes a W3C PROV-compatible representation"
- **governing_authority:** Law 11 (`:69`); feature-completeness §6 (`:121`)
- **blocked_by:** IK-020 (inferred)
- **files:** new `crates/mv-core/src/protocol/prov.rs`, `crates/mv-engine/src/export.rs`
- **acceptance:** `cargo test -p mv-core -- prov_mapping` — a PROV export retains principal/acting-actor separation, grants, approvals, source versions, and receipts
- **evidence:** —

#### PROTO-008 — Protocol negotiation and explicit failure
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:142-148` — "Unknown required capabilities fail before data transfer or action ... Translation loss is surfaced; security-relevant fields are never silently dropped ... Adapter failure cannot commit false canonical success"
- **governing_authority:** Law 14 (`:75`)
- **blocked_by:** IK-022 (inferred)
- **files:** `crates/mv-core/src/protocol/`
- **acceptance:** `cargo test -p mv-core -- protocol_negotiation` — an unknown required capability fails before transfer; a policy-permitted downgrade is recorded; adapter failure never produces a canonical commit
- **evidence:** —

#### PROTO-009 — OpenTelemetry trace/metric/log correlation
- **priority:** P2 — protocol map entry
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:29` — "Telemetry | OpenTelemetry | Trace, metric, and log correlation"
- **governing_authority:** Law 9 (`:64`)
- **blocked_by:** IK-002 (inferred — correlation IDs come from the envelope)
- **files:** `crates/mv-server/src/metrics.rs`, `crates/mv-server/src/lib.rs`
- **acceptance:** `cargo test -p mv-server -- otel_correlation` — the action-envelope correlation ID appears in traces, metrics, and logs for one request
- **evidence:** —

---

# F. SRC — source authority and connectors

ADR 011 §Implementation order step 5. `SOURCE_AUTHORITY_MODEL.md:149-153`
names six gated items plus a cursor-atomicity constraint.

#### SRC-001 — Connector observation ingestion
- **priority:** P1 — named gated work
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:149-151` — "Connector observation ingestion, cursor advancement, external command delivery, conflict execution, deletion enforcement, and uninstall/reinstall fixtures remain gated work"
- **governing_authority:** Law 6 (`:57`)
- **blocked_by:** IK-001 (inferred)
- **files:** `crates/mv-engine/src/adapters/`, `crates/mv-core/src/model/interoperability.rs`
- **acceptance:** `cargo test -p mv-engine -- source_observation` — an observation resolves its active binding, respects materialization mode, and records provenance
- **evidence:** —

#### SRC-002 — Cursor advancement committed atomically with its observation
- **priority:** P1 — an explicit design constraint, not just a missing feature
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:151-153` — "No standalone cursor mutation API exists in this slice; cursor progress must later commit atomically with the governed source observation it acknowledges"
- **governing_authority:** Law 6 (`:57`), Law 4 (`:52`)
- **blocked_by:** SRC-001 (declared)
- **files:** `crates/mv-storage/src/sqlite.rs`, `migrations/033_governed_interoperability_registries.sql` successor
- **acceptance:** `cargo test -p mv-storage -- cursor_atomicity` — no API advances a cursor alone; a failed observation leaves the cursor unmoved (conformance gate `SOURCE_AUTHORITY_MODEL.md:130-131`)
- **evidence:** —

#### SRC-003 — External command delivery through the seven-step mutation routing
- **priority:** P1 — named gated work
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:76-86` — the seven steps; `:87-88` — "A local database update cannot simulate success against an external authority"
- **governing_authority:** Law 6 (`:57`), Law 8 (`:62`)
- **blocked_by:** IK-005 (declared — step 4 enqueues through the outbox)
- **files:** `crates/mv-engine/src/adapters/`, `crates/mv-engine/src/engine/outbox_dispatch.rs`
- **acceptance:** `cargo test -p mv-engine -- external_mutation_routing` — all seven steps occur in order; a provider failure leaves no local success state
- **evidence:** —

#### SRC-004 — Conflict execution and freshness transitions
- **priority:** P1 — named gated work
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:91-98` — "Last-write-wins is not a default conflict policy ... A stale projection may support discovery but cannot silently authorize a consequential action"; `interoperability-baseline.md:47` — "Add connector-driven freshness/materialization transitions"
- **governing_authority:** Law 6 (`:57`)
- **blocked_by:** SRC-001 (inferred)
- **files:** `crates/mv-core/src/model/interoperability.rs`, `crates/mv-engine/src/adapters/`
- **acceptance:** `cargo test -p mv-engine -- source_conflict` — a conflict preserves both observed versions and names the authoritative field owner; a `stale` projection cannot authorize a consequential action
- **evidence:** —

#### SRC-005 — Governed deletion enforcement
- **priority:** P1 — named gated work
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:100-111` — "source deletion may create a tombstone, purge a cache, retain a legal hold, or require review"; `interoperability-baseline.md:47` — "and governed deletion execution"
- **governing_authority:** Law 6 (`:57`), Law 11 (`:69`)
- **blocked_by:** SRC-001 (inferred)
- **files:** `crates/mv-engine/src/adapters/`, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-engine -- source_deletion` — each of the four deletion semantics is selectable per binding; revoked access invalidates unauthorized caches
- **evidence:** —

#### SRC-006 — Uninstall/reinstall fixtures preserving canonical resources
- **priority:** P1 — named gated work and a conformance gate
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:132-133` — "Uninstall and reinstallation preserve canonical resources and explainable provenance"; `:105-107` — "must not corrupt independently canonical derived resources"
- **governing_authority:** Law 6 (`:57`), Law 11 (`:69`)
- **blocked_by:** SRC-005 (inferred)
- **files:** `crates/mv-engine/tests/`
- **acceptance:** `cargo test -p mv-engine -- connector_uninstall_reinstall` — derived knowledge survives uninstall; provenance stays explainable; reinstall does not duplicate bindings
- **evidence:** —

#### SRC-007 — Standardized connector contract (8 verbs) with durable cursors
- **priority:** P2 — required disposition
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:45` — "Standardize discover/read/search/subscribe/import/export/execute/health plus source bindings and durable cursors"; `interoperability-kernel-v1.md:104-107` — "Connector polling, credential access, content materialization, and external execution are outside this slice"
- **governing_authority:** Law 6 (`:57`); feature-completeness §1-3 (`:116-118`)
- **blocked_by:** SRC-001, SRC-002 (inferred)
- **files:** `crates/mv-engine/src/adapters/mod.rs`, `crates/mv-server/src/rest/adapters.rs`
- **acceptance:** `cargo test -p mv-engine -- connector_contract` — every adapter implements all eight verbs or explicitly declares them unsupported; each has a durable cursor
- **evidence:** —

#### SRC-008 — Portable conformance fixtures across connector implementations
- **priority:** P2 — conformance gate
- **status:** not_started
- **mandate:** `SOURCE_AUTHORITY_MODEL.md:133-135` — "Conflict, deletion, stale-source, and authority-transfer fixtures are portable across connector implementations"; `:135` — "No vendor SDK type crosses the connector-to-domain boundary"
- **governing_authority:** Law 1 (`:44`); feature-completeness §10 (`:125`)
- **blocked_by:** SRC-007 (inferred)
- **files:** new `docs/architecture/fixtures/source-authority/`, `crates/mv-engine/tests/`
- **acceptance:** `cargo test -p mv-engine -- source_conformance_fixtures` — the same fixture set passes against at least two adapter implementations; a compile-time or test assertion proves no vendor SDK type crosses into `mv-core`
- **evidence:** —

---

# G. FED — federation

ADR 011 §Implementation order step 6: *"Enable federated nodes only after
threat-model release gates pass."* `FEDERATION_THREAT_MODEL.md:212`: "High items
block production federation." All TM priorities below are copied verbatim.

#### FED-000 — Keep production federation disabled until FED-001..FED-010 are verified
- **priority:** P0 — the governing constraint for this group
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:22` — "Production federation must remain disabled until the high-priority release gates in this document are satisfied"
- **governing_authority:** Law 13 (`:73`); ADR 011 step 6 (`docs/adr/011-sovereign-interoperability-fabric.md:123`)
- **blocked_by:** none
- **files:** `crates/mv-server/src/rest/federation.rs`, `config/default.toml`
- **acceptance:** `cargo test -p mv-server -- federation_disabled_by_default` — federation routes return 501/403 unless an explicit non-default development flag is set, and the flag logs a warning naming this backlog item
- **evidence:** —

#### FED-001 — TM-001 SSRF: endpoint policy, HTTPS, address validation, redirect control
- **priority:** P0 — `FEDERATION_THREAT_MODEL.md:198` priority column: **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:198` (TM-001) — "No HTTPS requirement, URL canonicalization, DNS/IP allow policy, redirect control, rebinding defense, egress allowlist, or admin-only enrollment"
- **governing_authority:** Law 13 (`:73`); `FEDERATION_THREAT_MODEL.md:81` (TB-2)
- **blocked_by:** PROG-003 (declared)
- **files:** `crates/mv-engine/src/federation.rs`, `crates/mv-server/src/rest/federation.rs`
- **acceptance:** `cargo test -p mv-engine -- federation_endpoint_policy` — the focus-path evidence at `FEDERATION_THREAT_MODEL.md:222`: schemes, IP classes, DNS rebinding, redirects, proxies, IPv4/IPv6, metadata endpoints, egress failure
- **evidence:** —

#### FED-002 — TM-002 node identity: signed descriptor, nonce challenge, proof of possession
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:199` (TM-002) — "No proof of key possession, signed descriptor, trust anchor, certificate binding, fingerprint confirmation, rotation, revocation, or quarantine state"
- **governing_authority:** Law 13 (`:73`); `CONTEXT_NODE_MODEL.md:99-111` (7-step trust establishment)
- **blocked_by:** FED-001 (inferred)
- **files:** `crates/mv-engine/src/federation.rs`, `crates/mv-core/src/model/interoperability.rs`
- **acceptance:** `cargo test -p mv-engine -- node_identity_proof` — evidence at `FEDERATION_THREAT_MODEL.md:223`: signed descriptor, proof-of-possession handshake, trust approval, key rotation/revocation, identity-change audit. Also satisfies `CONTEXT_NODE_MODEL.md:173` "Node identity survives endpoint rotation"
- **evidence:** —

#### FED-003 — TM-003 query minimization and namespace intersection
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:200` (TM-003) — "Full query fans out to all enabled peers; zero or multiple allowed namespaces produce no namespace in the request; no per-peer purpose, data-class, audience, field, or retention grant"
- **governing_authority:** Law 7 (`:60`), Law 13 (`:73`)
- **blocked_by:** IK-001 (declared — needs per-peer Context Grants)
- **files:** `crates/mv-engine/src/federation.rs`
- **acceptance:** `cargo test -p mv-engine -- query_minimization` — evidence at `FEDERATION_THREAT_MODEL.md:225`: zero/one/many namespaces, purpose and data classes, per-peer redaction, user disclosure, **denial on ambiguity**
- **evidence:** —

#### FED-004 — TM-004 remote response validation and signed Context Capsules
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:201` (TM-004) — "No signed response envelope, response-byte cap, provenance verification, trust class, schema negotiation, content separation, or score normalization"; `CONTEXT_NODE_MODEL.md:143-157` (capsule contract)
- **governing_authority:** Law 13 (`:73`), Law 14 (`:75`)
- **blocked_by:** FED-002, IK-013 (declared)
- **files:** `crates/mv-engine/src/federation.rs`, `crates/mv-core/src/model/interoperability.rs`
- **acceptance:** `cargo test -p mv-engine -- context_capsule` — evidence at `FEDERATION_THREAT_MODEL.md:226`; a capsule carries objective, recipient, source authority, freshness, permitted operations, model-use restrictions, expiration, redistribution rules, revocation handle, and issuer signature; remote content renders as untrusted data and requires explicit promotion before persistence
- **evidence:** —

#### FED-005 — TM-005 federation authentication middleware with replay defense
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:202` (TM-005) — "Verifier is only referenced by tests; outbound HMAC is optional; no nonce/request ID cache; ordinary REST auth does not consume the HMAC headers"
- **governing_authority:** Law 13 (`:73`), Law 14 (`:75`)
- **blocked_by:** FED-002 (declared)
- **files:** `crates/mv-server/src/rest/federation.rs`, `crates/mv-server/src/auth.rs`, `crates/mv-engine/src/federation.rs`
- **acceptance:** `cargo test -p mv-server -- federation_auth_middleware` — evidence at `FEDERATION_THREAT_MODEL.md:224`: end-to-end signed request proving audience, grant, nonce, expiry, body digest, replay rejection, fail-closed
- **evidence:** —

#### FED-006 — TM-006 resource-exhaustion budgets
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:203` (TM-006) — "No response-byte/depth cap, global federation concurrency budget, per-peer circuit breaker, max peer fan-out, streaming parser, cancellation budget, or durable backoff"
- **governing_authority:** Law 13 (`:73`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/federation.rs`
- **acceptance:** `cargo test -p mv-engine -- federation_budgets` — evidence at `FEDERATION_THREAT_MODEL.md:230`: fan-out, compressed and oversized responses, slow peers, parser depth, circuit breakers, quotas, cancellation
- **evidence:** —

#### FED-007 — TM-007 asymmetric keys / secret broker for peer credentials
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:204` (TM-007) — "Secret is accepted in JSON and stored as `Option<String>` in process memory; no broker handle, envelope encryption, per-direction key, automated rotation, or explicit redaction contract"
- **governing_authority:** Law 13 (`:73`), Law 15 (`:77`)
- **blocked_by:** EXT-003 (inferred — shares the broker)
- **files:** `crates/mv-engine/src/federation.rs`, `crates/mv-engine/src/keychain.rs`, `crates/mv-server/src/rest/federation.rs`
- **acceptance:** `cargo test -p mv-engine -- federation_secrets` — no `Option<String>` secret in DTOs; inbound/outbound and per-audience keys are separate; rotation and revocation tested; DTO/log/trace redaction asserted
- **evidence:** —

#### FED-008 — TM-008 durable peer registry, mandatory audit, revocation propagation
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:205` (TM-008) — "Audit defaults off; in-memory peer registry and bounded audit buffer; no mandatory federation event taxonomy, durable Trust Ledger, signed receipt, or revocation acknowledgement"
- **governing_authority:** Law 13 (`:73`); feature-completeness §7 (`:122`)
- **blocked_by:** IK-020 (declared — the Trust Ledger)
- **files:** new migration, `crates/mv-engine/src/federation.rs`, `crates/mv-server/src/audit.rs`
- **acceptance:** `cargo test -p mv-server -- federation_durable_state` — evidence at `FEDERATION_THREAT_MODEL.md:227`: durable state migration, mandatory audit, append-only decision/receipt records, crash recovery, revocation propagation; startup fails closed if state cannot load
- **evidence:** —

#### FED-009 — TM-009 extension confused-deputy across the federation boundary
- **priority:** P0 — **High**
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:206` (TM-009); `:76` — "They must not use federation as a confused deputy to obtain network, data, or mutation authority they were not explicitly granted"
- **governing_authority:** Law 15 (`:77`), Law 3 (`:49`)
- **blocked_by:** EXT-004, EXT-006 (declared)
- **files:** `crates/mv-plugin/src/sandbox.rs`, `crates/mv-engine/src/federation.rs`
- **acceptance:** `cargo test -p mv-plugin -- federation_confused_deputy` — evidence at `FEDERATION_THREAT_MODEL.md:228`: signed package identity, Tool Grant propagation, destination-scoped network access, secret broker isolation, adversarial plugin tests
- **evidence:** —

#### FED-010 — TM-010 event sync convergence
- **priority:** P0 — `FEDERATION_THREAT_MODEL.md:207` priority column: **"High release blocker"** (the only item labeled this)
- **status:** not_started
- **mandate:** `FEDERATION_THREAT_MODEL.md:207` (TM-010) — "No implemented signed event envelope, durable outbox/inbox, sequence/replay state, idempotency key, authority check, conflict ledger, or convergence test"
- **governing_authority:** Law 4 (`:52`), Law 6 (`:57`), Law 13 (`:73`)
- **blocked_by:** IK-004, IK-006, IK-013, IK-015, SRC-004 (declared)
- **files:** `crates/mv-engine/src/engine/inbox_consumer.rs`, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-engine -- multi_node_convergence` — evidence at `FEDERATION_THREAT_MODEL.md:229`: transactional outbox/inbox, idempotency, signatures, tombstones, conflict quarantine, reconciliation, multi-node convergence
- **evidence:** —

#### FED-011 — Context Node public discovery and federated query planner
- **priority:** P1 — named a release gate
- **status:** not_started
- **mandate:** `CONTEXT_NODE_MODEL.md:190-192` — "public discovery, remote key proof, signature verification, federated query, and action-envelope enforcement remain release gates"; `:126-141` (federated query pipeline)
- **governing_authority:** Law 13 (`:73`)
- **blocked_by:** FED-002, FED-003, FED-004 (declared)
- **files:** `crates/mv-engine/src/federation.rs`, `crates/mv-server/src/rest/federation.rs`
- **acceptance:** `cargo test -p mv-engine -- federated_query_planner` — the planner enforces budget, timeout, sensitivity, residency, and minimum-trust policy; remote results are evidence, never automatically canonical
- **evidence:** —

#### FED-012 — Context Node conformance gates (6)
- **priority:** P1 — release gate for the node model
- **status:** not_started
- **mandate:** `CONTEXT_NODE_MODEL.md:171-179` — six gates
- **governing_authority:** Law 13 (`:73`); feature-completeness §10 (`:125`)
- **blocked_by:** FED-002, FED-011 (declared)
- **files:** `crates/mv-engine/tests/`
- **acceptance:** `cargo test -p mv-engine -- context_node_conformance` — identity survives endpoint rotation; malicious discovery cannot select private/link-local targets; capability advertisement cannot self-authorize; revocation terminates within the documented bound; cross-node results retain source/freshness/schema/signature status; offline queues replay idempotently
- **evidence:** —

---

# H. PORT — portability and conformance

#### PORT-001 — Signed portable bundle manifest (19 fields)
- **priority:** P1 — required for every machine-portable or recovery export
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:31-54` — "Every machine-portable or recovery bundle contains a signed manifest"
- **governing_authority:** Law 11 (`:69`); feature-completeness §8 (`:123`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/export.rs`, `crates/mv-core/src/model/`
- **acceptance:** `cargo test -p mv-engine -- portable_manifest` — a bundle missing any of the 19 fields is invalid; the manifest explains every material omission (`:69-70`)
- **evidence:** —

#### PORT-002 — Exported resources retain identity, authority, and provenance
- **priority:** P1
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:56-67` — 10 retention requirements; `:81` — "No exported identity depends solely on an internal database row number"
- **governing_authority:** Law 11 (`:69`)
- **blocked_by:** PORT-001 (inferred)
- **files:** `crates/mv-engine/src/export.rs`
- **acceptance:** `cargo test -p mv-engine -- export_retention` — all 10 requirements asserted; secrets, session tokens, raw key material, and non-exportable credentials are excluded (`:68-69`)
- **evidence:** —

#### PORT-003 — Nine-step staged import
- **priority:** P1
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:86-95` — the nine steps; `:97-99` — "Import never silently broadens grants, activates agents, installs extensions, restores credentials, or triggers automations"
- **governing_authority:** Law 11 (`:69`), Law 15 (`:77`)
- **blocked_by:** PORT-001 (declared)
- **files:** `crates/mv-engine/src/export.rs`, `crates/mv-server/src/rest/sync.rs`
- **acceptance:** `cargo test -p mv-engine -- staged_import` — all nine steps in order; unknown schemas become opaque source artifacts, never canonical executable objects (`:97-98`)
- **evidence:** —

#### PORT-004 — Round-trip guarantees (6)
- **priority:** P1
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:103-111` — six guarantees
- **governing_authority:** Law 11 (`:69`)
- **blocked_by:** PORT-003 (inferred)
- **files:** `crates/mv-engine/tests/`
- **acceptance:** `cargo test -p mv-engine -- portability_round_trip` — identity/content/relationships/provenance/authority preserved; projections rebuildable; repeated import idempotent; conflicting ownership requires resolution; failed import leaves the target unchanged; partial exports cannot claim completeness
- **evidence:** —

#### PORT-005 — Deletion and exit contract (6 obligations)
- **priority:** P1
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:115-124` — six obligations including "source disconnection without deletion of independent derived knowledge"
- **governing_authority:** Law 11 (`:69`), Law 12 (`:71`)
- **blocked_by:** SRC-005 (inferred)
- **files:** `crates/mv-engine/src/export.rs`, `crates/mv-server/src/rest/`
- **acceptance:** `cargo test -p mv-engine -- deletion_exit` — all six obligations; a deletion receipt describes deleted, retained, externally owned, and unreachable data
- **evidence:** —

#### PORT-006 — Portability conformance suite (7 categories)
- **priority:** P1 — "Portability releases require"
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:126-138` — the seven categories
- **governing_authority:** feature-completeness §10 (`:125`)
- **blocked_by:** PORT-001..PORT-005 (declared)
- **files:** new `docs/architecture/fixtures/portability/`, `crates/mv-engine/tests/`
- **acceptance:** `cargo test -p mv-engine -- portability_conformance` — golden fixtures per public schema version; deterministic manifests and checksums; corrupted/truncated/oversized/malicious-path/decompression-bomb tests; cross-version migration and downgrade-loss reports; Personal Vault, Space, source-binding, WorkOrder/AgentRun, Trust Ledger, and Context Capsule round trips; **restore without network access**
- **evidence:** —

#### PORT-007 — Independent non-product validator for the open bundle format
- **priority:** P2 — required by the conformance suite but externally scoped
- **status:** not_started
- **mandate:** `DATA_PORTABILITY_CONTRACT.md:137-138` — "independent parser documentation and at least one non-product validation implementation for the open bundle format"
- **governing_authority:** Law 11 (`:69`); `INTEROPERABILITY_CONSTITUTION.md:130-137` (open interoperability layer)
- **blocked_by:** PORT-006 (declared)
- **files:** new `python/` or `scripts/` validator, `docs/api/`
- **acceptance:** a validator outside the Rust workspace validates the golden fixtures using only published documentation
- **evidence:** —

---

# I. SLICE — end-to-end vertical slice

ADR 011 §Implementation order step 7. `docs/adr/011-sovereign-interoperability-fabric.md:126-127`:
*"Empty crate scaffolding, connector quantity, or UI prototypes do not count as progress against these gates."*

#### SLICE-001 — Interoperability conformance fixtures
- **priority:** P1 — release gate before the end-to-end proof
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:140-141` — "admission enforcement, and conformance fixtures remain required before the end-to-end proof"
- **governing_authority:** feature-completeness §10 (`:125`)
- **blocked_by:** IK-001, IK-002, IK-009 (declared)
- **files:** new `docs/architecture/fixtures/interoperability/`, `crates/mv-server/tests/`
- **acceptance:** `cargo test --workspace -- interop_conformance` — fixtures exist for every registered public schema version, grant kind, materialization mode, and delivery outcome
- **evidence:** —

#### SLICE-002 — External-meeting-to-team-report proof chain
- **priority:** P1 — the constitution's named first proof
- **status:** not_started
- **mandate:** `INTEROPERABILITY_CONSTITUTION.md:155-164` — the 8-stage chain; `:166-167` — "Vendor-specific UI, marketplace scale, unrestricted automation, and broad connector quantity do not unlock this slice. Contract conformance does"
- **governing_authority:** `INTEROPERABILITY_CONSTITUTION.md:139-167`
- **blocked_by:** SLICE-001, IK-005, IK-006, PROTO-001, PROTO-003, SPACE-002 (declared)
- **files:** `crates/mv-server/tests/`, `crates/mv-engine/tests/`
- **acceptance:** one end-to-end test walks all 8 stages: external meeting source → governed ingestion → candidate decisions/tasks → human-approved canonical context → retrieval from two independent MCP hosts → Work Order execution by an A2A-adapted agent → verified artifact and team report → approved delivery with a complete action receipt
- **evidence:** —

#### SLICE-003 — Eight acceptance conditions for the first proof
- **priority:** P1 — the acceptance criteria for SLICE-002
- **status:** not_started
- **mandate:** `docs/architecture/interoperability-baseline.md:152-159` — eight bullets; `interoperability-kernel-v1.md:284-285` — "End-to-end completion may be claimed only when authenticated delivery can be joined to the independently queryable consumer application receipt and checkpoint"
- **governing_authority:** `INTEROPERABILITY_CONSTITUTION.md:112-128`
- **blocked_by:** SLICE-002 (declared)
- **files:** `crates/mv-server/tests/`
- **acceptance:** each of the eight is a named assertion — no vendor-specific type in the core; duplicate delivery creates no duplicate canonical action; every result retains source authority and provenance; candidate knowledge requires explicit or policy-approved promotion; two independent MCP hosts retrieve only granted context; two independent A2A-adapted agents execute bounded work; external delivery uses an outbox and complete action receipt; export/restore and connector uninstall preserve canonical integrity
- **evidence:** —

---

# J. WS — knowledge workspace

Stage 1 is implemented (`knowledge-workspace.md:398-400`). Stage 2 onward is
open, and migration 031 shipped schema for five tables that nothing reads.

## J.1 Dead schema

#### WS-001 — `workspace_events` has no reader or writer
- **priority:** P0 — Law 4 requires durable events for workspace mutations; the table exists to satisfy it and is empty of code
- **status:** not_started
- **mandate:** `migrations/031_knowledge_workspace_manifest.sql:70` creates the table; the only Rust reference is a table-existence assertion at `crates/mv-storage/src/sqlite.rs:9980`; `knowledge-workspace-baseline.md:41` — "add mandatory durable workspace mutation journal"
- **governing_authority:** Law 4 (`:52`); ADR 009 §Costs (`docs/adr/009-workspace-identity-and-mutation-boundary.md:211-212`) — "Existing audit and node-version implementations are insufficient for the durable workspace journal"
- **blocked_by:** none
- **files:** `crates/mv-core/src/traits.rs`, `crates/mv-storage/src/sqlite.rs`, `crates/mv-engine/src/engine/workspace_ops.rs`
- **acceptance:** `cargo test -p mv-storage -- workspace_event_journal` — every mount, reconcile, projection, and (later) mutation writes a journal row with correlation ID and before/after hashes; the row survives restart
- **evidence:** —

#### WS-002 — `workspace_document_versions` has no reader or writer
- **priority:** P0 — canonical rollback depends on it
- **status:** not_started
- **mandate:** `migrations/031_knowledge_workspace_manifest.sql:127`; only reference is `crates/mv-storage/src/sqlite.rs:9981`; `knowledge-workspace-document-contract.md:130` — "Every successful MindVault write records the before-image"; `:140` — "This replaces the current node-local 40-version cap"
- **governing_authority:** ADR 009 §Costs (`:211-212`); feature-completeness §6 (`:121`)
- **blocked_by:** WS-006 (inferred — before-images need a write path)
- **files:** `crates/mv-core/src/traits.rs`, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-storage -- document_versions` — content-addressed snapshots deduplicate by hash; pinned evidence cannot be pruned (`document-contract:136-137`); sealed mode encrypts snapshot content (`:138`)
- **evidence:** —

#### WS-003 — `workspace_conflicts` has no reader or writer, and `GET .../conflicts` is unrouted
- **priority:** P0 — expected-hash conflicts are the core write-safety mechanism
- **status:** not_started
- **mandate:** `migrations/031_knowledge_workspace_manifest.sql:148`; only reference is `crates/mv-storage/src/sqlite.rs:9982`; ADR 009:155 specifies `GET /api/v1/workspaces/{workspace_id}/conflicts`, which is not among the 5 routes mounted at `crates/mv-server/src/rest.rs:171-188`
- **governing_authority:** ADR 009 §Workspace API shape (`:137-155`); ADR 009 gate 4 (`:221`)
- **blocked_by:** none
- **files:** `crates/mv-storage/src/sqlite.rs`, `crates/mv-server/src/rest/workspaces.rs`, `crates/mv-server/src/rest.rs`
- **acceptance:** `cargo test -p mv-server -- workspace_conflicts` — an expected-hash mismatch creates a conflict row and leaves canonical bytes unchanged; the route lists open conflicts with the four documented resolutions (`document-contract:169-171`)
- **evidence:** —

#### WS-004 — `workspace_migrations` has no reader or writer
- **priority:** P1 — Stage 3 depends on it
- **status:** not_started
- **mandate:** `migrations/031_knowledge_workspace_manifest.sql:179`; only reference is `crates/mv-storage/src/sqlite.rs:9983`; `knowledge-workspace-migration.md:307-332` (dry run / apply / commit and rollback)
- **governing_authority:** ADR 008 §Decision; `knowledge-workspace.md:434-442` (Stage 3)
- **blocked_by:** WS-006 (declared — migration writes files)
- **files:** `crates/mv-core/src/traits.rs`, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-storage -- workspace_migration_state` — a migration run persists state across restart and supports checkpoint + rollback
- **evidence:** —

#### WS-005 — `workspace_migration_items` has no reader or writer
- **priority:** P1 — per-item migration state and the report schema depend on it
- **status:** not_started
- **mandate:** `migrations/031_knowledge_workspace_manifest.sql:208`; only reference is `crates/mv-storage/src/sqlite.rs:9984`
- **governing_authority:** `knowledge-workspace.md:441-442` — "content, attachments, links, tags, IDs, and counts meet the migration acceptance report"
- **blocked_by:** WS-004 (declared)
- **files:** `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-storage -- workspace_migration_items` — per-item state transitions are persisted and produce a report matching `docs/architecture/fixtures/knowledge-workspace/migration-report.schema.json`
- **evidence:** —

> Note on `crates/mv-storage/src/sqlite.rs:9973-9985`
> (`knowledge_workspace_migration_installs_complete_manifest_contract`): this test
> asserts only that eight tables exist. It is the reason five dead tables passed
> review. Once WS-001..WS-005 are verified it should be replaced by behavioural
> tests; until then it must not be cited as coverage.

## J.2 Stage 2 — guarded native writes

`knowledge-workspace.md:399-400`: *"Stage 2 editor and guarded file-mutation
operations remain deliberately out of scope for this foundation."*

#### WS-006 — Seven missing Stage 2 REST routes
- **priority:** P0 — Stage 2 is the declared next stage and no route exists
- **status:** not_started
- **mandate:** `docs/adr/009-workspace-identity-and-mutation-boundary.md:144-155` specifies `POST folders`, `POST documents`, `PUT documents` (with required `expected_content_hash`), `operations/move`, `operations/trash`, `operations/restore`, `GET conflicts`; `crates/mv-server/src/rest.rs:171-188` mounts only 5 read/reconcile routes
- **governing_authority:** ADR 009 §Workspace API shape; feature-completeness §2 (`:117`)
- **blocked_by:** WS-010 (declared — the six writable-behaviour gates)
- **files:** `crates/mv-server/src/rest/workspaces.rs`, `crates/mv-server/src/rest.rs`, `crates/mv-engine/src/engine/workspace_ops.rs`, `crates/mv-server/src/openapi.rs`
- **acceptance:** `cargo test -p mv-server -- workspace_write_routes` — all seven exist, appear in OpenAPI, and return the documented mutation response (stable ID, normalized path, resulting content hash, projection state, journal correlation ID, conflict details) per ADR 009:161-163
- **evidence:** —

#### WS-007 — All mutations route through `WorkspaceService`; no adapter writes arbitrary paths
- **priority:** P0 — ADR 009 conformance gate 3
- **status:** not_started
- **mandate:** `docs/adr/009-workspace-identity-and-mutation-boundary.md:111-113` — "UI, REST, MCP, plugins, automations, and AI may request operations but may not write arbitrary workspace paths"; gate 3 at `:220`
- **governing_authority:** ADR 009 §Mutation service; Law 3 (`:49`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/workspace.rs`, `crates/mv-plugin/src/sandbox.rs`, `crates/mv-mcp/src/tools.rs`
- **acceptance:** `cargo test --workspace -- workspace_single_write_path` — no code path outside `WorkspaceService` performs a canonical file write; MCP and plugin path arguments are rejected
- **evidence:** —

#### WS-008 — Recoverable trash (5-step prepared-delete protocol)
- **priority:** P1 — Stage 2 exit criterion
- **status:** not_started
- **mandate:** `knowledge-workspace-document-contract.md:144-154` — the five steps; `:156-157` — "Operating-system trash integration can be added later but cannot replace the managed recovery proof"
- **governing_authority:** ADR 009 §Workspace API shape (`:152-153`)
- **blocked_by:** WS-002, WS-006 (declared)
- **files:** `crates/mv-engine/src/workspace.rs`, `crates/mv-server/src/rest/workspaces.rs`
- **acceptance:** `cargo test -p mv-engine -- workspace_trash` — interruption at each of the five steps recovers; restore uses the same expected-hash and path policy; an occupied original path requires explicit resolution
- **evidence:** —

#### WS-009 — Prepare/apply/finalize crash recovery
- **priority:** P0 — writable unlock gate 4
- **status:** not_started
- **mandate:** `knowledge-workspace-migration.md:281` — "[ ] prepare/apply/finalize recovery converges after interruption"; `knowledge-workspace.md:519` — "crash/interruption tests around canonical writes and manifest updates"
- **governing_authority:** ADR 009 gate 1 (`:218`)
- **blocked_by:** WS-006 (inferred)
- **files:** `crates/mv-engine/src/workspace.rs`, `crates/mv-storage/src/sqlite.rs`
- **acceptance:** `cargo test -p mv-engine -- workspace_crash_recovery` — interruption between file write and manifest commit converges to a consistent state with no silent overwrite
- **evidence:** —

#### WS-010 — The six writable-behaviour unlock gates
- **priority:** P0 — the explicit gate set for enabling any workspace writer
- **status:** not_started
- **mandate:** `knowledge-workspace-migration.md:275-283` — "Writable behavior may be enabled for an adapter only when executable tests prove:" followed by six unchecked boxes
- **governing_authority:** ADR 009 §Implementation conformance gates (`:214-223`)
- **blocked_by:** WS-007, WS-009, WS-012, WS-013 (declared)
- **files:** `crates/mv-engine/tests/`, `crates/mv-server/tests/api_integration.rs`
- **acceptance:** `cargo test --workspace -- workspace_writable_gates` — all six pass: generic node mutation fails closed for projected documents; all path-capable adapters share path and authorization policy; stale expected hashes leave canonical bytes unchanged; prepare/apply/finalize recovery converges; identity-aware backup and restore preserve document IDs; the adapter preserves the approved Markdown and migration fixtures
- **evidence:** —

#### WS-011 — Stage 2 exit: no tested failure mode causes silent overwrite or root escape
- **priority:** P1 — Stage 2 exit criterion
- **status:** not_started
- **mandate:** `knowledge-workspace.md:431-432` — "**Exit:** no tested failure mode causes a silent overwrite or a workspace-root escape"
- **governing_authority:** ADR 009 gate 4 (`:221`)
- **blocked_by:** WS-006..WS-010 (declared)
- **files:** `crates/mv-engine/tests/`, `crates/mv-core/src/workspace_path.rs`
- **acceptance:** `cargo test --workspace -- workspace_stage2_exit` — property tests for path containment and rename/move invariants (`knowledge-workspace.md:516`) plus symlink/traversal/excluded-path security tests (`:522-523`)
- **evidence:** —

#### WS-012 — Identity-aware backup and restore
- **priority:** P0 — writable unlock gate 5
- **status:** not_started
- **mandate:** `knowledge-workspace-migration.md:282` — "[ ] identity-aware backup and restore preserve document IDs"; `docs/adr/009-workspace-identity-and-mutation-boundary.md:189-190` — "The existing database-only backup and data-directory archive are foundations, not sufficient implementations"
- **governing_authority:** ADR 009 §Backup; Law 11 (`:69`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/backup.rs`, `crates/mv-cli/src/commands/backup.rs`
- **acceptance:** `cargo test -p mv-engine -- identity_aware_backup` — a full round trip preserves document UUIDs, manifest state, relationships, versions, and the audit checkpoint across a filesystem snapshot plus DB checkpoint
- **evidence:** —

#### WS-013 — Lossless Markdown parse/render fixture enforcement
- **priority:** P0 — writable unlock gate 6; baseline blocker "Lossy Markdown parsing"
- **status:** not_started
- **mandate:** `knowledge-workspace-migration.md:283` — "[ ] the adapter preserves the approved Markdown and migration fixtures"; `knowledge-workspace-baseline.md:91` — "Lossless parse/render fixtures pass without semantic edits"; `knowledge-workspace.md:347-348` — "Obsidian-specific syntax that MindVault does not understand must be preserved losslessly"
- **governing_authority:** ADR 008 §Decision; ADR 009 gate 4 (`:221`)
- **blocked_by:** WS-014 (declared — the fixture must be wired first)
- **files:** `crates/mv-engine/src/import/obsidian.rs`, `crates/mv-core/src/`, `docs/architecture/fixtures/knowledge-workspace/markdown-preservation.md`
- **acceptance:** `cargo test -p mv-engine -- markdown_preservation` — the fixture round-trips byte-identically
- **evidence:** —

## J.3 Dead fixtures

#### WS-014 — Six approved fixtures have zero consuming test
- **priority:** P0 — same defect class as the five dead tables; the fixtures are marked approved
- **status:** not_started
- **mandate:** `knowledge-workspace-document-contract.md:185-186` — "Any parser, migration, or write-path implementation must run these fixtures without modifying their expected artifacts"; `knowledge-workspace-migration.md:262,267` mark them `[x]` approved. Verified: only `portable-path-cases.json` is consumed (`crates/mv-core/src/workspace_path.rs:328`). Unconsumed: `markdown-preservation.md`, `legacy-nodes.json`, `expected-migration-plan.json`, `expected-migration-report.json`, `migration-report.schema.json`, `migration-expected/**`
- **governing_authority:** feature-completeness §10 (`:125`)
- **blocked_by:** none
- **files:** `crates/mv-engine/tests/`, `docs/architecture/fixtures/knowledge-workspace/`
- **acceptance:** `cargo test --workspace -- workspace_fixtures` — every file under `docs/architecture/fixtures/knowledge-workspace/` is loaded by at least one test; a guard test fails if a fixture file is added without a consumer
- **evidence:** —

#### WS-015 — Filesystem platform fixtures for macOS, Linux, Windows
- **priority:** P1 — required validation
- **status:** not_started
- **mandate:** `knowledge-workspace.md:518` — "platform fixtures for macOS, Linux, and Windows path behavior"; `:421-422` — "**Exit:** external creates, edits, moves, and deletes converge correctly across the supported filesystem matrix"
- **governing_authority:** ADR 008 §Decision; `knowledge-workspace-document-contract.md:21` (portable profile)
- **blocked_by:** none
- **files:** `crates/mv-core/src/workspace_path.rs`, `.github/workflows/`
- **acceptance:** `cargo test -p mv-core -- platform_path_matrix` runs in CI on all three platforms; case-collision and Unicode-normalization behaviour is asserted per platform
- **evidence:** —

## J.4 Test coverage

#### WS-016 — Branch and error-path coverage for the workspace REST and engine modules
- **priority:** P1 — derived: 513 lines with zero inline tests behind 3 happy-path integration tests
- **status:** not_started
- **mandate:** `knowledge-workspace.md:513-525` — "Implementation must include: unit tests for normalization, collision, identity, and state transitions". Verified: `crates/mv-server/src/rest/workspaces.rs` (279 lines) and `crates/mv-engine/src/engine/workspace_ops.rs` (234 lines) contain **zero** `#[test]`/`#[tokio::test]`. Coverage exists only via 3 integration tests at `crates/mv-server/tests/api_integration.rs:116`, `:233`, `:425`, all happy-path or single-fail-closed
- **governing_authority:** feature-completeness §10 (`:125`)
- **blocked_by:** none
- **files:** `crates/mv-server/src/rest/workspaces.rs`, `crates/mv-engine/src/engine/workspace_ops.rs`
- **acceptance:** `cargo test -p mv-server -- workspaces::` and `cargo test -p mv-engine -- workspace_ops::` — inline tests cover sealed-mode mount rejection, non-allowlisted root, malformed workspace/document UUID, cross-workspace document read, non-UTF-8 document read, reconcile on a deleted root, rebuild on a failed projection, and `mark_initial_workspace_error`
- **evidence:** —

#### WS-017 — Watcher-loss and full-reconciliation tests
- **priority:** P1 — required validation; foundation gate step 6
- **status:** not_started
- **mandate:** `knowledge-workspace.md:520` — "watcher-loss and full-reconciliation tests"; `:487` — "recover correct state after watcher interruption through reconciliation"
- **governing_authority:** ADR 008 §Decision
- **blocked_by:** none
- **files:** `crates/mv-server/src/workspace_watch.rs` (686 lines, 7 tests), `crates/mv-server/tests/`
- **acceptance:** `cargo test -p mv-server -- watcher_loss` — dropped and overflowed native events converge via the periodic full-scan fallback; a workspace mounted while the watcher is down is rediscovered
- **evidence:** —

#### WS-018 — Performance baselines before numeric budgets
- **priority:** P2 — required validation, explicitly ordered after measurement
- **status:** not_started
- **mandate:** `knowledge-workspace.md:524-528` — "performance baselines for initial scan, incremental reconciliation, tree navigation, and search at agreed workspace sizes. Performance targets must be recorded from measured baselines before numeric budgets are committed"
- **governing_authority:** ADR 008 §Decision
- **blocked_by:** none
- **files:** `crates/mv-engine/benches/`
- **acceptance:** `cargo bench -p mv-engine -- workspace` records baselines at agreed sizes; no numeric budget is committed to any doc before the baseline exists
- **evidence:** —

## J.5 Stage 3-5 and residual blockers

#### WS-019 — Stage 3 Notes migration
- **priority:** P1 — staged delivery
- **status:** not_started
- **mandate:** `knowledge-workspace.md:434-442` — "dry-run current database notes; stage, verify, and reconcile a generated workspace; switch authority only after explicit confirmation; prove rollback"; `:509` — "Neither gate authorizes migration or deletion of current Notes data"
- **governing_authority:** ADR 008 §Decision
- **blocked_by:** WS-004, WS-005, WS-010 (declared)
- **files:** `crates/mv-engine/src/import/`, `crates/mv-cli/src/commands/`
- **acceptance:** `cargo test -p mv-engine -- notes_migration` — dry-run fails the gate on the conditions at `knowledge-workspace-migration.md:125`; commit requires explicit confirmation; rollback is proven
- **evidence:** —

#### WS-020 — Resolve the portability API contract mismatch
- **priority:** P1 — baseline blocker; blocks the migration UI
- **status:** not_started
- **mandate:** `knowledge-workspace-baseline.md:49` — "Client expects asynchronous job-style `POST /api/v1/export`, upload import, and status routes ... Existing contract mismatch must be resolved before using this UI for migration"; `:96` — "One tested server/client schema replaces both assumptions"
- **governing_authority:** Law 2 (`:46`), Law 11 (`:69`)
- **blocked_by:** none
- **files:** `crates/mv-server/src/rest.rs` (export/import), `frontend/src/lib/api/portability.ts`
- **acceptance:** `cargo test -p mv-server -- portability_contract` and `pnpm --dir frontend test portability` share one schema; the frontend no longer assumes routes the server does not serve
- **evidence:** —

#### WS-021 — Plaintext vs sealed-mode boundary in UI copy and security docs
- **priority:** P1 — baseline blocker; ADR 009 conformance gate 6
- **status:** not_started
- **mandate:** `knowledge-workspace-baseline.md:50` — "Documentation states all persisted vault artifacts are encrypted in sealed mode ... Add a clear exception/policy before plaintext Markdown workspaces ship"; `docs/adr/009-workspace-identity-and-mutation-boundary.md:170-174` and gate 6 at `:223` — "the UI accurately communicates plaintext and sealed-store boundaries"
- **governing_authority:** ADR 009 §Portable versus sealed storage; ADR 004
- **blocked_by:** none
- **files:** `docs/security.md`, `frontend/src/lib/components/LibraryWorkspaceBrowser.svelte`, `frontend/src/routes/notes/+page.svelte`
- **acceptance:** `pnpm --dir frontend test LibraryWorkspaceBrowser` asserts the plaintext notice renders for every mounted workspace; `docs/security.md` states the exception explicitly
- **evidence:** —

#### WS-022 — Honest disposition for all ten baseline blockers
- **priority:** P1 — several are closed by Stage 1 but the table still lists all ten as open
- **status:** not_started
- **mandate:** `knowledge-workspace-baseline.md:86-97` (ten rows); `:108-109` — "The blockers above remain runtime conformance gates"
- **governing_authority:** ADR 008, ADR 009
- **blocked_by:** none
- **files:** `docs/architecture/knowledge-workspace-baseline.md`
- **acceptance:** each of the ten rows carries either a backlog ID or the test name that closed it; the "No canonical ownership enforcement" row cites `crates/mv-server/tests/api_integration.rs:233` (`workspace_projections_are_searchable_linked_rebuildable_and_generic_mutation_safe`)
- **evidence:** —

---

# K. SPACE — collaborative spaces (ADR 010)

#### SPACE-001 — Actor/Workspace/Space/Membership schema and authorization matrix
- **priority:** P0 — `collaborative-spaces-baseline.md:65` labels it P0
- **status:** not_started
- **mandate:** `docs/architecture/collaborative-spaces-baseline.md:72-79` — "foreign-keyed Actor, Workspace, Space, Membership, and resource ownership schemas; deny-by-default authorization matrix tests for every actor kind; cross-Space read, write, subscription, search, and artifact isolation tests"
- **governing_authority:** ADR 010 §Membership and authorization (`docs/adr/010-personal-vault-and-collaborative-spaces.md:102-119`); ADR 010 acceptance gate 3 (`:263-264`)
- **blocked_by:** IK-003 (inferred)
- **files:** new migration, `crates/mv-core/src/model/`
- **acceptance:** `cargo test -- space_authorization_matrix` — deny-by-default for all four actor kinds; cross-Space isolation for read, write, subscription, search, artifact
- **evidence:** —

#### SPACE-002 — WorkOrder / AgentRun / Artifact state machines
- **priority:** P0 — baseline P0 "attribution and delegated authority"
- **status:** not_started
- **mandate:** `collaborative-spaces-baseline.md:89-93` — "one versioned action envelope used by REST commands, agent execution, proposals, adapter ingestion, and external effects; WorkOrder/AgentRun state-machine and retry tests; no action can approve or broaden its own grant"; ADR 010:121-134
- **governing_authority:** ADR 010 §Work and agent execution
- **blocked_by:** IK-002, SPACE-001 (declared)
- **files:** new migration, `crates/mv-core/src/model/`, `crates/mv-engine/src/`
- **acceptance:** `cargo test -- work_order_agent_run` — state machines and retries; a run cannot approve or broaden its own grant; a task title or plan step cannot masquerade as an AgentRun (ADR 010:132-134)
- **evidence:** —

#### SPACE-003 — Reliable effects: persisted adapter bindings and provider delivery IDs
- **priority:** P0 — baseline P0 "reliable effects"
- **status:** not_started
- **mandate:** `collaborative-spaces-baseline.md:97-106` — "Adapter registration is volatile, inbound ingestion has no durable idempotency constraint, polling can advance after partial failure"; unlock evidence includes "transactional inbox/outbox with unique provider delivery IDs" and "crash tests covering each commit/delivery boundary"
- **governing_authority:** Law 4 (`:52`); ADR 010 acceptance gate 6 (`:269`)
- **blocked_by:** IK-004, IK-006 (declared)
- **files:** `crates/mv-engine/src/adapters/`, new migration
- **acceptance:** `cargo test -p mv-engine -- adapter_reliable_effects` — bindings persist across restart; duplicate provider delivery IDs are rejected; polling does not advance after partial failure
- **evidence:** —

#### SPACE-004 — Communication-to-knowledge promotion boundary
- **priority:** P0 — baseline P0; was a live violation
- **status:** verified
- **mandate:** `collaborative-spaces-baseline.md:110-112` — "The relay currently inserts every allowed message into the knowledge graph. That violates the promotion boundary and can leak low-quality or private chat into retrieval, agent context, and downstream projections"; ADR 010:154-169
- **governing_authority:** ADR 010 §Communication and knowledge promotion; ADR 010 acceptance gate 4 (`:265-266`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/relay.rs`, `crates/mv-engine/src/engine/relay_ops.rs`, `crates/mv-engine/src/ingest.rs`, `crates/mv-core/src/traits.rs`, `crates/mv-core/src/model/exchange.rs`, `crates/mv-storage/src/sqlite.rs`, `crates/mv-server/src/email.rs`
- **acceptance:** `cargo test -p mv-engine -- promotion_boundary` — a relay message does not enter the knowledge graph without explicit or policy-approved promotion; promotion records source message ID, extractor/actor, evidence, confidence, policy, approval; retraction re-indexes without rewriting source communication
- **evidence:** `cargo test -p mv-engine -- promotion_boundary` → 3/3 ok; commit `e3010b1`; 2026-07-31. Relay send/receive store communication only; `promote_relay_message` / `retract_relay_promotion` record provenance and retract without rewriting source messages.

#### SPACE-005 — Execution evidence: versioned artifacts, budgets, grant snapshots
- **priority:** P1 — baseline P1
- **status:** not_started
- **mandate:** `collaborative-spaces-baseline.md:123-132` — "Plans have step output but not versioned artifacts, budgets, evidence, verification, or receipts. Optional HTTP audit and Chronicle entries do not form a Trust Ledger"
- **governing_authority:** ADR 010 §Action envelope and trust ledger (`:135-152`); feature-completeness §7 (`:122`)
- **blocked_by:** SPACE-002, IK-020 (declared)
- **files:** `crates/mv-engine/src/`, new migration
- **acceptance:** `cargo test -- agent_run_evidence` — immutable artifact versions with verification state; per-run grant snapshots, usage, and budget enforcement; correlation across work, approval, delivery, and report publication
- **evidence:** —

#### SPACE-006 — `DocumentWorkspace` rename before any shared Workspace API ships
- **priority:** P1 — a hard naming precondition with a live collision risk
- **status:** not_started
- **mandate:** `docs/adr/010-personal-vault-and-collaborative-spaces.md:77-80` — "Before shared Workspace APIs ship, implementation names must distinguish that concept as `DocumentWorkspace` (or an equally explicit name) from the collaboration `Workspace`. Neither may inherit the other's authorization semantics by name"
- **governing_authority:** ADR 010 §Product topology
- **blocked_by:** none (must precede SPACE-001)
- **files:** `crates/mv-core/src/model/workspace.rs`, `cratests/mv-engine/src/workspace.rs`, `crates/mv-server/src/rest/workspaces.rs`, `migrations/031_knowledge_workspace_manifest.sql`, `frontend/src/lib/api/workspaces.ts`
- **acceptance:** `cargo test --workspace` passes after the rename; `grep -rn "\bWorkspace\b" crates/` shows no ambiguous use; the `/api/v1/workspaces` path is either renamed or documented as the document-workspace namespace
- **evidence:** —

#### SPACE-007 — Space vertical-slice implementation gate (7 contracts)
- **priority:** P1 — the declared unlock for collaboration feature work
- **status:** not_started
- **mandate:** `collaborative-spaces-baseline.md:167-182` — seven numbered contracts; `:166-167` — "Do not start with Slack UI or broad schema coverage"; `:184-190` — "No feature code should be added until the P0 contracts above are reviewed"
- **governing_authority:** ADR 010 §First collaboration segment (`:202-220`)
- **blocked_by:** SPACE-001..SPACE-005 (declared)
- **files:** `docs/architecture/collaborative-spaces-baseline.md`, `crates/`
- **acceptance:** all seven contracts approved with tests, including the PostgreSQL transaction/outbox/realtime consistency model (contract 6) and end-to-end acceptance fixtures (contract 7)
- **evidence:** —

#### SPACE-008 — ADR 010 acceptance gate: seven conditions
- **priority:** P1 — required to move ADR 010 from Proposed to Accepted
- **status:** not_started
- **mandate:** `docs/adr/010-personal-vault-and-collaborative-spaces.md:255-271` — seven numbered conditions
- **governing_authority:** ADR 010 §Acceptance gate
- **blocked_by:** SPACE-001..SPACE-007, SLICE-003 (declared — condition 5 requires "the constitution's end-to-end interoperability acceptance and threat-model cases")
- **files:** `docs/adr/010-personal-vault-and-collaborative-spaces.md`
- **acceptance:** each of the seven has a named owner, disposition, or passing test; condition 7 ("Personal Vault data remains private unless an explicit transfer is approved") has an isolation test
- **evidence:** —

---

# L. PARK — explicitly deferred

Recorded so they are never mistaken for oversights. Do not start without an ADR
amendment.

#### PARK-001 — Native multi-device sync provider
- **priority:** P3 — `knowledge-workspace.md:534` deferred
- **status:** not_started
- **mandate:** `knowledge-workspace.md:536` — "selection or implementation of a native multi-device sync provider"; `knowledge-workspace-document-contract.md:161` — "MindVault does not implement a sync provider in the first workspace slice"
- **governing_authority:** `knowledge-workspace.md:530-539` §Deferred post-foundation decisions
- **blocked_by:** WS-010 (inferred)
- **files:** —
- **acceptance:** an ADR amendment reopens it; until then, closed
- **evidence:** —

#### PARK-002 — Non-portable MindVault-managed encrypted-document mode
- **priority:** P3 — deferred
- **status:** not_started
- **mandate:** `knowledge-workspace.md:537`; `docs/adr/009-workspace-identity-and-mutation-boundary.md:176-177` — "A future MindVault-managed encrypted-document mode would be a separate workspace mode"
- **governing_authority:** ADR 009 §Portable versus sealed storage
- **blocked_by:** WS-021 (inferred)
- **files:** —
- **acceptance:** ADR amendment
- **evidence:** —

#### PARK-003 — Automatic history/trash pruning
- **priority:** P3 — deferred
- **status:** not_started
- **mandate:** `knowledge-workspace.md:538`; `knowledge-workspace-document-contract.md:134-135` — "Initial releases perform no silent version-count or age pruning. Pruning is a separate, explicit, auditable operation with a dry-run report"
- **governing_authority:** `knowledge-workspace-document-contract.md:126-140`
- **blocked_by:** WS-002 (inferred)
- **files:** —
- **acceptance:** ADR amendment; when built, it must be explicit, auditable, dry-runnable, and must not prune pinned evidence
- **evidence:** —

#### PARK-004 — Collaborative block-level editing / CRDT
- **priority:** P3 — deferred by two documents
- **status:** not_started
- **mandate:** `knowledge-workspace.md:539`; ADR 010:251 lists "full CRDT document editing" under non-goals; ADR 010:183-185 — "general CRDT editing ... deferred until measured scale or product requirements justify them"
- **governing_authority:** ADR 010 §Non-goals (`:244-253`)
- **blocked_by:** none
- **files:** —
- **acceptance:** ADR amendment citing measured scale
- **evidence:** —

#### PARK-005 — Kafka / microservice decomposition
- **priority:** P3 — deferred
- **status:** not_started
- **mandate:** `docs/adr/010-personal-vault-and-collaborative-spaces.md:183-185` — "Kafka, a broad microservice split, general CRDT editing, and an unrestricted workflow engine are deferred until measured scale or product requirements justify them"; ADR 010:252
- **governing_authority:** ADR 010 §Storage and synchronization
- **blocked_by:** none
- **files:** —
- **acceptance:** ADR amendment citing measured scale
- **evidence:** —

#### PARK-006 — Matrix bridge and Solid adapter
- **priority:** P3 — optional by contract
- **status:** not_started
- **mandate:** `PROTOCOL_BOUNDARIES.md:30-31` — "Optional messaging compatibility" / "Optional storage compatibility"; ADR 011:70 — "Matrix and Solid through optional bridges or adapters, not mandatory cores"
- **governing_authority:** Law 9 (`:64`)
- **blocked_by:** PROTO-008 (inferred)
- **files:** —
- **acceptance:** built only after the core adapter contract is stable; never as an internal domain model
- **evidence:** —

---

# M. AGENT — governed agent execution graph (ADR 012)

Landed 2026-07-26/27 after the rest of this backlog was drafted. Migration 038
plus ~4,400 lines across mv-core, mv-engine, mv-server and mv-plugin, with 61
tests. Unlike migration 031, **no table here is dead** — all ten have Rust
readers and writers. The gap is different: the graph is fully governed and
completely inert.

ADR 012 names seven implementation gates (`docs/adr/012-governed-agent-execution-graph.md:169-177`).
Gates 1-6 have landed; gate 7 has not.

#### AGENT-001 — No executor exists; `start_run` executes nothing
- **priority:** P0 — the graph is admissible, leasable, gateable, approvable and completable, but nothing ever runs
- **status:** verified
- **mandate:** `crates/mv-engine/src/engine/work_order_ops.rs:506` — "It executes nothing"; mirrored at `crates/mv-server/src/rest/work_orders.rs:313` and `crates/mv-server/src/rest.rs:200`
- **governing_authority:** ADR 012 decision (`:72-113`); `docs/architecture/WORK_ORDER_MODEL.md:250`
- **blocked_by:** IK-001 (public grant admission is named a hard prerequisite for outbound execution — `docs/adr/012-governed-agent-execution-graph.md:124`, `WORK_ORDER_MODEL.md:252`)
- **files:** `crates/mv-engine/src/engine/work_order_ops.rs`, `crates/mv-engine/src/engine/agent_run_executor.rs`
- **acceptance:** a run drives a node contract to a terminal state and produces a digested artifact with provenance, without an external dispatcher
- **evidence:** `cargo test -p mv-engine -- agent_run_executor` → 1/1 ok; commit `ebf61a8`; 2026-07-31. `execute_run` drives leased Engine runs through Running→artifact→Gated→required Low gates→Completed with digested provenance-linked artifact.

#### AGENT-002 — Retire the superseded `plans` / `plan_steps` schema
- **priority:** P1 — ADR 012's only unlanded implementation gate
- **status:** not_started
- **mandate:** `docs/adr/012-governed-agent-execution-graph.md:177` — "Retire the superseded plan schema and endpoints"; `migrations/038_work_orders_and_agent_runs.sql:16` — "The superseded tables are left in place for this revision and are retired separately"
- **governing_authority:** ADR 012 `Supersedes:` (`:8-9`)
- **blocked_by:** none
- **files:** new `migrations/039_retire_plans.sql`, `crates/mv-server/src/rest/plans.rs`, `migrations/024_plans.sql`
- **acceptance:** the tables are dropped or formally frozen with a migration note; endpoints already return 410 Gone
- **evidence:** endpoints return 410 as of `crates/mv-server/src/rest/plans.rs`

#### AGENT-003 — `item_10_conformance_coverage_is_declared` cannot fail and its counts are stale
- **priority:** P1 — a declaration standing in for a check, which is exactly what ADR 012:179-180 forbids
- **status:** not_started
- **mandate:** `docs/adr/012-governed-agent-execution-graph.md:179` — "Scaffolding, endpoint count, or a rendered run view do not count as progress against these gates. Conformance tests do."
- **governing_authority:** feature-completeness item 10 (`INTEROPERABILITY_CONSTITUTION.md:125`)
- **blocked_by:** none
- **files:** `crates/mv-server/tests/work_order_conformance.rs:469-480`
- **acceptance:** the test asserts real counts or is deleted. Declared vs actual today: mv-storage 11 vs **14**, mv-engine 8 vs **13**, mv-server integration 4 vs **5**; the assertion is only `assert_eq!(enforcing.len(), 5)`
- **evidence:** —

#### AGENT-004 — Runtime isolation is PARTIAL; external executor kinds are gated on it
- **priority:** P1
- **status:** not_started
- **mandate:** `docs/architecture/EXECUTION_ISOLATION_MODEL.md:122` — "External executor kinds must not be enabled on the strength of this dimension alone"; `:115-116` — no process/port/namespace separation, no per-run memory or disk quotas
- **governing_authority:** `EXECUTION_ISOLATION_MODEL.md:14`
- **blocked_by:** none
- **files:** `crates/mv-engine/src/engine/work_order_ops.rs`, `crates/mv-plugin/src/sandbox.rs`
- **acceptance:** per-run resource metering exists, so `resource` edges are a guarantee rather than a scheduling hint (`EXECUTION_ISOLATION_MODEL.md:121`); a run's credentials expire no later than its terminal state (`:158`)
- **evidence:** —

#### AGENT-005 — Reconcile ADR 010 and ADR 012; no reconciling text exists
- **priority:** P1
- **status:** not_started
- **mandate:** ADR 012 `:8-9` supersedes only migration 024, never ADR 010; ADR 010 contains zero references to ADR 012
- **governing_authority:** `INTEROPERABILITY_CONSTITUTION.md:33-37` (conflicts need a dated, scoped ADR exception)
- **blocked_by:** none
- **files:** `docs/adr/010-personal-vault-and-collaborative-spaces.md`, `docs/adr/012-governed-agent-execution-graph.md`
- **acceptance:** an addendum states which ADR 010 fields ADR 012 relocated and which are unimplemented. Two concrete divergences: (a) ADR 010:125 requires `Space` and `assignee` on WorkOrder — neither exists in `crates/mv-core/src/model/work_order.rs` or `migrations/038`, which substitute `governing_node_uri` + `actor_uri`, so a Work Order cannot be scoped to a Space; (b) 6 of ADR 010:127's 11 `AgentRun` fields (model/runtime identity, usage, receipts) have no home in the 038 schema
- **evidence:** —

#### AGENT-006 — `work_order_history` has a writer and no reader
- **priority:** P2
- **status:** not_started
- **mandate:** written at `crates/mv-storage/src/sqlite.rs:6232`; zero readers in production or test
- **governing_authority:** feature-completeness item 3, query API (`INTEROPERABILITY_CONSTITUTION.md:117`)
- **blocked_by:** none
- **files:** `crates/mv-storage/src/sqlite.rs`, `crates/mv-server/src/rest/work_orders.rs`
- **acceptance:** either a revision-history query API exists, or the table is documented as export-only with the forbid-delete trigger on `work_orders` (`migrations/038:148`) cited as the real retention mechanism
- **evidence:** —

#### AGENT-007 — `work_order_node_write_targets` invariant is enforced only in SQL
- **priority:** P2
- **status:** not_started
- **mandate:** production reader is the trigger `enforce_write_lease_insert` (`migrations/038_work_orders_and_agent_runs.sql:421-427`); the only Rust reader is a test
- **governing_authority:** ADR 012 rule 1 (`:85-87`)
- **blocked_by:** none
- **files:** `crates/mv-engine/src/engine/work_order_ops.rs`
- **acceptance:** a lease outside declared write scope surfaces as a typed error rather than a trigger ABORT string
- **evidence:** —

#### AGENT-008 — `rest/work_orders.rs` has zero tests across 1,041 lines
- **priority:** P2
- **status:** not_started
- **mandate:** feature-completeness item 10 (`INTEROPERABILITY_CONSTITUTION.md:125`)
- **governing_authority:** same
- **blocked_by:** none
- **files:** `crates/mv-server/src/rest/work_orders.rs`
- **acceptance:** branch and error-path coverage exists at the handler layer; today all coverage is indirect via two integration files
- **evidence:** —

---

# N. HYG — engineering hygiene (found 2026-07-27)

Discovered while verifying the in-flight work. Not mandated by a contract
document, but each one either hides real defects or makes verification lie.

#### HYG-001 — An ambient `OPENAI_API_KEY` silently enabled a cloud LLM
- **priority:** P0
- **status:** **verified** (fixed 2026-07-27)
- **mandate:** Law 12 (`INTEROPERABILITY_CONSTITUTION.md:71`) — "Cloud services may enhance but cannot become an undeclared dependency"; ADR 002 single-owner sovereignty
- **governing_authority:** Law 12
- **blocked_by:** none
- **files:** `crates/mv-engine/src/config.rs`, `crates/mv-engine/src/llm.rs`, `config/default.toml`, `crates/mv-cli/src/commands/mod.rs`
- **acceptance:** `cargo test -p mv-engine --lib cloud_fallback` — 3 tests
- **evidence:** LLM API keys resolve through the credential store, whose backend chain ends in environment variables (`crates/mv-core/src/credentials.rs:677`, documented at `:652`). `crates/mv-engine/src/llm.rs` step 4 then added an OpenAI provider whenever a key was present **and** `llm.enabled` was false — the shipped default. So any developer or user with `OPENAI_API_KEY` exported had chat/assist sending vault content to OpenAI from a vault configured for local-only operation. Proven by the flaky test `chat_returns_grounded_heuristic_answer`: failed with the key set (`provider=native-rag-llm`), passed with it unset. Fixed by adding `LlmConfig.allow_cloud_fallback`, default **false**; local providers are unaffected.

#### HYG-002 — mv-server tests were not hermetic against local AI services
- **priority:** P1
- **status:** **verified** (fixed 2026-07-27)
- **mandate:** —
- **governing_authority:** —
- **blocked_by:** none
- **files:** `crates/mv-server/src/rest.rs` (test factory `create_state_with_embedding_and_mode_and_unseal`)
- **acceptance:** `cargo test -p mv-server --lib` passes with Ollama running and `OPENAI_API_KEY` set
- **evidence:** `LlmConfig::auto_detect` defaults true and probes `http://localhost:11434/v1` (`crates/mv-engine/src/llm.rs:649-651`), so on a machine running Ollama the engine acquired a real LLM and AI handlers took the LLM branch instead of the deterministic heuristic. The test factory now sets `auto_detect = false`. Other `EngineConfig` construction sites in `lib.rs`, `grpc.rs`, `workspace_watch.rs`, `oauth.rs` and the four integration harnesses were not changed — they pass today, but should adopt the same guard if they ever exercise AI paths.

#### HYG-003 — `rust-toolchain.toml` is not in force locally
- **priority:** P2
- **status:** not_started
- **mandate:** —
- **governing_authority:** —
- **blocked_by:** none
- **files:** `rust-toolchain.toml`, `CONTRIBUTING.md`
- **acceptance:** `cargo --version` resolves to the pinned toolchain on a fresh developer machine, or the divergence is documented
- **evidence:** `/opt/homebrew/bin/cargo` (1.95.0) precedes the rustup shim on PATH and does not honour `rust-toolchain.toml` or `+toolchain`. CI pins 1.91.1. The ~29 GB warm build cache is stamped `rustc 1.95.0` in `target/.rustc_info.json`. Consequence: code using a stdlib API stabilized after 1.91.1 compiles locally and fails CI's build. `cargo fmt` is the one step safe to run on the pinned toolchain at zero build cost.

#### HYG-004 — CI has no frontend job
- **priority:** P2
- **status:** not_started
- **mandate:** —
- **governing_authority:** feature-completeness item 10 (`INTEROPERABILITY_CONSTITUTION.md:125`)
- **blocked_by:** none
- **files:** `.github/workflows/ci.yml`
- **acceptance:** CI runs `pnpm check`, `pnpm lint` and `pnpm exec vitest run`
- **evidence:** `.github/workflows/ci.yml` has three jobs — `rust`, `sealed-gates`, `connectors`. None runs vitest, svelte-check, eslint, prettier or playwright, so ~509 frontend test cases across 62 files and all Svelte type-checking are ungated. The `frontend/src/lib/stores/websocket.ts` change in this branch exports new types with no CI coverage.

#### HYG-006 — The integration suite shares one process-global rate-limit bucket
- **priority:** P2
- **status:** **verified** (mitigated 2026-07-27)
- **mandate:** —
- **governing_authority:** —
- **blocked_by:** none
- **files:** `crates/mv-server/src/limits.rs`, `crates/mv-server/tests/api_integration.rs`
- **acceptance:** `cargo test -p mv-server --test api_integration -- --test-threads=1`
- **evidence:** `RATE_LIMITER` is a `OnceLock` (`limits.rs:167`) holding one bucket for the whole process, defaulting to 120 requests per 60 seconds. The suite runs serially in a single process and completes in ~39s, so the entire run sat just under the ceiling — and *adding two tests* pushed an unrelated later test (`workspace_projections_are_searchable_...`) to fail with 429. Failure was a function of total test count, not of the test that failed. Mitigated by raising the ceiling in `setup_with_config` before the lock initializes; the two suite tests that assert 429 exercise the namespace node quota, not the rate limiter, so no assertion is weakened. The underlying design — production-sized global limiter shared by a test suite — is unchanged and will bite again if the suite is parallelized.

#### HYG-005 — `scripts/verify_all.sh` and `CONTRIBUTING.md` prescribe a command that fails
- **priority:** P2
- **status:** not_started
- **mandate:** —
- **governing_authority:** —
- **blocked_by:** none
- **files:** `scripts/verify_all.sh`, `CONTRIBUTING.md`
- **acceptance:** the documented command matches what CI actually runs
- **evidence:** both prescribe a bare `cargo test --workspace`, the exact parallel configuration commits `37b39ef` and `5839238` patched CI to avoid. `verify_all.sh` also chains `scripts/smoke_test.sh`, which downloads an embedding model and needs network.

---

# U. UX — product experience

Canonical analysis and full UX-001..UX-025 catalog:
`docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md`. Items below are the execution-tracked
subset with acceptance commands. Do not mark product "best-in-class" until the
audit §18.5 gates are met with evidence.

#### UX-001 — Elevate Trusted Work in primary IA
- **priority:** P0 — audit §4.3 / §15 Phase 1; category wedge otherwise undiscoverable
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` — "Elevate Work Orders + Approvals in sidebar; demote Goals/Plugins"
- **governing_authority:** `docs/MINDVAULT_NEXT_MASTER_PLAN.md` Locked Decisions (agents first-class); `docs/strategy/unicorn/CATEGORY_DESIGN_AND_POSITIONING.md`
- **blocked_by:** none
- **files:** `frontend/src/routes/+layout.svelte`, `frontend/src/lib/components/MobileNav.svelte`, `frontend/src/lib/command-palette/actions.ts`
- **acceptance:** Sidebar shows Trusted Work above personal-OS items; nav label "Daily" is renamed to "Focus" (still routes to `/focus`); `pnpm -C frontend exec vitest run` passes for any touched store/nav tests
- **evidence:** —

#### UX-002 — Today home replaces widget dashboard
- **priority:** P0 — audit §11.1 / §18.5 gate
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` — "Today: Approvals · Due · Capture · Briefing · Continue"
- **governing_authority:** master plan UX direction; category wedge
- **blocked_by:** UX-001 (inferred)
- **files:** `frontend/src/routes/+page.svelte`, `frontend/src/lib/components/*Widget*.svelte`
- **acceptance:** `/` first viewport prioritizes awaiting approvals + due work + capture; uses shared EmptyState; Playwright or vitest smoke asserts approvals region when fixture runs present
- **evidence:** —

#### UX-003 — Replace native confirm/prompt with AlertDialog
- **priority:** P0 — a11y + consistency; audit §5.10 / WCAG 2.4/4.1
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` — "Zero window.confirm/prompt in frontend/src"
- **governing_authority:** master plan §46 Dialog/AlertDialog; WCAG 2.2
- **blocked_by:** UX-004 (inferred)
- **files:** all `confirm(`/`prompt(` call sites under `frontend/src/` (notes, chat, search, tags, settings, goals, …)
- **acceptance:** `rg -n "\\b(confirm|prompt)\\(" frontend/src --glob '*.{svelte,ts}'` returns no `window`/global dialog usages for UX flows; AlertDialog has focus trap tests
- **evidence:** —

#### UX-004 — Complete Dialog primitive (focus trap/restore)
- **priority:** P0 — WCAG 2.4.3 / 2.1.1; master plan §46
- **status:** not_started
- **mandate:** `docs/MINDVAULT_NEXT_MASTER_PLAN.md` §46 — "Dialog/AlertDialog"; audit §7 Modal gaps
- **governing_authority:** master plan §46 Accessibility release requirements
- **blocked_by:** none
- **files:** `frontend/src/lib/components/Modal.svelte` (or `frontend/src/lib/ui/overlays/Dialog.svelte`)
- **acceptance:** Unit test proves focus is trapped while open and restored on close; Escape closes; `aria-modal` + labelled title
- **evidence:** —

#### UX-005 — Global focus-visible and minimum target size
- **priority:** P0 — WCAG 2.4.7 / 2.5.8
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` §9.2; prior `docs/ux-audit.md` S5
- **governing_authority:** master plan §46 "visible focus"
- **blocked_by:** none
- **files:** `frontend/src/app.css`, inbox/task interactive controls
- **acceptance:** Primary routes pass axe `serious`=0 for focus-related rules in Playwright (see UX-016)
- **evidence:** —

#### UX-007 — UI kit foundation under `frontend/src/lib/ui`
- **priority:** P0 — master plan §46 required before expanding UI surface
- **status:** not_started
- **mandate:** `docs/MINDVAULT_NEXT_MASTER_PLAN.md` §46 — "Before expanding UI surface, build a shared, tested design system."
- **governing_authority:** master plan §46
- **blocked_by:** none
- **files:** `frontend/src/lib/ui/**`, `frontend/src/lib/styles/tokens.css`, `frontend/src/app.css`
- **acceptance:** Button, Dialog, FormField, EmptyState, Skeleton, PageHeader exported from `lib/ui`; Today + Work Orders consume Button/EmptyState; Storybook or Histoire build succeeds for primitives
- **evidence:** —

#### UX-009 — Work Orders experience v1 (approval craft)
- **priority:** P0 — primary wedge surface
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` §11.13 — "Linear-speed list + ApprovalCard + Evidence timeline + keyboard approve/reject"
- **governing_authority:** category wedge; AGENT execution graph productization
- **blocked_by:** UX-007 (inferred); AGENT-008 (inferred for API hardness)
- **files:** `frontend/src/routes/work-orders/+page.svelte`, `frontend/src/lib/api/workOrders.ts`
- **acceptance:** Keyboard approve/reject path documented and covered by Playwright; empty/loading/error states use ui kit; no native confirm
- **evidence:** —

#### UX-010 — Onboarding teaches Trusted Work
- **priority:** P0 — promise-vs-delivery
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` §5.1 / §11.16
- **governing_authority:** category positioning
- **blocked_by:** none
- **files:** `frontend/src/routes/onboarding/+page.svelte`, `frontend/src/lib/stores/onboarding.ts`
- **acceptance:** FTUE copy references owned context + governed agent work; includes connection health step; does not only teach note+task PKM
- **evidence:** —

#### UX-016 — Accessibility CI on primary routes
- **priority:** P0 — master plan §46 automated checks; HYG-004 adjacency
- **status:** not_started
- **mandate:** `docs/MINDVAULT_NEXT_MASTER_PLAN.md` §46 — "automated checks plus human verification"
- **governing_authority:** master plan §46
- **blocked_by:** HYG-004 (inferred)
- **files:** `frontend/e2e/**`, `.github/workflows/ci.yml`
- **acceptance:** CI runs Playwright+axe on `/`, `/inbox`, `/notes`, `/tasks`, `/work-orders`, `/search`, `/settings` and fails on axe `serious`+
- **evidence:** —

#### UX-012 — Settings nested layout split
- **priority:** P1 — cognitive load; audit §4.4
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` §4.4
- **governing_authority:** feature-completeness / UX honesty
- **blocked_by:** UX-007 (inferred)
- **files:** `frontend/src/routes/settings/**`
- **acceptance:** `/settings` uses nested layout with secondary nav; megapage section count reduced by extracting ≥4 routes; `pnpm -C frontend check` passes
- **evidence:** —

#### UX-013 — Unify note editors
- **priority:** P1 — consistency; audit §2.4
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` — "One editor package"
- **governing_authority:** master plan §46 "accessible rich-text editor surface"
- **blocked_by:** none
- **files:** `frontend/src/lib/components/RichNoteEditor.svelte`, `frontend/src/lib/components/MvEditor.svelte`, `frontend/src/lib/components/editor/**`, `frontend/src/routes/daily/+page.svelte`
- **acceptance:** Daily + Notes share one editor entrypoint; dead duplicate path removed or re-exported; editor a11y helpers still used
- **evidence:** —

#### UX-018 — Browser API client auth/session honesty
- **priority:** P0 — error prevention; audit §5.7
- **status:** not_started
- **mandate:** `docs/UI_UX_PRODUCT_EXPERIENCE_AUDIT.md` — "fetchJson does not attach Authorization"
- **governing_authority:** security product plane; onboarding honesty
- **blocked_by:** none
- **files:** `frontend/src/lib/api/client.ts`, settings connection UI, chat error mapping
- **acceptance:** Documented local auth path works end-to-end from Settings → API calls; chat 401 copy matches actual client behavior; vitest covers header attachment when token configured
- **evidence:** —

---

## Suggested execution order

1. **DOC-001..DOC-007, PROG-001..PROG-003** — a day's work; stops false-completeness
   claims and gives every P0 an owner. Add DOC-010 for ADR 012 and the AGENT group.
2. **WS-001..WS-003, WS-014, WS-016, AGENT-003** — the dead-schema, dead-fixture and
   vacuous-test defects. Self-contained, and they close the credibility gap.
3. **IK-001, IK-002, IK-003** — the constitution's declared next slice
   (`interoperability-kernel-v1.md:283-284`). Nothing in the FED or PROTO groups
   is legitimately startable before these, and **AGENT-001 is blocked on IK-001**
   because grant admission is a named prerequisite for outbound execution.
4. **IK-004..IK-007, IK-016** — give the outbox and inbox a runtime.
5. **AGENT-001, AGENT-002** — give the execution graph an executor, then retire the
   superseded plan schema.
6. **WS-006..WS-013** — Stage 2 guarded writes behind the six unlock gates.
7. **FED-000 first, then FED-001..FED-010** — federation stays off until all ten
   are `verified`.
8. **SLICE-001..SLICE-003** — the end-to-end proof, last.
9. **UX-004 → UX-003 → UX-001 → UX-007 → UX-002/UX-009/UX-010** in parallel with
   foundation spines once HYG-004 unlocks UX-016 — experience work must not invent
   Spaces/Federation claims ahead of SPACE/FED verification (see experience audit §18.5).
