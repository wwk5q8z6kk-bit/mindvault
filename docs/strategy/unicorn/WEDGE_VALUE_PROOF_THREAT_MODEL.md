# Wedge Value Proof Threat Model

**Date:** 2026-07-31

| Threat | Mitigation in proof |
|---|---|
| Unauthorized write scope | Grant G0 admission |
| Caller-asserted gates | Server-evaluated gates; G2 not client-asserted |
| Silent chat→knowledge | Out of path; SPACE-004 holds |
| Replay execute | Must fail closed after completed |
| External side effects | Engine/Low only; no publisher |
| Principal spoof via header | Actor from request but grant still required; auth subject for admit |
