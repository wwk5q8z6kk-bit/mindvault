# Privacy Contract

## Default

Core processing, storage, indexing, retrieval, audit, and model execution are local by default.
Network egress is denied unless the user enables a named provider or connector.

## Before any disclosure

The system shows:

- destination and operator;
- data categories and exact selected scope;
- purpose;
- model/connector retention and training terms when known;
- credential and transport used;
- redaction/minimization result;
- whether content is sealed, private, or restricted;
- an allow-once or scoped policy decision.

Secrets and encryption keys are never model context.

## Required controls

- namespace and evidence-level authorization before retrieval;
- prompt construction after authorization and classification;
- local redaction/minimization before cloud egress;
- per-provider egress policy and kill switch;
- no telemetry content by default;
- bounded local logs with content-free defaults;
- visible audit of disclosures and returned artifacts;
- revocable connector grants;
- encrypted backups with independent recovery verification.

## Retention

Canonical retention is user-controlled. Derived indexes may be deleted and rebuilt. Cloud retention
is disclosed but cannot be guaranteed by MindVault; unsupported guarantees are never implied.
Audit records have a declared retention policy and are protected from silent modification.

## Current exceptions

The legacy UI stores an AI API key in localStorage, general audit logging is off by default, and
cloud fallback can be selected through environment/config paths without a unified disclosure
ledger. These are blockers, not accepted privacy behavior.
