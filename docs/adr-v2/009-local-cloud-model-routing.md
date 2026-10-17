# ADR 009: Local/Cloud Model Routing

Status: proposed

## Problem

Use interchangeable models without silent cloud disclosure or provider-specific authority.

## Current state

Rust providers and a Python AI service support local/remote paths. Some operations fall back to
OpenAI/environment credentials. There is no universal data-classification and disclosure ledger.

## Alternatives

1. local models only;
2. cloud models only;
3. local-first gateway with explicit scoped cloud policy;
4. per-feature provider logic.

## Measurements required

task quality, latency, startup/RSS, disclosure size, offline success, provider failure behavior,
cost, and reproducibility across the first-slice corpus.

## Security implications

The gateway prevents credential spread and enforces classification, authorization, redaction,
timeouts, and output treatment.

## Privacy implications

Cloud is deny-by-default. Each disclosure names destination, selected evidence spans, purpose,
retention/training terms when known, and user/policy approval.

## Migration implications

Route all embedding/chat/extraction/model calls through one port; remove silent environment
fallbacks; migrate credentials to the keychain broker.

## Chosen direction

Local model/algorithm is the default. Cloud use is explicit per provider, classification, and
scope; no silent local-to-cloud fallback. Model output is derived/proposal data.

## Rejected alternatives

Cloud-only violates offline sovereignty. Local-only unnecessarily forbids authorized compute.
Per-feature logic makes policy unenforceable.

## Reversal path

Disable any provider and rebuild model-derived data from canonical evidence with another registered
model.

## Acceptance criteria

Offline slice works, all egress passes the gateway and audit, secrets never enter prompts/logs,
model/version provenance is stored, and denial/failure cannot trigger an alternate cloud.
