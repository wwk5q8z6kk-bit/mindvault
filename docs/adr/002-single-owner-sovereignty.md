# ADR-002: Single-Owner Sovereignty Model

## Status
Accepted

## Context
MindVault is a personal knowledge management system. The architecture must decide between multi-tenant SaaS patterns vs. single-owner sovereignty.

## Decision
Adopt a **single-owner model** where:
- One vault = one owner with full control.
- External agents (MCP clients, webhooks, watchers) interact via the Exchange Inbox, submitting proposals that require owner approval.
- Access keys with permission templates gate third-party access (read-only, scoped write, admin).
- The autonomy gate controls how much autonomous action is permitted.
- Federation enables vault-to-vault communication without surrendering sovereignty.

## Consequences
- **Positive:** Owner has complete control over data and agent actions.
- **Positive:** No multi-tenant complexity (auth, isolation, billing).
- **Positive:** Exchange inbox provides a clear boundary between trusted and untrusted actions.
- **Negative:** Collaboration requires federation rather than shared access.
- **Negative:** No built-in team features (by design).
