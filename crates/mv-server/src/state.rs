use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use mv_engine::engine::MindVaultEngine;
use mv_plugin::{PluginManager, PluginRegistry};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

pub use mv_core::ChangeNotification;
use mv_core::{CapturedIntent, ChronicleEntry, ProactiveInsight};

/// Shared application state.
pub struct AppState {
    pub engine: Arc<MindVaultEngine>,
    pub change_tx: broadcast::Sender<ChangeNotification>,
    pub reminder_tx: broadcast::Sender<ReminderNotification>,
    pub agent_tx: broadcast::Sender<AgentNotification>,
    pub webhook_config: WebhookConfig,
    pub plugin_registry: Arc<RwLock<PluginRegistry>>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentRelatedNode {
    pub id: String,
    pub title: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentNotification {
    Chronicle {
        entry: ChronicleEntry,
        namespace: Option<String>,
    },
    Intent {
        intent: CapturedIntent,
        namespace: Option<String>,
    },
    InsightDiscovered {
        insight: ProactiveInsight,
        namespace: Option<String>,
    },
    RelatedContext {
        nodes: Vec<AgentRelatedNode>,
        namespace: Option<String>,
    },
    NodeEnriched {
        node_id: String,
        namespace: Option<String>,
    },
}

impl AgentNotification {
    pub fn namespace(&self) -> Option<&str> {
        match self {
            Self::Chronicle { namespace, .. }
            | Self::Intent { namespace, .. }
            | Self::InsightDiscovered { namespace, .. }
            | Self::RelatedContext { namespace, .. }
            | Self::NodeEnriched { namespace, .. } => namespace.as_deref(),
        }
    }
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
        Self::new_with_change_tx(engine, change_tx)
    }

    pub fn new_with_change_tx(
        engine: Arc<MindVaultEngine>,
        change_tx: broadcast::Sender<ChangeNotification>,
    ) -> Self {
        let (agent_tx, _) = broadcast::channel(256);
        Self::new_with_channels(engine, change_tx, agent_tx)
    }

    pub fn new_with_channels(
        engine: Arc<MindVaultEngine>,
        change_tx: broadcast::Sender<ChangeNotification>,
        agent_tx: broadcast::Sender<AgentNotification>,
    ) -> Self {
        let (reminder_tx, _) = broadcast::channel(256);
        let plugins_dir = std::env::var("MINDVAULT_PLUGINS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("plugins"));
        Self {
            engine,
            change_tx,
            reminder_tx,
            agent_tx,
            webhook_config: WebhookConfig::from_env(),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
            plugin_manager: Arc::new(RwLock::new(PluginManager::new(plugins_dir))),
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

    pub fn notify_agent(&self, notification: AgentNotification) {
        let _ = self.agent_tx.send(notification);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_core::{InsightType, IntentType};

    #[test]
    fn agent_notification_serializes_with_expected_type_tag() {
        let mut intent = CapturedIntent::new(uuid::Uuid::now_v7(), IntentType::ExtractTask);
        intent.confidence = 0.8;
        let notification = AgentNotification::Intent {
            intent,
            namespace: Some("default".to_string()),
        };

        let json = serde_json::to_value(notification).unwrap();
        assert_eq!(json["type"], "intent");
        assert_eq!(json["namespace"], "default");

        let insight = ProactiveInsight::new("t", "c", InsightType::Trend);
        let insight_json = serde_json::to_value(AgentNotification::InsightDiscovered {
            insight,
            namespace: Some("default".to_string()),
        })
        .unwrap();
        assert_eq!(insight_json["type"], "insight_discovered");
    }

    #[test]
    fn agent_notification_namespace_accessor() {
        let entry = ChronicleEntry::new("step", "logic");
        let notification = AgentNotification::Chronicle {
            entry,
            namespace: Some("ops".to_string()),
        };
        assert_eq!(notification.namespace(), Some("ops"));
    }
}
