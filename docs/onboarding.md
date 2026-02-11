# MindVault Onboarding Guide

## 1) Build and start the server

```bash
cargo build --workspace
cargo run -p mv-cli -- server start --foreground
```

REST defaults to `http://127.0.0.1:9470`.

## 2) Create your owner profile

Use the UI at `/settings/profile` or set config/env:

```toml
[profile]
display_name = "MindVault Owner"
primary_email = "me@example.com"
timezone = "UTC"
signature = "— Sent from MindVault"
```

## 3) Choose embeddings (default: local)

Local embeddings are the default provider:

```toml
[embedding]
provider = "local_fastembed"
model = "bge-small-en-v1.5"
```

If you build without `local-embeddings`, the system falls back to no-op embeddings.

## 4) Optional: enable encryption at rest

```toml
[encryption]
enabled = true
```

Initialize and unseal the vault:

```bash
mv keychain init
mv keychain unseal
```

## 5) Optional: enable admin auth (recommended)

Set a shared admin token (or JWT) to lock down the API before issuing delegated credentials:

```bash
export MINDVAULT_AUTH_TOKEN="replace-with-a-long-random-token"
export MINDVAULT_AUTH_ROLE="admin"
```

Restart the server after setting env vars.

## 6) Issue access keys or OAuth clients

Use the UI at `/settings/profiles` or CLI:

```bash
mv profile create --name "personal-agent"
```

## 7) Optional: create a public share link

Create a read-only share URL for a node:

```bash
curl -X POST http://127.0.0.1:9470/api/v1/shares \
  -H "Content-Type: application/json" \
  -d '{"node_id":"<UUID>"}'
```

The token is shown once; revoke and recreate to rotate.
Open the returned URL in a browser to view the read‑only page, or request JSON with `Accept: application/json`.

## 8) Enable adapters (email, Slack, Discord)

Email example:

```toml
[email]
enabled = true
imap_host = "imap.example.com"
smtp_host = "smtp.example.com"
```

Secrets:

```bash
mv secret set MINDVAULT_EMAIL_IMAP_PASSWORD
mv secret set MINDVAULT_EMAIL_SMTP_PASSWORD
```

## 9) Optional: Google Calendar sync

Configure OAuth credentials and calendar settings:

```toml
[google_calendar]
enabled = true
namespace = "calendar"
calendar_id = "primary"
import_events = true
export_events = false
```

Environment variables:

```bash
export MINDVAULT_GOOGLE_CALENDAR_CLIENT_ID="..."
export MINDVAULT_GOOGLE_CALENDAR_CLIENT_SECRET="..."
export MINDVAULT_GOOGLE_CALENDAR_REFRESH_TOKEN="..."
```

Run a manual sync and check status:

```bash
curl -X POST http://127.0.0.1:9470/api/v1/calendar/google/sync
curl http://127.0.0.1:9470/api/v1/calendar/google/status
```

## 10) Optional: AI sidecar proxy

Enable the AI sidecar proxy if you run a local OpenAI‑compatible service:

```toml
[ai_sidecar]
enabled = true
base_url = "http://127.0.0.1:8100"
timeout_secs = 30
```

Then check:

```bash
curl http://127.0.0.1:9470/api/v1/ai/health
```

## 11) Verify health + diagnostics

```bash
curl http://127.0.0.1:9470/api/v1/health
curl http://127.0.0.1:9470/api/v1/diagnostics/embedding
curl http://127.0.0.1:9470/api/v1/diagnostics/health
```

## 12) Explore API docs + metrics

Swagger UI and OpenAPI JSON are served by the API:

```bash
open http://127.0.0.1:9470/api/docs
curl http://127.0.0.1:9470/api/openapi.json
```

Prometheus-style metrics:

```bash
curl http://127.0.0.1:9470/metrics
curl http://127.0.0.1:9470/api/v1/metrics/summary
```

## 13) Optional: Meeting notes AI + comments

Meeting-style transforms:

```bash
curl -X POST http://127.0.0.1:9470/api/v1/assist/transform \
  -H "Content-Type: application/json" \
  -d '{"mode":"meeting","text":"Discussed launch risks. Decided to delay by 1 week. Next: update timeline?"}'
```

Node comments:

```bash
curl -X POST http://127.0.0.1:9470/api/v1/nodes/{node_id}/comments \
  -H "Content-Type: application/json" \
  -d '{"body":"Capture the decision rationale."}'
```
