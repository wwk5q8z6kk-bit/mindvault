# Extension Security Model

- **Status:** Required architecture contract
- **Effective:** 2026-07-26
- **Governing law:** `INTEROPERABILITY_CONSTITUTION.md`

## Scope

This contract governs connectors, agents, model providers, transformers,
indexers, storage providers, automations, views, commands, policy packs,
importers, exporters, notification channels, authentication providers, and
domain modules.

Extensions are untrusted by default. Being installed, signed, popular, or
first-party does not grant database, secret, network, file, context, or action
authority.

## Trust classes

| Class        | Meaning                                              |
| ------------ | ---------------------------------------------------- |
| `official`   | Maintained by the product team                       |
| `verified`   | Identity, conformance, and security review completed |
| `community`  | Public and signed without formal audit               |
| `enterprise` | Approved by an organization administrator            |
| `local`      | Installed for one owner or device                    |
| `unverified` | Explicit warning and narrow sandbox required         |
| `blocked`    | Revoked, malicious, or incompatible                  |

Trust class informs policy but never replaces declared capabilities and grants.
No mandatory central registry is required.

## Manifest contract

Every extension declares a signed, versioned manifest with:

```text
manifest_version
extension_id
extension_type
version
publisher_identity
artifact_digest
entrypoints
protocols
capabilities
context_permissions
action_permissions
network_destinations
secret_slots
file_scopes
data_classes
retention
schemas_produced
schemas_consumed
ui_surfaces
compatibility
update_policy
audit_requirements
signature
```

Undeclared fields never widen authority. Unknown security-relevant fields or
versions fail closed.

## Installation lifecycle

1. Resolve package and publisher through an allowed source.
2. Verify digest, signature, manifest schema, and compatibility.
3. Display requested context, action, network, secret, file, retention, and
   data-class permissions separately.
4. Run static and conformance checks appropriate to the trust class.
5. Install into an isolated, versioned location.
6. Issue no standing credentials; register brokered secret and grant handles.
7. Activate only after explicit policy approval.
8. Record install, upgrade, permission change, disable, and removal receipts.

Updates that add permissions require new approval. Rollback preserves the
previous signed package and migration evidence.

## Runtime boundaries

- Extensions never connect directly to canonical databases.
- All reads use the query gateway and a Context Grant.
- All mutations and effects use the command bus, Tool Grant, policy decision,
  transactional outbox, and action envelope.
- Secrets are resolved by a broker for a specific destination and operation;
  raw secret values are not exposed to models.
- Network access is destination-, method-, protocol-, and data-class scoped.
- File access uses explicit handles or mounted scopes, not ambient host paths.
- CPU, memory, wall time, output size, concurrency, and action rate are bounded.
- Extension logs are structured, size-limited, redacted, and correlated.

The existing WASM permission gate and fuel metering are reusable foundations,
not proof of this complete boundary.

## User-interface tiers

| Tier             | Allowed surface                                            |
| ---------------- | ---------------------------------------------------------- |
| `declarative`    | Platform-rendered cards, tables, forms, settings, commands |
| `sandboxed_web`  | Isolated frame/webview with strict CSP and message API     |
| `trusted_native` | Signed and explicitly approved first-party/enterprise code |

No tier receives ambient application JavaScript access or direct database
handles. UI requests pass through the same command/query contracts as any
other client.

## Prompt-injection containment

Imported content is labeled as one of:

- data;
- instructions from an authenticated principal;
- untrusted embedded instructions;
- executable requests;
- verified policy.

Content cannot grant itself authority. An external page, transcript, message,
tool description, Agent Card, MCP server, or connector response is data until a
trusted policy path classifies it otherwise. Tool descriptions and model output
remain untrusted inputs to authorization.

## Supply-chain and revocation controls

- Packages are content-addressed and signatures bind manifest to artifact.
- Registries publish discovery and trust metadata, not irrevocable permission.
- Known-bad publisher, digest, version, endpoint, or key can be blocked locally
  or by organization policy.
- Revocation stops new execution, credentials, subscriptions, and deliveries.
- Removal deletes extension-owned caches according to policy but does not
  delete independently canonical user data.
- Side-loaded and development extensions are clearly identified and cannot
  silently enter verified trust classes.

## Required tests

- manifest/schema compatibility and unknown-field fail-closed behavior;
- signature, digest, downgrade, and permission-escalation tests;
- network, file, secret, context, action, and resource-limit isolation;
- prompt-injection and confused-deputy fixtures;
- crash/retry/idempotency at inbox, command, outbox, and receipt boundaries;
- uninstall/reinstall without canonical-data corruption;
- audit correlation from extension request to external provider receipt.
