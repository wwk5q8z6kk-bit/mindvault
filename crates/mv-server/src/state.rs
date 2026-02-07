use std::sync::Arc;

use chrono::{DateTime, Utc};
use mv_engine::engine::MindVaultEngine;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Shared application state.
pub struct AppState {
    pub engine: Arc<MindVaultEngine>,
    pub change_tx: broadcast::Sender<ChangeNotification>,
    pub reminder_tx: broadcast::Sender<ReminderNotification>,
    pub webhook_config: WebhookConfig,
}

#[derive(Clone, Debug)]
pub struct ChangeNotification {
    pub node_id: String,
    pub operation: String,
    pub timestamp: String,
    pub namespace: Option<String>,
}

/// Notification for task reminders.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReminderNotification {
    pub node_id: String,
    pub title: Option<String>,
    pub content_preview: String,
    pub due_at: Option<DateTime<Utc>>,
    pub namespace: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub notification_type: String,
}

/// Configuration for webhook notifications.
#[derive(Clone, Debug, Default)]
pub struct WebhookConfig {
    pub reminder_url: Option<String>,
    pub change_url: Option<String>,
    pub timeout_secs: u64,
}

impl WebhookConfig {
    pub fn from_env() -> Self {
        Self {
            reminder_url: std::env::var("MINDVAULT_WEBHOOK_REMINDER_URL")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            change_url: std::env::var("MINDVAULT_WEBHOOK_CHANGE_URL")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            timeout_secs: std::env::var("MINDVAULT_WEBHOOK_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        }
    }
}

impl AppState {
    pub fn new(engine: Arc<MindVaultEngine>) -> Self {
        let (change_tx, _) = broadcast::channel(256);
        let (reminder_tx, _) = broadcast::channel(256);
        Self {
            engine,
            change_tx,
            reminder_tx,
            webhook_config: WebhookConfig::from_env(),
        }
    }

    pub fn notify_change(&self, node_id: &str, operation: &str, namespace: Option<&str>) {
        let _ = self.change_tx.send(ChangeNotification {
            node_id: node_id.to_string(),
            operation: operation.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            namespace: namespace.map(|ns| ns.to_string()),
        });
    }

    /// Send a reminder notification.
    pub fn notify_reminder(&self, notification: ReminderNotification) {
        let _ = self.reminder_tx.send(notification.clone());

        // Fire-and-forget webhook dispatch
        if let Some(ref url) = self.webhook_config.reminder_url {
            let url = url.clone();
            let timeout = self.webhook_config.timeout_secs;
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(timeout))
                    .build()
                    .ok();
                if let Some(client) = client {
                    let _ = client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .json(&notification)
                        .send()
                        .await;
                }
            });
        }
    }
}
