# Feature Truth Matrix

Classification is based on production call paths and tests, not filenames or documentation.

| Advertised capability | Classification | Evidence and condition |
|---|---|---|
| SQLite local knowledge store | production-integrated | Main engine reads/writes SQLite; consistency gaps with derived stores remain |
| Human-readable Markdown as canonical | missing | Markdown/Obsidian is copied into SQLite; original bytes/version ledger are not canonicalized |
| Tantivy full-text search | production-integrated | Main ingest commits and recall queries it; no repair/outbox baseline |
| LanceDB vector retrieval | production-integrated | Main engine can upsert/search when configured; model/index/version reproducibility unproved |
| Exact lookup | production-integrated | Node ID/source store paths exist; benchmark absent |
| Graph storage/traversal | production-integrated | SQLite graph store is used by engine and recall |
| Hybrid ranking | implemented but unverified | RRF/boost/rerank code exists; no relevance corpus or regression threshold |
| Citation-backed answers | partial | Citation UI/server helpers exist, including uncommitted user work; no immutable answer-to-evidence ledger |
| Contradiction detection | partial | Best-effort conflict alerts exist; semantics/quality not benchmarked |
| Typed proposals | production-integrated | Persisted proposal model and approve/reject endpoints exist |
| Proposal-only AI writes | missing | Auto-tag, sync, intent, and enrichment bypasses prevent enforcement |
| Visible diff | partial | String/typed payloads exist; not universal or schema-validated for all mutations |
| Rollback/undo | partial | Undo snapshots with retention exist; execution/audit/rollback are not one transaction |
| Tamper-evident global audit | missing | General REST audit is default-off and mutable; keychain audit chaining is narrower |
| Temporal/versioned knowledge | partial | Node temporal version fields exist; no universal event/version ledger or CAS |
| Immutable original evidence | missing | Attachments are mutable path-based blobs; imports do not preserve originals as evidence |
| Sealed/encrypted vault | partial | Encrypted payload/keychain/blob paths exist; derived plaintext and lifecycle guarantees need proof |
| Key management | implemented but unverified | SQLite keychain, OS keyring/env fallbacks, rotation concepts; recovery and endpoint integration unverified |
| Authentication | partial | JWT/shared/access/consumer tokens and bind safety exist; desktop client cannot use configured auth cleanly |
| Authorization/least privilege | partial | roles/namespaces/templates exist; unscoped consumer write and route/path variance remain |
| Local-first operation | production-integrated | Main server/storage can run on loopback without cloud; some features fall back to cloud |
| Explicit cloud data policy | missing | Provider/config paths exist without a unified classification/egress approval ledger |
| Encrypted database-only backup | implemented but unverified | Round-trip unit test; not a coherent full-vault backup |
| Full-vault backup/restore | partial | CLI archive exists; no proved quiesce, manifest, integrity, or complete recovery |
| Sync | prototype | Snapshot/vector-clock code exists but state is in memory, capped, and bypasses indexes |
| Obsidian import | implemented but unverified | Engine-level importer called by CLI; mtime dedup and silent unresolved links |
| Filesystem watcher | prototype | Default-off agent watcher; lifecycle and canonical semantics unproved |
| Attachment extraction | partial | Text/PDF/image/audio/video paths exist; external tools, privacy, failure cleanup, and performance unverified |
| REST API | production-integrated | Main frontend and desktop use it |
| WebSocket events | production-integrated | UI uses change/reminder/agent/collab sockets; auth incompatibility exists |
| gRPC API | implemented but unverified | Server starts service and enforces auth; no shipped UI consumer or parity gate |
| Unix-domain socket | implemented but unverified | Server lifecycle exists; production client path unclear |
| MCP | partial | Read and proposal-oriented tools exist; scope/config fallback needs hardening |
| CLI | production-integrated | Main operator/server/import surface; paths diverge from UI |
| Tauri desktop | partial | Buildable shell in design; external server lifecycle/signing/notarization not proved |
| Svelte UI | implemented but unverified | Broad page/component coverage and specs; no full CI/accessibility/release gate |
| Accessibility | partial | Some labels/semantic elements; no WCAG audit, keyboard/focus/screen-reader proof |
| WASM plugin sandbox | prototype | Wasmtime runtime exists behind feature; server does not enable it by default |
| Plugin marketplace | prototype | Registry/metadata routes exist; trust/signature/install policy incomplete |
| Connectors | prototype | Connector packages and REST records exist; isolation and credential boundaries unproved |
| Python AI service | implemented but unverified | Separate FastAPI service with models/chat/embedding/fine-tune; not in primary CI |
| Tasks/goals/habits/calendar | partial | Significant optional module code; outside core and not end-to-end verified |
| Relay/messaging/federation | prototype | Models/routes exist; security and production deployment not proved |
| Public sharing | partial | Token-hash share routes exist; threat/abuse/privacy controls need verification |
| Release pipeline | partial | CLI release path exists; desktop/package completeness and current CI health fail |
| C++ acceleration | missing | Correctly absent; no benchmark permits introduction |

## Documentation truth

Documentation describes many capabilities as complete that are partial or prototype in the table
above. The existing ADRs are directional records, not evidence that acceptance criteria were met.
Documentation must link to a test/benchmark/release artifact for any “production” claim.
