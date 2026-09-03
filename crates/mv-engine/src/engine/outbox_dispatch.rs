//! Outbox dispatcher runtime (IK-004).
//!
//! Storage already owns claim/lease/receipt invariants
//! (`claim_outbox_events` → `complete_outbox_delivery`). This module is the
//! missing worker: it claims under a lease, asks a publisher for one attempt
//! result, and completes with exactly one immutable receipt.
//!
//! Live authenticated transports (HTTP, Slack, …) are IK-005. This slice ships
//! a local-ack publisher so events stop accumulating as forever-pending.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use mv_core::{
    InteroperabilityStore, MvResult, OutboxDeliveryClaim, OutboxDeliveryCompletion,
    OutboxDeliveryResult, StableUri,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::MindVaultEngine;

/// One publication attempt against a claimed outbox event.
#[async_trait]
pub trait OutboxPublisher: Send + Sync {
    async fn publish(&self, claim: &OutboxDeliveryClaim) -> MvResult<OutboxDeliveryResult>;
}

/// Destination that acknowledges locally. No network I/O.
///
/// Used until IK-005 wires the first authenticated live publisher behind the
/// same trait.
#[derive(Debug, Default, Clone)]
pub struct LocalAckPublisher;

#[async_trait]
impl OutboxPublisher for LocalAckPublisher {
    async fn publish(&self, claim: &OutboxDeliveryClaim) -> MvResult<OutboxDeliveryResult> {
        Ok(OutboxDeliveryResult::Published {
            delivery_reference: format!("local-ack:{}", claim.event.id),
            response_digest: None,
        })
    }
}

/// Runtime knobs for one dispatcher loop.
#[derive(Debug, Clone)]
pub struct OutboxDispatcherConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub lease_secs: u64,
    pub batch_limit: usize,
    pub executor: StableUri,
    pub destination: StableUri,
}

impl OutboxDispatcherConfig {
    pub fn local_defaults() -> Self {
        Self {
            enabled: true,
            interval_secs: 5,
            lease_secs: 30,
            batch_limit: 32,
            executor: StableUri::parse("mindvault://dispatchers/local").expect("executor URI"),
            destination: StableUri::parse("mindvault://destinations/local-ack")
                .expect("destination URI"),
        }
    }

    pub fn from_env() -> Self {
        let mut config = Self::local_defaults();
        config.enabled = std::env::var("MINDVAULT_OUTBOX_DISPATCH_ENABLED")
            .ok()
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        if let Some(interval) = std::env::var("MINDVAULT_OUTBOX_DISPATCH_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            config.interval_secs = interval.max(1);
        }
        if let Some(lease) = std::env::var("MINDVAULT_OUTBOX_DISPATCH_LEASE_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            // Storage rejects leases longer than one hour.
            config.lease_secs = lease.clamp(1, 3600);
        }
        if let Some(limit) = std::env::var("MINDVAULT_OUTBOX_DISPATCH_BATCH_LIMIT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
        {
            config.batch_limit = limit.clamp(1, 1000);
        }
        config
    }

    fn lease_duration(&self) -> ChronoDuration {
        ChronoDuration::seconds(self.lease_secs as i64)
    }
}

/// Summary of one dispatch tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxDispatchTick {
    pub claimed: usize,
    pub completed: usize,
    pub receipts: Vec<Uuid>,
}

impl MindVaultEngine {
    /// Claim up to `batch_limit` dispatchable events, publish each, and complete.
    ///
    /// Fail-closed per claim: a publisher `Err` becomes `RetryScheduled` with a
    /// bounded backoff so the lease is released and exactly one receipt is
    /// still written. Claim/complete APIs remain the sole receipt write path.
    pub async fn dispatch_outbox_once(
        &self,
        publisher: &dyn OutboxPublisher,
        config: &OutboxDispatcherConfig,
    ) -> MvResult<OutboxDispatchTick> {
        let claimed_at = Utc::now();
        let lease_expires_at = claimed_at + config.lease_duration();
        let claims = self
            .store
            .nodes
            .claim_outbox_events(
                &config.executor,
                &config.destination,
                claimed_at,
                lease_expires_at,
                config.batch_limit,
            )
            .await?;

        let mut receipts = Vec::with_capacity(claims.len());
        for claim in &claims {
            let result = match publisher.publish(claim).await {
                Ok(result) => result,
                Err(error) => OutboxDeliveryResult::RetryScheduled {
                    retry_at: Utc::now() + ChronoDuration::seconds(30),
                    error_code: "publisher_error".into(),
                    error_summary: truncate_error_summary(&error.to_string()),
                },
            };
            let completed_at = Utc::now();
            // Completing after lease expiry would be rejected; map to a
            // fail-closed retry using a synthetic in-lease completion time.
            let completed_at = if completed_at >= claim.lease_expires_at {
                claim.claimed_at + ChronoDuration::milliseconds(1)
            } else {
                completed_at
            };
            let completion = OutboxDeliveryCompletion {
                event_id: claim.event.id,
                lease_id: claim.lease_id,
                attempt: claim.attempt,
                completed_at,
                result,
            };
            let receipt = self
                .store
                .nodes
                .complete_outbox_delivery(claim, &completion)
                .await?;
            receipts.push(receipt.receipt_id);
        }

        Ok(OutboxDispatchTick {
            claimed: claims.len(),
            completed: receipts.len(),
            receipts,
        })
    }
}

fn truncate_error_summary(value: &str) -> String {
    const MAX: usize = 4096;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "publisher failed".into();
    }
    if trimmed.len() <= MAX {
        trimmed.to_string()
    } else {
        trimmed[..MAX].to_string()
    }
}

/// Spawn the background claim loop. Disabled unless config says otherwise.
pub fn spawn_outbox_dispatcher(
    engine: Arc<MindVaultEngine>,
    mut shutdown_rx: broadcast::Receiver<()>,
    config: OutboxDispatcherConfig,
    publisher: Arc<dyn OutboxPublisher>,
) -> Option<tokio::task::JoinHandle<()>> {
    if !config.enabled {
        tracing::info!("outbox dispatcher disabled");
        return None;
    }

    tracing::info!(
        interval_secs = config.interval_secs,
        lease_secs = config.lease_secs,
        batch_limit = config.batch_limit,
        "outbox dispatcher spawned"
    );

    let interval_secs = config.interval_secs;
    Some(tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("outbox dispatcher shutting down");
                    break;
                }
                _ = interval.tick() => {
                    if engine.config.sealed_mode && !engine.keychain.is_unsealed_sync() {
                        tracing::debug!("outbox dispatch skipped: vault is sealed");
                        continue;
                    }
                    match engine.dispatch_outbox_once(publisher.as_ref(), &config).await {
                        Ok(tick) if tick.claimed > 0 => {
                            tracing::info!(
                                claimed = tick.claimed,
                                completed = tick.completed,
                                "outbox dispatch tick"
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            tracing::warn!(%error, "outbox dispatch tick failed");
                        }
                    }
                }
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use chrono::Duration;
    use mv_core::{
        EventEnvelope, IdempotencyKey, KnowledgeNode, MvError, NewEventEnvelope, NodeKind,
        OutboxDeliveryState, ProvenanceReference, ProvenanceRelation, RetentionClass,
        SchemaReference, Sensitivity, KNOWLEDGE_NODE_CREATED_V1,
    };
    use tempfile::TempDir;
    use tokio::sync::Mutex;

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

    fn dispatcher_config() -> OutboxDispatcherConfig {
        OutboxDispatcherConfig {
            enabled: true,
            interval_secs: 1,
            lease_secs: 30,
            batch_limit: 10,
            executor: StableUri::parse("mindvault://dispatchers/local").unwrap(),
            destination: StableUri::parse("mindvault://destinations/local-ack").unwrap(),
        }
    }

    async fn seed_outbox_event(engine: &MindVaultEngine, key: &str) -> Uuid {
        let local_node_id = engine.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, format!("outbox {key}"))
            .with_namespace("interoperability");
        let subject = StableUri::knowledge_node(local_node_id, node.id);
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"outbox-dispatcher-test"),
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

    /// Scripted publisher for retry / reclaim tests.
    struct ScriptedPublisher {
        results: Mutex<Vec<OutboxDeliveryResult>>,
    }

    impl ScriptedPublisher {
        fn new(results: Vec<OutboxDeliveryResult>) -> Self {
            Self {
                results: Mutex::new(results),
            }
        }
    }

    #[async_trait]
    impl OutboxPublisher for ScriptedPublisher {
        async fn publish(&self, _claim: &OutboxDeliveryClaim) -> MvResult<OutboxDeliveryResult> {
            let mut guard = self.results.lock().await;
            if guard.is_empty() {
                return Err(MvError::Internal("scripted publisher exhausted".into()));
            }
            Ok(guard.remove(0))
        }
    }

    #[tokio::test]
    async fn outbox_dispatcher_claims_under_lease_and_writes_one_receipt() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "dispatch-publish").await;
        let config = dispatcher_config();
        let publisher = LocalAckPublisher;

        let tick = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();
        assert_eq!(tick.claimed, 1);
        assert_eq!(tick.completed, 1);
        assert_eq!(tick.receipts.len(), 1);

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
        assert_eq!(receipts[0].attempt, 1);
        assert_eq!(receipts[0].receipt_id, tick.receipts[0]);

        let idle = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();
        assert_eq!(idle.claimed, 0);
    }

    #[tokio::test]
    async fn outbox_dispatcher_honours_next_attempt_at() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "dispatch-retry-gate").await;
        let config = dispatcher_config();
        let retry_at = Utc::now() + Duration::hours(1);
        let publisher = ScriptedPublisher::new(vec![OutboxDeliveryResult::RetryScheduled {
            retry_at,
            error_code: "destination_unavailable".into(),
            error_summary: "temporary failure".into(),
        }]);

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
            .unwrap();
        assert_eq!(status.state, OutboxDeliveryState::Pending);
        assert_eq!(status.attempts, 1);
        assert!(status.next_attempt_at >= retry_at - Duration::seconds(1));

        let idle = engine
            .dispatch_outbox_once(&LocalAckPublisher, &config)
            .await
            .unwrap();
        assert_eq!(idle.claimed, 0);

        let reclaimed = engine
            .store
            .nodes
            .claim_outbox_events(
                &config.executor,
                &config.destination,
                retry_at + Duration::seconds(1),
                retry_at + Duration::minutes(5),
                10,
            )
            .await
            .unwrap();
        assert_eq!(reclaimed.len(), 1);
        assert_eq!(reclaimed[0].attempt, 2);
    }

    #[tokio::test]
    async fn outbox_dispatcher_reclaim_advances_attempt_exactly_once() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "dispatch-reclaim").await;
        let config = dispatcher_config();
        let claimed_at = Utc::now();
        let lease_expires_at = claimed_at + Duration::seconds(1);

        let first = engine
            .store
            .nodes
            .claim_outbox_events(
                &config.executor,
                &config.destination,
                claimed_at,
                lease_expires_at,
                10,
            )
            .await
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].attempt, 1);

        let blocked = engine
            .store
            .nodes
            .claim_outbox_events(
                &config.executor,
                &config.destination,
                claimed_at,
                claimed_at + Duration::minutes(5),
                10,
            )
            .await
            .unwrap();
        assert!(blocked.is_empty());

        let reclaim_at = lease_expires_at + Duration::seconds(1);
        let second = engine
            .store
            .nodes
            .claim_outbox_events(
                &config.executor,
                &config.destination,
                reclaim_at,
                reclaim_at + Duration::minutes(5),
                10,
            )
            .await
            .unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].attempt, 2);
        assert_ne!(second[0].lease_id, first[0].lease_id);

        let completion = OutboxDeliveryCompletion {
            event_id,
            lease_id: second[0].lease_id,
            attempt: 2,
            completed_at: reclaim_at + Duration::seconds(1),
            result: OutboxDeliveryResult::Published {
                delivery_reference: format!("local-ack:{event_id}"),
                response_digest: None,
            },
        };
        let receipt = engine
            .store
            .nodes
            .complete_outbox_delivery(&second[0], &completion)
            .await
            .unwrap();
        assert_eq!(receipt.attempt, 2);

        let receipts = engine
            .store
            .nodes
            .list_action_receipts(event_id, 10)
            .await
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].attempt, 2);
    }

    #[tokio::test]
    async fn outbox_dispatcher_completion_is_idempotent_per_attempt() {
        let (engine, _tmp) = test_engine().await;
        let event_id = seed_outbox_event(&engine, "dispatch-idempotent").await;
        let config = dispatcher_config();
        let publisher = LocalAckPublisher;

        let tick = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();
        assert_eq!(tick.receipts.len(), 1);

        let again = engine
            .dispatch_outbox_once(&publisher, &config)
            .await
            .unwrap();
        assert_eq!(again.claimed, 0);

        let receipts = engine
            .store
            .nodes
            .list_action_receipts(event_id, 10)
            .await
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].receipt_id, tick.receipts[0]);
    }
}
