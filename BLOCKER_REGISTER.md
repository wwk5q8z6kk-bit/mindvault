# Prioritized Blocker Register

| ID | Priority | Blocker | Evidence | Required exit |
|---|---|---|---|---|
| B-001 | P0 | Original active work and runtime data are not yet captured in two verified backups | Dirty worktree; no runtime directory on this host | Procedure executed, hashes verified, restore rehearsed |
| B-002 | P0 | Live credential exposed through a process argument | Read-only process inspection during audit | Credential rotated; process/config changed; history/log review complete |
| B-003 | P0 | Public baseline fails CI and local format check | Exact-commit GitHub run and local check | Each required lane green or explicitly waived with owner/risk |
| B-004 | P0 | Canonical and derived writes/deletes are non-atomic | `ingest.rs` operation order | Durable outbox/tombstone design accepted and failure-injection tests specified |
| B-005 | P0 | AI/direct store paths bypass proposals | auto-tag, sync, intent, enrichment | Complete writer inventory; one enforceable canonical mutation capability |
| B-006 | P0 | Backup/restore cannot prove full-vault recovery | database-only and archive implementations | Consistent full manifest; new-root restore; integrity/query/hash equivalence |
| B-007 | P0 | Original evidence is not immutably preserved | Obsidian importer and attachment storage | Evidence schema/content hashing/retention contract accepted |
| B-008 | P0 | License/authorship boundary is unresolved | Apache root vs MIT packages; generic Git authors | Package license map, contributor provenance, notices, legal disposition |
| B-009 | P0 | Critical/high dependency findings are dispositioned but unresolved | [Reachability report](VULNERABILITY_REACHABILITY_AND_DISPOSITION.md): active `lz4_flex`; conditional Intel Torch/model loading; default NLTK; optional/dormant and target-specific findings | Patch/isolation plus verified tests or named time-bounded acceptance for all 55 critical/high advisory IDs |
| B-010 | P0 | Desktop auth is incompatible with normal HTTP/WebSocket paths | frontend API/WS clients | Shipped authenticated local session tested end-to-end |
| B-011 | P0 | Secret stored in browser localStorage | settings page | Migrated to keychain broker; cleanup/rotation UI; negative test |
| B-012 | P1 | Migration ledger is incomplete/non-contiguous | migrations 025–028/keychain runner | Checksummed ordered ledger, fixtures, resume/rollback tests |
| B-013 | P1 | Sync bypasses indexes and durable causality | `sync/mod.rs` | Quarantined or redesigned as signed durable proposal/version replication |
| B-014 | P1 | Sealed data may leak into derived indexes/extraction/logs | split encryption/index paths | Lock/delete/rotation leakage tests pass |
| B-015 | P1 | Plugin/connector isolation and trust are unproved | optional WASM feature/manifest paths | Brokered capability sandbox, quotas, signatures, abuse tests |
| B-016 | P1 | Global audit is default-off and not tamper evident | `audit.rs` | Durable hash-chained transactional audit with verification/export |
| B-017 | P1 | No relevance/performance/memory/startup baseline | all 12 mandatory adapters are still required | Correctness adapters complete and framework run on a provisioned reference host |
| B-018 | P1 | Accessibility is not release-gated | UI audit | WCAG 2.2 AA automated/manual evidence for vertical slice |
| B-019 | P1 | Transport surface exceeds verified consumers | REST/WS/gRPC/UDS | One supported local API selected; others quarantined or parity-tested |
| B-020 | P2 | Documentation overstates completion | feature truth matrix | Capability claims link to current verification artifacts |
| B-021 | P0 | The implementation lineage moved after baseline capture and is not covered by this audit | Original checkout changed from captured `main`/`dcd7dbd` dirty state to `feat/product-evolution-session`/`ef19da3` during package validation | Preserve and compare both lineages; map commits to captured work; rerun changed architecture, security, license, migration, test, and benchmark evidence; explicitly select the v2 starting point |

No P0 blocker may be deferred into v2 implementation.
