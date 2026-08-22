# North Star and Product Metrics

**Date:** 2026-07-31

## North star

**Weekly Accepted Trusted Works (WATW):** count of Agent Runs reaching `completed` with all required gates Pass and human acceptance recorded (acceptance UI may lag; use completed+export as interim).

## Input metrics

Activation (first completed execute), time-to-first-trusted-work, weekly retention of users with ≥1 accepted run, gate fail rate, re-execute conflict rate, grant denial rate, support tickets/user, gross margin.

## Instrumentation (2026-07-31)

- Counter: `trusted_work_completed` on `MindVaultEngine.metrics` (incremented in `execute_run` after required gates Pass).
- Histogram: `trusted_work_gate_count`.
- Exposed via `/api/v1/metrics/snapshot` counters map and `mv trusted-work watw`.
- CLI demo: `mv trusted-work demo`.
- Acceptance: human-acceptance UI still future; interim = completed governed Engine runs.

