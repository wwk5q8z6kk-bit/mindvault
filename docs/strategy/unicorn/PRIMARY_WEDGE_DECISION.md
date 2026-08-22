# Primary Wedge Decision

**Date:** 2026-07-31  
**Confidence:** Medium-high on technical fit (E1/E3); Medium on commercial demand (H)

## Recommendation

**Ship MindVault as the Trusted Agent Work system for AI-native knowledge teams:** humans bind private evidence into Work Orders, agents execute under grants and leases, and the product returns provenance-linked artifacts with human gates—local-first first, managed-private later.

## Locked choices

| Dimension | Choice |
|---|---|
| First user | Staff+ engineer / tech lead / research lead running agents daily |
| First buyer | Eng Manager, Head of AI/Platform, or security-conscious CTO |
| Painful workflow | “Agent did something in chat; nobody can prove what context or authority was used, and results aren’t durable work.” |
| Product promise | Complete trusted work with evidence—not another chatbot |
| 10x outcome | Hours of context reconstruction + trust anxiety → minutes to an accepted, attributable artifact |
| Deployment | Local-first single-node team vault; managed-private as expansion |
| Integrations (v1) | Vault nodes + Work Order API (+ optional Obsidian import later); MCP as adapter after |
| E2E workflow | Evidence nodes → WorkOrder → lease → `POST .../runs/:id/execute` → artifact → human accept |
| Category language | **Trusted Human–Agent Work System** (sovereign context fabric remains the platform end-state) |
| Time to first trusted value | <1 hour for technical user on API/CLI; <1 day with thin UI |
| Price hypothesis (H) | $49–$99/user/mo team; design-partner $1–2k/mo; later $80k–150k managed-private |
| Proof to buy | Weekly accepted governed runs with citation-valid artifacts |
| Distribution | Founder-led design partnerships inside AI-native 20–200 person companies |
| Path to platform | Personal/team trusted work → Spaces → live effects → packs → ecosystem |

## Fallback

**T02 Sovereign personal AI context OS** if team sales stall—use PLG passive capture to seed T01 expansion.

## Reversal conditions

Reverse primary if within 90 days:
1. 5 consecutive design-partner candidates refuse because Linear/Notion AI already suffices; or
2. Security buyers only want a reverse proxy and reject work/evidence semantics; or
3. Production-path trusted-work proof cannot be demoed reliably.

## Explicitly not the wedge

Pure MCP gateway, Slack clone, federation network, Domain Pack marketplace, or “second brain” consumer app.
