# Current Architecture and Data Flow

**Date:** 2026-07-31

```text
[Clients: CLI | REST | gRPC | UDS | UI | MCP]
                |
                v
        [Auth / Role / Token]
                |
                v
     [Command admission + ActionEnvelope]  <-- partial route coverage
                |
                v
           [MindVaultEngine]
           /      |       \
          v       v        v
   [Node/KG] [Relay msgs] [WorkOrders/Runs/Artifacts]
          |       |        |
          v       v        v
        [SQLite canonical store]
          |       |
          v       v
   [FTS/Vector/Graph projections]   [Outbox → LocalAck (only)]
```

## Consistency model (observed)

- Canonical writes are SQLite transactional on engine paths that use the store APIs.
- Projections are derived; crash recovery worker (IK-016) not verified.
- Communication and knowledge are separate after SPACE-004.
- Agent runs use write leases; awaiting-approval cannot hold leases.

## Intended vs actual authority

| Intended (constitution/ADRs) | Actual |
|---|---|
| Context Grant ≠ Tool Grant | Partially modeled; MCP split incomplete |
| Trust Ledger for all consequential actions | IK-020 not_started |
| Human authority + reversibility | Present on promotion/retraction and gates; incomplete for external effects |
| Extensions fail-closed | Intended; EXT gateway incomplete |
