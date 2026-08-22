# Keep / Refactor / Modularize / Replace / Retire

**Date:** 2026-07-31

| Asset | Action |
|---|---|
| WorkOrder + execute_run + HTTP proof | Keep / productize |
| Grants + envelopes | Harden / extend |
| Promotion boundary | Keep |
| Local SQLite vault | Keep |
| Plans schema/endpoints | **Retire** (AGENT-002 WIP) |
| Vacuous conformance declarations | Replace with failing tests (AGENT-003) |
| Orphan workspace tables | Wire or freeze (WS-001+) |
| Federation production paths | Keep gated |
| Broad CRDT/Matrix/Solid | Park |
| Multi-language client ambitions | Defer |
