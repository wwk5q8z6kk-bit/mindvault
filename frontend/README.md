# MindVault Frontend (Tauri + SvelteKit)

This is the desktop-first MindVault UI powered by SvelteKit + Tauri.

## Key Features

- **Rich Note Editor** (TipTap + Markdown canonical)
- AI assist endpoints: autocomplete, suggestions, transform, wiki-linking
- Task management dashboards and command palette
- Local-first, offline-friendly UX

## Development

```sh
pnpm install
pnpm dev
```

## Tauri

```sh
pnpm tauri:dev
```

## Tests

```sh
pnpm check
pnpm test:unit -- --run
```

## Notes

- Markdown is the canonical storage format.
- Rich editor supports WYSIWYG/Markdown/Split modes.
- AI endpoints are served by the Rust backend under `/api/v1/assist/*`.
