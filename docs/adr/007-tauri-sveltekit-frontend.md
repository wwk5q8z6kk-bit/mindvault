# ADR-007: Tauri + SvelteKit Frontend

## Status
Accepted

## Context
MindVault needs a desktop application with a modern web-based UI. Options include Electron, Tauri, native frameworks, or web-only.

## Decision
Use **Tauri 2** with **SvelteKit** and **Svelte 5** (runes).

- Tauri provides native window management with Rust backend integration.
- SvelteKit handles routing, SSR-compatible page structure, and build tooling.
- Svelte 5 runes (`$state`, `$derived`, `$effect`) replace Svelte 4 stores for reactive state.
- Dexie (IndexedDB) provides offline-first caching with optimistic updates.
- The frontend communicates with `mv-server` via REST API at `localhost:9470`.

Key patterns:
- Stores in `$lib/stores/` manage state with API sync
- API modules in `$lib/api/` use a shared `fetchJson` client
- 34 pages cover tasks, notes, chat, inbox, calendar, kanban, graph, settings, etc.
- pnpm as package manager (npm fails due to peer dep conflicts)

## Consequences
- **Positive:** Tauri produces small, fast native binaries (~10MB vs Electron's ~150MB).
- **Positive:** Svelte 5 runes provide fine-grained reactivity without boilerplate.
- **Positive:** Web-based UI enables rapid iteration and familiar tooling.
- **Negative:** Tauri 2 ecosystem is newer than Electron's.
- **Negative:** Two runtimes (Rust + Node) complicate the build pipeline.
