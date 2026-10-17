# ADR 013: API and Transport Reduction

Status: proposed

## Problem

REST, multiple WebSockets, gRPC, and UDS duplicate auth, policy, schema, testing, and compatibility
burden without demonstrated consumers.

## Current state

Svelte/Tauri uses REST and WebSocket. gRPC and UDS are implemented and started but have no shipped
primary client. Desktop auth does not work uniformly across HTTP/WebSocket.

## Alternatives

1. retain all transports;
2. one versioned localhost HTTP API plus one authenticated event stream;
3. Tauri IPC/UDS only;
4. gRPC only.

## Measurements required

consumer inventory, call/stream latency, startup, auth correctness, schema parity, accessibility
impact, and packaging/cross-platform cost.

## Security implications

Fewer listeners and codecs reduce attack surface. Local API still needs per-install/session auth,
Origin/Host controls, CSRF where relevant, quotas, and consistent policy.

## Privacy implications

One gateway centralizes disclosure and audit. Debug payload logging remains disabled.

## Migration implications

Version DTOs, provide compatibility shims, migrate events to one channel, and stop starting
unneeded listeners by default.

## Chosen direction

Support one versioned local API plus one authenticated event stream for the first slice. Quarantine
gRPC and UDS until a measured consumer requirement exists.

## Rejected alternatives

Keeping all transports multiplies verification. Tauri-only IPC blocks CLI/MCP adapters. gRPC-only
adds browser/webview friction.

## Reversal path

Public domain ports permit a future transport adapter without schema migration.

## Acceptance criteria

Complete consumer map, desktop/CLI/MCP compatibility, one authorization matrix, contract tests,
stream reconnect semantics, and no default unused listener.
