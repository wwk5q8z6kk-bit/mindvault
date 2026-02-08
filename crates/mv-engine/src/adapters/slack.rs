//! Slack adapter — sends messages via incoming webhook, polls via Web API.
//!
//! Configuration keys:
//! - `webhook_url`: Slack incoming webhook URL for outbound messages
//! - `bot_token`: (optional) Slack Bot Token for polling conversations.history
//! - `channel_id`: (optional) Default channel ID for polling

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use mv_core::{MvError, MvResult};

use super::{
    AdapterConfig, AdapterInboundMessage, AdapterOutboundMessage, AdapterStatus, AdapterType,
    ExternalAdapter,
};

pub struct SlackAdapter {
    config: AdapterConfig,
    client: reqwest::Client,
    last_send: Mutex<Option<DateTime<Utc>>>,
    last_receive: Mutex<Option<DateTime<Utc>>>,
    last_error: Mutex<Option<String>>,
}

impl SlackAdapter {
    pub fn new(config: AdapterConfig) -> MvResult<Self> {
        if config.get_setting("webhook_url").is_none() {
            return Err(MvError::Config(
                "Slack adapter requires 'webhook_url' setting".into(),
            ));
        }
        Ok(Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| MvError::Internal(e.to_string()))?,
            last_send: Mutex::new(None),
            last_receive: Mutex::new(None),
            last_error: Mutex::new(None),
        })
    }
}

#[async_trait]
impl ExternalAdapter for SlackAdapter {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn adapter_type(&self) -> AdapterType {
        AdapterType::Slack
    }

    async fn send(&self, message: &AdapterOutboundMessage) -> MvResult<()> {
        let webhook_url = self
            .config
            .get_setting("webhook_url")
            .ok_or_else(|| MvError::Config("missing webhook_url".into()))?;

        let mut payload = serde_json::json!({
            "text": message.content,
        });

        // Override channel if specified in the message
        if !message.channel.is_empty() {
            payload["channel"] = serde_json::Value::String(message.channel.clone());
        }

        if let Some(ref thread_ts) = message.thread_id {
            payload["thread_ts"] = serde_json::Value::String(thread_ts.clone());
        }

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MvError::Internal(format!("slack send failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err = format!("slack webhook returned {status}: {body}");
            *self.last_error.lock().unwrap() = Some(err.clone());
            return Err(MvError::Internal(err));
        }

        *self.last_send.lock().unwrap() = Some(Utc::now());
        *self.last_error.lock().unwrap() = None;
        Ok(())
    }

    async fn poll(&self, cursor: Option<&str>) -> MvResult<(Vec<AdapterInboundMessage>, String)> {
        let bot_token = match self.config.get_setting("bot_token") {
            Some(t) => t,
            None => return Ok((vec![], cursor.unwrap_or("0").to_string())),
        };

        let channel_id = match self.config.get_setting("channel_id") {
            Some(c) => c,
            None => return Ok((vec![], cursor.unwrap_or("0").to_string())),
        };

        let mut params = vec![("channel", channel_id.to_string()), ("limit", "20".into())];
        if let Some(oldest) = cursor {
            params.push(("oldest", oldest.to_string()));
        }

        let resp = self
            .client
            .get("https://slack.com/api/conversations.history")
            .bearer_auth(bot_token)
            .query(&params)
            .send()
            .await
            .map_err(|e| MvError::Internal(format!("slack poll failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MvError::Internal(format!("slack poll parse failed: {e}")))?;

        if body.get("ok") != Some(&serde_json::Value::Bool(true)) {
            let err = body
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown");
            return Err(MvError::Internal(format!("slack API error: {err}")));
        }

        let messages = body
            .get("messages")
            .and_then(|m| m.as_array())
            .map(|msgs| {
                msgs.iter()
                    .filter_map(|m| {
                        let ts = m.get("ts")?.as_str()?;
                        let text = m.get("text")?.as_str()?;
                        let user = m
                            .get("user")
                            .and_then(|u| u.as_str())
                            .unwrap_or("unknown");
                        let thread_ts = m.get("thread_ts").and_then(|t| t.as_str()).map(String::from);

                        // Parse Slack timestamp (epoch.seq format)
                        let timestamp = ts
                            .split('.')
                            .next()
                            .and_then(|s| s.parse::<i64>().ok())
                            .and_then(|secs| {
                                DateTime::from_timestamp(secs, 0)
                            })
                            .unwrap_or_else(Utc::now);

                        Some(AdapterInboundMessage {
                            external_id: ts.to_string(),
                            channel: channel_id.to_string(),
                            sender: user.to_string(),
                            content: text.to_string(),
                            thread_id: thread_ts,
                            timestamp,
                            metadata: HashMap::new(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Use the latest message timestamp as cursor
        let new_cursor = messages
            .first()
            .map(|m| m.external_id.clone())
            .unwrap_or_else(|| cursor.unwrap_or("0").to_string());

        if !messages.is_empty() {
            *self.last_receive.lock().unwrap() = Some(Utc::now());
        }

        Ok((messages, new_cursor))
    }

    async fn health_check(&self) -> MvResult<bool> {
        // If we have a bot token, test the auth.test endpoint
        if let Some(token) = self.config.get_setting("bot_token") {
            let resp = self
                .client
                .post("https://slack.com/api/auth.test")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| MvError::Internal(format!("slack health check failed: {e}")))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| MvError::Internal(e.to_string()))?;

            return Ok(body.get("ok") == Some(&serde_json::Value::Bool(true)));
        }

        // If only webhook, we can't easily test it — assume healthy if configured
        Ok(self.config.get_setting("webhook_url").is_some())
    }

    fn status(&self) -> AdapterStatus {
        AdapterStatus {
            adapter_type: AdapterType::Slack,
            name: self.config.name.clone(),
            connected: self.last_error.lock().unwrap().is_none(),
            last_send: *self.last_send.lock().unwrap(),
            last_receive: *self.last_receive.lock().unwrap(),
            error: self.last_error.lock().unwrap().clone(),
        }
    }
}
