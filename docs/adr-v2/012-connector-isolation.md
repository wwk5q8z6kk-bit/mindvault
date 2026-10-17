# ADR 012: Connector Isolation

Status: proposed

## Problem

Connectors handle hostile remote data and powerful credentials while importing/exporting private
context.

## Current state

Connector packages, provider records, polling state, calendar/relay/webhook/MCP paths exist, but
isolation, uniform grants, and credential use are not end-to-end proven.

## Alternatives

1. connectors in the server;
2. isolated worker per connector;
3. remote connector service;
4. no connectors.

## Measurements required

worker startup/RSS, throughput, retry/idempotency, credential rotation, crash recovery, rate limits,
and malicious-payload tests.

## Security implications

Separate workers reduce blast radius. The broker grants named operations, source scopes,
destinations, quotas, and proposal submission—not database/key access.

## Privacy implications

Every import/export has a data-flow policy and disclosure log; connector telemetry excludes content
by default.

## Migration implications

Wrap current connectors with a versioned envelope, move credentials to the keychain broker, and
quarantine connectors that require ambient environment or vault access.

## Chosen direction

Run each connector as an isolated, supervised least-privilege worker communicating with a brokered
local API. Connector imports become evidence and proposals according to source policy.

## Rejected alternatives

In-server connectors share crashes/secrets. A mandatory remote connector service violates
local-first operation. Removing all connectors blocks legitimate optional use.

## Reversal path

Revoke the grant and stop the worker; source evidence and audit remain readable, and connector state
is separately exportable/removable.

## Acceptance criteria

No raw database/root key, explicit capabilities and egress domains, idempotent cursor/retry,
resource quotas, signed package/SBOM, revocation, and hostile connector integration tests.
