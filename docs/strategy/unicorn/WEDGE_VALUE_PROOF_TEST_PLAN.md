# Wedge Value Proof Test Plan

**Date:** 2026-07-31

```bash
cargo test -p mv-server --test work_order_conformance -- \
  wedge_value_proof_trusted_work_completes_over_http \
  work_order_routes_are_described_by_the_authoritative_openapi_document \
  -- --test-threads=1

cargo test -p mv-engine -- agent_run_executor promotion_boundary -- --test-threads=1
```
