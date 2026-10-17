# Data Flow Map

## Legacy write flow

```text
human/API/import/agent/sync
        |
        v
KnowledgeNode
        |
        +--> optional AI auto-tags the mutable node
        |
        +--> SQLite insert/update (canonical)
        +--> Tantivy commit
        +--> embedding provider --> LanceDB
        +--> conflict detector --> conflict rows/proposals
        +--> graph relationship writes
        +--> WebSocket notification
```

Each arrow can fail independently. There is no durable outbox describing which derived projections
must catch up to which canonical version.

## Filesystem/Obsidian flow

```text
Markdown file
  -> read UTF-8
  -> parse frontmatter/body/wikilinks
  -> source URI + mtime metadata
  -> SQLite node
  -> derived indexes
```

Missing: byte preservation, content hash, file identity independent of path, observed timestamp,
parser version, delete/rename semantics, attachment evidence, deterministic re-ingestion, and a
user-visible proposal when an AI-derived interpretation differs.

## Attachment flow

```text
multipart bytes
  -> size/name checks
  -> optional encryption
  -> blob file write
  -> external/local extraction
  -> extracted text in node metadata
  -> node update + reindex
```

The blob precedes canonical metadata; later failures can orphan it. Extracted text is mixed into
canonical node metadata even though it is a derived artifact. Delete can remove the blob before
the canonical node deletion commits.

## Retrieval/answer flow

```text
authorized query
  -> FTS / vector / graph candidates
  -> hybrid ranking
  -> optional provider rewrite/HyDE/rerank
  -> answer/chat
  -> citations assembled by UI/server paths
```

Authorization is not demonstrably rechecked on every candidate after all expansion/ranking steps.
Citation records do not yet form an immutable answer-evidence ledger with span and version hashes.

## Sync flow

```text
SQLite node snapshot + in-memory vector clock
  -> remote import
  -> direct SQLite insert/update OR conflict proposal
```

Direct imports bypass FTS/vector/graph maintenance. Device identity, clock, and last-sync state are
not durable enough for authoritative multi-device causality.

## Required v2 flow

```text
source bytes / approved user edit
  -> classify + authorize
  -> immutable evidence (hash, bytes, source, observed_at)
  -> SQLite transaction:
       document version
       provenance edges
       policy result
       audit event
       outbox events
  -> commit
  -> idempotent projectors:
       FTS | vector | graph
  -> retrieval snapshot
  -> citation-backed answer
  -> contradiction candidate
  -> typed proposal
  -> human approval/rejection
  -> compare-and-swap canonical transaction + rollback version
```

Cloud model egress branches only after classification, minimization, explicit policy, and an audit
event.
