//! Grant admission for public commands.
//!
//! Contract: `docs/architecture/AUTHORITY_GRANT_MODEL.md`.
//! Slice: `docs/architecture/interoperability-kernel-v1.md:283-284`.
//!
//! Admission is a handler-local call, not middleware. Middleware would apply to
//! every route on the router, which is precisely the blast radius this slice
//! avoids; the existing `authorize_write` and `enforce_namespace_quota` guards
//! are called the same way, from inside the handler that needs them.
//!
//! Nothing here replaces those guards. `interoperability-kernel-v1.md:252-253`
//! requires the existing authorization and quota checks to remain in place, and
//! constitutional law 8 makes grants an additional axis rather than a
//! substitute for role-based access.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Extension, Json,
};
use chrono::{Duration, Utc};
use mv_core::{
    ActionEnvelope, ActorKind, AdmissionDecision, AuthorityGrant, AuthorityGrantKind,
    AuthorityGrantStatus, CommandAdmissionRequest, ContextCapability, ContextCapabilityManifest,
    ContextNodeEndpoint, ContextNodePublicKey, ContextNodeRecord, ContextNodeStatus,
    ContextNodeType, EventEnvelope, IdempotencyKey, IdentityRecord, InteroperabilityStore,
    MaterializationMode, MvError, NewEventEnvelope, ProvenanceReference, ProvenanceRelation,
    PublicSchemaRecord, RetentionClass, SchemaReference, Sensitivity, SourceBinding,
    SourceConflictPolicy, SourceDeletionPolicy, SourceFreshness, StableUri, SyncDirection,
    CONTEXT_NODE_REGISTERED_V1, PUBLIC_SCHEMA_REGISTERED_V1, SOURCE_BINDING_REGISTERED_V1,
};
use mv_engine::engine::{DelegateAuthorityGrantRequest, IssueAuthorityGrantRequest};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{authorize_read, authorize_write, AuthContext};
use crate::state::AppState;

/// Stable code returned to a caller whose command was refused.
///
/// The bounded `AdmissionDenialReason` is deliberately not returned: it
/// distinguishes "you hold no grant" from "your grant expired", which is a
/// probing oracle. Law 15 requires a denial be recorded, not disclosed, so the
/// reason goes to tracing and the audit trail instead.
pub(crate) const COMMAND_ADMISSION_DENIED: &str = "command_admission_denied";

/// The identities a command is attributed to.
///
/// `actor` equals `principal` in this slice when no distinct acting actor is
/// recorded separately. Both resolve through the governed identity registry.
///
/// There is deliberately no header for overriding the actor. An unauthenticated
/// header naming an arbitrary actor would let a caller attribute its own
/// commands to someone else.
#[derive(Debug)]
pub(crate) struct CommandIdentity {
    pub principal: StableUri,
    pub actor: StableUri,
}

impl CommandIdentity {
    /// Resolve the principal URI through the governed identity registry.
    pub(crate) async fn derive_async(
        state: &AppState,
        auth: &AuthContext,
        local_node_id: Uuid,
    ) -> Result<Self, (StatusCode, String)> {
        // Admission `off` is a compatibility mode: it must preserve the
        // pre-kernel principal derivation and must not introduce a registry
        // precondition for otherwise ordinary node mutations. Observe and
        // enforce remain fail-closed through the governed identity registry.
        if !state.command_admission.is_active() {
            return Ok(Self::derive_v5(auth, local_node_id));
        }
        let principal = state
            .engine
            .resolve_command_identity(local_node_id, auth.subject.as_deref())
            .await
            .map_err(map_context_node_error)?;
        Ok(Self {
            actor: principal.clone(),
            principal,
        })
    }

    /// Transitional v5 derivation retained for admission-off compatibility.
    fn derive_v5(auth: &AuthContext, local_node_id: Uuid) -> Self {
        let subject = auth.subject.as_deref().unwrap_or("local-system");
        let principal_id = IdentityRecord::principal_id_for_subject(local_node_id, subject);
        let principal = StableUri::principal(local_node_id, principal_id);
        Self {
            actor: principal.clone(),
            principal,
        }
    }
}

/// Build the admission question for a node create.
///
/// `resource` is the governing node URI, not the node being created. Grant
/// targets are exact stable-URI matches with no wildcards or prefixes
/// (`AUTHORITY_GRANT_MODEL.md:44-45`), and a created node's id is minted
/// microseconds earlier, so no pre-existing grant could name it. A node-scoped
/// Tool Grant therefore authorizes creating any resource under that node;
/// per-resource scoping for creates stays with the unchanged namespace checks.
///
/// `sensitivity` and `retention` mirror the node-create envelope exactly. If
/// they drifted, a grant could admit a command whose declared ceilings it does
/// not actually cover.
pub(crate) fn node_command_admission_request(
    identity: &CommandIdentity,
    local_node_id: Uuid,
    subject: StableUri,
    idempotency_key: IdempotencyKey,
    correlation_id: Uuid,
    causation_id: Option<Uuid>,
) -> CommandAdmissionRequest {
    CommandAdmissionRequest {
        request_id: Uuid::now_v7(),
        correlation_id,
        causation_id,
        principal: identity.principal.clone(),
        actor: identity.actor.clone(),
        governing_node: StableUri::node(local_node_id),
        resource: StableUri::node(local_node_id),
        subject,
        operation: ContextCapability::Command,
        required_grant_kind: AuthorityGrantKind::Tool,
        idempotency_key,
        sensitivity: Sensitivity::Internal,
        retention: RetentionClass::Durable,
        requested_at: chrono::Utc::now(),
    }
}

/// Backward-compatible name for create admission (same shape as update/delete).
pub(crate) fn node_create_admission_request(
    identity: &CommandIdentity,
    local_node_id: Uuid,
    subject: StableUri,
    idempotency_key: IdempotencyKey,
    correlation_id: Uuid,
    causation_id: Option<Uuid>,
) -> CommandAdmissionRequest {
    node_command_admission_request(
        identity,
        local_node_id,
        subject,
        idempotency_key,
        correlation_id,
        causation_id,
    )
}

/// Resolve admission according to the configured mode.
///
/// Returns `Ok(None)` when the mode is `Off`, so the caller emits an event that
/// is byte-identical to pre-admission behaviour. Returns `Err` only under
/// `Enforce`.
pub(crate) async fn admit_command(
    state: &AppState,
    request: &CommandAdmissionRequest,
) -> Result<Option<ActionEnvelope>, (StatusCode, String)> {
    if !state.command_admission.is_active() {
        return Ok(None);
    }

    let enforcing = state.command_admission.enforces();
    let decision = match state.engine.resolve_command_admission(request).await {
        Ok(decision) => decision,
        // A resolver that cannot answer denies. It must never fall through to a
        // successful mutation (`AUTHORITY_GRANT_MODEL.md:86`).
        Err(MvError::VaultSealed) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "vault is sealed".to_string(),
            ));
        }
        Err(MvError::IdempotencyConflict(message)) => {
            return Err((StatusCode::CONFLICT, message));
        }
        Err(error) => {
            tracing::error!(%error, "command admission resolver failed");
            if enforcing {
                return Err((StatusCode::FORBIDDEN, COMMAND_ADMISSION_DENIED.to_string()));
            }
            return Ok(None);
        }
    };

    if let AdmissionDecision::Denied { reason, .. } = &decision {
        // Law 15: denied and recorded. The reason is audit-only.
        tracing::warn!(
            actor = %request.actor,
            resource = %request.resource,
            capability = request.operation.as_str(),
            reason = reason.as_str(),
            enforcing,
            "command admission denied"
        );
        if enforcing {
            return Err((StatusCode::FORBIDDEN, COMMAND_ADMISSION_DENIED.to_string()));
        }
    }

    let envelope = ActionEnvelope::from_admission(request, &decision).map_err(|message| {
        tracing::error!(%message, "action envelope construction failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "action_envelope_invalid".to_string(),
        )
    })?;
    Ok(Some(envelope))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthRole;

    fn auth_with_subject(subject: Option<&str>) -> AuthContext {
        let mut auth = AuthContext::system_admin();
        auth.subject = subject.map(str::to_string);
        auth.role = AuthRole::Admin;
        auth
    }

    /// The replay index is principal-scoped, so this derivation is a
    /// compatibility surface, not an implementation detail.
    #[test]
    fn command_identity_derives_a_stable_principal_uri() {
        let node_id = Uuid::now_v7();
        let identity = CommandIdentity::derive_v5(&auth_with_subject(Some("owner")), node_id);

        let expected = StableUri::principal(node_id, Uuid::new_v5(&node_id, b"owner"));
        assert_eq!(identity.principal, expected);
        assert_eq!(
            identity.actor, expected,
            "actor and principal are the same identity until a governed identity registry exists"
        );

        // Stable across calls.
        let again = CommandIdentity::derive_v5(&auth_with_subject(Some("owner")), node_id);
        assert_eq!(again.principal, expected);

        // A different subject is a different principal.
        let other = CommandIdentity::derive_v5(&auth_with_subject(Some("delegate")), node_id);
        assert_ne!(other.principal, expected);
    }

    /// An unauthenticated-subject caller still gets a deterministic principal.
    #[test]
    fn a_missing_subject_derives_the_local_system_principal() {
        let node_id = Uuid::now_v7();
        let identity = CommandIdentity::derive_v5(&auth_with_subject(None), node_id);
        let expected = StableUri::principal(node_id, Uuid::new_v5(&node_id, b"local-system"));
        assert_eq!(identity.principal, expected);
    }

    /// The admission question must mirror the node-create envelope exactly.
    #[test]
    fn a_node_create_request_targets_the_governing_node() {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let identity = CommandIdentity::derive_v5(&auth_with_subject(Some("owner")), node_id);
        let request = node_create_admission_request(
            &identity,
            node_id,
            StableUri::knowledge_node(node_id, resource_id),
            IdempotencyKey::parse("k1").unwrap(),
            Uuid::now_v7(),
            None,
        );

        assert_eq!(request.resource, StableUri::node(node_id));
        assert_ne!(
            request.resource, request.subject,
            "the grant target is the node, not the resource being created"
        );
        assert_eq!(request.required_grant_kind, AuthorityGrantKind::Tool);
        assert_eq!(request.operation, ContextCapability::Command);
        // Must match the envelope at the call site or a grant could admit a
        // command whose declared ceilings it does not cover.
        assert_eq!(request.sensitivity, Sensitivity::Internal);
        assert_eq!(request.retention, RetentionClass::Durable);
        request.validate().expect("request should be coherent");
    }
}

// ---------------------------------------------------------------------------
// Local Context Node registration (IK-001a)
// ---------------------------------------------------------------------------

/// Operator-facing view of the local Context Node descriptor.
///
/// Carries identity, status, and advertised capabilities — enough to confirm
/// the bootstrap prerequisite for grant issuance — without endpoints or public
/// keys, which this bootstrap slice never populates and which belong behind a
/// fuller registry transport (IK-009).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LocalContextNodeView {
    pub node_id: Uuid,
    pub revision: u64,
    pub node_uri: String,
    pub node_type: String,
    pub display_name: String,
    pub status: String,
    pub trust_class: String,
    pub capabilities: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub newly_registered: bool,
}

impl LocalContextNodeView {
    fn from_record(record: &ContextNodeRecord, newly_registered: bool) -> Self {
        Self {
            node_id: record.node_id,
            revision: record.revision,
            node_uri: record.node_uri.as_str().to_string(),
            node_type: record.node_type.as_str().to_string(),
            display_name: record.display_name.clone(),
            status: record.status.as_str().to_string(),
            trust_class: record.trust_class.as_str().to_string(),
            capabilities: record
                .capability_manifest
                .capabilities
                .iter()
                .map(|capability| capability.as_str().to_string())
                .collect(),
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
            newly_registered,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RegisterLocalContextNodeRequest {
    /// Optional display name. Defaults to "Personal Vault". Ignored when the
    /// descriptor already exists — registration is idempotent on identity, not
    /// a rename command.
    pub display_name: Option<String>,
}

fn require_admin(auth: &AuthContext) -> Result<(), (StatusCode, String)> {
    authorize_write(auth)?;
    if !auth.is_admin() {
        return Err((StatusCode::FORBIDDEN, "admin access required".into()));
    }
    Ok(())
}

fn map_context_node_error(err: MvError) -> (StatusCode, String) {
    match &err {
        MvError::VaultSealed => (StatusCode::LOCKED, err.to_string()),
        MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

/// `POST /api/v1/context-nodes/local` — register this vault's Context Node.
///
/// Admin-only. Idempotent: a second call returns the existing descriptor with
/// `newly_registered: false` and HTTP 200. A first call returns 201.
pub(crate) async fn register_local_context_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterLocalContextNodeRequest>,
) -> Result<(StatusCode, Json<LocalContextNodeView>), (StatusCode, String)> {
    require_admin(&auth)?;

    let display_name = body
        .display_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Personal Vault".into());

    let registration = state
        .engine
        .register_local_context_node(&display_name)
        .await
        .map_err(map_context_node_error)?;

    let status = if registration.newly_registered {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(LocalContextNodeView::from_record(
            &registration.record,
            registration.newly_registered,
        )),
    ))
}

/// `GET /api/v1/context-nodes/local` — read the local Context Node, if registered.
///
/// Readable by any authenticated reader: knowing whether the bootstrap
/// prerequisite exists is not a privileged secret, and grant issuance (IK-001b)
/// will still be admin-gated.
pub(crate) async fn get_local_context_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<LocalContextNodeView>, (StatusCode, String)> {
    authorize_read(&auth)?;

    let record = state
        .engine
        .local_context_node()
        .await
        .map_err(map_context_node_error)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "local Context Node is not registered".into(),
            )
        })?;

    Ok(Json(LocalContextNodeView::from_record(&record, false)))
}

// ---------------------------------------------------------------------------
// Public governed registries (IK-009)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct RegistryCommandContext {
    local_node_id: Uuid,
    identity: CommandIdentity,
    idempotency_key: IdempotencyKey,
    correlation_id: Uuid,
    causation_id: Option<Uuid>,
}

async fn registry_command_context(
    state: &AppState,
    auth: &AuthContext,
    headers: &HeaderMap,
) -> Result<RegistryCommandContext, (StatusCode, String)> {
    let local_node_id = state
        .engine
        .local_context_node_id()
        .await
        .map_err(map_context_node_error)?;
    let identity = CommandIdentity::derive_async(state, auth, local_node_id).await?;
    let idempotency_key = super::request_idempotency_key(headers)?;
    let correlation_id = super::optional_uuid_header(headers, super::CORRELATION_ID_HEADER)?
        .unwrap_or_else(Uuid::now_v7);
    let causation_id = super::optional_uuid_header(headers, super::CAUSATION_ID_HEADER)?;
    Ok(RegistryCommandContext {
        local_node_id,
        identity,
        idempotency_key,
        correlation_id,
        causation_id,
    })
}

async fn admit_registry_command(
    state: &AppState,
    command: &RegistryCommandContext,
    subject: StableUri,
) -> Result<(), (StatusCode, String)> {
    admit_command(
        state,
        &node_command_admission_request(
            &command.identity,
            command.local_node_id,
            subject,
            command.idempotency_key.clone(),
            command.correlation_id,
            command.causation_id,
        ),
    )
    .await?;
    Ok(())
}

fn registry_event(
    command: RegistryCommandContext,
    event_type: &str,
    event_schema_name: &str,
    subject: StableUri,
    data: serde_json::Value,
    payload_digest: String,
) -> Result<EventEnvelope, (StatusCode, String)> {
    EventEnvelope::new(NewEventEnvelope {
        event_type: event_type.into(),
        source: StableUri::node(command.local_node_id),
        subject: subject.clone(),
        schema: SchemaReference::new(
            StableUri::schema(event_schema_name)
                .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?,
            "1.0.0",
        )
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?,
        principal: command.identity.principal,
        actor: command.identity.actor,
        correlation_id: command.correlation_id,
        causation_id: command.causation_id,
        idempotency_key: command.idempotency_key,
        payload_digest,
        sensitivity: Sensitivity::Internal,
        retention: RetentionClass::Durable,
        provenance: vec![ProvenanceReference {
            resource: subject,
            relation: ProvenanceRelation::PrimarySource,
        }],
        data,
    })
    .map_err(|message| (StatusCode::BAD_REQUEST, message))
}

fn map_registry_error(err: MvError) -> (StatusCode, String) {
    match &err {
        MvError::VaultSealed => (StatusCode::LOCKED, err.to_string()),
        MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        MvError::IdempotencyConflict(message)
        | MvError::Conflict(message)
        | MvError::CanonicalSourceConflict(message) => (StatusCode::CONFLICT, message.clone()),
        MvError::NotFound(message) => (StatusCode::NOT_FOUND, message.clone()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RegistryRegistrationView<T> {
    pub record: T,
    pub newly_registered: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterPublicSchemaRequest {
    pub name: String,
    pub version: String,
    pub definition: serde_json::Value,
}

/// `POST /api/v1/schemas` — register one immutable public schema version.
pub(crate) async fn register_public_schema(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RegisterPublicSchemaRequest>,
) -> Result<
    (
        StatusCode,
        Json<RegistryRegistrationView<PublicSchemaRecord>>,
    ),
    (StatusCode, String),
> {
    require_admin(&auth)?;
    let schema = SchemaReference::new(
        StableUri::schema(&body.name).map_err(|err| (StatusCode::BAD_REQUEST, err))?,
        body.version,
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let subject = StableUri::schema_version(&schema.uri, &schema.version)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let command = registry_command_context(&state, &auth, &headers).await?;
    admit_registry_command(&state, &command, subject.clone()).await?;
    let record =
        PublicSchemaRecord::new(schema, body.definition, command.identity.principal.clone())
            .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let data = serde_json::json!({
        "schema_uri": record.schema.uri.as_str(),
        "schema_version": record.schema.version,
        "content_digest": record.content_digest,
    });
    let event = registry_event(
        command,
        PUBLIC_SCHEMA_REGISTERED_V1,
        "public-schema-registered",
        subject,
        data.clone(),
        mv_core::canonical_json_sha256(&data),
    )?;
    let registration = state
        .engine
        .register_public_schema(record, event)
        .await
        .map_err(map_registry_error)?;
    Ok((
        if registration.newly_registered {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(RegistryRegistrationView {
            record: registration.record,
            newly_registered: registration.newly_registered,
        }),
    ))
}

/// `GET /api/v1/schemas/{name}/versions/{version}`.
pub(crate) async fn get_public_schema(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<PublicSchemaRecord>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let reference = SchemaReference::new(
        StableUri::schema(&name).map_err(|err| (StatusCode::BAD_REQUEST, err))?,
        version,
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let record = state
        .engine
        .get_public_schema(&reference)
        .await
        .map_err(map_registry_error)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "public schema version not found".into(),
            )
        })?;
    Ok(Json(record))
}

/// `GET /api/v1/schemas/{name}/versions`.
pub(crate) async fn list_public_schema_versions(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<PublicSchemaRecord>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let schema_uri = StableUri::schema(&name).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    Ok(Json(
        state
            .engine
            .list_public_schema_versions(&schema_uri)
            .await
            .map_err(map_registry_error)?,
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterSourceBindingRequest {
    pub resource_uri: StableUri,
    pub external_system: String,
    pub external_account_id: String,
    pub external_object_id: String,
    pub authoritative_source: StableUri,
    pub provenance_ref: StableUri,
    pub authoritative_fields: Option<Vec<String>>,
    pub sync_direction: Option<SyncDirection>,
    pub last_seen_version: Option<String>,
    pub last_seen_at: Option<chrono::DateTime<Utc>>,
    pub last_sync_cursor: Option<String>,
    pub content_hash: Option<String>,
    pub materialization_mode: Option<MaterializationMode>,
    pub freshness: Option<SourceFreshness>,
    pub conflict_policy: Option<SourceConflictPolicy>,
    pub deletion_policy: Option<SourceDeletionPolicy>,
    pub retention_class: Option<RetentionClass>,
    pub sensitivity: Option<Sensitivity>,
}

/// `POST /api/v1/source-bindings` — register one governed source mapping.
pub(crate) async fn register_source_binding(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RegisterSourceBindingRequest>,
) -> Result<(StatusCode, Json<RegistryRegistrationView<SourceBinding>>), (StatusCode, String)> {
    require_admin(&auth)?;
    let command = registry_command_context(&state, &auth, &headers).await?;
    let local_node_id = command.local_node_id;
    let binding_id = Uuid::new_v5(
        &local_node_id,
        format!(
            "mindvault:rest-source-binding:v1:{}:{}",
            command.identity.principal.as_str(),
            command.idempotency_key.as_str()
        )
        .as_bytes(),
    );
    let mut record = SourceBinding::new(
        body.resource_uri,
        StableUri::node(local_node_id),
        body.external_system,
        body.external_account_id,
        body.external_object_id,
        body.authoritative_source,
        body.provenance_ref,
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    record.binding_id = binding_id;
    if let Some(value) = body.authoritative_fields {
        record.authoritative_fields = value;
    }
    if let Some(value) = body.sync_direction {
        record.sync_direction = value;
    }
    record.last_seen_version = body.last_seen_version;
    record.last_seen_at = body.last_seen_at;
    record.last_sync_cursor = body.last_sync_cursor;
    record.content_hash = body.content_hash;
    if let Some(value) = body.materialization_mode {
        record.materialization_mode = value;
    }
    if let Some(value) = body.freshness {
        record.freshness = value;
    }
    if let Some(value) = body.conflict_policy {
        record.conflict_policy = value;
    }
    if let Some(value) = body.deletion_policy {
        record.deletion_policy = value;
    }
    if let Some(value) = body.retention_class {
        record.retention_class = value;
    }
    if let Some(value) = body.sensitivity {
        record.sensitivity = value;
    }
    record
        .validate()
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let subject = StableUri::source_binding(local_node_id, record.binding_id);
    admit_registry_command(&state, &command, subject.clone()).await?;
    // Digest every semantic command field without persisting sensitive provider
    // identifiers in the public event body. This makes same-key reuse with any
    // changed registration term fail closed while keeping the outbox minimal.
    let command_payload = serde_json::json!({
        "binding_id": record.binding_id,
        "revision": record.revision,
        "resource_uri": record.resource_uri,
        "context_node": record.context_node,
        "external_system": record.external_system,
        "external_account_id": record.external_account_id,
        "external_object_id": record.external_object_id,
        "authoritative_source": record.authoritative_source,
        "authoritative_fields": record.authoritative_fields,
        "sync_direction": record.sync_direction,
        "last_seen_version": record.last_seen_version,
        "last_seen_at": record.last_seen_at,
        "last_sync_cursor": record.last_sync_cursor,
        "content_hash": record.content_hash,
        "materialization_mode": record.materialization_mode,
        "freshness": record.freshness,
        "conflict_policy": record.conflict_policy,
        "deletion_policy": record.deletion_policy,
        "retention_class": record.retention_class,
        "sensitivity": record.sensitivity,
        "provenance_ref": record.provenance_ref,
        "status": record.status,
        "supersedes_binding_id": record.supersedes_binding_id,
    });
    let payload_digest = mv_core::canonical_json_sha256(&command_payload);
    let data = serde_json::json!({
        "binding_id": record.binding_id,
        "resource_uri": record.resource_uri.as_str(),
        "external_system": record.external_system,
        "materialization_mode": record.materialization_mode.as_str(),
    });
    let event = registry_event(
        command,
        SOURCE_BINDING_REGISTERED_V1,
        "source-binding-registered",
        subject,
        data,
        payload_digest,
    )?;
    let registration = state
        .engine
        .register_source_binding(record, event)
        .await
        .map_err(map_registry_error)?;
    Ok((
        if registration.newly_registered {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(RegistryRegistrationView {
            record: registration.record,
            newly_registered: registration.newly_registered,
        }),
    ))
}

pub(crate) async fn get_source_binding(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SourceBinding>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let record = state
        .engine
        .get_source_binding(id)
        .await
        .map_err(map_registry_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Source Binding not found".into()))?;
    Ok(Json(record))
}

pub(crate) async fn list_source_bindings(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SourceBinding>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    Ok(Json(
        state
            .engine
            .list_source_bindings()
            .await
            .map_err(map_registry_error)?,
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterContextNodeRequest {
    pub node_id: Uuid,
    pub node_type: ContextNodeType,
    pub owner_actor_id: StableUri,
    pub governing_node_id: StableUri,
    pub display_name: String,
    #[serde(default)]
    pub capabilities: Vec<ContextCapability>,
    #[serde(default)]
    pub supported_protocols: Vec<mv_core::ContextProtocolProfile>,
    #[serde(default)]
    pub supported_schema_versions: Vec<SchemaReference>,
    #[serde(default)]
    pub public_keys: Vec<ContextNodePublicKey>,
    #[serde(default)]
    pub endpoints: Vec<ContextNodeEndpoint>,
    #[serde(default)]
    pub data_residency: Vec<String>,
}

/// `POST /api/v1/context-nodes` — register an untrusted discovery descriptor.
pub(crate) async fn register_context_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RegisterContextNodeRequest>,
) -> Result<
    (
        StatusCode,
        Json<RegistryRegistrationView<ContextNodeRecord>>,
    ),
    (StatusCode, String),
> {
    require_admin(&auth)?;
    let manifest = ContextCapabilityManifest::new(
        body.capabilities,
        body.supported_protocols,
        body.supported_schema_versions,
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let mut record = ContextNodeRecord::discovered(
        body.node_id,
        body.node_type,
        body.owner_actor_id,
        body.governing_node_id,
        body.display_name,
        manifest,
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    record.public_keys = body.public_keys;
    record.endpoints = body.endpoints;
    record.data_residency = body.data_residency;
    record
        .validate()
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let subject = record.node_uri.clone();
    let command = registry_command_context(&state, &auth, &headers).await?;
    admit_registry_command(&state, &command, subject.clone()).await?;
    let data = serde_json::json!({
        "node_id": record.node_id,
        "revision": record.revision,
        "node_type": record.node_type.as_str(),
        "status": record.status.as_str(),
        "record_digest": record.semantic_digest(),
        "capability_digest": record.capability_manifest.content_digest,
    });
    let event = registry_event(
        command,
        CONTEXT_NODE_REGISTERED_V1,
        "context-node-registered",
        subject,
        data,
        record.semantic_digest(),
    )?;
    let registration = state
        .engine
        .register_context_node(record, event)
        .await
        .map_err(map_registry_error)?;
    Ok((
        if registration.newly_registered {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(RegistryRegistrationView {
            record: registration.record,
            newly_registered: registration.newly_registered,
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListContextNodesQuery {
    pub status: Option<ContextNodeStatus>,
}

pub(crate) async fn get_context_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ContextNodeRecord>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let record = state
        .engine
        .get_context_node(id)
        .await
        .map_err(map_registry_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Context Node not found".into()))?;
    Ok(Json(record))
}

pub(crate) async fn list_context_nodes(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListContextNodesQuery>,
) -> Result<Json<Vec<ContextNodeRecord>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    Ok(Json(
        state
            .engine
            .list_context_nodes(query.status)
            .await
            .map_err(map_registry_error)?,
    ))
}

// ---------------------------------------------------------------------------
// Authority Grant issuance and lifecycle (IK-001b)
// ---------------------------------------------------------------------------

/// Operator-facing view of an Authority Grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AuthorityGrantView {
    pub grant_id: Uuid,
    pub revision: u64,
    pub grant_uri: String,
    pub kind: String,
    pub grantor: String,
    pub grantee: String,
    pub governing_node: String,
    pub targets: Vec<String>,
    pub capabilities: Vec<String>,
    pub sensitivity_ceiling: String,
    pub retention_ceiling: String,
    pub allow_redistribution: bool,
    pub allow_model_training: bool,
    pub purpose: String,
    pub parent_grant_id: Option<Uuid>,
    pub delegation_depth_remaining: u8,
    pub status: String,
    pub status_reason: Option<String>,
    pub not_before: String,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub newly_issued: bool,
}

impl AuthorityGrantView {
    fn from_grant(grant: &AuthorityGrant, newly_issued: bool) -> Self {
        Self {
            grant_id: grant.grant_id,
            revision: grant.revision,
            grant_uri: grant.grant_uri.as_str().to_string(),
            kind: grant.kind.as_str().to_string(),
            grantor: grant.grantor.as_str().to_string(),
            grantee: grant.grantee.as_str().to_string(),
            governing_node: grant.governing_node.as_str().to_string(),
            targets: grant
                .targets
                .iter()
                .map(|t| t.as_str().to_string())
                .collect(),
            capabilities: grant
                .capabilities
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            sensitivity_ceiling: grant.sensitivity_ceiling.as_str().to_string(),
            retention_ceiling: grant.retention_ceiling.as_str().to_string(),
            allow_redistribution: grant.allow_redistribution,
            allow_model_training: grant.allow_model_training,
            purpose: grant.purpose.clone(),
            parent_grant_id: grant.parent_grant_id,
            delegation_depth_remaining: grant.delegation_depth_remaining,
            status: grant.status.as_str().to_string(),
            status_reason: grant.status_reason.clone(),
            not_before: grant.not_before.to_rfc3339(),
            expires_at: grant.expires_at.to_rfc3339(),
            created_at: grant.created_at.to_rfc3339(),
            updated_at: grant.updated_at.to_rfc3339(),
            newly_issued,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct IssueAuthorityGrantBody {
    /// `tool` or `context`. Defaults to `tool` — the kind enforce needs for creates.
    #[serde(default = "default_tool_kind")]
    pub kind: String,
    /// Full grantee principal URI. Mutually exclusive with `grantee_subject`.
    pub grantee: Option<String>,
    /// Auth subject string derived with the same v5 scheme as command admission.
    /// Use `"local-system"` to grant the default unauthenticated admin principal.
    pub grantee_subject: Option<String>,
    /// Exact target URIs. Defaults to the local Context Node URI (required for creates).
    pub targets: Option<Vec<String>>,
    /// Capability tokens. Defaults to `["command"]` for tool grants.
    pub capabilities: Option<Vec<String>>,
    pub purpose: String,
    /// RFC3339 expiry. Defaults to now + 30 days when omitted.
    pub expires_at: Option<String>,
    #[serde(default = "default_internal")]
    pub sensitivity_ceiling: String,
    /// Defaults to `durable` so a freshly issued Tool Grant can admit node creates.
    #[serde(default = "default_durable")]
    pub retention_ceiling: String,
    #[serde(default)]
    pub allow_redistribution: bool,
    #[serde(default)]
    pub allow_model_training: bool,
    #[serde(default)]
    pub delegation_depth_remaining: u8,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DelegateAuthorityGrantBody {
    /// Full grantee principal URI. Mutually exclusive with `grantee_subject`.
    pub grantee: Option<String>,
    pub grantee_subject: Option<String>,
    /// Omitted terms safely inherit the parent's exact boundary.
    pub targets: Option<Vec<String>>,
    pub capabilities: Option<Vec<String>>,
    pub purpose: String,
    pub not_before: Option<String>,
    pub expires_at: Option<String>,
    pub sensitivity_ceiling: Option<String>,
    pub retention_ceiling: Option<String>,
    pub allow_redistribution: Option<bool>,
    pub allow_model_training: Option<bool>,
    pub delegation_depth_remaining: Option<u8>,
    pub idempotency_key: String,
}

fn default_tool_kind() -> String {
    "tool".into()
}
fn default_internal() -> String {
    "internal".into()
}
fn default_durable() -> String {
    "durable".into()
}

#[derive(Debug, Deserialize)]
pub(crate) struct TransitionAuthorityGrantBody {
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListAuthorityGrantsQuery {
    pub grantee: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
}

fn map_grant_error(err: MvError) -> (StatusCode, String) {
    match &err {
        MvError::VaultSealed => (StatusCode::LOCKED, err.to_string()),
        MvError::NotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        MvError::AccessDenied(_) => (StatusCode::FORBIDDEN, err.to_string()),
        MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn parse_kind(value: &str) -> Result<AuthorityGrantKind, (StatusCode, String)> {
    AuthorityGrantKind::from_str(value).map_err(|err| (StatusCode::BAD_REQUEST, err))
}

fn parse_capability(value: &str) -> Result<ContextCapability, (StatusCode, String)> {
    ContextCapability::from_str(value).map_err(|err| (StatusCode::BAD_REQUEST, err))
}

async fn resolve_grantee(
    state: &AppState,
    local_node_id: Uuid,
    grantee: Option<&str>,
    grantee_subject: Option<&str>,
) -> Result<StableUri, (StatusCode, String)> {
    match (grantee, grantee_subject) {
        (Some(uri), None) => StableUri::parse(uri).map_err(|err| (StatusCode::BAD_REQUEST, err)),
        (None, Some(subject)) => {
            if subject.trim().is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "grantee_subject must not be empty".into(),
                ));
            }
            state
                .engine
                .principal_for_subject(local_node_id, subject)
                .await
                .map_err(map_grant_error)
        }
        (None, None) => Err((
            StatusCode::BAD_REQUEST,
            "provide grantee or grantee_subject".into(),
        )),
        (Some(_), Some(_)) => Err((
            StatusCode::BAD_REQUEST,
            "provide only one of grantee or grantee_subject".into(),
        )),
    }
}

/// `POST /api/v1/authority-grants` — issue a Context or Tool Grant.
///
/// Admin-only. The grantor is always the vault's local owner principal; the
/// caller chooses the grantee. Idempotent on `idempotency_key`.
pub(crate) async fn issue_authority_grant(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<IssueAuthorityGrantBody>,
) -> Result<(StatusCode, Json<AuthorityGrantView>), (StatusCode, String)> {
    require_admin(&auth)?;

    let local_node_id = state
        .engine
        .store
        .nodes
        .local_context_node_id()
        .await
        .map_err(map_grant_error)?;

    let grantee = resolve_grantee(
        &state,
        local_node_id,
        body.grantee.as_deref(),
        body.grantee_subject.as_deref(),
    )
    .await?;

    let kind = parse_kind(&body.kind)?;
    let targets = match body.targets {
        Some(values) if !values.is_empty() => values
            .into_iter()
            .map(|value| StableUri::parse(value).map_err(|err| (StatusCode::BAD_REQUEST, err)))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err((StatusCode::BAD_REQUEST, "targets must not be empty".into()));
        }
        None => vec![StableUri::node(local_node_id)],
    };
    let capabilities = match body.capabilities {
        Some(values) if !values.is_empty() => values
            .iter()
            .map(|value| parse_capability(value))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "capabilities must not be empty".into(),
            ));
        }
        None => match kind {
            AuthorityGrantKind::Tool => vec![ContextCapability::Command],
            AuthorityGrantKind::Context => vec![ContextCapability::Discover],
        },
    };
    let expires_at = match body.expires_at {
        Some(value) => chrono::DateTime::parse_from_rfc3339(&value)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|err| (StatusCode::BAD_REQUEST, format!("expires_at: {err}")))?,
        None => Utc::now() + Duration::days(30),
    };
    let sensitivity_ceiling = Sensitivity::from_str(&body.sensitivity_ceiling)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let retention_ceiling = RetentionClass::from_str(&body.retention_ceiling)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let idempotency_key = IdempotencyKey::parse(&body.idempotency_key)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    if body.purpose.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "purpose must not be empty".into()));
    }

    let issuance = state
        .engine
        .issue_authority_grant(IssueAuthorityGrantRequest {
            kind,
            grantee,
            targets,
            capabilities,
            purpose: body.purpose,
            expires_at,
            sensitivity_ceiling,
            retention_ceiling,
            allow_redistribution: body.allow_redistribution,
            allow_model_training: body.allow_model_training,
            delegation_depth_remaining: body.delegation_depth_remaining,
            idempotency_key,
        })
        .await
        .map_err(map_grant_error)?;

    let status = if issuance.newly_issued {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(AuthorityGrantView::from_grant(
            &issuance.grant,
            issuance.newly_issued,
        )),
    ))
}

/// `POST /api/v1/authority-grants/:id/delegate` — derive narrower authority.
///
/// The authenticated writer must resolve to the parent grantee. Omitted terms
/// inherit the parent boundary, while remaining delegation depth defaults to
/// one less. Domain and storage validation reject every widening attempt.
pub(crate) async fn delegate_authority_grant(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(parent_grant_id): Path<Uuid>,
    Json(body): Json<DelegateAuthorityGrantBody>,
) -> Result<(StatusCode, Json<AuthorityGrantView>), (StatusCode, String)> {
    authorize_write(&auth)?;
    if body.purpose.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "purpose must not be empty".into()));
    }

    let parent = state
        .engine
        .get_authority_grant(parent_grant_id)
        .await
        .map_err(map_grant_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "authority grant not found".into()))?;
    let local_node_id = parent.governing_node.context_node_uuid().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "invalid governing Context Node URI".into(),
        )
    })?;
    let grantor = state
        .engine
        .principal_for_subject(
            local_node_id,
            auth.subject.as_deref().unwrap_or("local-system"),
        )
        .await
        .map_err(map_grant_error)?;
    let grantee = resolve_grantee(
        &state,
        local_node_id,
        body.grantee.as_deref(),
        body.grantee_subject.as_deref(),
    )
    .await?;

    let targets = match body.targets {
        Some(values) if !values.is_empty() => values
            .into_iter()
            .map(|value| StableUri::parse(value).map_err(|err| (StatusCode::BAD_REQUEST, err)))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err((StatusCode::BAD_REQUEST, "targets must not be empty".into()));
        }
        None => parent.targets.clone(),
    };
    let capabilities = match body.capabilities {
        Some(values) if !values.is_empty() => values
            .iter()
            .map(|value| parse_capability(value))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "capabilities must not be empty".into(),
            ));
        }
        None => parent.capabilities.clone(),
    };
    let not_before = match body.not_before {
        Some(value) => chrono::DateTime::parse_from_rfc3339(&value)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|err| (StatusCode::BAD_REQUEST, format!("not_before: {err}")))?,
        None => Utc::now(),
    };
    let expires_at = match body.expires_at {
        Some(value) => chrono::DateTime::parse_from_rfc3339(&value)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|err| (StatusCode::BAD_REQUEST, format!("expires_at: {err}")))?,
        None => parent.expires_at,
    };
    let sensitivity_ceiling = match body.sensitivity_ceiling {
        Some(value) => {
            Sensitivity::from_str(&value).map_err(|err| (StatusCode::BAD_REQUEST, err))?
        }
        None => parent.sensitivity_ceiling,
    };
    let retention_ceiling = match body.retention_ceiling {
        Some(value) => {
            RetentionClass::from_str(&value).map_err(|err| (StatusCode::BAD_REQUEST, err))?
        }
        None => parent.retention_ceiling,
    };
    let idempotency_key = IdempotencyKey::parse(&body.idempotency_key)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let issuance = state
        .engine
        .delegate_authority_grant(DelegateAuthorityGrantRequest {
            parent_grant_id,
            grantor,
            grantee,
            targets,
            capabilities,
            purpose: body.purpose,
            not_before,
            expires_at,
            sensitivity_ceiling,
            retention_ceiling,
            allow_redistribution: body
                .allow_redistribution
                .unwrap_or(parent.allow_redistribution),
            allow_model_training: body
                .allow_model_training
                .unwrap_or(parent.allow_model_training),
            delegation_depth_remaining: body
                .delegation_depth_remaining
                .unwrap_or_else(|| parent.delegation_depth_remaining.saturating_sub(1)),
            idempotency_key,
        })
        .await
        .map_err(map_grant_error)?;
    let status = if issuance.newly_issued {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(AuthorityGrantView::from_grant(
            &issuance.grant,
            issuance.newly_issued,
        )),
    ))
}

/// `GET /api/v1/authority-grants` — list grants (admin).
pub(crate) async fn list_authority_grants(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListAuthorityGrantsQuery>,
) -> Result<Json<Vec<AuthorityGrantView>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let grantee = match query.grantee {
        Some(value) => Some(StableUri::parse(value).map_err(|err| (StatusCode::BAD_REQUEST, err))?),
        None => None,
    };
    let kind = match query.kind {
        Some(value) => Some(parse_kind(&value)?),
        None => None,
    };
    let status = match query.status {
        Some(value) => Some(
            AuthorityGrantStatus::from_str(&value).map_err(|err| (StatusCode::BAD_REQUEST, err))?,
        ),
        None => None,
    };
    let grants = state
        .engine
        .list_authority_grants(grantee.as_ref(), kind, status)
        .await
        .map_err(map_grant_error)?;
    Ok(Json(
        grants
            .iter()
            .map(|grant| AuthorityGrantView::from_grant(grant, false))
            .collect(),
    ))
}

/// `GET /api/v1/authority-grants/:id` — read one grant (admin).
pub(crate) async fn get_authority_grant(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(grant_id): Path<Uuid>,
) -> Result<Json<AuthorityGrantView>, (StatusCode, String)> {
    require_admin(&auth)?;
    let grant = state
        .engine
        .get_authority_grant(grant_id)
        .await
        .map_err(map_grant_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "authority grant not found".into()))?;
    Ok(Json(AuthorityGrantView::from_grant(&grant, false)))
}

async fn transition_grant(
    auth: &AuthContext,
    state: &AppState,
    grant_id: Uuid,
    to_status: AuthorityGrantStatus,
    body: TransitionAuthorityGrantBody,
) -> Result<Json<AuthorityGrantView>, (StatusCode, String)> {
    require_admin(auth)?;
    if body.reason.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "reason must not be empty".into()));
    }
    let idempotency_key = IdempotencyKey::parse(&body.idempotency_key)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let transition = state
        .engine
        .transition_authority_grant(grant_id, to_status, body.reason, idempotency_key)
        .await
        .map_err(map_grant_error)?;
    Ok(Json(AuthorityGrantView::from_grant(
        &transition.grant,
        false,
    )))
}

/// `POST /api/v1/authority-grants/:id/suspend`
pub(crate) async fn suspend_authority_grant(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(grant_id): Path<Uuid>,
    Json(body): Json<TransitionAuthorityGrantBody>,
) -> Result<Json<AuthorityGrantView>, (StatusCode, String)> {
    transition_grant(
        &auth,
        &state,
        grant_id,
        AuthorityGrantStatus::Suspended,
        body,
    )
    .await
}

/// `POST /api/v1/authority-grants/:id/revoke`
pub(crate) async fn revoke_authority_grant(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(grant_id): Path<Uuid>,
    Json(body): Json<TransitionAuthorityGrantBody>,
) -> Result<Json<AuthorityGrantView>, (StatusCode, String)> {
    transition_grant(&auth, &state, grant_id, AuthorityGrantStatus::Revoked, body).await
}

/// `POST /api/v1/authority-grants/:id/resume` — suspended → active.
pub(crate) async fn resume_authority_grant(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(grant_id): Path<Uuid>,
    Json(body): Json<TransitionAuthorityGrantBody>,
) -> Result<Json<AuthorityGrantView>, (StatusCode, String)> {
    // Resume clears the reason in the engine; the request still requires one
    // for audit attribution of the transition intent.
    transition_grant(&auth, &state, grant_id, AuthorityGrantStatus::Active, body).await
}

// ---------------------------------------------------------------------------
// Governed identity registry (IK-003)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IdentityView {
    pub principal_id: Uuid,
    pub revision: u64,
    pub principal_uri: String,
    pub governing_node_uri: String,
    pub actor_kind: String,
    pub display_name: String,
    pub subject_binding: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub newly_registered: bool,
}

impl IdentityView {
    fn from_record(record: &IdentityRecord, newly_registered: bool) -> Self {
        Self {
            principal_id: record.principal_id,
            revision: record.revision,
            principal_uri: record.principal_uri.as_str().to_string(),
            governing_node_uri: record.governing_node_uri.as_str().to_string(),
            actor_kind: record.actor_kind.as_str().to_string(),
            display_name: record.display_name.clone(),
            subject_binding: record.subject_binding.clone(),
            status: record.status.as_str().to_string(),
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
            newly_registered,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterIdentityRequest {
    pub subject_binding: String,
    pub actor_kind: String,
    pub display_name: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListIdentitiesQuery {
    pub governing_node_uri: Option<String>,
}

fn parse_actor_kind(value: &str) -> Result<ActorKind, (StatusCode, String)> {
    value.parse().map_err(|err: String| (StatusCode::BAD_REQUEST, err))
}

/// `POST /api/v1/identities` — register one governed identity.
pub(crate) async fn register_identity(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterIdentityRequest>,
) -> Result<(StatusCode, Json<IdentityView>), (StatusCode, String)> {
    require_admin(&auth)?;
    let local_node_id = state
        .engine
        .store
        .nodes
        .local_context_node_id()
        .await
        .map_err(map_context_node_error)?;
    let actor_kind = parse_actor_kind(&body.actor_kind)?;
    let idempotency_key = IdempotencyKey::parse(body.idempotency_key)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let registration = state
        .engine
        .register_identity(
            local_node_id,
            &body.subject_binding,
            actor_kind,
            &body.display_name,
            idempotency_key,
        )
        .await
        .map_err(map_context_node_error)?;
    let status = if registration.newly_registered {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(IdentityView::from_record(
            &registration.record,
            registration.newly_registered,
        )),
    ))
}

/// `GET /api/v1/identities` — list governed identities.
pub(crate) async fn list_identities(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListIdentitiesQuery>,
) -> Result<Json<Vec<IdentityView>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let governing_node_uri = match query.governing_node_uri {
        Some(value) => Some(
            StableUri::parse(value).map_err(|err| (StatusCode::BAD_REQUEST, err))?,
        ),
        None => None,
    };
    let records = state
        .engine
        .store
        .nodes
        .list_identities(governing_node_uri.as_ref())
        .await
        .map_err(map_context_node_error)?;
    Ok(Json(
        records
            .iter()
            .map(|record| IdentityView::from_record(record, false))
            .collect(),
    ))
}

fn map_redrive_error(err: MvError) -> (StatusCode, String) {
    match &err {
        MvError::VaultSealed => (StatusCode::LOCKED, err.to_string()),
        MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        MvError::IdempotencyConflict(message) => (StatusCode::CONFLICT, message.clone()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeadLetterRedriveBody {
    pub idempotency_key: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeadLetterRedriveView {
    pub source_event_id: Uuid,
    pub redrive_event_id: Uuid,
    pub causation_id: Uuid,
    pub replayed: bool,
}

impl DeadLetterRedriveView {
    fn from_outcome(outcome: &mv_engine::engine::DeadLetterRedriveOutcome) -> Self {
        Self {
            source_event_id: outcome.source_event_id,
            redrive_event_id: outcome.redrive_event.id,
            causation_id: outcome.source_event_id,
            replayed: outcome.replayed,
        }
    }
}

/// `POST /api/v1/outbox/events/:id/redrive` — governed outbox dead-letter redrive.
pub(crate) async fn redrive_outbox_dead_letter(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(source_event_id): Path<Uuid>,
    Json(body): Json<DeadLetterRedriveBody>,
) -> Result<(StatusCode, Json<DeadLetterRedriveView>), (StatusCode, String)> {
    require_admin(&auth)?;
    if body.reason.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "reason must not be empty".into()));
    }
    let idempotency_key = IdempotencyKey::parse(&body.idempotency_key)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let local_node_id = state
        .engine
        .store
        .nodes
        .local_context_node_id()
        .await
        .map_err(map_redrive_error)?;
    let identity = CommandIdentity::derive_async(&state, &auth, local_node_id).await?;
    let outcome = state
        .engine
        .redrive_outbox_dead_letter(
            source_event_id,
            identity.principal,
            identity.actor,
            idempotency_key,
            body.reason,
        )
        .await
        .map_err(map_redrive_error)?;
    let status = if outcome.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(DeadLetterRedriveView::from_outcome(&outcome))))
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConsumerDeadLetterRedriveBody {
    pub consumer: String,
    pub idempotency_key: String,
    pub reason: String,
}

/// `POST /api/v1/consumer-inbox/events/:id/redrive` — governed inbox dead-letter redrive.
pub(crate) async fn redrive_consumer_inbox_dead_letter(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(source_event_id): Path<Uuid>,
    Json(body): Json<ConsumerDeadLetterRedriveBody>,
) -> Result<(StatusCode, Json<DeadLetterRedriveView>), (StatusCode, String)> {
    require_admin(&auth)?;
    if body.reason.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "reason must not be empty".into()));
    }
    let consumer = StableUri::parse(&body.consumer)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let idempotency_key = IdempotencyKey::parse(&body.idempotency_key)
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let local_node_id = state
        .engine
        .store
        .nodes
        .local_context_node_id()
        .await
        .map_err(map_redrive_error)?;
    let identity = CommandIdentity::derive_async(&state, &auth, local_node_id).await?;
    let outcome = state
        .engine
        .redrive_consumer_inbox_dead_letter(
            consumer,
            source_event_id,
            identity.principal,
            identity.actor,
            idempotency_key,
            body.reason,
        )
        .await
        .map_err(map_redrive_error)?;
    let status = if outcome.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(DeadLetterRedriveView::from_outcome(&outcome))))
}

/// `GET /api/v1/identities/{principal_id}` — read one identity by principal id.
pub(crate) async fn get_identity(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(principal_id): Path<String>,
) -> Result<Json<IdentityView>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let principal_id = Uuid::parse_str(&principal_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid principal id".into()))?;
    let record = state
        .engine
        .store
        .nodes
        .get_identity(principal_id)
        .await
        .map_err(map_context_node_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "identity not found".into()))?;
    Ok(Json(IdentityView::from_record(&record, false)))
}
