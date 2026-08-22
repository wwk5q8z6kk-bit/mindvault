# Wedge Architecture

**Date:** 2026-07-31

```text
Evidence nodes (Vault)
        |
        v
Tool Grant (exact targets) -- admits --> Work Order
        |
        v
Start Run (actor attributed) -> optional Approve
        |
        v
POST .../execute (Engine, RiskTier::Low)
        |
        v
Digested artifact + gate results (G0/G1/G2/G6) -> Completed
```

**Out of wedge scope:** external executors, live publishers, Spaces membership, federation, Domain Packs.
