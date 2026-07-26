# Security Threat Model

Status: **conditional baseline**. The user has not yet confirmed deployment assumptions. This model
assumes a single expert owner on macOS, loopback-only core service by default, optional cloud
models/connectors, and possible future public sharing/sync. If LAN exposure, multi-user accounts,
or hostile shared-machine users are in scope, authentication, transport, and OS isolation
requirements become release blockers immediately.

## Scope and assets

In scope: Tauri/Svelte UI, Rust REST/WebSocket/gRPC/UDS server, CLI/MCP, SQLite/keychain, files and
blobs, Tantivy/LanceDB, Python AI service, extraction tools, plugins, connectors, sync, backups,
public sharing, and provider egress.

Critical assets:

- human-authored files and immutable evidence;
- canonical database and temporal history;
- root/storage keys and connector/model credentials;
- proposal approvals and audit integrity;
- authorization policy and namespaces;
- backups/exports and local model/fine-tune data;
- availability and correctness of retrieval/citations.

## Trust boundaries

1. untrusted document/attachment → parser/extractor;
2. WebView/browser → localhost service;
3. REST/WebSocket/gRPC/UDS → canonical transaction service;
4. canonical store → derived indexers;
5. core → model/provider;
6. core → plugin/connector/MCP process;
7. local device → sync peer/public recipient;
8. live data → backup/restore media;
9. human approval UI → proposal executor.

## Priority threats

| Priority | Threat scenario | Existing control | Gap | Required mitigation |
|---|---|---|---|---|
| P0 | Malicious/compromised AI, agent, sync peer, or intent path writes canonical knowledge directly | Some MCP mutations create proposals | Multiple direct-store paths exist | One storage capability for canonical mutations; proposal origin/type/base-version checks; deny direct AI credentials |
| P0 | Ingest/delete failure loses evidence or creates inconsistent retrieval | Errors propagate; some vector failures tolerated | No transaction/outbox; destructive delete order | SQLite transaction + durable outbox; tombstone first; asynchronous idempotent cleanup; repair scanner |
| P0 | Credential theft via localStorage, argv, logs, or environment | Separate keychain exists | UI stores an AI key in localStorage; live argv exposure observed | OS/keychain broker; redact logs; prohibit argv/localStorage; rotation workflow |
| P0 | Malicious attachment exploits parser or extraction binary | File size/name checks; some subprocess boundaries | Broad file types, temp files, external tools, no uniform sandbox | Content sniffing, decompression limits, sandboxed worker, no network, CPU/RSS/time quotas, temp cleanup, patched parsers |
| P0 | Backup is corrupt/incomplete and restore overwrites good data | Encryption/HMAC and round-trip unit test | No coherent SQLite snapshot/full manifest/rehearsal | Full manifest, online backup/quiesce, checksums, atomic staged restore, recurring restore test |
| P0 | Auth enabled but desktop/WebSocket cannot authenticate, leading operators to disable it | Loopback bind safety | UI has no uniform bearer/session/WS auth path | Authenticated local session broker or Tauri IPC; origin checks; short-lived tokens; compatible WS protocol |
| P1 | Consumer/access token reads or writes outside intended scope | roles, namespace, templates, policies | Consumer context is unscoped write; enforcement varies | Capability tokens with explicit namespace/action/resource; centralized authorization and negative tests |
| P1 | Cloud model receives sealed/private content through fallback | Provider configs and local options | No universal egress classification/approval choke point | Model gateway with deny-by-default policy, data minimization, no silent fallback, disclosure audit |
| P1 | Derived index leaks sealed plaintext after lock/delete | Sealed payload encryption | FTS/vector/extracted text lifecycle not proven | Per-scope index encryption or exclude sealed data; wipe/rebuild on lock/rotation; leakage tests |
| P1 | Plugin/connector escapes or exfiltrates the vault | Wasmtime fuel and permission host calls exist | Runtime default-off; no complete memory/time/signature/network policy | Separate process/WASM capability broker, signed manifests, memory/epoch limits, no ambient FS/network |
| P1 | Local malicious webpage/process attacks localhost API or WebSockets | loopback default, CORS/bind controls | Auth disabled means admin; WebView CSP uses unsafe-inline | per-install secret/session, strict Origin/Host, CSRF for browser flows, strict CSP, socket permissions |
| P1 | Proposal approval is raced against a changed base | Proposal state and undo snapshot exist | No universal compare-and-swap/atomic audit | base hash/version CAS, idempotency, atomic execute+audit+rollback version |
| P1 | Sync replay/rollback/identity collision corrupts history | vector-clock concepts | random/non-durable device state, direct writes, 10k cap | durable device keys/clock, signed encrypted batches, monotonic sequence, proposal-only conflicts |
| P2 | Public share token abuse or stale disclosure | hashed tokens, expiry/revoke fields | rate limits/content pinning/metadata minimization unproved | immutable version pin, high-entropy token, rate limiting, preview, no ambient linked-data access |
| P2 | Audit evidence is altered or lost | keychain hash chain; optional REST sinks | general audit off/default memory, fire-and-forget webhook | append-only hash-chained durable audit, transactional mutation linkage, export verification |
| P2 | Dependency/build compromise | lockfiles, generated SBOM, timestamped vulnerability result, [call-path disposition](VULNERABILITY_REACHABILITY_AND_DISPOSITION.md) | 3 critical/56 high occurrences remain unresolved; active Tantivy/LZ4 and conditional model-loading paths; no signing/provenance | patch/isolate findings, pinned toolchains, deny policy, signed SBOM/provenance, isolated release builders |

## Abuse cases and security tests

- prompt-injected document asks agent to disclose other namespaces or approve a proposal;
- image/PDF bomb exhausts disk/RSS or launches a vulnerable helper;
- replayed approval executes twice;
- stale citation points to a changed/deleted version;
- attacker webpage calls loopback delete/share endpoints;
- consumer token accesses an unlisted secret or namespace;
- sealed node remains searchable after lock;
- interrupted migration/restore leaves a partially upgraded live database;
- malicious plugin requests path traversal or network access;
- sync peer replays an older signed snapshot.

Each case needs a negative integration test at the real transport and storage boundary.

## Release security gates

- no P0 open;
- threat assumptions confirmed;
- all canonical writes pass the proposal/transaction boundary;
- secrets removed from browser storage and argv;
- full restore rehearsal;
- parser/plugin/connector sandbox limits verified;
- auth works in the shipped desktop client and all retained transports;
- sealed-data index leakage test passes;
- complete critical/high vulnerability disposition, license review, and signed release artifacts.
