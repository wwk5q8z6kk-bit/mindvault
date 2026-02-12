# MindVault P0/P1 Execution Checklist (Feb 2026)

This checklist tracks immediate security and operability hardening work with
clear file-level anchors.

## P0: Reliability and Trust Signals

- [ ] Unify frontend connectivity state (`Live`, `Degraded`, `Offline`) across status pill and banners.
  - Files: `web/src/admin.js`, `web/src/features/settings.js`, `web/styles.css`
  - Done when: no contradictory "Live" + backend-unavailable UI states.

- [ ] Harden offline API error parsing to avoid noisy JSON parse errors in UI logs.
  - Files: `web/src/admin.js`
  - Done when: backend-down smoke run has zero uncaught page errors and expected console warnings only.

- [ ] Add transport parity integration checks for representative REST and gRPC flows.
  - Files: `crates/mv-server/tests/`, `crates/mv-server/src/grpc.rs`, `crates/mv-server/src/rest.rs`
  - Done when: parity suite runs in CI and validates equivalent auth + payload behavior.

## P1: Security and Observability Depth

- [ ] Add at-rest encryption key lifecycle test matrix (boot, rotate, restore, failure paths).
  - Files: `crates/mv-storage/src/crypto.rs`, `crates/mv-storage/tests/`
  - Done when: automated tests cover key rotation and corrupted key material handling.

- [ ] Expand metrics from counters/gauges to latency histograms for critical endpoints.
  - Files: `crates/mv-server/src/metrics.rs`, `crates/mv-server/src/rest.rs`
  - Done when: `/metrics` includes request latency buckets for core API groups.

- [ ] Add alert-oriented metrics summaries for operational dashboards.
  - Files: `crates/mv-server/src/rest.rs` (`/api/v1/metrics/summary`)
  - Done when: summary endpoint includes thresholds/health hints for degraded states.

- [ ] Extend auth edge-case tests (namespace-scoped token + RBAC + quota/rate-limit interaction).
  - Files: `crates/mv-server/tests/`, `crates/mv-server/src/auth.rs`, `crates/mv-server/src/limits.rs`
  - Done when: tests cover deny/allow matrix by role, namespace, and quota status.

## Owners and Tracking

- [ ] Assign an owner for each item and link PRs.
- [ ] Update status weekly in this file.
- [ ] Keep references synced with `docs/review-report.md`.
