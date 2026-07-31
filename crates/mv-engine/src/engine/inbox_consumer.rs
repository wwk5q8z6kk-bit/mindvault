//! Consumer inbox runtime (IK-006).
//!
//! Storage already owns admission, exclusive leases, application receipts, and
//! checkpoint invariants (`admit_consumer_event` → `claim_consumer_events` →
//! `complete_consumer_event`). This module is the missing worker: it admits
//! inbound envelopes, claims under a lease, invokes a domain handler, and
//! completes with exactly one immutable receipt.
//!
//! Remote transport listeners and signature verification are deferred (IK-014).
//! This slice ships in-process fixtures and an env-gated background loop.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use mv_core::{
    ConsumerApplicationCompletion, ConsumerApplicationResult, ConsumerInboxAdmission,
    ConsumerInboxClaim, EventEnvelope, InteroperabilityStore, MvResult, StableUri,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::MindVaultEngine;

/// One domain application attempt against a claimed inbox event.
#[async_trait]
pub trait InboxDomainHandler: Send + Sync {
    async fn apply(&self, claim: &ConsumerInboxClaim) -> MvResult<ConsumerApplicationResult>;
}

/// Applies locally with a deterministic application reference. No network I/O.
#[derive(Debug, Default, Clone)]
pub struct LocalProjectionHandler;

#[async_trait]
impl InboxDomainHandler for LocalProjectionHandler {
    async fn apply(&self, claim: &ConsumerInboxClaim) -> MvResult<ConsumerApplicationResult> {
        Ok(ConsumerApplicationResult::Applied {
            application_reference: format!("local-projection:{}", claim.event.id),
            effect_digest: None,
        })
    }
}

/// No-op handler that always reports a durable local application.
#[derive(Debug, Default, Clone)]
pub struct NoopDomainHandler;

#[async_trait]
impl InboxDomainHandler for NoopDomainHandler {
    async fn apply(&self, _claim: &ConsumerInboxClaim) -> MvResult<ConsumerApplicationResult> {
        Ok(ConsumerApplicationResult::Applied {
            application_reference: "noop-application".into(),
            effect_digest: None,
        })
    }
}

/// Runtime knobs for one consumer loop.
#[derive(Debug, Clone)]
pub struct InboxConsumerConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub lease_secs: u64,
    pub batch_limit: usize,
    pub consumer: StableUri,
    pub processor: StableUri,
}

impl InboxConsumerConfig {
    pub fn local_defaults() -> Self {
        Self {
            enabled: true,
            interval_secs: 5,
            lease_secs: 30,
            batch_limit: 32,
            consumer: StableUri::parse("mindvault://consumers/local").expect("consumer URI"),
            processor: StableUri::parse("mindvault://processors/local").expect("processor URI"),
        }
    }

    pub fn from_env() -> Self {
        let mut config = Self::local_defaults();
        config.enabled = std::env::var("MINDVAULT_INBOX_CONSUMER_ENABLED")
            .ok()
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        if let Some(interval) = std::env::var("MINDVAULT_INBOX_CONSUMER_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            config.interval_secs = interval.max(1);
        }
        if let Some(lease) = std::env::var("MINDVAULT_INBOX_CONSUMER_LEASE_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            config.lease_secs = lease.clamp(1, 3600);
        }
        if let Some(limit) = std::env::var("MINDVAULT_INBOX_CONSUMER_BATCH_LIMIT")
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

/// Summary of one consumer tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxConsumerTick {
    pub claimed: usize,
    pub completed: usize,
    pub receipts: Vec<Uuid>,
}

impl MindVaultEngine {
    /// Durably admit one inbound event for a consumer.
    pub async fn admit_inbound_event(
        &self,
        consumer: &StableUri,
        event: &EventEnvelope,
    ) -> MvResult<ConsumerInboxAdmission> {
        self.ensure_unsealed_for_node_io().await?;
        let received_at = Utc::now();
        self.store
            .nodes
            .admit_consumer_event(consumer, event, received_at)
            .await
    }

    /// Claim eligible inbox events, apply each via the handler, and complete.
    pub async fn consume_inbox_once(
        &self,
        handler: &dyn InboxDomainHandler,
        config: &InboxConsumerConfig,
    ) -> MvResult<InboxConsumerTick> {
        self.ensure_unsealed_for_node_io().await?;
        let claimed_at = Utc::now();
        let lease_expires_at = claimed_at + config.lease_duration();
        let claims = self
            .store
            .nodes
            .claim_consumer_events(
                &config.consumer,
                &config.processor,
                claimed_at,
                lease_expires_at,
                config.batch_limit,
            )
            .await?;

        let mut receipts = Vec::with_capacity(claims.len());
        for claim in &claims {
            let result = match handler.apply(claim).await {
                Ok(result) => result,
                Err(error) => ConsumerApplicationResult::RetryScheduled {
                    retry_at: Utc::now() + ChronoDuration::seconds(30),
                    error_code: "handler_error".into(),
                    error_summary: truncate_error_summary(&error.to_string()),
                },
            };
            let completed_at = Utc::now();
            let completed_at = if completed_at >= claim.lease_expires_at {
                claim.claimed_at + ChronoDuration::milliseconds(1)
            } else {
                completed_at
            };
            let completion = ConsumerApplicationCompletion {
                inbox_sequence: claim.inbox_sequence,
                event_id: claim.event.id,
                lease_id: claim.lease_id,
                attempt: claim.attempt,
                completed_at,
                result,
            };
            let receipt = self
                .store
                .nodes
                .complete_consumer_event(claim, &completion)
                .await?;
            receipts.push(receipt.receipt_id);
        }

        Ok(InboxConsumerTick {
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
        return "handler failed".into();
    }
    if trimmed.len() <= MAX {
        trimmed.to_string()
    } else {
        trimmed[..MAX].to_string()
    }
}

/// Spawn the background claim loop. Disabled unless config says otherwise.
pub fn spawn_inbox_consumer(
    engine: Arc<MindVaultEngine>,
    mut shutdown_rx: broadcast::Receiver<()>,
    config: InboxConsumerConfig,
    handler: Arc<dyn InboxDomainHandler>,
) {
    if !config.enabled {
        tracing::info!("inbox consumer disabled");
        return;
    }

    tracing::info!(
        interval_secs = config.interval_secs,
        lease_secs = config.lease_secs,
        batch_limit = config.batch_limit,
        consumer = %config.consumer,
        "inbox consumer spawned"
    );

    let interval_secs = config.interval_secs;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("inbox consumer shutting down");
                    break;
                }
                _ = interval.tick() => {
                    if engine.config.sealed_mode && !engine.keychain.is_unsealed_sync() {
                        tracing::debug!("inbox consume skipped: vault is sealed");
                        continue;
                    }
                    match engine.consume_inbox_once(handler.as_ref(), &config).await {
                        Ok(tick) if tick.claimed > 0 => {
                            tracing::info!(
                                claimed = tick.claimed,
                                completed = tick.completed,
                                "inbox consumer tick"
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            tracing::warn!(%error, "inbox consumer tick failed");
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use chrono::Duration;
    use mv_core::{
        ConsumerApplicationOutcome, ConsumerInboxState, EventEnvelope, IdempotencyKey,
        KnowledgeNode, MvError, NewEventEnvelope, NodeKind, ProvenanceReference,
        ProvenanceRelation, RetentionClass, SchemaReference, Sensitivity,
        KNOWLEDGE_NODE_CREATED_V1,
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

    fn consumer_config() -> InboxConsumerConfig {
        InboxConsumerConfig {
            enabled: true,
            interval_secs: 1,
            lease_secs: 30,
            batch_limit: 10,
            consumer: StableUri::parse("mindvault://consumers/local-index").unwrap(),
            processor: StableUri::parse("mindvault://processors/local-index").unwrap(),
        }
    }

    async fn seed_inbox_event(engine: &MindVaultEngine, key: &str) -> EventEnvelope {
        let local_node_id = engine.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, format!("inbox {key}"))
            .with_namespace("interoperability");
        let subject = StableUri::knowledge_node(local_node_id, node.id);
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"inbox-consumer-test"),
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
        engine
            .store
            .nodes
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();
        event
    }

    struct ScriptedDomainHandler {
        results: Mutex<Vec<ConsumerApplicationResult>>,
    }

    impl ScriptedDomainHandler {
        fn new(results: Vec<ConsumerApplicationResult>) -> Self {
            Self {
                results: Mutex::new(results),
            }
        }
    }

    #[async_trait]
    impl InboxDomainHandler for ScriptedDomainHandler {
        async fn apply(&self, _claim: &ConsumerInboxClaim) -> MvResult<ConsumerApplicationResult> {
            let mut guard = self.results.lock().await;
            if guard.is_empty() {
                return Err(MvError::Internal("scripted handler exhausted".into()));
            }
            Ok(guard.remove(0))
        }
    }

    #[tokio::test]
    async fn inbox_consumer_happy_path_admits_claims_applies_and_advances_checkpoint() {
        let (engine, _tmp) = test_engine().await;
        let config = consumer_config();
        let event = seed_inbox_event(&engine, "consumer-happy").await;

        let admission = engine
            .admit_inbound_event(&config.consumer, &event)
            .await
            .unwrap();
        assert!(!admission.replayed);
        assert_eq!(admission.inbox_sequence, 1);

        let tick = engine
            .consume_inbox_once(&LocalProjectionHandler, &config)
            .await
            .unwrap();
        assert_eq!(tick.claimed, 1);
        assert_eq!(tick.completed, 1);
        assert_eq!(tick.receipts.len(), 1);

        let status = engine
            .store
            .nodes
            .get_consumer_inbox_status(&config.consumer, event.id)
            .await
            .unwrap()
            .expect("status");
        assert_eq!(status.state, ConsumerInboxState::Applied);
        assert_eq!(status.attempts, 1);

        let checkpoint = engine
            .store
            .nodes
            .get_consumer_checkpoint(&config.consumer, &event.source)
            .await
            .unwrap()
            .expect("checkpoint");
        assert_eq!(checkpoint.last_dispositioned_sequence, admission.inbox_sequence);
        assert_eq!(checkpoint.last_dispositioned_event_id, event.id);
        assert_eq!(
            checkpoint.last_applied_sequence,
            Some(admission.inbox_sequence)
        );
        assert_eq!(checkpoint.applied_count, 1);

        let idle = engine
            .consume_inbox_once(&LocalProjectionHandler, &config)
            .await
            .unwrap();
        assert_eq!(idle.claimed, 0);
    }

    #[tokio::test]
    async fn inbox_consumer_retry_does_not_advance_checkpoint() {
        let (engine, _tmp) = test_engine().await;
        let config = consumer_config();
        let event = seed_inbox_event(&engine, "consumer-retry").await;

        engine
            .admit_inbound_event(&config.consumer, &event)
            .await
            .unwrap();

        let retry_at = Utc::now() + Duration::hours(1);
        let handler = ScriptedDomainHandler::new(vec![ConsumerApplicationResult::RetryScheduled {
            retry_at,
            error_code: "projection_unavailable".into(),
            error_summary: "temporary projection failure".into(),
        }]);

        let tick = engine.consume_inbox_once(&handler, &config).await.unwrap();
        assert_eq!(tick.claimed, 1);
        assert_eq!(tick.completed, 1);

        assert!(
            engine
                .store
                .nodes
                .get_consumer_checkpoint(&config.consumer, &event.source)
                .await
                .unwrap()
                .is_none(),
            "retry must not advance checkpoint"
        );

        let status = engine
            .store
            .nodes
            .get_consumer_inbox_status(&config.consumer, event.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status.state, ConsumerInboxState::Pending);
        assert_eq!(status.attempts, 1);
        assert!(status.next_attempt_at >= retry_at - Duration::seconds(1));

        let blocked = engine
            .consume_inbox_once(&LocalProjectionHandler, &config)
            .await
            .unwrap();
        assert_eq!(blocked.claimed, 0);

        let reclaimed = engine
            .store
            .nodes
            .claim_consumer_events(
                &config.consumer,
                &config.processor,
                retry_at,
                retry_at + Duration::minutes(5),
                10,
            )
            .await
            .unwrap();
        assert_eq!(reclaimed.len(), 1);
        assert_eq!(reclaimed[0].attempt, 2);

        let applied = ConsumerApplicationCompletion {
            inbox_sequence: reclaimed[0].inbox_sequence,
            event_id: event.id,
            lease_id: reclaimed[0].lease_id,
            attempt: reclaimed[0].attempt,
            completed_at: retry_at + Duration::seconds(1),
            result: ConsumerApplicationResult::Applied {
                application_reference: "local-index-entry".into(),
                effect_digest: Some("3".repeat(64)),
            },
        };
        engine
            .store
            .nodes
            .complete_consumer_event(&reclaimed[0], &applied)
            .await
            .unwrap();

        let checkpoint = engine
            .store
            .nodes
            .get_consumer_checkpoint(&config.consumer, &event.source)
            .await
            .unwrap()
            .expect("checkpoint after apply");
        assert_eq!(checkpoint.last_dispositioned_sequence, 1);
        assert_eq!(checkpoint.applied_count, 1);

        let receipts = engine
            .store
            .nodes
            .list_consumer_application_receipts(&config.consumer, event.id, 10)
            .await
            .unwrap();
        assert_eq!(receipts.len(), 2);
        assert_eq!(
            receipts[0].outcome,
            ConsumerApplicationOutcome::RetryScheduled
        );
        assert_eq!(receipts[1].outcome, ConsumerApplicationOutcome::Applied);
    }

    #[tokio::test]
    async fn inbox_consumer_exclusive_claim_blocks_second_claim_while_leased() {
        let (engine, _tmp) = test_engine().await;
        let config = consumer_config();
        let event = seed_inbox_event(&engine, "consumer-exclusive").await;

        engine
            .admit_inbound_event(&config.consumer, &event)
            .await
            .unwrap();

        let claimed_at = Utc::now();
        let lease_expires_at = claimed_at + Duration::minutes(5);
        let first = engine
            .store
            .nodes
            .claim_consumer_events(
                &config.consumer,
                &config.processor,
                claimed_at,
                lease_expires_at,
                1,
            )
            .await
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].attempt, 1);

        let blocked = engine
            .store
            .nodes
            .claim_consumer_events(
                &config.consumer,
                &config.processor,
                claimed_at + Duration::seconds(1),
                claimed_at + Duration::minutes(10),
                1,
            )
            .await
            .unwrap();
        assert!(blocked.is_empty());

        let reclaim_at = lease_expires_at + Duration::seconds(1);
        let second = engine
            .store
            .nodes
            .claim_consumer_events(
                &config.consumer,
                &config.processor,
                reclaim_at,
                reclaim_at + Duration::minutes(5),
                1,
            )
            .await
            .unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].attempt, 2);
        assert_ne!(second[0].lease_id, first[0].lease_id);
    }
}
