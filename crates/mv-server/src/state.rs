use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use mv_engine::engine::MindVaultEngine;
use mv_plugin::{PluginManager, PluginRegistry, PluginRuntime};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

pub use mv_core::ChangeNotification;
use mv_core::{CapturedIntent, ChronicleEntry, EventEnvelope, ProactiveInsight};

/// Shared application state.
pub struct AppState {
    pub engine: Arc<MindVaultEngine>,
    pub change_tx: broadcast::Sender<ChangeNotification>,
    pub reminder_tx: broadcast::Sender<ReminderNotification>,
    pub agent_tx: broadcast::Sender<AgentNotification>,
    pub webhook_config: WebhookConfig,
    pub plugin_registry: Arc<RwLock<PluginRegistry>>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
    pub plugin_runtime: Arc<RwLock<PluginRuntime>>,
    /// Shared HTTP client for AI sidecar proxy and other outbound requests.
    pub http_client: reqwest::Client,
    /// Counter: requests rejected by sealed-mode middleware.
    pub sealed_blocked_requests: AtomicU64,
    /// Filesystem roots beneath which an administrator may mount workspaces.
    pub workspace_root_policy: WorkspaceRootPolicy,
    /// Whether public commands must resolve an authorizing grant.
    pub command_admission: CommandAdmissionPolicy,
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
    /// A governed agent run changed lifecycle state.
    ///
    /// Carries identifiers and status only — never artifact content or declared
    /// scope. The stream is an observation surface, and a run's write scope is
    /// governed detail that belongs behind the query API where the caller's
    /// read authorization is checked.
    ///
    /// `awaiting_approval` is the state the approval UI listens for; it is
    /// unbounded by design and must never be rendered as an error or a timeout.
    AgentRunTransitioned {
        run_id: String,
        work_order_id: String,
        status: String,
        /// Present only on terminal failure.
        failure_class: Option<String>,
        namespace: Option<String>,
    },
    /// Gate evidence was recorded for a run.
    ///
    /// Immutable once emitted, matching the underlying evidence: a client that
    /// sees a `fail` will never see it revised to a `pass` for the same run and
    /// gate.
    AgentRunGateRecorded {
        run_id: String,
        work_order_id: String,
        gate: String,
        outcome: String,
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
            | Self::NodeEnriched { namespace, .. }
            | Self::AgentRunTransitioned { namespace, .. }
            | Self::AgentRunGateRecorded { namespace, .. } => namespace.as_deref(),
        }
    }
}

impl AgentNotification {
    /// Whether this notification may be delivered to a socket with `scope`.
    ///
    /// An unscoped session receives everything. A namespace-scoped session
    /// receives only notifications carrying that namespace — which means it
    /// receives no execution-graph events at all, because the governed
    /// execution graph is not namespace-partitioned: a Work Order is bounded by
    /// its governing node and its AuthorityGrant, not by a namespace.
    ///
    /// That exclusion is deliberate and fails closed. A scoped token is an
    /// assertion of "limit me to this namespace", and it may belong to a
    /// delegate rather than the owner (System Principle 8). Pushing vault-wide
    /// governance signal to it would widen access beyond what was requested.
    /// Those clients poll the query API instead, where their authorization is
    /// checked per request.
    ///
    /// Named and tested rather than left inline so the exclusion cannot be
    /// undone by accident.
    pub fn deliverable_to(&self, scope: Option<&str>) -> bool {
        match scope {
            None => true,
            Some(namespace) => self.namespace() == Some(namespace),
        }
    }

    /// Observation of a run lifecycle transition.
    ///
    /// `namespace` is `None` because the execution graph is not namespace-scoped
    /// data — a Work Order is governed by its governing node URI and its
    /// AuthorityGrant, not by a namespace.
    ///
    /// Consequence, stated rather than discovered later: `handle_agent_socket`
    /// drops any notification whose namespace does not equal the client's scope,
    /// so a namespace-scoped WebSocket client receives none of these. Unscoped
    /// clients receive them all. This is deliberate — bypassing the scope filter
    /// would push cross-scope signal to a client that asked to be limited — but
    /// it means a scoped client must use the query API rather than the stream.
    pub fn run_transitioned(run: &mv_core::AgentRun) -> Self {
        Self::AgentRunTransitioned {
            run_id: run.run_id.to_string(),
            work_order_id: run.work_order_id.to_string(),
            status: run.status.as_str().to_string(),
            failure_class: run.failure_class.map(|class| class.as_str().to_string()),
            namespace: None,
        }
    }

    /// Observation of recorded gate evidence.
    pub fn gate_recorded(result: &mv_core::GateResult) -> Self {
        Self::AgentRunGateRecorded {
            run_id: result.run_id.to_string(),
            work_order_id: result.work_order_id.to_string(),
            gate: result.gate.as_str().to_string(),
            outcome: result.outcome.as_str().to_string(),
            namespace: None,
        }
    }
}

/// Configuration for webhook notifications.
#[derive(Clone, Debug, Default)]
pub struct WebhookConfig {
    pub reminder_url: Option<String>,
    pub change_url: Option<String>,
    pub keychain_alert_url: Option<String>,
    pub timeout_secs: u64,
}

/// Server-side capability boundary for local filesystem mounting.
///
/// The environment value uses the host path-list separator (`:` on Unix,
/// `;` on Windows). An empty policy disables REST-based root mounting.
#[derive(Clone, Debug, Default)]
pub struct WorkspaceRootPolicy {
    allowed_roots: Vec<PathBuf>,
}

impl WorkspaceRootPolicy {
    pub fn from_env() -> Self {
        let allowed_roots = std::env::var_os("MINDVAULT_WORKSPACE_ALLOWED_ROOTS")
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default();
        Self { allowed_roots }
    }

    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        Self { allowed_roots }
    }

    pub fn authorize(&self, requested_root: &std::path::Path) -> Result<PathBuf, String> {
        if self.allowed_roots.is_empty() {
            return Err(
                "workspace mounting is disabled; configure MINDVAULT_WORKSPACE_ALLOWED_ROOTS"
                    .into(),
            );
        }
        let requested = std::fs::canonicalize(requested_root)
            .map_err(|error| format!("workspace root unavailable: {error}"))?;
        if !requested.is_dir() {
            return Err("workspace root must be a directory".into());
        }

        let authorized = self.allowed_roots.iter().any(|allowed| {
            std::fs::canonicalize(allowed)
                .map(|allowed| requested == allowed || requested.starts_with(allowed))
                .unwrap_or(false)
        });
        if authorized {
            Ok(requested)
        } else {
            Err("workspace root is outside the configured allowlist".into())
        }
    }
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
            keychain_alert_url: std::env::var("MINDVAULT_WEBHOOK_KEYCHAIN_ALERT_URL")
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
        let plugin_manager = PluginManager::new(plugins_dir);
        let plugin_runtime = PluginRuntime::new(plugin_manager.clone());
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            engine,
            change_tx,
            reminder_tx,
            agent_tx,
            webhook_config: WebhookConfig::from_env(),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
            plugin_manager: Arc::new(RwLock::new(plugin_manager)),
            plugin_runtime: Arc::new(RwLock::new(plugin_runtime)),
            http_client,
            sealed_blocked_requests: AtomicU64::new(0),
            workspace_root_policy: WorkspaceRootPolicy::from_env(),
            command_admission: CommandAdmissionPolicy::from_env(),
        }
    }

    pub fn with_workspace_allowed_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.workspace_root_policy = WorkspaceRootPolicy::new(roots);
        self
    }

    /// Set the admission mode without touching process-global environment.
    ///
    /// Server tests share one process, so a test that set the environment
    /// variable would leak enforcement into unrelated tests as mysterious 403s.
    pub fn with_command_admission(mut self, mode: CommandAdmissionMode) -> Self {
        self.command_admission = CommandAdmissionPolicy::new(mode);
        self
    }

    pub fn notify_change(&self, node_id: &str, operation: &str, namespace: Option<&str>) {
        self.notify_change_with_event(node_id, operation, namespace, None);
    }

    pub fn notify_change_with_event(
        &self,
        node_id: &str,
        operation: &str,
        namespace: Option<&str>,
        event: Option<EventEnvelope>,
    ) {
        let _ = self.change_tx.send(ChangeNotification {
            node_id: node_id.to_string(),
            operation: operation.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            namespace: namespace.map(|ns| ns.to_string()),
            event,
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

    /// Fire-and-forget webhook dispatch for keychain breach alerts.
    pub fn notify_keychain_alert(&self, alert: &mv_core::model::keychain::BreachAlert) {
        if let Some(ref url) = self.webhook_config.keychain_alert_url {
            let url = url.clone();
            let timeout = self.webhook_config.timeout_secs;
            let payload = serde_json::json!(alert);
            tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(timeout))
                    .build()
                    .ok();
                if let Some(client) = client {
                    let _ = client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .json(&payload)
                        .send()
                        .await;
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_core::{InsightType, IntentType};
    use tempfile::tempdir;

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

    #[test]
    fn workspace_root_policy_is_disabled_until_explicitly_allowlisted() {
        let directory = tempdir().unwrap();
        let error = WorkspaceRootPolicy::default()
            .authorize(directory.path())
            .unwrap_err();
        assert!(error.contains("mounting is disabled"));
    }

    #[test]
    fn workspace_root_policy_accepts_descendants_and_rejects_other_roots() {
        let allowed = tempdir().unwrap();
        let child = allowed.path().join("Knowledge");
        std::fs::create_dir(&child).unwrap();
        let outside = tempdir().unwrap();
        let policy = WorkspaceRootPolicy::new(vec![allowed.path().to_path_buf()]);

        assert_eq!(
            policy.authorize(&child).unwrap(),
            std::fs::canonicalize(&child).unwrap()
        );
        assert!(policy
            .authorize(outside.path())
            .unwrap_err()
            .contains("outside"));
    }

    #[cfg(unix)]
    #[test]
    fn workspace_root_policy_resolves_symlinks_before_authorizing() {
        use std::os::unix::fs::symlink;

        let allowed = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let link = allowed.path().join("escape");
        symlink(outside.path(), &link).unwrap();
        let policy = WorkspaceRootPolicy::new(vec![allowed.path().to_path_buf()]);

        assert!(policy.authorize(&link).is_err());
    }
}
