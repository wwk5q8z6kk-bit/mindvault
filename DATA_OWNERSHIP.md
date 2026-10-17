# Data Ownership

The user owns human documents, original evidence, annotations, canonical records, proposals,
decisions, audit history, exports, and locally produced model inputs/outputs unless a separately
accepted third-party license says otherwise.

## Data classes

| Class | Examples | Authority | Portability |
|---|---|---|---|
| Human source | Markdown, frontmatter, attachments | File owner/user | Byte-for-byte plus open manifest |
| Canonical record | IDs, versions, provenance, policy, approvals | SQLite and registered files | SQLite + JSON/JSONL + files |
| Original evidence | Imported bytes, hashes, source URI, observed time | Immutable evidence store | Content-addressed export |
| Derived index | FTS, embeddings, ANN, graph projections | Rebuild workers | Disposable/rebuildable |
| AI artifact | answer, extraction, summary, candidate relation | Derived until approved | Export with model/prompt/policy metadata |
| Secret | provider key, connector token, root key | Keychain/OS secret service | Separate encrypted export |
| Operational telemetry | logs, performance, crash data | Local by default | Retention-limited export/delete |

## Required user rights

The user can inspect, export, back up, restore, delete, and migrate their data without a cloud
service. Deletion semantics distinguish canonical records, immutable evidence subject to retention,
derived caches, and backups. Export includes stable IDs, versions, timestamps, provenance,
proposal decisions, and checksums.

## Prohibitions

- no ownership claim over user knowledge;
- no training or remote telemetry without explicit opt-in;
- no secret in browser localStorage, logs, argv, exports, or proposal diffs;
- no derived index treated as the sole copy;
- no destructive migration without a verified rollback point.
