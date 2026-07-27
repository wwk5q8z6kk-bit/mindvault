# Architecture Decision Records (ADR Index)

This index keeps ADRs discoverable and lightweight. Each ADR is a single file
under `docs/adr/`.

## ADRs

1. **001-local-first-storage** — Local‑first storage and offline guarantees  
   `docs/adr/001-local-first-storage.md`
2. **002-single-owner-sovereignty** — Personal Vault sovereignty; superseded
   in part by ADR 010
   `docs/adr/002-single-owner-sovereignty.md`
3. **003-hybrid-search-strategy** — Hybrid vector + FTS + graph search  
   `docs/adr/003-hybrid-search-strategy.md`
4. **004-encryption-at-rest** — Encryption at rest (AES‑256‑GCM + Argon2id)  
   `docs/adr/004-encryption-at-rest.md`
5. **005-wasm-plugin-sandbox** — WASM plugin runtime and isolation  
   `docs/adr/005-wasm-plugin-sandbox.md`
6. **006-embedding-provider-abstraction** — Embedding provider abstraction  
   `docs/adr/006-embedding-provider-abstraction.md`
7. **007-tauri-sveltekit-frontend** — Tauri + SvelteKit UI stack  
   `docs/adr/007-tauri-sveltekit-frontend.md`
8. **008-file-first-knowledge-workspace** — Canonical Markdown folders/files
   with structured and search projections
   `docs/adr/008-file-first-knowledge-workspace.md`
9. **009-workspace-identity-and-mutation-boundary** — Stable document identity,
   managed manifest, and guarded workspace writes
   `docs/adr/009-workspace-identity-and-mutation-boundary.md`
10. **010-personal-vault-and-collaborative-spaces** — Sovereign Personal Vaults
    plus governed shared Spaces for humans and agents
    `docs/adr/010-personal-vault-and-collaborative-spaces.md`
11. **011-sovereign-interoperability-fabric** — Logically central, physically
    decentralized context fabric governed by source authority and open
    protocols
    `docs/adr/011-sovereign-interoperability-fabric.md`
12. **012-governed-agent-execution-graph** — Work Orders, Agent Runs, and
    Artifacts with grant-bound write scope, lease-based conflict control, and
    risk-scaled verification gates; supersedes the `plans` schema
    `docs/adr/012-governed-agent-execution-graph.md`
13. **013-admin-authority-grant-apis** — Lift the grant-API deferral for
    admin-only issue / suspend / revoke / resume so `enforce` is usable
    after local Context Node registration
    `docs/adr/013-admin-authority-grant-apis.md`

14. **014-versioned-action-envelope** — Kernel-scoped action envelope validating
    action/correlation IDs, principal, actor, resource, operation, grant IDs,
    and policy decision on mutating node commands (IK-002)
    `docs/adr/014-versioned-action-envelope.md`
