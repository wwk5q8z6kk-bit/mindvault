//! Authenticated HTTP outbox publisher (IK-005).
//!
//! Posts the claimed event envelope as JSON to a configured destination.
//! Receipts are written only by the dispatcher via `complete_outbox_delivery`.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use mv_core::{MvError, MvResult, OutboxDeliveryClaim, OutboxDeliveryResult};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use url::Url;

use super::OutboxPublisher;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_RETRY_BACKOFF_SECS: i64 = 30;
const DELIVERY_REFERENCE_MAX: usize = 2048;

/// Configuration for one HTTP outbox destination.
#[derive(Debug, Clone)]
pub struct HttpOutboxPublisherConfig {
    pub endpoint: Url,
    pub bearer_token: Option<String>,
    pub timeout: Duration,
}

impl HttpOutboxPublisherConfig {
    pub fn new(endpoint: Url) -> Self {
        Self {
            endpoint,
            bearer_token: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Load config when HTTP publisher env is set and command admission is active.
    pub fn from_env_if_admission_active(admission_active: bool) -> Option<Self> {
        let enabled = std::env::var("MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED")
            .ok()
            .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"));
        if !enabled {
            return None;
        }
        if !admission_active {
            tracing::warn!(
                "MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED is set but command admission is off;                  keeping LocalAckPublisher"
            );
            return None;
        }

        let destination_url = std::env::var("MINDVAULT_OUTBOX_HTTP_DESTINATION_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let Some(destination_url) = destination_url else {
            tracing::warn!(
                "MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED requires                  MINDVAULT_OUTBOX_HTTP_DESTINATION_URL; keeping LocalAckPublisher"
            );
            return None;
        };

        let endpoint = match Url::parse(&destination_url) {
            Ok(url) => url,
            Err(error) => {
                tracing::warn!(%error, destination_url, "invalid HTTP outbox destination URL");
                return None;
            }
        };

        let mut config = Self::new(endpoint);
        if let Ok(token) = std::env::var("MINDVAULT_OUTBOX_HTTP_BEARER_TOKEN") {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                config = config.with_bearer_token(trimmed);
            }
        }

        if let Some(timeout_secs) = std::env::var("MINDVAULT_OUTBOX_HTTP_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            config = config.with_timeout(Duration::from_secs(timeout_secs.max(1)));
        }

        Some(config)
    }
}

/// Live HTTP publisher for outbox events.
///
/// Disabled by default at the server layer; operators must opt in via env gates
/// documented in `mv-server/src/outbox_dispatch.rs`.
#[derive(Debug, Clone)]
pub struct HttpOutboxPublisher {
    client: reqwest::Client,
    config: HttpOutboxPublisherConfig,
}

impl HttpOutboxPublisher {
    pub fn new(config: HttpOutboxPublisherConfig) -> MvResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|error| MvError::Internal(format!("http outbox client: {error}")))?;
        Ok(Self { client, config })
    }

    fn map_http_response(
        &self,
        status: StatusCode,
        headers: &reqwest::header::HeaderMap,
        body: &[u8],
    ) -> OutboxDeliveryResult {
        if status.is_success() {
            return OutboxDeliveryResult::Published {
                delivery_reference: delivery_reference(status, headers, &self.config.endpoint),
                response_digest: Some(sha256_hex(body)),
            };
        }

        let error_code = format!("http_{}", status.as_u16());
        let error_summary = truncate_summary(&response_error_summary(status, body));

        if is_transient_status(status) {
            OutboxDeliveryResult::RetryScheduled {
                retry_at: Utc::now() + ChronoDuration::seconds(DEFAULT_RETRY_BACKOFF_SECS),
                error_code,
                error_summary,
            }
        } else {
            OutboxDeliveryResult::DeadLettered {
                error_code,
                error_summary,
            }
        }
    }

    fn map_transport_error(&self, error: reqwest::Error) -> OutboxDeliveryResult {
        let error_summary = truncate_summary(&error.to_string());
        OutboxDeliveryResult::RetryScheduled {
            retry_at: Utc::now() + ChronoDuration::seconds(DEFAULT_RETRY_BACKOFF_SECS),
            error_code: "http_transport_error".into(),
            error_summary,
        }
    }
}

#[async_trait]
impl OutboxPublisher for HttpOutboxPublisher {
    async fn publish(&self, claim: &OutboxDeliveryClaim) -> MvResult<OutboxDeliveryResult> {
        let mut request = self
            .client
            .post(self.config.endpoint.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&claim.event);

        if let Some(token) = &self.config.bearer_token {
            request = request.bearer_auth(token);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => return Ok(self.map_transport_error(error)),
        };

        let status = response.status();
        let headers = response.headers().clone();
        let body = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => return Ok(self.map_transport_error(error)),
        };

        Ok(self.map_http_response(status, &headers, &body))
    }
}

fn is_transient_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn delivery_reference(
    status: StatusCode,
    headers: &reqwest::header::HeaderMap,
    endpoint: &Url,
) -> String {
    for name in [
        "x-request-id",
        "x-correlation-id",
        "location",
        "content-location",
    ] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return bound_reference(&format!("http:{status}:{trimmed}"));
            }
        }
    }
    bound_reference(&format!("http:{}:{}", status.as_u16(), endpoint))
}

fn bound_reference(value: &str) -> String {
    if value.len() <= DELIVERY_REFERENCE_MAX {
        value.to_string()
    } else {
        value[..DELIVERY_REFERENCE_MAX].to_string()
    }
}

fn response_error_summary(status: StatusCode, body: &[u8]) -> String {
    let body_text = String::from_utf8_lossy(body);
    let trimmed = body_text.trim();
    if trimmed.is_empty() {
        format!("HTTP {} from destination", status.as_u16())
    } else {
        format!("HTTP {}: {trimmed}", status.as_u16())
    }
}

fn truncate_summary(value: &str) -> String {
    const MAX: usize = 4096;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "destination request failed".into();
    }
    if trimmed.len() <= MAX {
        trimmed.to_string()
    } else {
        trimmed[..MAX].to_string()
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use mv_core::{
        EventEnvelope, IdempotencyKey, InteroperabilityStore, KnowledgeNode,
        NewEventEnvelope, NodeKind,
        OutboxDeliveryState, ProvenanceReference, ProvenanceRelation, RetentionClass,
        SchemaReference, Sensitivity, StableUri, KNOWLEDGE_NODE_CREATED_V1,
    };
    use tempfile::TempDir;
    use uuid::Uuid;

    use crate::engine::{MindVaultEngine, OutboxDispatcherConfig};

    async fn test_engine() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.llm.auto_detect = false;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    fn http_dispatcher_config() -> OutboxDispatcherConfig {
        OutboxDispatcherConfig {
            enabled: true,
            interval_secs: 1,
            lease_secs: 30,
            batch_limit: 10,
            executor: StableUri::parse("mindvault://dispatchers/http").unwrap(),
            destination: StableUri::parse("mindvault://destinations/http").unwrap(),
        }
    }

    async fn seed_outbox_event(engine: &MindVaultEngine, key: &str) -> Uuid {
        let local_node_id = engine.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, format!("outbox {key}"))
            .with_namespace("interoperability");
        let subject = StableUri::knowledge_node(local_node_id, node.id);
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"outbox-http-publisher-test"),
        );
        let event = EventEnvelope::new(NewEventEnvelope {
            event_type: KNOWLEDGE_NODE_CREATED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: subject.clone(),
            schema: SchemaReference::new(
                StableUri::schema("knowledge-node-created").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: principal.clone(),
            actor: principal,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(key).unwrap(),
            payload_digest: "a".repeat(64),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: subject,
                relation: ProvenanceRelation::PrimarySource,
            }],
            data: serde_json::json!({
                "resource_kind": "knowledge_node",
                "node_kind": "fact",
                "namespace": "interoperability",
            }),
        })
        .unwrap();
        let event_id = event.id;
        engine
            .store
            .nodes
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();
        event_id
    }

    fn http_publisher(server: &mockito::ServerGuard) -> HttpOutboxPublisher {
        HttpOutboxPublisher::new(HttpOutboxPublisherConfig::new(
            Url::parse(&server.url()).expect("mockito url"),
        ))
        .expect("http publisher")
    }

    #[tokio::test]
    async fn publisher_http_success_produces_published_receipt() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "http-publish-success").await;
        let mut server = mockito::Server::new_async().await;
        let body = r#"{"accepted":true}"#;
        let _mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let publisher = http_publisher(&server);
        let config = http_dispatcher_config();
        let tick = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();

        assert_eq!(tick.claimed, 1);
        assert_eq!(tick.completed, 1);

        let status = engine
            .store
            .nodes
            .get_outbox_delivery_status(event_id)
            .await
            .unwrap()
            .expect("status");
        assert_eq!(status.state, OutboxDeliveryState::Published);
        assert_eq!(status.attempts, 1);

        let receipts = engine
            .store
            .nodes
            .list_action_receipts(event_id, 10)
            .await
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].outcome, mv_core::ActionReceiptOutcome::Published);
        assert_eq!(
            receipts[0].response_digest.as_deref(),
            Some(sha256_hex(body.as_bytes()).as_str())
        );
    }

    #[tokio::test]
    async fn publisher_http_transient_failure_schedules_retry() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "http-publish-retry").await;
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(503)
            .with_body("upstream unavailable")
            .create_async()
            .await;

        let publisher = http_publisher(&server);
        let config = http_dispatcher_config();
        let before = Utc::now();
        let tick = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();

        assert_eq!(tick.claimed, 1);
        assert_eq!(tick.completed, 1);

        let status = engine
            .store
            .nodes
            .get_outbox_delivery_status(event_id)
            .await
            .unwrap()
            .expect("status");
        assert_eq!(status.state, OutboxDeliveryState::Pending);
        assert_eq!(status.attempts, 1);
        assert!(status.next_attempt_at > before);

        let receipts = engine
            .store
            .nodes
            .list_action_receipts(event_id, 10)
            .await
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].outcome,
            mv_core::ActionReceiptOutcome::RetryScheduled
        );
    }

    #[tokio::test]
    async fn publisher_http_terminal_failure_dead_letters() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "http-publish-dead-letter").await;
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/")
            .with_status(400)
            .with_body("invalid payload")
            .create_async()
            .await;

        let publisher = http_publisher(&server);
        let config = http_dispatcher_config();
        let tick = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();

        assert_eq!(tick.claimed, 1);
        assert_eq!(tick.completed, 1);

        let status = engine
            .store
            .nodes
            .get_outbox_delivery_status(event_id)
            .await
            .unwrap()
            .expect("status");
        assert_eq!(status.state, OutboxDeliveryState::DeadLetter);
        assert_eq!(status.attempts, 1);

        let receipts = engine
            .store
            .nodes
            .list_action_receipts(event_id, 10)
            .await
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].outcome,
            mv_core::ActionReceiptOutcome::DeadLettered
        );

        let idle = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();
        assert_eq!(idle.claimed, 0);
    }

    #[test]
    fn publisher_http_maps_transient_status_codes() {
        assert!(is_transient_status(StatusCode::REQUEST_TIMEOUT));
        assert!(is_transient_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_transient_status(StatusCode::BAD_GATEWAY));
        assert!(is_transient_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(is_transient_status(StatusCode::GATEWAY_TIMEOUT));
        assert!(!is_transient_status(StatusCode::BAD_REQUEST));
        assert!(!is_transient_status(StatusCode::INTERNAL_SERVER_ERROR));
    }

    #[test]
    fn publisher_http_delivery_reference_prefers_response_header() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-request-id",
            reqwest::header::HeaderValue::from_static("req-123"),
        );
        let endpoint = Url::parse("http://127.0.0.1:8080/events").unwrap();
        let reference = delivery_reference(StatusCode::OK, &headers, &endpoint);
        assert_eq!(reference, "http:200 OK:req-123");
    }
}
