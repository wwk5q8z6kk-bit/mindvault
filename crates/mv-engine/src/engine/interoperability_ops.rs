//! Command admission against the authority-grant layer.
//!
//! Contract: `docs/architecture/AUTHORITY_GRANT_MODEL.md`.
//! Slice: `docs/architecture/interoperability-kernel-v1.md:283-284` — "the next
//! gated kernel slice should wire action-envelope and grant admission ahead of
//! any live provider publisher".
//!
//! Constitutional law 7 requires an explicit grant for context access; law 8
//! requires action authority to be a separate axis, so reading never implies
//! authority to mutate, execute, transmit, or spend. This module answers one
//! question — may this actor perform this operation on this target, now — and
//! answers it fail-closed.
//!
//! Admission is additive. It never replaces the role, namespace, or quota
//! checks that already guard a command; `interoperability-kernel-v1.md:252-253`
//! requires those to stay in place.

use chrono::Utc;
use mv_core::*;

use super::MindVaultEngine;

impl MindVaultEngine {
    /// Resolve whether one command is authorized by an effective grant.
    ///
    /// Returns `Denied` rather than an error when authorization simply fails —
    /// a denial is a decision, and the caller records it. `Err` is reserved for
    /// a resolver that could not reach an answer, which callers must treat as a
    /// denial too (`AUTHORITY_GRANT_MODEL.md:86`, fail closed).
    pub async fn resolve_command_admission(
        &self,
        request: &CommandAdmissionRequest,
    ) -> MvResult<AdmissionDecision> {
        // Ordered first so a sealed vault yields the canonical VaultSealed error
        // rather than the decrypt failure that reading a sealed governance
        // record would otherwise produce.
        self.ensure_unsealed_for_node_io().await?;

        request
            .validate()
            .map_err(|err| MvError::InvalidInput(format!("command admission request: {err}")))?;

        let grant = self
            .store
            .nodes
            .find_authorizing_grant(GrantQuery {
                grantee: &request.actor,
                kind: request.required_grant_kind,
                target: &request.resource,
                capability: request.operation,
                sensitivity: request.sensitivity,
                retention: request.retention,
                at: request.requested_at,
            })
            .await?;

        match grant {
            Some(grant) => Ok(AdmissionDecision::Admitted {
                grant_id: grant.grant_id,
                grant_uri: grant.grant_uri.clone(),
                grant_kind: grant.kind,
                capability: request.operation,
                delegation_depth_remaining: grant.delegation_depth_remaining,
                decided_at: Utc::now(),
            }),
            None => Ok(AdmissionDecision::Denied {
                reason: self.classify_admission_denial(request).await,
                decided_at: Utc::now(),
            }),
        }
    }

    /// Narrow a refusal to a bounded reason, for the audit trail only.
    ///
    /// Runs strictly after the resolver already returned `None`, so it cannot
    /// admit anything — the worst case is a less precise audit line. Reasons are
    /// never returned to callers: distinguishing "you hold no grant" from "your
    /// grant expired" is a probing oracle. Law 15 requires a denial be recorded,
    /// not disclosed.
    async fn classify_admission_denial(
        &self,
        request: &CommandAdmissionRequest,
    ) -> AdmissionDenialReason {
        let candidates = match self
            .store
            .nodes
            .list_authority_grants(Some(&request.actor), None, None)
            .await
        {
            Ok(candidates) => candidates,
            // The lookup already failed closed; we simply cannot say why.
            Err(_) => return AdmissionDenialReason::ResolverUnavailable,
        };

        if candidates.is_empty() {
            return AdmissionDenialReason::NoEffectiveGrant;
        }

        // Report the failure of the closest candidate: one that matched kind and
        // target is more informative than one that matched neither.
        let mut best = AdmissionDenialReason::NoEffectiveGrant;
        let mut best_rank = 0u8;
        for grant in &candidates {
            let (reason, rank) = Self::rank_grant_refusal(grant, request);
            if rank > best_rank {
                best = reason;
                best_rank = rank;
            }
        }
        best
    }

    /// Score how far one grant got before refusing. Higher means closer.
    fn rank_grant_refusal(
        grant: &AuthorityGrant,
        request: &CommandAdmissionRequest,
    ) -> (AdmissionDenialReason, u8) {
        if grant.kind != request.required_grant_kind {
            return (AdmissionDenialReason::GrantKindMismatch, 1);
        }
        if !grant.targets.iter().any(|t| t == &request.resource) {
            return (AdmissionDenialReason::TargetNotGranted, 2);
        }
        if !grant.capabilities.contains(&request.operation) {
            return (AdmissionDenialReason::CapabilityNotGranted, 3);
        }
        if request.sensitivity.rank() > grant.sensitivity_ceiling.rank() {
            return (AdmissionDenialReason::SensitivityCeilingExceeded, 4);
        }
        if request.retention.rank() > grant.retention_ceiling.rank() {
            return (AdmissionDenialReason::RetentionCeilingExceeded, 5);
        }
        if !grant.is_effective_at(request.requested_at) {
            return (AdmissionDenialReason::GrantNotEffective, 6);
        }
        // Every term this grant states is satisfied, yet the resolver refused.
        // The difference is the parent chain, which the resolver walks and this
        // per-grant view does not.
        (AdmissionDenialReason::DelegationChainIneffective, 7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;
    use chrono::Duration;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn test_engine() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = "noop".into();
        // Keep the fixture hermetic: auto-detect probes a local Ollama.
        config.llm.auto_detect = false;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    /// Register the local Context Node so governance events are admissible.
    ///
    /// `commit_authority_grant_with_event` refuses a grant whose governing node
    /// has no Active registered descriptor, so every grant fixture needs this.
    async fn register_local_node(engine: &MindVaultEngine) -> Uuid {
        let local_node_id = engine.store.nodes.local_context_node_id().await.unwrap();
        let manifest = ContextCapabilityManifest::new(
            vec![ContextCapability::Discover],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let owner = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"local-context-owner"),
        );
        let mut record = ContextNodeRecord::discovered(
            local_node_id,
            ContextNodeType::Personal,
            owner,
            StableUri::node(local_node_id),
            "Personal Vault",
            manifest,
        )
        .unwrap();
        record.trust_class = ContextNodeTrustClass::local();
        record.status = ContextNodeStatus::Active;

        let data = serde_json::json!({
            "node_id": record.node_id,
            "node_type": record.node_type.as_str(),
            "status": record.status.as_str(),
            "record_digest": record.semantic_digest(),
            "capability_digest": record.capability_manifest.content_digest,
        });
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"admission-test-principal"),
        );
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: CONTEXT_NODE_REGISTERED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: record.node_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("context-node-registered").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: principal.clone(),
            actor: principal,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse("admission-register-local-node").unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: record.node_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        event.payload_digest = record.semantic_digest();

        engine
            .store
            .nodes
            .commit_context_node_with_event(&record, &event)
            .await
            .unwrap();
        local_node_id
    }

    async fn commit_grant(engine: &MindVaultEngine, local_node_id: Uuid, grant: &AuthorityGrant) {
        let data = serde_json::json!({
            "grant_id": grant.grant_id,
            "grant_kind": grant.kind.as_str(),
            "grantee_uri": grant.grantee.as_str(),
            "governing_node_uri": grant.governing_node.as_str(),
            "record_digest": grant.semantic_digest(),
        });
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: AUTHORITY_GRANT_ISSUED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: grant.grant_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("authority-grant-issued").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: grant.grantor.clone(),
            actor: grant.grantor.clone(),
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(format!("admission-grant-{}", grant.grant_id))
                .unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: grant.grant_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap();
        event.payload_digest = grant.semantic_digest();

        engine
            .store
            .nodes
            .commit_authority_grant_with_event(grant, &event)
            .await
            .unwrap();
    }

    fn node_create_request(
        local_node_id: Uuid,
        grantee: &StableUri,
        kind: AuthorityGrantKind,
        operation: ContextCapability,
    ) -> CommandAdmissionRequest {
        CommandAdmissionRequest {
            request_id: Uuid::now_v7(),
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            principal: grantee.clone(),
            actor: grantee.clone(),
            governing_node: StableUri::node(local_node_id),
            // The only satisfiable target for a create: the subject id is minted
            // after admission, so no grant could name it.
            resource: StableUri::node(local_node_id),
            subject: StableUri::knowledge_node(local_node_id, Uuid::now_v7()),
            operation,
            required_grant_kind: kind,
            idempotency_key: IdempotencyKey::parse("admission-node-create").unwrap(),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            requested_at: Utc::now(),
        }
    }

    fn grantee_of(local_node_id: Uuid) -> StableUri {
        StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"admission-grantee"),
        )
    }

    /// The grant shape a node create actually needs, end to end.
    ///
    /// Targets the governing node URI and raises `retention_ceiling` to
    /// `Durable`, because `new_tool` defaults it to `Operational` while the
    /// node-create envelope declares `Durable`.
    #[tokio::test]
    async fn a_node_scoped_tool_grant_authorizes_a_local_command() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let node_uri = StableUri::node(local_node_id);

        let mut grant = AuthorityGrant::new_tool(
            node_uri.clone(),
            node_uri.clone(),
            grantee.clone(),
            vec![node_uri.clone()],
            vec![ContextCapability::Command],
            "admit node creates for the owner",
            Utc::now() + Duration::days(1),
        )
        .unwrap();
        grant.retention_ceiling = RetentionClass::Durable;
        commit_grant(&engine, local_node_id, &grant).await;

        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );
        let decision = engine.resolve_command_admission(&request).await.unwrap();

        assert!(decision.is_admitted(), "unexpected refusal: {decision:?}");
        match decision {
            AdmissionDecision::Admitted {
                grant_id,
                grant_kind,
                capability,
                ..
            } => {
                assert_eq!(grant_id, grant.grant_id);
                assert_eq!(grant_kind, AuthorityGrantKind::Tool);
                assert_eq!(capability, ContextCapability::Command);
            }
            AdmissionDecision::Denied { .. } => unreachable!(),
        }
    }

    /// Law 8, end to end: reading never implies authority to mutate.
    ///
    /// A Context Grant covering the same target with the same grantee must not
    /// admit a command, and the refusal must be recorded as a kind mismatch
    /// rather than a bare "no grant".
    #[tokio::test]
    async fn a_context_grant_never_authorizes_a_command() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let node_uri = StableUri::node(local_node_id);

        let mut grant = AuthorityGrant::new_context(
            node_uri.clone(),
            node_uri.clone(),
            grantee.clone(),
            vec![node_uri.clone()],
            vec![ContextCapability::Read],
            "read the vault",
            Utc::now() + Duration::days(1),
        )
        .unwrap();
        grant.retention_ceiling = RetentionClass::Durable;
        commit_grant(&engine, local_node_id, &grant).await;

        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );
        let decision = engine.resolve_command_admission(&request).await.unwrap();

        assert!(!decision.is_admitted(), "a context grant admitted a mutation");
        assert!(matches!(
            decision,
            AdmissionDecision::Denied {
                reason: AdmissionDenialReason::GrantKindMismatch,
                ..
            }
        ));
    }

    /// With no grants at all the refusal is unambiguous.
    #[tokio::test]
    async fn an_actor_without_any_grant_is_refused() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);

        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );
        let decision = engine.resolve_command_admission(&request).await.unwrap();

        assert!(matches!(
            decision,
            AdmissionDecision::Denied {
                reason: AdmissionDenialReason::NoEffectiveGrant,
                ..
            }
        ));
    }

    /// The default retention ceiling refuses a durable create, and the audit
    /// reason says so rather than blaming the target or capability.
    #[tokio::test]
    async fn a_default_retention_ceiling_refuses_a_durable_command() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let node_uri = StableUri::node(local_node_id);

        // Deliberately left at the `new_tool` default of Operational.
        let grant = AuthorityGrant::new_tool(
            node_uri.clone(),
            node_uri.clone(),
            grantee.clone(),
            vec![node_uri.clone()],
            vec![ContextCapability::Command],
            "admit node creates for the owner",
            Utc::now() + Duration::days(1),
        )
        .unwrap();
        assert_eq!(grant.retention_ceiling, RetentionClass::Operational);
        commit_grant(&engine, local_node_id, &grant).await;

        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );
        let decision = engine.resolve_command_admission(&request).await.unwrap();

        assert!(matches!(
            decision,
            AdmissionDecision::Denied {
                reason: AdmissionDenialReason::RetentionCeilingExceeded,
                ..
            }
        ));
    }

    /// A request that pairs a Context Grant with an effectful capability is a
    /// construction error, caught before any storage lookup.
    #[tokio::test]
    async fn an_incoherent_request_is_rejected_before_resolution() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);

        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Context,
            ContextCapability::Command,
        );
        let error = engine
            .resolve_command_admission(&request)
            .await
            .expect_err("a context grant cannot carry a command");
        assert!(matches!(error, MvError::InvalidInput(_)), "got {error:?}");
    }
}
