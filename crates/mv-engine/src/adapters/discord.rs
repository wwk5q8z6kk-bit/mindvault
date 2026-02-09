//! Discord adapter — sends messages via webhook URL.
//!
//! Configuration keys:
//! - `webhook_url`: Discord webhook URL for outbound messages

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use mv_core::{MvError, MvResult};

use super::{
    AdapterConfig, AdapterInboundMessage, AdapterOutboundMessage, AdapterStatus, AdapterType,
    ExternalAdapter,
};

#[derive(Debug)]
pub struct DiscordAdapter {
    config: AdapterConfig,
    client: reqwest::Client,
    last_send: Mutex<Option<DateTime<Utc>>>,
    last_error: Mutex<Option<String>>,
}

impl DiscordAdapter {
    pub fn new(config: AdapterConfig) -> MvResult<Self> {
        if config.get_setting("webhook_url").is_none() {
            return Err(MvError::Config(
                "Discord adapter requires 'webhook_url' setting".into(),
            ));
        }
        Ok(Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| MvError::Internal(e.to_string()))?,
            last_send: Mutex::new(None),
            last_error: Mutex::new(None),
        })
    }
}

#[async_trait]
impl ExternalAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn adapter_type(&self) -> AdapterType {
        AdapterType::Discord
    }

    async fn send(&self, message: &AdapterOutboundMessage) -> MvResult<()> {
        let webhook_url = self
            .config
            .get_setting("webhook_url")
            .ok_or_else(|| MvError::Config("missing webhook_url".into()))?;

        // Discord webhooks accept JSON with "content" field
        // Max 2000 chars per message — truncate if needed
        let content = if message.content.len() > 2000 {
            format!("{}...", &message.content[..1997])
        } else {
            message.content.clone()
        };

        let payload = serde_json::json!({
            "content": content,
        });

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MvError::Internal(format!("discord send failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err = format!("discord webhook returned {status}: {body}");
            *self.last_error.lock().unwrap() = Some(err.clone());
            return Err(MvError::Internal(err));
        }

        *self.last_send.lock().unwrap() = Some(Utc::now());
        *self.last_error.lock().unwrap() = None;
        Ok(())
    }

    async fn poll(
        &self,
        cursor: Option<&str>,
    ) -> MvResult<(Vec<AdapterInboundMessage>, String)> {
        // Discord webhooks are outbound-only. Inbound requires a bot with
        // gateway connection, which is beyond the scope of this adapter.
        // Inbound Discord messages would need a separate bot adapter.
        Ok((vec![], cursor.unwrap_or("0").to_string()))
    }

    async fn health_check(&self) -> MvResult<bool> {
        // Test the webhook URL with a GET (Discord returns webhook info)
        let webhook_url = match self.config.get_setting("webhook_url") {
            Some(url) => url,
            None => return Ok(false),
        };

        let resp = self
            .client
            .get(webhook_url)
            .send()
            .await
            .map_err(|e| MvError::Internal(format!("discord health check failed: {e}")))?;

        Ok(resp.status().is_success())
    }

    fn status(&self) -> AdapterStatus {
        AdapterStatus {
            adapter_type: AdapterType::Discord,
            name: self.config.name.clone(),
            connected: self.last_error.lock().unwrap().is_none(),
            last_send: *self.last_send.lock().unwrap(),
            last_receive: None, // Discord webhooks are outbound-only
            error: self.last_error.lock().unwrap().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discord_config_with_webhook() -> AdapterConfig {
        AdapterConfig::new(AdapterType::Discord, "test-discord")
            .with_setting("webhook_url", "https://discord.com/api/webhooks/1234/abcd")
    }

    #[test]
    fn new_requires_webhook_url() {
        // No reqwest::Client created on the error path, so #[test] is fine
        let config = AdapterConfig::new(AdapterType::Discord, "no-webhook");
        let result = DiscordAdapter::new(config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("webhook_url"), "expected webhook_url error, got: {err}");
    }

    #[tokio::test]
    async fn new_succeeds_with_webhook_url() {
        let adapter = DiscordAdapter::new(discord_config_with_webhook());
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn name_returns_config_name() {
        let adapter = DiscordAdapter::new(discord_config_with_webhook()).unwrap();
        assert_eq!(adapter.name(), "test-discord");
    }

    #[tokio::test]
    async fn adapter_type_is_discord() {
        let adapter = DiscordAdapter::new(discord_config_with_webhook()).unwrap();
        assert_eq!(adapter.adapter_type(), AdapterType::Discord);
    }

    #[tokio::test]
    async fn initial_status_is_connected_no_receive() {
        let adapter = DiscordAdapter::new(discord_config_with_webhook()).unwrap();
        let status = adapter.status();
        assert!(status.connected);
        assert!(status.last_send.is_none());
        assert!(status.last_receive.is_none()); // outbound-only
        assert!(status.error.is_none());
        assert_eq!(status.adapter_type, AdapterType::Discord);
    }

    #[tokio::test]
    async fn poll_always_returns_empty() {
        let adapter = DiscordAdapter::new(discord_config_with_webhook()).unwrap();
        let (messages, cursor) = adapter.poll(None).await.unwrap();
        assert!(messages.is_empty());
        assert_eq!(cursor, "0");

        let (messages, cursor) = adapter.poll(Some("custom-cursor")).await.unwrap();
        assert!(messages.is_empty());
        assert_eq!(cursor, "custom-cursor");
    }

    #[test]
    fn message_truncation_at_2000_chars() {
        // Verify the truncation logic directly
        let long_content = "x".repeat(3000);
        let truncated = if long_content.len() > 2000 {
            format!("{}...", &long_content[..1997])
        } else {
            long_content.clone()
        };
        assert_eq!(truncated.len(), 2000);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn short_message_not_truncated() {
        let content = "Hello Discord!".to_string();
        let result = if content.len() > 2000 {
            format!("{}...", &content[..1997])
        } else {
            content.clone()
        };
        assert_eq!(result, "Hello Discord!");
    }

    #[test]
    fn message_exactly_2000_chars_not_truncated() {
        let content = "x".repeat(2000);
        let result = if content.len() > 2000 {
            format!("{}...", &content[..1997])
        } else {
            content.clone()
        };
        assert_eq!(result.len(), 2000);
        assert!(!result.ends_with("..."));
    }
}
