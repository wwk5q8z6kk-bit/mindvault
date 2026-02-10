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

## 5) Issue access keys or OAuth clients

Use the UI at `/settings/profiles` or CLI:

```bash
mv profile create --name "personal-agent"
```

## 6) Enable adapters (email, Slack, Discord)

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

## 7) Verify health + diagnostics

```bash
curl http://127.0.0.1:9470/api/v1/health
curl http://127.0.0.1:9470/api/v1/diagnostics/embedding
```
