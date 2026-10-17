# ADR 011: Plugin Sandbox

Status: proposed

## Problem

Allow extension without granting arbitrary access to private context, files, network, secrets, or
canonical writes.

## Current state

Manifest/permission and Wasmtime code exists; execution is behind an optional feature not enabled by
the server. Fuel/output limits exist, but complete memory/time/network/signature policy is absent.

## Alternatives

1. in-process Rust plugins;
2. WASM capability sandbox;
3. isolated OS processes;
4. no plugins.

## Measurements required

startup/call latency, memory, cancellation, quota enforcement, crash containment, signature/update
flow, and abuse-case tests.

## Security implications

No ambient authority. Host capabilities are typed, scoped, revocable, metered, and audited.
Canonical mutation capability only submits proposals.

## Privacy implications

Plugins receive minimum selected records/evidence and cannot enumerate vault paths or raw secrets.
Network permission names destinations and data classes.

## Migration implications

Disable execution by default, version/sign manifests, replace path access with broker calls, and
move high-risk/native helpers to isolated processes.

## Chosen direction

Use WASM for pure/portable plugins plus isolated processes for explicitly approved native tools,
both behind the same capability broker. No plugin execution in the first slice.

## Rejected alternatives

In-process native plugins have unacceptable ambient/crash risk. A permanent no-plugin stance is
unnecessary, but execution stays disabled until gates pass.

## Reversal path

Revoke/uninstall the plugin; canonical data remains in core formats and derived output is identified
by plugin/version.

## Acceptance criteria

Signed manifest, deny-by-default capabilities, memory/fuel/wall-time/output limits, no ambient
FS/network/secrets, proposal-only mutation, audit, revocation, and hostile-plugin test suite.
