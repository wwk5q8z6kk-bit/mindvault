# MindVault Docs

- `onboarding.md`: First-run setup and verification steps.
- `architecture/system-overview.md`: High-level MindVault architecture diagram and data flow.
- `architecture/components.md`: Component boundaries and invariants (legacy RLM notes).
- `architecture/invariants.md`: Runtime invariants (legacy RLM notes).
- `plugin-development.md`: WASM plugin authoring and lifecycle.
- API docs: `/api/docs` (Swagger UI) and `/api/openapi.json` (OpenAPI JSON).
- Sharing API: `/api/v1/shares` and `/public/shares/{token}` (see REST OpenAPI).
- Google Calendar sync: `/api/v1/calendar/google/*` endpoints (see onboarding).
- AI sidecar proxy: `/api/v1/ai/*` endpoints (see README for config).
- Comments API: `/api/v1/nodes/{id}/comments` (resolve/delete subroutes).
- MCP marketplace: `/api/v1/mcp/connectors` registry endpoints.
- Meeting notes AI: `POST /api/v1/assist/transform` with `mode=meeting`.
- `security.md`: Security model and keychain concepts.
- `performance.md`: Performance considerations and tuning.
