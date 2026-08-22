# Phase 0 — Repository Protection Baseline

**Date:** 2026-07-31  
**Branch:** `strategy/unicorn-2026-07-31`  
**Baseline HEAD:** `3de2371` (`Record AGENT-001 verification evidence.`)  
**Remote:** `origin` → `https://github.com/wwk5q8z6kk-bit/mindvault.git`

## Working-tree condition at mission start

Preserved without reset, stash drop, rebase, or history rewrite.

| Class | Paths |
|---|---|
| Pre-existing WIP (AGENT-002 / plans retirement) | `migrations/024_plans.sql`, `migrations/038_work_orders_and_agent_runs.sql`, `migrations/041_retire_plans.sql` (untracked), workspace/plans Rust paths, `DEVELOPMENT_PLAN.md`, `IMPLEMENTATION_BACKLOG.md` |
| Cursor hook state | `.cursor/hooks/state/continual-learning*.json` |
| Mission inputs (new) | `docs/strategy/unicorn/` |

## Worktrees present

| Path | Branch | HEAD |
|---|---|---|
| `/Users/mac/Projects/mindvault` | `strategy/unicorn-2026-07-31` | `3de2371` |
| `/private/tmp/mindvault-ui-ux-next` | `codex/mindvault-ui-ux-next` | `fcc6646` |
| `/private/tmp/mv-sec-review` | `backup/product-evolution-local-20260727` | `fdf1fcd` |

## Discipline rules for this mission

1. Preserve all user WIP; do not discard plans-retirement work.
2. No destructive git operations; no merge/release/license/visibility changes without owner approval.
3. Strategy artifacts land under `docs/strategy/unicorn/`.
4. Code changes only for the selected Wedge Value Proof after strategy gates pass.
5. Commits only when the owner requests them.
6. Migrations count at baseline: **41** SQL files under `migrations/`.

## Critical early corrections to stale planning memory

| Claim | Prior memory | Live backlog status (2026-07-31) | Evidence label |
|---|---|---|---|
| SPACE-004 message→knowledge auto-promotion | Live violation | **verified** | E3 — `cargo test -p mv-engine -- promotion_boundary`; commit `e3010b1` |
| AGENT-001 no executor | Missing | **verified** | E3 — `cargo test -p mv-engine -- agent_run_executor`; commit `ebf61a8` |
| Plans schema still canonical | Assumed live | **AGENT-002 in_progress** (WIP in tree) | E4/E5 pending commit |
| Live external publishers | Incomplete | **IK-005 / IK-006 not_started** | E4 schemas + local ack only |
| Production federation | Must stay off | Still gated (**FED-*** unverified) | E4 |

## Next gate

Phase 1 — Repository Truth and Leverage Audit.

## Follow-up correction

Federation was **not** safely gated at mission start (routes live). **FED-000** now returns 501 unless `MINDVAULT_FEDERATION_ENABLED=1` (verified 2026-07-31).

