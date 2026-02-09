use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// How a secret should be injected into an HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SecretInjection {
    BearerHeader,
    BasicAuth { username: String },
    Header { name: String },
    QueryParam { name: String },
}

/// An HTTP proxy request to execute with credential injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProxyRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub secret_ref: String,
    pub inject_as: SecretInjection,
    pub intent: String,
}

/// Result of an HTTP proxy execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProxyResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: String,
    pub sanitized: bool,
}

/// A command execution proxy request with credential env injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecProxyRequest {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env_inject: HashMap<String, String>,
    pub working_dir: Option<String>,
    #[serde(default = "default_exec_timeout")]
    pub timeout_seconds: u64,
    pub intent: String,
}

fn default_exec_timeout() -> u64 {
    30
}

/// Result of a command execution proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecProxyResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub sanitized: bool,
}

/// An audit entry for proxy operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuditEntry {
    pub id: Uuid,
    pub consumer: String,
    pub secret_ref: String,
    pub action: String,
    pub target: String,
    pub intent: String,
    pub timestamp: DateTime<Utc>,
    pub success: Option<bool>,
    pub sanitized: bool,
    pub error: Option<String>,
    pub request_summary: String,
    pub response_status: Option<i32>,
}

impl ProxyAuditEntry {
    pub fn new(
        consumer: impl Into<String>,
        secret_ref: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        intent: impl Into<String>,
        request_summary: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            consumer: consumer.into(),
            secret_ref: secret_ref.into(),
            action: action.into(),
            target: target.into(),
            intent: intent.into(),
            timestamp: Utc::now(),
            success: None,
            sanitized: false,
            error: None,
            request_summary: request_summary.into(),
            response_status: None,
        }
    }
}
