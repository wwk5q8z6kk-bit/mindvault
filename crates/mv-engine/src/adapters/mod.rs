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
            None => Err(mv_core::MvError::InvalidInput(format!(
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

// ---------------------------------------------------------------------------
// Poll Cycle
// ---------------------------------------------------------------------------

/// Run a single poll cycle across all enabled adapters, persisting cursor state.
///
/// For each adapter:
/// 1. Load the last cursor from the poll state store
/// 2. Call `poll(cursor)` on the adapter
/// 3. Convert inbound messages to vault nodes
/// 4. Persist the new cursor
///
/// Returns the total number of new messages received.
pub async fn run_poll_cycle<S>(
    registry: &AdapterRegistry,
    poll_store: &S,
    node_callback: impl Fn(AdapterInboundMessage, AdapterType) + Send + Sync,
) -> usize
where
    S: mv_core::AdapterPollStore,
{
    let configs = registry.list_configs().await;
    let mut total = 0;

    for config in &configs {
        if !config.enabled {
            continue;
        }

        let adapter = match registry.get(config.id).await {
            Some(a) => a,
            None => continue,
        };

        let adapter_name = config.name.clone();

        // Load cursor
        let cursor = match poll_store.get_poll_state(&adapter_name).await {
            Ok(Some(state)) => Some(state.cursor),
            _ => None,
        };

        // Poll
        let (messages, new_cursor) = match adapter.poll(cursor.as_deref()).await {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(
                    adapter = %adapter_name,
                    error = %e,
                    "poll cycle failed"
                );
                continue;
            }
        };

        let msg_count = messages.len();
        if msg_count > 0 {
            for msg in messages {
                node_callback(msg, config.adapter_type);
            }

            // Persist cursor
            if let Err(e) = poll_store
                .upsert_poll_state(&adapter_name, &new_cursor, msg_count as u64)
                .await
            {
                tracing::warn!(
                    adapter = %adapter_name,
                    error = %e,
                    "failed to persist poll cursor"
                );
            }

            total += msg_count;
        } else if cursor.as_deref() != Some(&new_cursor) {
            // Update cursor even if no messages (e.g., cursor changed)
            let _ = poll_store
                .upsert_poll_state(&adapter_name, &new_cursor, 0)
                .await;
        }
    }

    total
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

    // --- AdapterConfig tests ---

    #[test]
    fn adapter_config_new_sets_defaults() {
        let config = AdapterConfig::new(AdapterType::Email, "my-email");
        assert_eq!(config.adapter_type, AdapterType::Email);
        assert_eq!(config.name, "my-email");
        assert!(config.enabled);
        assert!(config.settings.is_empty());
        assert!(config.updated_at.is_none());
    }

    #[test]
    fn adapter_config_with_setting_and_get() {
        let config = AdapterConfig::new(AdapterType::Slack, "test")
            .with_setting("webhook_url", "https://hooks.slack.com/xxx")
            .with_setting("channel", "#general");

        assert_eq!(
            config.get_setting("webhook_url"),
            Some("https://hooks.slack.com/xxx")
        );
        assert_eq!(config.get_setting("channel"), Some("#general"));
        assert_eq!(config.get_setting("nonexistent"), None);
    }

    // --- AdapterType tests ---

    #[test]
    fn adapter_type_display_and_from_str() {
        assert_eq!(AdapterType::Slack.to_string(), "slack");
        assert_eq!(AdapterType::Discord.to_string(), "discord");
        assert_eq!(AdapterType::Email.to_string(), "email");

        assert_eq!("slack".parse::<AdapterType>().unwrap(), AdapterType::Slack);
        assert_eq!("discord".parse::<AdapterType>().unwrap(), AdapterType::Discord);
        assert_eq!("email".parse::<AdapterType>().unwrap(), AdapterType::Email);
    }

    #[test]
    fn adapter_type_from_str_rejects_unknown() {
        assert!("webhook".parse::<AdapterType>().is_err());
        assert!("Slack".parse::<AdapterType>().is_err()); // case-sensitive
        assert!("".parse::<AdapterType>().is_err());
    }

    #[test]
    fn adapter_type_as_str_matches_display() {
        for at in [AdapterType::Slack, AdapterType::Discord, AdapterType::Email] {
            assert_eq!(at.as_str(), at.to_string());
        }
    }

    // --- Message serialization tests ---

    #[test]
    fn outbound_message_serialization_roundtrip() {
        let msg = AdapterOutboundMessage {
            channel: "#dev".into(),
            content: "build passed".into(),
            thread_id: Some("t-123".into()),
            metadata: HashMap::from([("key".into(), "val".into())]),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AdapterOutboundMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.channel, "#dev");
        assert_eq!(deserialized.content, "build passed");
        assert_eq!(deserialized.thread_id.as_deref(), Some("t-123"));
        assert_eq!(deserialized.metadata.get("key").unwrap(), "val");
    }

    #[test]
    fn inbound_message_serialization_roundtrip() {
        let msg = AdapterInboundMessage {
            external_id: "ext-1".into(),
            channel: "#support".into(),
            sender: "user42".into(),
            content: "help me".into(),
            thread_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AdapterInboundMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.external_id, "ext-1");
        assert_eq!(deserialized.sender, "user42");
        assert!(deserialized.thread_id.is_none());
    }

    #[test]
    fn adapter_status_serialization() {
        let status = AdapterStatus {
            adapter_type: AdapterType::Discord,
            name: "team-discord".into(),
            connected: false,
            last_send: None,
            last_receive: None,
            error: Some("timeout".into()),
        };

        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["adapter_type"], "discord");
        assert_eq!(json["connected"], false);
        assert_eq!(json["error"], "timeout");
    }

    // --- Registry edge case tests ---

    #[tokio::test]
    async fn list_configs_returns_all_registered() {
        let registry = AdapterRegistry::new();
        assert!(registry.list_configs().await.is_empty());

        let a1 = Arc::new(MockAdapter {
            name: "a1".into(),
            sent: Arc::new(Mutex::new(Vec::new())),
        });
        let a2 = Arc::new(MockAdapter {
            name: "a2".into(),
            sent: Arc::new(Mutex::new(Vec::new())),
        });

        registry
            .register(AdapterConfig::new(AdapterType::Slack, "slack-1"), a1)
            .await;
        registry
            .register(AdapterConfig::new(AdapterType::Discord, "discord-1"), a2)
            .await;

        let configs = registry.list_configs().await;
        assert_eq!(configs.len(), 2);
    }

    #[tokio::test]
    async fn list_statuses_returns_all() {
        let registry = AdapterRegistry::new();
        let adapter = Arc::new(MockAdapter {
            name: "stat".into(),
            sent: Arc::new(Mutex::new(Vec::new())),
        });
        registry
            .register(AdapterConfig::new(AdapterType::Slack, "stat"), adapter)
            .await;

        let statuses = registry.list_statuses().await;
        assert_eq!(statuses.len(), 1);
        assert!(statuses[0].connected);
        assert_eq!(statuses[0].name, "stat");
    }

    #[tokio::test]
    async fn send_to_nonexistent_adapter_returns_error() {
        let registry = AdapterRegistry::new();
        let msg = AdapterOutboundMessage {
            channel: "#test".into(),
            content: "msg".into(),
            thread_id: None,
            metadata: HashMap::new(),
        };

        let result = registry.send(Uuid::now_v7(), &msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn remove_nonexistent_returns_false() {
        let registry = AdapterRegistry::new();
        assert!(!registry.remove(Uuid::now_v7()).await);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let registry = AdapterRegistry::new();
        assert!(registry.get(Uuid::now_v7()).await.is_none());
    }

    #[tokio::test]
    async fn default_creates_empty_registry() {
        let registry = AdapterRegistry::default();
        assert!(registry.list_configs().await.is_empty());
        assert!(registry.list_statuses().await.is_empty());
    }
}
