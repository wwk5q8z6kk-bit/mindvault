# MindVault Clipper Extension

Chrome/Edge Manifest V3 extension for sending page clips directly to MindVault.

## Features

- Save current page from popup.
- Save highlighted selection via popup, context menu, or keyboard command.
- Popup supports per-save toggles for dedupe and linked note creation.
- Direct API ingestion to `POST /api/v1/clips/import`.
- Optional pre-save enrichment via `POST /api/v1/clips/enrich` (enabled by default).
- Fallback handoff to `MindVault /bookmarks` when API is unavailable.
- Configurable API URL, app URL, namespace, and optional bearer token.

## Install (Developer Mode)

1. Open `chrome://extensions` (or `edge://extensions`).
2. Enable `Developer mode`.
3. Click `Load unpacked`.
4. Select `extensions/mindvault-clipper` from this repo.

## Default Settings

- API base URL: `http://127.0.0.1:9470`
- App fallback URL: `http://localhost:5173`
- Namespace: `default`

If API auth is enabled in MindVault, set the same bearer token in extension options.
`Enrich clip metadata before save` can be toggled in options.

## Keyboard Shortcut

- Default: `Ctrl+Shift+Y` (`Command+Shift+Y` on macOS)
- Opens selection capture flow and saves directly to MindVault.
