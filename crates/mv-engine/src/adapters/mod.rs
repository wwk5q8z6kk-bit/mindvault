//! External adapter framework for bridging messaging platforms into the relay engine.
//!
//! Each adapter implements [`ExternalAdapter`] to send/receive messages through
//! external services (Slack, Discord, email, etc.). Inbound messages are converted
//! to [`RelayMessage`] objects and flow through the relay engine, automatically
//! creating vault nodes for searchability.

pub mod discord;
pub mod email;
pub mod slack;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use mv_core::MvResult;

// ---------------------------------------------------------------------------
// Adapter Configuration
// ---------------------------------------------------------------------------

/// Configuration for an adapter instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub id: Uuid,
    pub adapter_type: AdapterType,
    pub name: String,
    pub enabled: bool,
    pub settings: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl AdapterConfig {
    pub fn new(adapter_type: AdapterType, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            adapter_type,
            name: name.into(),
            enabled: true,
            settings: HashMap::new(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn with_setting(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.settings.insert(key.into(), value.into());
        self
    }

    pub fn get_setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str())
    }
}

/// Supported adapter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterType {
    Slack,
    Discord,
    Email,
}

impl AdapterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Discord => "discord",
            Self::Email => "email",
        }
    }
}

impl std::str::FromStr for AdapterType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "slack" => Ok(Self::Slack),
            "discord" => Ok(Self::Discord),
            "email" => Ok(Self::Email),
            _ => Err(format!("unknown adapter type: {s}")),
        }
    }
}

impl std::fmt::Display for AdapterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Adapter Message Types
// ---------------------------------------------------------------------------

/// An outbound message to be sent through an adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterOutboundMessage {
    pub channel: String,
    pub content: String,
    pub thread_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// An inbound message received from an external platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInboundMessage {
    pub external_id: String,
    pub channel: String,
    pub sender: String,
    pub content: String,
    pub thread_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Status of an adapter instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatus {
    pub adapter_type: AdapterType,
    pub name: String,
    pub connected: bool,
    pub last_send: Option<DateTime<Utc>>,
    pub last_receive: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Adapter Trait
// ---------------------------------------------------------------------------

/// Trait implemented by each external messaging adapter.
///
/// Adapters bridge external platforms into MindVault's relay system.
/// They are responsible for:
/// - Sending messages outbound to the external platform
/// - Polling for or receiving inbound messages
/// - Reporting health/connection status
#[async_trait]
pub trait ExternalAdapter: Send + Sync {
    /// Human-readable name for this adapter instance.
    fn name(&self) -> &str;

    /// The type of adapter (slack, discord, email).
    fn adapter_type(&self) -> AdapterType;

    /// Send a message to the external platform.
    async fn send(&self, message: &AdapterOutboundMessage) -> MvResult<()>;

    /// Poll for new inbound messages since the given cursor.
    /// Returns messages and a new cursor for the next poll.
    async fn poll(&self, cursor: Option<&str>) -> MvResult<(Vec<AdapterInboundMessage>, String)>;

    /// Check if the adapter is healthy and can communicate.
    async fn health_check(&self) -> MvResult<bool>;

    /// Get current status of the adapter.
    fn status(&self) -> AdapterStatus;
}

// ---------------------------------------------------------------------------
// Adapter Registry
// ---------------------------------------------------------------------------

/// Registry managing all active adapter instances.
///
/// Thread-safe via interior `RwLock`. Adapters are stored as `Arc<dyn ExternalAdapter>`
/// keyed by their config ID for O(1) lookup.
pub struct AdapterRegistry {
    adapters: RwLock<HashMap<Uuid, Arc<dyn ExternalAdapter>>>,
    configs: RwLock<Vec<AdapterConfig>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
            configs: RwLock::new(Vec::new()),
        }
    }

    /// Register a new adapter with its configuration.
    pub async fn register(
        &self,
        config: AdapterConfig,
        adapter: Arc<dyn ExternalAdapter>,
    ) {
        let id = config.id;
        self.configs.write().await.push(config);
        self.adapters.write().await.insert(id, adapter);
    }

    /// Remove an adapter by its config ID.
    pub async fn remove(&self, id: Uuid) -> bool {
        let removed = self.adapters.write().await.remove(&id).is_some();
        if removed {
            self.configs.write().await.retain(|c| c.id != id);
        }
        removed
    }

    /// Get an adapter by ID.
    pub async fn get(&self, id: Uuid) -> Option<Arc<dyn ExternalAdapter>> {
        self.adapters.read().await.get(&id).cloned()
    }

    /// List all registered adapter configurations.
    pub async fn list_configs(&self) -> Vec<AdapterConfig> {
        self.configs.read().await.clone()
    }

    /// List all adapter statuses.
    pub async fn list_statuses(&self) -> Vec<AdapterStatus> {
        let adapters = self.adapters.read().await;
        adapters.values().map(|a| a.status()).collect()
    }

    /// Send a message through a specific adapter.
    pub async fn send(
        &self,
        adapter_id: Uuid,
        message: &AdapterOutboundMessage,
    ) -> MvResult<()> {
        let adapters = self.adapters.read().await;
        match adapters.get(&adapter_id) {
            Some(adapter) => adapter.send(message).await,
            None => Err(mv_core::MvError::NotFound(format!(
                "adapter {adapter_id} not found"
            ))),
        }
    }

    /// Poll all enabled adapters for inbound messages.
    pub async fn poll_all(
        &self,
        cursors: &HashMap<Uuid, String>,
    ) -> Vec<(Uuid, Vec<AdapterInboundMessage>, String)> {
        let adapters = self.adapters.read().await;
        let configs = self.configs.read().await;

        let mut results = Vec::new();

        for config in configs.iter() {
            if !config.enabled {
                continue;
            }
            if let Some(adapter) = adapters.get(&config.id) {
                let cursor = cursors.get(&config.id).map(|s| s.as_str());
                match adapter.poll(cursor).await {
                    Ok((messages, new_cursor)) => {
                        if !messages.is_empty() {
                            results.push((config.id, messages, new_cursor));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            adapter = %config.name,
                            error = %e,
                            "adapter poll failed"
                        );
                    }
                }
            }
        }

        results
    }

    /// Run health checks on all adapters.
    pub async fn health_check_all(&self) -> HashMap<Uuid, bool> {
        let adapters = self.adapters.read().await;
        let mut results = HashMap::new();

        for (&id, adapter) in adapters.iter() {
            let healthy = adapter.health_check().await.unwrap_or(false);
            results.insert(id, healthy);
        }

        results
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockAdapter {
        name: String,
        sent: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ExternalAdapter for MockAdapter {
        fn name(&self) -> &str {
            &self.name
        }

        fn adapter_type(&self) -> AdapterType {
            AdapterType::Slack
        }

        async fn send(&self, message: &AdapterOutboundMessage) -> MvResult<()> {
            self.sent.lock().unwrap().push(message.content.clone());
            Ok(())
        }

        async fn poll(
            &self,
            _cursor: Option<&str>,
        ) -> MvResult<(Vec<AdapterInboundMessage>, String)> {
            Ok((vec![], "0".into()))
        }

        async fn health_check(&self) -> MvResult<bool> {
            Ok(true)
        }

        fn status(&self) -> AdapterStatus {
            AdapterStatus {
                adapter_type: AdapterType::Slack,
                name: self.name.clone(),
                connected: true,
                last_send: None,
                last_receive: None,
                error: None,
            }
        }
    }

    #[tokio::test]
    async fn register_and_send() {
        let registry = AdapterRegistry::new();
        let sent = Arc::new(Mutex::new(Vec::new()));
        let adapter = Arc::new(MockAdapter {
            name: "test".into(),
            sent: Arc::clone(&sent),
        });
        let config = AdapterConfig::new(AdapterType::Slack, "test-slack");
        let id = config.id;

        registry.register(config, adapter).await;

        let msg = AdapterOutboundMessage {
            channel: "#general".into(),
            content: "hello".into(),
            thread_id: None,
            metadata: HashMap::new(),
        };
        registry.send(id, &msg).await.unwrap();

        assert_eq!(sent.lock().unwrap().len(), 1);
        assert_eq!(sent.lock().unwrap()[0], "hello");
    }

    #[tokio::test]
    async fn remove_adapter() {
        let registry = AdapterRegistry::new();
        let adapter = Arc::new(MockAdapter {
            name: "rm".into(),
            sent: Arc::new(Mutex::new(Vec::new())),
        });
        let config = AdapterConfig::new(AdapterType::Slack, "rm-slack");
        let id = config.id;

        registry.register(config, adapter).await;
        assert!(registry.get(id).await.is_some());

        assert!(registry.remove(id).await);
        assert!(registry.get(id).await.is_none());
    }

    #[tokio::test]
    async fn health_check_all() {
        let registry = AdapterRegistry::new();
        let adapter = Arc::new(MockAdapter {
            name: "hc".into(),
            sent: Arc::new(Mutex::new(Vec::new())),
        });
        let config = AdapterConfig::new(AdapterType::Slack, "hc-slack");
        let id = config.id;

        registry.register(config, adapter).await;

        let results = registry.health_check_all().await;
        assert_eq!(results.get(&id), Some(&true));
    }
}
