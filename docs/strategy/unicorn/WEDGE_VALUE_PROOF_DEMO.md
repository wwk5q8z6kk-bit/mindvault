# Wedge Value Proof Demo

**Date:** 2026-07-31

Deterministic demo = the automated test flow:

1. `POST /api/v1/context-nodes/local`
2. `POST /api/v1/authority-grants` with target `mindvault://schemas/executable`
3. `POST /api/v1/work-orders` (engine/low)
4. `POST .../nodes/{node_id}/runs`
5. `POST .../runs/{run_id}/approve` if parked
6. `POST .../runs/{run_id}/execute` → `status=completed` + `artifact_digest`
7. `GET .../artifacts/{artifact_id}/content` → `executor=engine`

## CLI (deterministic)

```bash
mv --config /path/to/config.toml trusted-work demo
mv --config /path/to/config.toml trusted-work watw
```

Expect `status: completed`, non-empty `artifact_digest`, `watw_counter >= 1`.

