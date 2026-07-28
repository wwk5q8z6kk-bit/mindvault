# MindVault P0/P1 Execution Checklist (Feb 2026)

This checklist tracks immediate security and operability hardening work with
clear file-level anchors.

## P0: Reliability and Trust Signals

- [x] Unify frontend connectivity state (`Live`, `Degraded`, `Offline`) across status pill and banners.
  - Files: `frontend/src/lib/connection/status.ts`, `frontend/src/routes/+layout.svelte`, `frontend/src/lib/components/ApiHealthBanner.svelte`, `web/src/admin.js`, `web/index.html`, `web/styles.css`
  - Done when: no contradictory "Live" + backend-unavailable UI states.

- [x] Harden offline API error parsing to avoid noisy JSON parse errors in UI logs.
  - Files: `web/src/admin.js`
  - Done when: backend-down smoke run has zero uncaught page errors and expected console warnings only.

- [x] Add transport parity integration checks for representative REST and gRPC flows.
  - Files: `crates/mv-server/tests/transport_parity.rs`, `crates/mv-server/src/grpc.rs`, `crates/mv-server/src/rest.rs`, `.github/workflows/ci.yml`
  - Done when: parity suite runs in CI and validates equivalent auth + payload behavior.
  - Shipped: health, cross-transport store/get, recall, list, relationships (REST↔gRPC neighbors/overview), quota deny, shared-token + JWT role/namespace claim deny/allow, read-role write deny; CI step `Transport Parity REST/gRPC` (`--test-threads=1`).

## P1: Security and Observability Depth

- [x] Add at-rest encryption key lifecycle test matrix (boot, rotate, restore, failure paths).
  - Files: `crates/mv-storage/src/crypto.rs`, `crates/mv-storage/src/vault_crypto.rs`, `crates/mv-storage/tests/encryption_key_lifecycle.rs`
  - Done when: automated tests cover key rotation and corrupted key material handling.
  - Shipped: KeyManager env boot / restore / rotate / corruption matrix + VaultCrypto grace-epoch rotate/re-encrypt/restore-wrapped-key / wrong-password / grace-cap eviction (`cargo test -p mv-storage --test encryption_key_lifecycle`).

- [x] Expand metrics from counters/gauges to latency histograms for critical endpoints.
  - Files: `crates/mv-server/src/metrics.rs`, `crates/mv-server/src/rest.rs`
  - Done when: `/metrics` includes request latency buckets for core API groups.
  - Shipped: aggregate + per-`api_group` Prometheus histograms (`health`, `nodes`, `recall`, `search`, `keychain`, `metrics`, `other`) via `mindvault_rest_request_duration_seconds(_by_group)`.

- [x] Add alert-oriented metrics summaries for operational dashboards.
  - Files: `crates/mv-server/src/rest.rs` (`/api/v1/metrics/summary`), `crates/mv-server/src/metrics.rs`
  - Done when: summary endpoint includes thresholds/health hints for degraded states.
  - Shipped: `/api/v1/metrics/summary` returns `health.status` (`healthy`/`degraded`/`unhealthy`), thresholded hints (error rate, p95 latency, vault lifecycle), and latency snapshots by API group.

- [x] Extend auth edge-case tests (namespace-scoped token + RBAC + quota/rate-limit interaction).
  - Files: `crates/mv-server/tests/api_integration.rs`, `crates/mv-server/tests/transport_parity.rs`, `crates/mv-server/src/auth.rs`, `crates/mv-server/src/limits.rs`
  - Done when: tests cover deny/allow matrix by role, namespace, and quota status.
  - Node-quota deny/allow + update-at-limit covered; `MINDVAULT_NAMESPACE_NODE_QUOTA` is read live (not OnceLock) for testability.

## Owners and Tracking

- [ ] Assign an owner for each item and link PRs.
- [ ] Update status weekly in this file.
- [ ] Keep references synced with `docs/review-report.md`.
