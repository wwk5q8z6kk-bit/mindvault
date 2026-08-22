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
use uuid::Uuid;

use super::MindVaultEngine;

/// Outcome of [`MindVaultEngine::register_local_context_node`].
///
/// `newly_registered` is decided by storage — either the early existing-descriptor
/// read, or the idempotent commit's `replayed` flag — so callers never race a
/// separate "is it there?" check against the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalContextNodeRegistration {
    pub record: ContextNodeRecord,
    pub newly_registered: bool,
}

/// Outcome of a governed public-schema registration command.
#[derive(Debug, Clone, PartialEq)]
pub struct PublicSchemaRegistration {
    pub record: PublicSchemaRecord,
    pub newly_registered: bool,
}

/// Outcome of a governed Source Binding registration command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBindingRegistration {
    pub record: SourceBinding,
    pub newly_registered: bool,
}

/// Outcome of registering a newly discovered Context Node descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextNodeRegistration {
    pub record: ContextNodeRecord,
    pub newly_registered: bool,
}

/// Outcome of [`MindVaultEngine::register_identity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRegistration {
    pub record: IdentityRecord,
    pub newly_registered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapLocalIdentities {
    pub local_system: IdentityRecord,
    pub local_context_owner: IdentityRecord,
}

/// Outcome of [`MindVaultEngine::issue_authority_grant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityGrantIssuance {
    pub grant: AuthorityGrant,
    pub newly_issued: bool,
}

/// Outcome of a grant lifecycle transition (suspend / revoke / resume).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityGrantTransition {
    pub grant: AuthorityGrant,
    pub replayed: bool,
}

/// Parameters for issuing one Context or Tool Grant through the governed command.
///
/// The grantor is always this vault's local owner principal. Admin REST auth
/// decides who may call the command; the record attributes issuance to the
/// self-governed owner so an admin can grant to their own acting principal
/// without violating `grantor != grantee`.
#[derive(Debug, Clone)]
pub struct IssueAuthorityGrantRequest {
    pub kind: AuthorityGrantKind,
    pub grantee: StableUri,
    pub targets: Vec<StableUri>,
    pub capabilities: Vec<ContextCapability>,
    pub purpose: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub sensitivity_ceiling: Sensitivity,
    pub retention_ceiling: RetentionClass,
    pub allow_redistribution: bool,
    pub allow_model_training: bool,
    pub delegation_depth_remaining: u8,
    pub idempotency_key: IdempotencyKey,
}

/// Parameters for deriving one strictly narrower grant from an existing parent.
#[derive(Debug, Clone)]
pub struct DelegateAuthorityGrantRequest {
    pub parent_grant_id: Uuid,
    pub grantor: StableUri,
    pub grantee: StableUri,
    pub targets: Vec<StableUri>,
    pub capabilities: Vec<ContextCapability>,
    pub purpose: String,
    pub not_before: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub sensitivity_ceiling: Sensitivity,
    pub retention_ceiling: RetentionClass,
    pub allow_redistribution: bool,
    pub allow_model_training: bool,
    pub delegation_depth_remaining: u8,
    pub idempotency_key: IdempotencyKey,
}

/// Capabilities the local node declares as a Context Node.
///
/// Deliberately narrow, and narrower than what the REST API can do. A
/// capability manifest is a public claim about what this node offers *through
/// governed Context Node contracts*, not an inventory of every handler. Query
/// and Read are absent because the governed query transport is still gated
/// (`interoperability-kernel-v1.md`); declaring them would over-claim exactly
/// the way the constitution's "no UI-only capability" law exists to prevent.
///
/// Expand this list when a governed contract for the capability actually ships,
/// and expect the manifest revision and digest to change with it.
/// Order matters. `ContextCapabilityManifest::validate` requires capabilities to
/// be strictly sorted by their wire token, so this list is in `as_str()` order
/// ("command", "discover", "health") rather than conceptual order.
const LOCAL_NODE_CAPABILITIES: [ContextCapability; 3] = [
    ContextCapability::Command,
    ContextCapability::Discover,
    ContextCapability::Health,
];

/// Outcome of [`MindVaultEngine::redrive_outbox_dead_letter`].
#[derive(Debug, Clone, PartialEq)]
pub struct DeadLetterRedriveOutcome {
    pub source_event_id: Uuid,
    pub redrive_event: EventEnvelope,
    pub replayed: bool,
}

impl MindVaultEngine {
    /// Register this vault's own Context Node descriptor, idempotently.
    ///
    /// Bootstrapping problem this solves: `commit_authority_grant_with_event`
    /// refuses a grant whose governing node has no active registered
    /// descriptor, and `ensure_local_context_node` only allocates the identity
    /// UUID — not the descriptor. So a fresh vault cannot be issued any grant,
    /// which in turn makes command admission unusable in `enforce`.
    ///
    /// Registering the self-governed local node directly as active is sanctioned
    /// by `docs/architecture/CONTEXT_NODE_MODEL.md`: "Initial local bootstrap
    /// may register the self-governed local node directly as active". That
    /// exemption is for *this* node only. Every remote node still requires
    /// discovery, key proof, and signature verification, which remain gated.
    ///
    /// Returns the existing descriptor unchanged when one is already present,
    /// so this is safe to call on every startup or from an operator command.
    pub async fn register_local_context_node(
        &self,
        display_name: &str,
    ) -> MvResult<LocalContextNodeRegistration> {
        self.ensure_unsealed_for_node_io().await?;
        let local_node_id = self.store.nodes.local_context_node_id().await?;

        if let Some(existing) = self.store.nodes.get_context_node(local_node_id).await? {
            // Migration 040 can be applied to a vault whose local Context Node
            // descriptor predates the identity registry. Re-running this
            // idempotent bootstrap ensures those vaults receive the canonical
            // local principals too.
            self.bootstrap_local_identities(local_node_id).await?;
            return Ok(LocalContextNodeRegistration {
                record: existing,
                newly_registered: false,
            });
        }

        let manifest = ContextCapabilityManifest::new(
            LOCAL_NODE_CAPABILITIES.to_vec(),
            Vec::new(),
            Vec::new(),
        )
        .map_err(MvError::InvalidInput)?;

        let owner = self.derived_owner_principal(local_node_id);
        let mut record = ContextNodeRecord::discovered(
            local_node_id,
            ContextNodeType::Personal,
            owner.clone(),
            StableUri::node(local_node_id),
            display_name,
            manifest,
        )
        .map_err(MvError::InvalidInput)?;
        record.trust_class = ContextNodeTrustClass::local();
        record.status = ContextNodeStatus::Active;

        let data = serde_json::json!({
            "node_id": record.node_id,
            "node_type": record.node_type.as_str(),
            "status": record.status.as_str(),
            "record_digest": record.semantic_digest(),
            "capability_digest": record.capability_manifest.content_digest,
        });
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: CONTEXT_NODE_REGISTERED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: record.node_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("context-node-registered").map_err(MvError::InvalidInput)?,
                "1.0.0",
            )
            .map_err(MvError::InvalidInput)?,
            principal: owner.clone(),
            actor: owner,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            // Scoped to the node identity, so a concurrent second call collapses
            // onto the same registration rather than creating a rival one.
            idempotency_key: IdempotencyKey::parse(format!("register-local-node-{local_node_id}"))
                .map_err(MvError::InvalidInput)?,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: record.node_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)?;
        event.payload_digest = record.semantic_digest();
        let commit = self
            .store
            .nodes
            .commit_context_node_with_event(&record, &event)
            .await?;
        let registration = LocalContextNodeRegistration {
            record: commit.context_node,
            newly_registered: !commit.replayed,
        };
        self.bootstrap_local_identities(local_node_id).await?;
        Ok(registration)
    }

    /// Read the local Context Node descriptor, if it has been registered.
    pub async fn local_context_node(&self) -> MvResult<Option<ContextNodeRecord>> {
        self.ensure_unsealed_for_node_io().await?;
        let local_node_id = self.store.nodes.local_context_node_id().await?;
        self.store.nodes.get_context_node(local_node_id).await
    }

    /// Register one immutable public schema version and its durable event.
    pub async fn register_public_schema(
        &self,
        record: PublicSchemaRecord,
        event: EventEnvelope,
    ) -> MvResult<PublicSchemaRegistration> {
        self.ensure_unsealed_for_node_io().await?;
        let commit = self
            .store
            .nodes
            .commit_public_schema_with_event(&record, &event)
            .await?;
        Ok(PublicSchemaRegistration {
            record: commit.schema,
            newly_registered: !commit.replayed,
        })
    }

    /// Fetch one immutable public schema version.
    pub async fn get_public_schema(
        &self,
        reference: &SchemaReference,
    ) -> MvResult<Option<PublicSchemaRecord>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.get_public_schema(reference).await
    }

    /// List all registered versions for one stable public schema URI.
    pub async fn list_public_schema_versions(
        &self,
        schema_uri: &StableUri,
    ) -> MvResult<Vec<PublicSchemaRecord>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store
            .nodes
            .list_public_schema_versions(schema_uri)
            .await
    }

    /// Register one governed Source Binding and its durable event.
    pub async fn register_source_binding(
        &self,
        record: SourceBinding,
        event: EventEnvelope,
    ) -> MvResult<SourceBindingRegistration> {
        self.ensure_unsealed_for_node_io().await?;
        let commit = self
            .store
            .nodes
            .commit_source_binding_with_event(&record, &event)
            .await?;
        Ok(SourceBindingRegistration {
            record: commit.binding,
            newly_registered: !commit.replayed,
        })
    }

    /// Fetch one Source Binding by stable identifier.
    pub async fn get_source_binding(&self, binding_id: Uuid) -> MvResult<Option<SourceBinding>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.get_source_binding(binding_id).await
    }

    /// List the governed Source Binding registry.
    pub async fn list_source_bindings(&self) -> MvResult<Vec<SourceBinding>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.list_source_bindings().await
    }

    /// Register one untrusted, discovered Context Node descriptor and its event.
    pub async fn register_context_node(
        &self,
        record: ContextNodeRecord,
        event: EventEnvelope,
    ) -> MvResult<ContextNodeRegistration> {
        self.ensure_unsealed_for_node_io().await?;
        if record.status != ContextNodeStatus::Discovered
            || record.trust_class != ContextNodeTrustClass::untrusted()
        {
            return Err(MvError::InvalidInput(
                "public Context Node registration only accepts untrusted discoveries".into(),
            ));
        }
        let commit = self
            .store
            .nodes
            .commit_context_node_with_event(&record, &event)
            .await?;
        Ok(ContextNodeRegistration {
            record: commit.context_node,
            newly_registered: !commit.replayed,
        })
    }

    /// Fetch one Context Node descriptor by stable identifier.
    pub async fn get_context_node(&self, node_id: Uuid) -> MvResult<Option<ContextNodeRecord>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.get_context_node(node_id).await
    }

    /// List Context Node descriptors with an optional lifecycle filter.
    pub async fn list_context_nodes(
        &self,
        status: Option<ContextNodeStatus>,
    ) -> MvResult<Vec<ContextNodeRecord>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.list_context_nodes(status).await
    }

    fn identity_legacy_fallback_enabled() -> bool {
        std::env::var("MINDVAULT_IDENTITY_LEGACY_FALLBACK")
            .ok()
            .as_deref()
            == Some("1")
    }

    fn derived_owner_principal(&self, local_node_id: Uuid) -> StableUri {
        StableUri::principal(
            local_node_id,
            IdentityRecord::principal_id_for_subject(local_node_id, "local-context-owner"),
        )
    }

    /// Bootstrap the vault's canonical local principals in the identity registry.
    pub async fn bootstrap_local_identities(
        &self,
        local_node_id: Uuid,
    ) -> MvResult<BootstrapLocalIdentities> {
        let local_system = self
            .register_identity(
                local_node_id,
                "local-system",
                ActorKind::Human,
                "Local System",
                IdempotencyKey::parse(format!("bootstrap-local-system-{local_node_id}"))
                    .map_err(MvError::InvalidInput)?,
            )
            .await?
            .record;
        let local_context_owner = self
            .register_identity(
                local_node_id,
                "local-context-owner",
                ActorKind::Human,
                "Local Context Owner",
                IdempotencyKey::parse(format!("bootstrap-local-context-owner-{local_node_id}"))
                    .map_err(MvError::InvalidInput)?,
            )
            .await?
            .record;
        Ok(BootstrapLocalIdentities {
            local_system,
            local_context_owner,
        })
    }

    /// Register one governed identity record, idempotently.
    pub async fn register_identity(
        &self,
        local_node_id: Uuid,
        subject_binding: &str,
        actor_kind: ActorKind,
        display_name: &str,
        idempotency_key: IdempotencyKey,
    ) -> MvResult<IdentityRegistration> {
        self.ensure_unsealed_for_node_io().await?;
        let identity =
            IdentityRecord::bootstrap(local_node_id, subject_binding, actor_kind, display_name)
                .map_err(MvError::InvalidInput)?;

        if let Some(existing) = self
            .store
            .nodes
            .get_identity_by_subject_binding(&StableUri::node(local_node_id), subject_binding)
            .await?
        {
            return Ok(IdentityRegistration {
                record: existing,
                newly_registered: false,
            });
        }

        let data = serde_json::json!({
            "principal_id": identity.principal_id,
            "actor_kind": identity.actor_kind.as_str(),
            "status": identity.status.as_str(),
            "record_digest": identity.semantic_digest(),
            "subject_binding_digest": identity.subject_binding_digest,
        });
        let owner = self.local_owner_principal(local_node_id).await?;
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: IDENTITY_REGISTERED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: identity.principal_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("identity-registered").map_err(MvError::InvalidInput)?,
                "1.0.0",
            )
            .map_err(MvError::InvalidInput)?,
            principal: owner.clone(),
            actor: owner,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: identity.principal_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)?;
        event.payload_digest = identity.semantic_digest();

        let commit = self
            .store
            .nodes
            .commit_identity_with_event(&identity, &event)
            .await?;
        Ok(IdentityRegistration {
            record: commit.identity,
            newly_registered: !commit.replayed,
        })
    }

    /// Resolve a command principal URI through the governed identity registry.
    pub async fn resolve_command_identity(
        &self,
        local_node_id: Uuid,
        subject: Option<&str>,
    ) -> MvResult<StableUri> {
        let subject = subject.unwrap_or("local-system");
        if let Some(record) = self
            .store
            .nodes
            .get_identity_by_subject_binding(&StableUri::node(local_node_id), subject)
            .await?
        {
            return Ok(record.principal_uri);
        }
        if Self::identity_legacy_fallback_enabled() {
            return Ok(self.derived_principal_for_subject(local_node_id, subject));
        }
        Err(MvError::InvalidInput(format!(
            "unknown identity subject binding: {subject}"
        )))
    }

    /// The owner principal this vault attributes its own governance acts to.
    pub async fn local_owner_principal(&self, local_node_id: Uuid) -> MvResult<StableUri> {
        if let Some(record) = self
            .store
            .nodes
            .get_identity_by_subject_binding(&StableUri::node(local_node_id), "local-context-owner")
            .await?
        {
            return Ok(record.principal_uri);
        }
        Ok(self.derived_owner_principal(local_node_id))
    }

    /// Issue one Context or Tool Grant governed by this vault's local node.
    ///
    /// Requires an active registered local Context Node (IK-001a). Idempotent
    /// on `idempotency_key`: a replay returns the historical grant with
    /// `newly_issued: false`.
    pub async fn issue_authority_grant(
        &self,
        request: IssueAuthorityGrantRequest,
    ) -> MvResult<AuthorityGrantIssuance> {
        self.ensure_unsealed_for_node_io().await?;
        let local_node_id = self.store.nodes.local_context_node_id().await?;
        let node_uri = StableUri::node(local_node_id);
        let grantor = self.local_owner_principal(local_node_id).await?;

        let mut grant = match request.kind {
            AuthorityGrantKind::Tool => AuthorityGrant::new_tool(
                node_uri.clone(),
                grantor.clone(),
                request.grantee.clone(),
                request.targets,
                request.capabilities,
                request.purpose,
                request.expires_at,
            ),
            AuthorityGrantKind::Context => AuthorityGrant::new_context(
                node_uri.clone(),
                grantor.clone(),
                request.grantee.clone(),
                request.targets,
                request.capabilities,
                request.purpose,
                request.expires_at,
            ),
        }
        .map_err(MvError::InvalidInput)?;
        // Deterministic id from the idempotency key. A retry with the same key
        // resolves to the same grant id and returns the stored record instead of
        // minting a rival grant whose digest would conflict.
        grant.grant_id = Uuid::new_v5(&local_node_id, request.idempotency_key.as_str().as_bytes());
        grant.grant_uri = StableUri::authority_grant(local_node_id, grant.grant_id);
        grant.sensitivity_ceiling = request.sensitivity_ceiling;
        grant.retention_ceiling = request.retention_ceiling;
        grant.allow_redistribution = request.allow_redistribution;
        grant.allow_model_training = request.allow_model_training;
        grant.delegation_depth_remaining = request.delegation_depth_remaining;
        grant.validate().map_err(MvError::InvalidInput)?;

        self.commit_authority_grant_issuance(grant, request.idempotency_key)
            .await
    }

    /// Derive a grant whose authority is a strict subset of an effective parent.
    ///
    /// The authenticated caller must resolve to the parent's grantee. The
    /// storage transaction remains the final authority for complete-chain
    /// effectiveness and subset validation, closing races with lifecycle
    /// transitions between this read and commit.
    pub async fn delegate_authority_grant(
        &self,
        request: DelegateAuthorityGrantRequest,
    ) -> MvResult<AuthorityGrantIssuance> {
        self.ensure_unsealed_for_node_io().await?;
        let parent = self
            .store
            .nodes
            .get_authority_grant(request.parent_grant_id)
            .await?
            .ok_or_else(|| MvError::NotFound("authority-grant parent does not exist".into()))?;
        if parent.grantee != request.grantor {
            return Err(MvError::AccessDenied(
                "only the parent grant grantee may delegate its authority".into(),
            ));
        }

        let mut grant = match parent.kind {
            AuthorityGrantKind::Tool => AuthorityGrant::new_tool(
                parent.governing_node.clone(),
                request.grantor,
                request.grantee,
                request.targets,
                request.capabilities,
                request.purpose,
                request.expires_at,
            ),
            AuthorityGrantKind::Context => AuthorityGrant::new_context(
                parent.governing_node.clone(),
                request.grantor,
                request.grantee,
                request.targets,
                request.capabilities,
                request.purpose,
                request.expires_at,
            ),
        }
        .map_err(MvError::InvalidInput)?;
        let local_node_id = parent
            .governing_node
            .context_node_uuid()
            .ok_or_else(|| MvError::InvalidInput("invalid governing Context Node URI".into()))?;
        let identity_material = format!(
            "authority-grant-delegation:{}:{}",
            parent.grant_id,
            request.idempotency_key.as_str()
        );
        grant.grant_id = Uuid::new_v5(&local_node_id, identity_material.as_bytes());
        grant.grant_uri = StableUri::authority_grant(local_node_id, grant.grant_id);
        grant.parent_grant_id = Some(parent.grant_id);
        grant.not_before = request.not_before;
        grant.sensitivity_ceiling = request.sensitivity_ceiling;
        grant.retention_ceiling = request.retention_ceiling;
        grant.allow_redistribution = request.allow_redistribution;
        grant.allow_model_training = request.allow_model_training;
        grant.delegation_depth_remaining = request.delegation_depth_remaining;
        grant.validate().map_err(MvError::InvalidInput)?;

        self.commit_authority_grant_issuance(grant, request.idempotency_key)
            .await
    }

    async fn commit_authority_grant_issuance(
        &self,
        grant: AuthorityGrant,
        idempotency_key: IdempotencyKey,
    ) -> MvResult<AuthorityGrantIssuance> {
        if let Some(existing) = self.store.nodes.get_authority_grant(grant.grant_id).await? {
            return Ok(AuthorityGrantIssuance {
                grant: existing,
                newly_issued: false,
            });
        }
        let node_uri = grant.governing_node.clone();
        let grantor = grant.grantor.clone();

        let data = serde_json::json!({
            "grant_id": grant.grant_id,
            "grant_kind": grant.kind.as_str(),
            "grantee_uri": grant.grantee.as_str(),
            "governing_node_uri": grant.governing_node.as_str(),
            "parent_grant_id": grant.parent_grant_id,
            "record_digest": grant.semantic_digest(),
        });
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: AUTHORITY_GRANT_ISSUED_V1.into(),
            source: node_uri,
            subject: grant.grant_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("authority-grant-issued").map_err(MvError::InvalidInput)?,
                "1.0.0",
            )
            .map_err(MvError::InvalidInput)?,
            principal: grantor.clone(),
            actor: grantor,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: grant.grant_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)?;
        event.payload_digest = grant.semantic_digest();

        let commit = self
            .store
            .nodes
            .commit_authority_grant_with_event(&grant, &event)
            .await?;
        Ok(AuthorityGrantIssuance {
            grant: commit.grant,
            newly_issued: !commit.replayed,
        })
    }

    /// Read one Authority Grant by id.
    pub async fn get_authority_grant(&self, grant_id: Uuid) -> MvResult<Option<AuthorityGrant>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store.nodes.get_authority_grant(grant_id).await
    }

    /// List Authority Grants with optional filters.
    pub async fn list_authority_grants(
        &self,
        grantee: Option<&StableUri>,
        kind: Option<AuthorityGrantKind>,
        status: Option<AuthorityGrantStatus>,
    ) -> MvResult<Vec<AuthorityGrant>> {
        self.ensure_unsealed_for_node_io().await?;
        self.store
            .nodes
            .list_authority_grants(grantee, kind, status)
            .await
    }

    /// Suspend, revoke, or resume an Authority Grant.
    ///
    /// Only status, reason, revision, and update time change — grant terms are
    /// immutable after issuance (`AUTHORITY_GRANT_MODEL.md`).
    pub async fn transition_authority_grant(
        &self,
        grant_id: Uuid,
        to_status: AuthorityGrantStatus,
        reason: impl Into<String>,
        idempotency_key: IdempotencyKey,
    ) -> MvResult<AuthorityGrantTransition> {
        self.ensure_unsealed_for_node_io().await?;
        let current = self
            .store
            .nodes
            .get_authority_grant(grant_id)
            .await?
            .ok_or_else(|| MvError::InvalidInput("authority grant does not exist".into()))?;

        let reason = reason.into();
        let mut replacement = current.clone();
        replacement.revision = current
            .revision
            .checked_add(1)
            .ok_or_else(|| MvError::InvalidInput("authority-grant revision overflow".into()))?;
        replacement.status = to_status;
        replacement.status_reason = match to_status {
            AuthorityGrantStatus::Active => None,
            _ => Some(reason),
        };
        replacement.updated_at = Utc::now();
        replacement.validate().map_err(MvError::InvalidInput)?;

        let local_node_id = self.store.nodes.local_context_node_id().await?;
        let grantor = current.grantor.clone();
        let data = serde_json::json!({
            "grant_id": replacement.grant_id,
            "revision": replacement.revision,
            "from_status": current.status.as_str(),
            "to_status": replacement.status.as_str(),
            "record_digest": replacement.semantic_digest(),
        });
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: AUTHORITY_GRANT_LIFECYCLE_TRANSITIONED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: replacement.grant_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("authority-grant-lifecycle-transitioned")
                    .map_err(MvError::InvalidInput)?,
                "1.0.0",
            )
            .map_err(MvError::InvalidInput)?,
            principal: grantor.clone(),
            actor: grantor,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key,
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: replacement.grant_uri.clone(),
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .map_err(MvError::InvalidInput)?;
        event.payload_digest = replacement.semantic_digest();

        let commit = self
            .store
            .nodes
            .transition_authority_grant_with_event(current.revision, &replacement, &event)
            .await?;
        Ok(AuthorityGrantTransition {
            grant: commit.grant,
            replayed: commit.replayed,
        })
    }

    /// Redrive one terminal outbox dead letter as a new pending publication event.
    pub async fn redrive_outbox_dead_letter(
        &self,
        source_event_id: Uuid,
        principal: StableUri,
        actor: StableUri,
        idempotency_key: IdempotencyKey,
        reason: impl Into<String>,
    ) -> MvResult<DeadLetterRedriveOutcome> {
        self.ensure_unsealed_for_node_io().await?;
        let command = DeadLetterRedriveCommand {
            principal,
            actor,
            idempotency_key,
            reason: reason.into(),
            redriven_at: Utc::now(),
        };
        let commit = self
            .store
            .nodes
            .redrive_outbox_dead_letter(source_event_id, &command)
            .await?;
        Ok(DeadLetterRedriveOutcome {
            source_event_id: commit.source_event_id,
            redrive_event: commit.redrive_event,
            replayed: commit.replayed,
        })
    }

    /// Redrive one terminal consumer inbox dead letter as a new pending admission.
    pub async fn redrive_consumer_inbox_dead_letter(
        &self,
        consumer: StableUri,
        source_event_id: Uuid,
        principal: StableUri,
        actor: StableUri,
        idempotency_key: IdempotencyKey,
        reason: impl Into<String>,
    ) -> MvResult<DeadLetterRedriveOutcome> {
        self.ensure_unsealed_for_node_io().await?;
        let command = DeadLetterRedriveCommand {
            principal,
            actor,
            idempotency_key,
            reason: reason.into(),
            redriven_at: Utc::now(),
        };
        let commit = self
            .store
            .nodes
            .redrive_consumer_inbox_dead_letter(&consumer, source_event_id, &command)
            .await?;
        Ok(DeadLetterRedriveOutcome {
            source_event_id: commit.source_event_id,
            redrive_event: commit.redrive_event,
            replayed: commit.replayed,
        })
    }

    /// Derive a principal URI the same way the REST command path does.
    ///
    /// Kept public so grant issuance can name a grantee that will match
    /// `CommandIdentity::derive` for a given auth subject without the caller
    /// reconstructing the v5 scheme.
    fn derived_principal_for_subject(&self, local_node_id: Uuid, subject: &str) -> StableUri {
        StableUri::principal(
            local_node_id,
            IdentityRecord::principal_id_for_subject(local_node_id, subject),
        )
    }

    /// Resolve a grantee/principal URI through the registry when possible.
    pub async fn principal_for_subject(
        &self,
        local_node_id: Uuid,
        subject: &str,
    ) -> MvResult<StableUri> {
        self.resolve_command_identity(local_node_id, Some(subject))
            .await
    }

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

        let decision = match grant {
            Some(grant) => AdmissionDecision::Admitted {
                grant_id: grant.grant_id,
                grant_uri: grant.grant_uri.clone(),
                grant_kind: grant.kind,
                capability: request.operation,
                delegation_depth_remaining: grant.delegation_depth_remaining,
                decided_at: Utc::now(),
            },
            None => AdmissionDecision::Denied {
                reason: self.classify_admission_denial(request).await,
                decided_at: Utc::now(),
            },
        };

        // Law 15: denials must be durable. Admitted decisions are recorded too
        // as the first Trust Ledger brick; they also continue to ride in the
        // event envelope for committed mutations. Persistence failure fails
        // closed — an unrecorded decision must not be treated as settled.
        let record = CommandAdmissionDecisionRecord::from_request_and_decision(request, &decision);
        self.store
            .nodes
            .commit_command_admission_decision(&record)
            .await?;

        Ok(decision)
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

    /// Register the local Context Node via the production command.
    ///
    /// Calling the real op rather than a hand-rolled fixture means these tests
    /// fail if registration regresses, instead of quietly testing a copy.
    async fn register_local_node(engine: &MindVaultEngine) -> Uuid {
        engine
            .register_local_context_node("Personal Vault")
            .await
            .unwrap()
            .record
            .node_id
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

    /// Root grants must be issued by a local principal, not by the node URI.
    fn grantor_of(local_node_id: Uuid) -> StableUri {
        StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"admission-grantor"),
        )
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
            grantor_of(local_node_id),
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
            grantor_of(local_node_id),
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

        assert!(
            !decision.is_admitted(),
            "a context grant admitted a mutation"
        );
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
            grantor_of(local_node_id),
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

    #[tokio::test]
    async fn identity_registry_bootstraps_local_system_and_owner() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = engine.store.nodes.local_context_node_id().await.unwrap();
        let bootstrapped = engine
            .bootstrap_local_identities(local_node_id)
            .await
            .unwrap();
        assert_eq!(bootstrapped.local_system.subject_binding, "local-system");
        assert_eq!(bootstrapped.local_system.actor_kind, ActorKind::Human);
        assert_eq!(
            bootstrapped.local_context_owner.subject_binding,
            "local-context-owner"
        );
    }

    #[tokio::test]
    async fn identity_registry_resolves_local_system_without_legacy_fallback() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let principal = engine
            .resolve_command_identity(local_node_id, None)
            .await
            .unwrap();
        assert_eq!(
            principal,
            StableUri::principal(
                local_node_id,
                IdentityRecord::principal_id_for_subject(local_node_id, "local-system"),
            )
        );
    }

    #[tokio::test]
    async fn identity_registry_unknown_subject_fails_without_legacy_fallback() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let err = engine
            .resolve_command_identity(local_node_id, Some("unknown-subject"))
            .await
            .unwrap_err();
        assert!(matches!(err, MvError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn identity_registry_bootstraps_when_local_context_node_already_exists() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = engine.store.nodes.local_context_node_id().await.unwrap();
        let owner = engine.derived_owner_principal(local_node_id);
        let manifest = ContextCapabilityManifest::new(
            LOCAL_NODE_CAPABILITIES.to_vec(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let mut record = ContextNodeRecord::discovered(
            local_node_id,
            ContextNodeType::Personal,
            owner.clone(),
            StableUri::node(local_node_id),
            "Pre-registry Personal Vault",
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
        let mut event = EventEnvelope::new(NewEventEnvelope {
            event_type: CONTEXT_NODE_REGISTERED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: record.node_uri.clone(),
            schema: SchemaReference::new(
                StableUri::schema("context-node-registered").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: owner.clone(),
            actor: owner,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse("pre-registry-local-node").unwrap(),
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
        assert!(engine
            .store
            .nodes
            .list_identities(Some(&StableUri::node(local_node_id)))
            .await
            .unwrap()
            .is_empty());

        let registration = engine
            .register_local_context_node("Ignored replacement name")
            .await
            .unwrap();
        assert!(!registration.newly_registered);
        let identities = engine
            .store
            .nodes
            .list_identities(Some(&StableUri::node(local_node_id)))
            .await
            .unwrap();
        assert_eq!(identities.len(), 2);
        assert_eq!(identities[0].subject_binding, "local-context-owner");
        assert_eq!(identities[1].subject_binding, "local-system");
    }

    /// Fresh vaults get an active self-governed descriptor they can issue grants against.
    #[tokio::test]
    async fn registers_the_local_context_node_as_active() {
        let (engine, _tmp) = test_engine().await;
        assert!(engine.local_context_node().await.unwrap().is_none());

        let registration = engine
            .register_local_context_node("Personal Vault")
            .await
            .unwrap();
        assert!(registration.newly_registered);
        let record = registration.record;

        assert_eq!(record.status, ContextNodeStatus::Active);
        assert_eq!(record.display_name, "Personal Vault");
        assert_eq!(record.node_type, ContextNodeType::Personal);
        assert_eq!(record.trust_class, ContextNodeTrustClass::local());
        assert!(
            record
                .capability_manifest
                .capabilities
                .contains(&ContextCapability::Command),
            "bootstrap must advertise Command so Tool Grants can authorize creates"
        );
        assert_eq!(
            engine.local_context_node().await.unwrap().unwrap().node_id,
            record.node_id
        );
    }

    /// A second call is a no-op: same descriptor, no rival registration.
    #[tokio::test]
    async fn local_context_node_registration_is_idempotent() {
        let (engine, _tmp) = test_engine().await;
        let first = engine
            .register_local_context_node("Personal Vault")
            .await
            .unwrap();
        assert!(first.newly_registered);
        let second = engine
            .register_local_context_node("A different name that must be ignored")
            .await
            .unwrap();
        assert!(!second.newly_registered);

        assert_eq!(first.record.node_id, second.record.node_id);
        assert_eq!(first.record.revision, second.record.revision);
        assert_eq!(first.record.display_name, second.record.display_name);
        assert_eq!(
            first.record.semantic_digest(),
            second.record.semantic_digest()
        );
    }

    /// IK-001b: the production issuance command creates a grant that admits.
    #[tokio::test]
    async fn issued_tool_grant_admits_a_node_create() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let node_uri = StableUri::node(local_node_id);

        let issuance = engine
            .issue_authority_grant(IssueAuthorityGrantRequest {
                kind: AuthorityGrantKind::Tool,
                grantee: grantee.clone(),
                targets: vec![node_uri],
                capabilities: vec![ContextCapability::Command],
                purpose: "admit node creates for the owner".into(),
                expires_at: Utc::now() + Duration::days(1),
                sensitivity_ceiling: Sensitivity::Internal,
                retention_ceiling: RetentionClass::Durable,
                allow_redistribution: false,
                allow_model_training: false,
                delegation_depth_remaining: 0,
                idempotency_key: IdempotencyKey::parse("issue-tool-for-create").unwrap(),
            })
            .await
            .unwrap();
        assert!(issuance.newly_issued);
        assert_eq!(issuance.grant.status, AuthorityGrantStatus::Active);

        let decision = engine
            .resolve_command_admission(&node_create_request(
                local_node_id,
                &grantee,
                AuthorityGrantKind::Tool,
                ContextCapability::Command,
            ))
            .await
            .unwrap();
        assert!(decision.is_admitted(), "unexpected refusal: {decision:?}");
    }

    /// Issuance is idempotent on the caller-supplied key.
    #[tokio::test]
    async fn authority_grant_issuance_is_idempotent() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let request = IssueAuthorityGrantRequest {
            kind: AuthorityGrantKind::Tool,
            grantee: grantee_of(local_node_id),
            targets: vec![StableUri::node(local_node_id)],
            capabilities: vec![ContextCapability::Command],
            purpose: "idempotent issue".into(),
            expires_at: Utc::now() + Duration::days(1),
            sensitivity_ceiling: Sensitivity::Internal,
            retention_ceiling: RetentionClass::Durable,
            allow_redistribution: false,
            allow_model_training: false,
            delegation_depth_remaining: 0,
            idempotency_key: IdempotencyKey::parse("issue-once").unwrap(),
        };
        let first = engine.issue_authority_grant(request.clone()).await.unwrap();
        let second = engine.issue_authority_grant(request).await.unwrap();
        assert!(first.newly_issued);
        assert!(!second.newly_issued);
        assert_eq!(first.grant.grant_id, second.grant.grant_id);
        assert_eq!(first.grant.revision, second.grant.revision);
    }

    /// Suspend then revoke through the production lifecycle command.
    #[tokio::test]
    async fn suspending_a_grant_stops_admission_and_revoke_is_terminal() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let issuance = engine
            .issue_authority_grant(IssueAuthorityGrantRequest {
                kind: AuthorityGrantKind::Tool,
                grantee: grantee.clone(),
                targets: vec![StableUri::node(local_node_id)],
                capabilities: vec![ContextCapability::Command],
                purpose: "lifecycle".into(),
                expires_at: Utc::now() + Duration::days(1),
                sensitivity_ceiling: Sensitivity::Internal,
                retention_ceiling: RetentionClass::Durable,
                allow_redistribution: false,
                allow_model_training: false,
                delegation_depth_remaining: 0,
                idempotency_key: IdempotencyKey::parse("issue-for-lifecycle").unwrap(),
            })
            .await
            .unwrap();

        let suspended = engine
            .transition_authority_grant(
                issuance.grant.grant_id,
                AuthorityGrantStatus::Suspended,
                "operator review",
                IdempotencyKey::parse("suspend-1").unwrap(),
            )
            .await
            .unwrap();
        assert!(!suspended.replayed);
        assert_eq!(suspended.grant.status, AuthorityGrantStatus::Suspended);

        let decision = engine
            .resolve_command_admission(&node_create_request(
                local_node_id,
                &grantee,
                AuthorityGrantKind::Tool,
                ContextCapability::Command,
            ))
            .await
            .unwrap();
        assert!(!decision.is_admitted());

        let revoked = engine
            .transition_authority_grant(
                issuance.grant.grant_id,
                AuthorityGrantStatus::Revoked,
                "no longer needed",
                IdempotencyKey::parse("revoke-1").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(revoked.grant.status, AuthorityGrantStatus::Revoked);

        let err = engine
            .transition_authority_grant(
                issuance.grant.grant_id,
                AuthorityGrantStatus::Active,
                "should fail",
                IdempotencyKey::parse("resume-revoked").unwrap(),
            )
            .await
            .expect_err("revoked grants cannot resume");
        assert!(matches!(err, MvError::InvalidInput(_)), "got {err:?}");
    }

    /// IK-001c: a denial is durable and idempotent on (principal, idempotency_key).
    #[tokio::test]
    async fn denied_command_admission_is_persisted_idempotently() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );

        let first = engine.resolve_command_admission(&request).await.unwrap();
        assert!(!first.is_admitted());

        let stored = engine
            .store
            .nodes
            .get_command_admission_decision(&request.principal, &request.idempotency_key)
            .await
            .unwrap()
            .expect("denial must be durable");
        assert!(stored.is_denied());
        assert_eq!(stored.admission_digest, request.admission_digest());
        assert_eq!(stored.principal, request.principal);

        let second = engine.resolve_command_admission(&request).await.unwrap();
        assert!(!second.is_admitted());
        let again = engine
            .store
            .nodes
            .get_command_admission_decision(&request.principal, &request.idempotency_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(again.decision_id, stored.decision_id);
        assert_eq!(again.decided_at, stored.decided_at);
    }

    /// Changing the admission question under the same idempotency key conflicts.
    #[tokio::test]
    async fn conflicting_admission_decision_replay_is_rejected() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let mut request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );
        engine.resolve_command_admission(&request).await.unwrap();

        // Same principal + idempotency key, different subject → different digest.
        request.subject = StableUri::knowledge_node(local_node_id, Uuid::now_v7());
        let err = engine
            .resolve_command_admission(&request)
            .await
            .expect_err("conflicting replay must fail closed");
        assert!(
            matches!(err, MvError::IdempotencyConflict(_)),
            "got {err:?}"
        );
    }

    /// An admitted decision is also durable (Trust Ledger brick).
    #[tokio::test]
    async fn admitted_command_admission_is_persisted() {
        let (engine, _tmp) = test_engine().await;
        let local_node_id = register_local_node(&engine).await;
        let grantee = grantee_of(local_node_id);
        let node_uri = StableUri::node(local_node_id);
        engine
            .issue_authority_grant(IssueAuthorityGrantRequest {
                kind: AuthorityGrantKind::Tool,
                grantee: grantee.clone(),
                targets: vec![node_uri],
                capabilities: vec![ContextCapability::Command],
                purpose: "persist admitted decision".into(),
                expires_at: Utc::now() + Duration::days(1),
                sensitivity_ceiling: Sensitivity::Internal,
                retention_ceiling: RetentionClass::Durable,
                allow_redistribution: false,
                allow_model_training: false,
                delegation_depth_remaining: 0,
                idempotency_key: IdempotencyKey::parse("issue-for-durable-admit").unwrap(),
            })
            .await
            .unwrap();

        let request = node_create_request(
            local_node_id,
            &grantee,
            AuthorityGrantKind::Tool,
            ContextCapability::Command,
        );
        let decision = engine.resolve_command_admission(&request).await.unwrap();
        assert!(decision.is_admitted());
        let stored = engine
            .store
            .nodes
            .get_command_admission_decision(&request.principal, &request.idempotency_key)
            .await
            .unwrap()
            .expect("admission must be durable");
        assert!(!stored.is_denied());
        assert!(stored.decision.is_admitted());
    }
}
