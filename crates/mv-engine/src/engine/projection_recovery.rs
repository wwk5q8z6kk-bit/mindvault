//! Durable projection checkpoint and recovery worker (IK-016).
//!
//! Canonical node create + outbox commit are atomic. FTS/vector/graph run
//! afterward. A crash between those boundaries leaves a pending (or even
//! transport-published) outbox event without local projections. This worker
//! discovers unrecovered `knowledge.node.created` events, re-applies
//! projections, and records a durable checkpoint — without claiming or
//! completing outbox delivery.

use chrono::Utc;
use mv_core::{
    InteroperabilityStore, MvResult, NodeStore, ProjectionCheckpoint, KNOWLEDGE_NODE_CREATED_V1,
};

use super::MindVaultEngine;

/// Result of one projection recovery tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectionReconcileTick {
    pub examined: usize,
    pub projected: usize,
    pub errors: usize,
}

impl MindVaultEngine {
    /// Reconcile local projections for outbox events missing a ready checkpoint.
    pub async fn reconcile_projections_once(
        &self,
        limit: usize,
    ) -> MvResult<ProjectionReconcileTick> {
        let events = self
            .store
            .nodes
            .list_outbox_events_needing_projection(KNOWLEDGE_NODE_CREATED_V1, limit)
            .await?;
        let mut tick = ProjectionReconcileTick {
            examined: events.len(),
            ..Default::default()
        };

        for event in events {
            let Some(node_id) = event.subject.trailing_uuid() else {
                tracing::warn!(
                    event_id = %event.id,
                    "projection recovery skipped: subject has no node UUID"
                );
                tick.errors += 1;
                continue;
            };

            let Some(node) = self.store.nodes.get(node_id).await? else {
                tracing::warn!(
                    event_id = %event.id,
                    node_id = %node_id,
                    "projection recovery skipped: canonical node missing"
                );
                tick.errors += 1;
                continue;
            };

            let now = Utc::now();
            match self.ingest.index_and_enrich(&node).await {
                Ok(()) => {
                    self.auto_link_node_to_daily_note_best_effort(&node).await;
                    self.auto_backlink_node_references_best_effort(&node)
                        .await;
                    self.store
                        .nodes
                        .upsert_projection_checkpoint(&ProjectionCheckpoint::ready(
                            event.id, node_id, now,
                        ))
                        .await?;
                    tick.projected += 1;
                }
                Err(error) => {
                    tracing::error!(
                        event_id = %event.id,
                        node_id = %node_id,
                        error = %error,
                        "projection recovery failed"
                    );
                    let _ = self
                        .store
                        .nodes
                        .upsert_projection_checkpoint(&ProjectionCheckpoint::failed(
                            event.id,
                            node_id,
                            now,
                            error.to_string(),
                        ))
                        .await;
                    tick.errors += 1;
                }
            }
        }

        Ok(tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use mv_core::{
        EventEnvelope, FullTextIndex, GraphStore, IdempotencyKey, KnowledgeNode, MvError,
        NewEventEnvelope, NodeKind, OutboxDeliveryState, ProjectionCheckpointStatus,
        ProvenanceReference, ProvenanceRelation, RelationKind, RetentionClass, SchemaReference,
        Sensitivity, StableUri,
    };
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn test_engine() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        config.llm.auto_detect = false;
        config.linking.auto_backlinks_enabled = true;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    #[tokio::test]
    async fn projection_recovery_reconciles_crash_between_commit_and_index() {
        let (engine, _tmp) = test_engine().await;
        let namespace = "projection-recovery".to_string();

        let target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Target note body")
                    .with_namespace(namespace.clone())
                    .with_title("Recovery Target"),
            )
            .await
            .unwrap();

        let local_node_id = engine.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "See [[Recovery Target]] for details unique-proj-token-42",
        )
        .with_namespace(namespace.clone())
        .with_title("Recovery Source");
        let subject = StableUri::knowledge_node(local_node_id, node.id);
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"projection-recovery-test"),
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
            idempotency_key: IdempotencyKey::parse("projection-recovery-crash").unwrap(),
            payload_digest: "b".repeat(64),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: subject,
                relation: ProvenanceRelation::PrimarySource,
            }],
            data: serde_json::json!({
                "resource_kind": "knowledge_node",
                "node_kind": "fact",
                "namespace": "projection-recovery",
            }),
        })
        .unwrap();
        let event_id = event.id;
        let node_id = node.id;

        // Simulate crash: durable commit without projections.
        engine
            .store
            .nodes
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();

        let pending = engine
            .store
            .nodes
            .list_pending_outbox_events(10)
            .await
            .unwrap();
        assert!(
            pending.iter().any(|e| e.id == event_id),
            "crash window must leave a pending outbox event"
        );
        assert!(engine
            .store
            .nodes
            .get_projection_checkpoint(event_id)
            .await
            .unwrap()
            .is_none());

        let fts_before = engine.fts.search("unique-proj-token-42", 10).unwrap();
        assert!(
            !fts_before.iter().any(|(id, _)| *id == node_id),
            "FTS must miss the node before recovery"
        );
        let edges_before = engine.graph.get_relationships_from(node_id).await.unwrap();
        assert!(
            !edges_before
                .iter()
                .any(|rel| rel.kind == RelationKind::References && rel.to_node == target.id),
            "graph must miss the References edge before recovery"
        );

        let tick = engine.reconcile_projections_once(10).await.unwrap();
        assert_eq!(tick.examined, 1);
        assert_eq!(tick.projected, 1);
        assert_eq!(tick.errors, 0);

        let fts_after = engine.fts.search("unique-proj-token-42", 10).unwrap();
        assert!(
            fts_after.iter().any(|(id, _)| *id == node_id),
            "FTS must converge after recovery"
        );

        let embedding = engine.store.embedder.embed(&node.content).await.unwrap();
        let vectors = engine.store.vectors.as_ref().expect("vector store");
        let vector_hits = vectors
            .search(embedding, 5, 0.0, Some(&namespace))
            .await
            .unwrap();
        assert!(
            vector_hits.iter().any(|(id, _)| *id == node_id),
            "vector index must converge after recovery"
        );

        let edges_after = engine.graph.get_relationships_from(node_id).await.unwrap();
        assert!(
            edges_after
                .iter()
                .any(|rel| rel.kind == RelationKind::References && rel.to_node == target.id),
            "graph must converge after recovery"
        );

        let checkpoint = engine
            .store
            .nodes
            .get_projection_checkpoint(event_id)
            .await
            .unwrap()
            .expect("projection checkpoint");
        assert_eq!(checkpoint.status, ProjectionCheckpointStatus::Ready);
        assert_eq!(checkpoint.node_id, node_id);

        // Transport delivery state stays pending — projection ≠ published.
        let status = engine
            .store
            .nodes
            .get_outbox_delivery_status(event_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status.state, OutboxDeliveryState::Pending);

        let second = engine.reconcile_projections_once(10).await.unwrap();
        assert_eq!(second.examined, 0);
        assert_eq!(second.projected, 0);
    }

    #[tokio::test]
    async fn projection_recovery_rejects_empty_event_type_limit() {
        let (engine, _tmp) = test_engine().await;
        let err = engine
            .store
            .nodes
            .list_outbox_events_needing_projection("", 10)
            .await
            .expect_err("empty event type");
        assert!(matches!(err, MvError::InvalidInput(_)));
    }
}
