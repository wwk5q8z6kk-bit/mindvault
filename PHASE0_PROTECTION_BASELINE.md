# Phase 0 Protection Baseline

Status: **recorded, with blockers**

Baseline date: 2026-07-25

Repository: `https://github.com/wwk5q8z6kk-bit/mindvault.git`

Public baseline commit: `dcd7dbdb8c87ca01e22777f79e8f196b993fca23`

## Protected source state

The user's active local MindVault checkout was inspected but not edited by this audit. Its absolute
account path is omitted from the publishable report.

| Item | Recorded state |
|---|---|
| Branch | `main` |
| HEAD | `dcd7dbdb8c87ca01e22777f79e8f196b993fca23` |
| Upstream | `origin/main`, same commit |
| Tracked changes | 38 modified files; none staged |
| Untracked changes | 19 files at initial capture, including migration `030_conversation_turn_sources.sql` |
| Active diff | 2,052 insertions, 465 deletions |
| Diff SHA-256 | `64247cbadcdb1deb2d0cead05709c99a3396cbfd98a6639a39e9d0a682701c05` |
| Tags | None |
| Submodules | None |
| Git LFS declaration | None found |
| Ignore population | Approximately 73,897 paths, dominated by `target/` and `frontend/node_modules/` |
| Audit worktree | Separate local Git worktree; account path omitted |
| Audit branch | `audit/phase0-3-baseline` |

At the capture-preservation recheck, the tracked diff, HEAD, branch, and staging state still matched
the baseline exactly. The untracked count had increased to 20 because
`.cursor/hooks/state/continual-learning.json` appeared in the active checkout during the audit.
The audit did not create, read into a report, or modify that local state file; it was included in a
redacted secret scan.

At the later package-validation recheck on 2026-07-25, concurrent workspace activity had switched
the original checkout to `feat/product-evolution-session` at
`ef19da3187e399a2fdbe04c6c6b33b0d8a371f06`. It then had no tracked changes and two untracked
entries. This audit did not perform that switch or those intervening commits. The evidence and
decisions in this package remain scoped to `dcd7dbd` plus the captured dirty-worktree state; they do
not audit the later branch. Reconcile and preserve that new lineage, then rerun affected evidence,
before treating this package as a current implementation baseline.

The exact uncommitted inventory belongs in the backup manifest produced by
[BACKUP_AND_RESTORE_PROCEDURE.md](BACKUP_AND_RESTORE_PROCEDURE.md). Do not reconstruct it from
this document.

## Ignored and local-only material

The ignored-path inventory contained 73,897 paths: 53,847 under Rust `target/` and 20,050 under
`frontend/` dependency/build output (`node_modules` and `.svelte-kit`). No other ignored top-level
location was present. These paths are non-canonical build inputs/caches, but the full-worktree
backup procedure retains them because deleting or regenerating user-local build state is outside
the audit's authority.

The unignored `.cursor/hooks/state/continual-learning.json` is local tool state, not product source.
Repository-local `.env`, private-key, database, and credential files were not found by the
tracked/untracked filename checks. Runtime-local configuration is inventoried below; the default
`~/.mindvault` root was absent on this host.

## Public-license boundary

The GitHub API reported the repository as public and unarchived at the final recheck. Its detected
license field was empty; the exact commit's root `LICENSE` text is Apache-2.0. The Rust workspace
packages generally declare Apache-2.0, but the Tauri package and Python package declare MIT. This
is an unresolved repository-wide licensing boundary, not evidence that the whole work is dual
licensed. No later private work, rename, or relicense may erase obligations attached to already
published code or third-party material. The recorded root-license SHA-256 is
`cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`.

Authorship is not sufficiently attributable from Git history: 133 of 138 commits use the generic
author identity “MindVault Contributors”; five use a GitHub identity. `ATTRIBUTIONS.md` does not
cover the dependency estate. Resolve actual authorship and contributor agreements before any
relicense.

## Scan results

Generated evidence is under `audit/evidence/`.

| Check | Result |
|---|---|
| Git history secret scan | Two `demo-token` documentation examples in `RUNBOOK.md`; classified as false positives |
| Working-tree diff secret scan | No gitleaks findings |
| Untracked-file secret scan | No gitleaks findings across 20 files at the final pass; repeat immediately before publication |
| Tracked private artifacts | No tracked `.env`, private key, database, or credential artifact found by filename/pattern scan |
| Personal paths in tracked source | None found in source content |
| Live-process exposure | A running editor worker exposed a live credential in its command-line arguments; rotate it and stop passing credentials through argv |
| SBOM | CycloneDX 1.6 document at `audit/SBOM.cdx.json`, 1,843 components |
| Vulnerabilities | Trivy found 217 lockfile occurrences / 203 unique IDs: 3 critical, 56 high, 114 medium, 44 low; all 55 unique critical/high IDs are dispositioned in `VULNERABILITY_REACHABILITY_AND_DISPOSITION.md`, but remain unresolved |
| License metadata | Collected at `audit/evidence/trivy-licenses.json` and Cargo metadata snapshots |

Never copy a live credential into this repository or an audit report. Treat shell history, process
lists, crash reports, logs, and screenshots as possible disclosure channels.

The critical findings are associated with locked `wasmtime` 27.0.0, Python `nltk` 3.9.2, and
Python `torch` 2.4.1. Scanner severity is an intake signal, not proof of exploitability or
reachability. The full timestamped result is
`audit/evidence/trivy-vulnerabilities.json`; every critical/high finding needs a reachable-path,
fixed-version, and release-disposition review. The scan excluded development/test dependencies by
Trivy's default behavior and excluded installed `target/`/`node_modules` trees.

## Persistent-data inventory

Locations are defaults or code-derived; deployments may override them.

| Location | Format | Role | Canonical? | Backup requirement |
|---|---|---|---|---|
| `~/.mindvault/data/mindvault.sqlite` plus `-wal`/`-shm` | SQLite | Nodes, relationships, proposals, policies, operational tables | Yes, in the legacy system | Consistent SQLite snapshot |
| `~/.mindvault/data/keychain.sqlite` plus sidecars | SQLite + encrypted values | Credentials, key metadata, keychain audit | Yes | Separate, security-sensitive snapshot |
| `~/.mindvault/data/blobs/` | Files | Attachments/evidence payloads | Yes | Byte-for-byte copy with hashes |
| `~/.mindvault/data/.sealed/` | Files/directories | Sealed-runtime material | Yes when present | Copy without transformation |
| `~/.mindvault/data/tantivy/` or configured equivalent | Tantivy index | Full-text index | Derived | Rebuild; optional cache backup |
| `~/.mindvault/data/lancedb/` or configured equivalent | LanceDB | Embeddings/vector index | Derived | Rebuild; optional cache backup |
| `~/.mindvault/config.toml` | TOML | Runtime configuration | Yes for reproducibility | Copy with restrictive permissions |
| `~/.mindvault/plugins/` | Manifests/WASM/config | Installed plugin material | Operational input | Copy and hash |
| `~/.mindvault/backups/` | Tar/encrypted backup variants | Legacy backups | Archive | Retain; do not assume complete |
| `~/.mindvault/mindvault.sock` | Unix socket | Ephemeral transport | No | Exclude |
| `~/.mindvault/ai-config.toml` | TOML | Python AI service config | Yes for reproducibility | Copy; classify secrets |
| `~/.mindvault/models/` | Model files | Local model artifacts | Replaceable or licensed cache | Inventory licenses/checksums |
| `~/.mindvault/finetune/` | Model/training outputs | Potentially private derived data | Treat as sensitive | Copy with provenance |
| Browser/Tauri localStorage | Web storage | Preferences, API/provider settings, health state | Operational; currently may contain a plaintext AI key | Export, then remove secret use |
| Browser IndexedDB/Dexie | IndexedDB | Local caches and UI state | Derived/operational | Export only if user state matters |
| Source Obsidian/filesystem vaults | Markdown and attachments | Human-authored source evidence | User-owned canonical source | Back up independently |

At audit time, no `~/.mindvault` directory existed on this machine. That absence is not a product
guarantee and does not eliminate the inventory above.

## Reproducibility baseline

Observed machine tools:

- Rust/Cargo 1.95.0
- Node.js 26.0.0, npm 11.12.1, pnpm 10.28.2
- Python 3.14.5
- Protocol Buffers compiler 34.1
- SQLite 3.51.0

These are observations, not supported-version declarations. Lockfiles exist, but no toolchain files
pin Rust, Node, pnpm, Python, or protoc. See
[LEGACY_BUILD_AND_VALIDATION_RUNBOOK.md](LEGACY_BUILD_AND_VALIDATION_RUNBOOK.md).

## Baseline validation result

The public baseline is not green. The GitHub Actions run for the exact commit failed Rust Format,
sealed-gates setup, and OpenClaw connector jobs. A local `cargo fmt --all -- --check` also failed.
This audit deliberately did not format product source because that would create a large,
non-audit mutation and could overlap the user's work.

## Protection gates

Before any implementation:

1. Capture and verify a complete active-worktree backup.
2. Rotate the credential exposed in process arguments.
3. Resolve root/package license declarations and contributor provenance.
4. Triage and remediate or explicitly disposition every critical/high vulnerability finding.
5. Produce a consistent, restorable backup of every canonical location.
6. Preserve the current public commit and publication evidence.
7. Keep v2 work isolated from the dirty user worktree.
