# ADR 008: Tauri versus Native Clients

Status: proposed

## Problem

Choose a desktop-client path while the backend contracts and primary workflow are still changing.

## Current state

SvelteKit has broad UI coverage; Tauri is a thin webview shell using a separately running localhost
server. Packaging, auth lifecycle, accessibility, signing, and notarization are unproved.

## Alternatives

1. harden Tauri/Svelte;
2. native SwiftUI on macOS;
3. multiple native clients;
4. browser-only client.

## Measurements required

startup/RSS/bundle size, editor/graph performance, accessibility completion, integration effort,
server lifecycle reliability, release/signing burden, and user-task success.

## Security implications

Tauri requires strict CSP/capabilities and authenticated local IPC. Native reduces web exposure but
does not remove backend/auth risk.

## Privacy implications

Either client must avoid browser secret storage and make cloud disclosures visible.

## Migration implications

Define stable API/view models and keep UI logic out of canonical core. Harden the current client for
the slice before considering replacement.

## Chosen direction

Retain Tauri/Svelte for the first slice, remove unsafe secret storage, establish server/session
lifecycle, tighten CSP/capabilities, and pass accessibility/release gates.

## Rejected alternatives

A native rewrite now duplicates a large UI before the core contract stabilizes. Browser-only is not
the sovereign desktop product goal.

## Reversal path

A versioned local API and open design tokens/view models permit a later native client without data
migration.

## Acceptance criteria

Signed/notarized test package, one-click local lifecycle, authenticated events, no localStorage
secrets, WCAG/VoiceOver slice pass, and startup/RSS budgets.
