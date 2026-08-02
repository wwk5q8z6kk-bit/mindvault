# Data Portability Contract

- **Status:** Required architecture contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`

## User right

An owner or authorized organization can export, inspect, validate, move, and
restore its governed data without continued access to a MindVault service.
Portability includes meaning and evidence, not only database rows or rendered
documents.

Export does not override another source’s ownership, license, retention,
privacy, or redistribution restrictions.

## Export classes

| Class              | Purpose                          | Expected content                                                       |
| ------------------ | -------------------------------- | ---------------------------------------------------------------------- |
| `human_portable`   | Direct reading and editing       | Markdown, JSON, HTML, media, standard calendar/contact formats         |
| `machine_portable` | Migration and integration        | Versioned schemas, stable URIs, relationships, grants, source bindings |
| `recovery`         | Complete owned-state restoration | Canonical data, events, receipts, configuration, indexes-as-optional   |
| `context_capsule`  | Scoped sharing                   | Purpose-bound selected context, grants, expiry, provenance, signature  |

A Context Capsule is not a backup. A search index or cache is not an export of
canonical data.

## Portable bundle

Every machine-portable or recovery bundle contains a signed manifest:

```text
format_version
export_id
export_type
issuer_node_id
subject_owners
created_at
scope
schema_catalog
object_inventory
relationship_inventory
source_binding_inventory
artifact_inventory
event_range
receipt_range
policy_inventory
encryption
signatures
checksums
omissions
restrictions
```

Owned resources retain:

- stable resource and actor URIs;
- type and schema version;
- content and version history required by the export scope;
- relationships and ordering;
- source authority and materialization mode;
- provenance, evidence, freshness, and sensitivity;
- deletion/tombstone semantics;
- grants, approvals, policies, and action receipts where authorized;
- artifact hashes, media types, and either bytes or resolvable references.

Secrets, active session tokens, raw key material, and non-exportable provider
credentials are excluded. The manifest explains every material omission.

## Format rules

- Human-authored documents prefer portable source formats, including Markdown,
  HTML, JSON Canvas, and original attachments.
- Structured objects use versioned JSON Schema representations.
- Events use the supported CloudEvents profile plus MindVault extensions.
- Provenance includes a W3C PROV-compatible representation.
- Calendar, contact, email, file, and code domains preserve established formats
  when they can represent the source faithfully.
- Binary artifacts are content-addressed and checksum verified.
- No exported identity depends solely on an internal database row number.

## Import and restore

Import is staged:

1. verify package signatures, hashes, format, and size limits;
2. parse in an isolated temporary domain;
3. validate schemas and extension namespaces;
4. resolve stable identities and detect collisions;
5. present source-authority, policy, and conflict consequences;
6. execute a dry run with a deterministic import plan;
7. commit through the command/event boundary;
8. rebuild projections;
9. produce an import receipt and exception report.

Unknown schemas may be retained as opaque source artifacts but cannot become
canonical executable objects. Import never silently broadens grants, activates
agents, installs extensions, restores credentials, or triggers automations.

## Round-trip guarantees

For supported schema versions:

- export -> import preserves stable identity, owned content, relationships,
  provenance, source authority, and verification status;
- export -> restore can rebuild derived search and graph projections;
- repeated import is idempotent;
- conflicting active ownership requires explicit resolution;
- a failed import leaves the target canonical domain unchanged;
- partial exports identify omitted dependencies and cannot claim completeness.

## Deletion and exit

The product provides:

- complete owned-data export before account, Space, or node deletion;
- machine-readable deletion and retention status;
- credential and grant revocation;
- source disconnection without deletion of independent derived knowledge;
- tombstones where required to prevent accidental resurrection;
- a receipt describing deleted, retained, externally owned, and unreachable
  data.

## Conformance suite

Portability releases require:

- golden fixtures for every public schema version;
- deterministic manifests and checksum validation;
- corrupted, truncated, oversized, malicious-path, and decompression-bomb
  tests;
- cross-version migration and downgrade-loss reports;
- Personal Vault, Space, source-binding, WorkOrder/AgentRun, Trust Ledger, and
  Context Capsule round trips;
- restore without network access for locally owned core data;
- independent parser documentation and at least one non-product validation
  implementation for the open bundle format.
