# Privacy Data Flow

## Data categories

| Category | Examples | Default classification |
|---|---|---|
| Knowledge | notes, tasks, habits, relationships | private |
| Evidence | files, media, source paths, extracted text | private; potentially highly sensitive |
| Identity | contacts, namespaces, account metadata | private |
| Secrets | model keys, connector tokens, encryption keys | secret; never model context |
| Model data | prompts, embeddings, outputs, fine-tunes | private/derived |
| Operations | logs, audit events, metrics, crash data | sensitive metadata |
| Public share | explicitly published node snapshot | public only for the granted scope/time |

## Trust boundaries

```text
User files
   | local ingestion
   v
Canonical local store ----> local indexes ----> local UI/CLI/MCP
   |                            |
   | classified model request   | retrieval results
   v                            v
Model gateway -----------> local model OR approved cloud provider
   |
Connector gateway -------> calendar/webhook/relay/MCP/other service
```

Tauri WebView, localhost server, Python service, plugin runtime, connector process, extraction
binary, model provider, and public share recipient are distinct principals. “Same machine” is not a
permission.

## Current exposure paths

- remote embeddings/chat/transcription/provider calls;
- connector and webhook payloads;
- public share endpoints;
- MCP read tools;
- model fine-tuning inputs/outputs;
- attachment extraction subprocesses and temp files;
- API keys in environment variables and one UI localStorage path;
- audit/diagnostic logs and process command lines;
- backups and exports;
- unencrypted derived indexes when sealed content is indexed.

## Required controls

Every egress event records actor, source evidence IDs, classification, selected fields, destination,
purpose, policy, redactions, timestamp, and result without recording secrets. Authorization occurs
before retrieval expansion and again before answer assembly. Cloud requests use minimum necessary
spans and cannot silently fall back from local to cloud.

Public shares use immutable snapshots or version pins, high-entropy hashed tokens, expiry,
revocation, rate limits, content-security headers, and a disclosure preview. Plugin/connector
processes receive brokered capabilities rather than the vault path or root key.

## Retention/deletion

Deleting a canonical item tombstones the version and schedules derived-index removal; evidence
retention follows an explicit policy. Backup expiry and cloud-provider deletion are separate
events. The user can enumerate residual copies and rebuild/delete derived data.
