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
    http::StatusCode,
    Extension, Json,
};
use chrono::{Duration, Utc};
use mv_core::{
    ActionEnvelope, AdmissionDecision, AuthorityGrant, AuthorityGrantKind, AuthorityGrantStatus,
    CommandAdmissionRequest, ContextCapability, ContextNodeRecord, IdempotencyKey,
    InteroperabilityStore, MvError, RetentionClass, Sensitivity, StableUri,
};
use mv_engine::engine::IssueAuthorityGrantRequest;
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
/// `actor` equals `principal` in this slice, and that is honest rather than a
/// placeholder: the authenticated credential *is* the acting identity today,
/// and there is no governed identity registry that could resolve a distinct
/// accountable human behind it. Separating them requires that registry.
///
/// There is deliberately no header for overriding the actor. An unauthenticated
/// header naming an arbitrary actor would let a caller attribute its own
/// commands to someone else.
pub(crate) struct CommandIdentity {
    pub principal: StableUri,
    pub actor: StableUri,
}

impl CommandIdentity {
    /// Derive the principal URI for an authenticated caller.
    ///
    /// The derivation is `Uuid::new_v5(local_node_id, subject)` and must stay
    /// byte-identical: the node-create replay index is principal-scoped, so a
    /// changed principal URI would silently orphan every prior idempotency key.
    ///
    /// Missing subjects no longer silently become `local-system` unless the
    /// explicit transition flag `MINDVAULT_ALLOW_LOCAL_SYSTEM_IDENTITY=1` is set
    /// (IK-003). Prefer registering a governed identity and authenticating a
    /// real subject.
    pub(crate) fn derive(auth: &AuthContext, local_node_id: Uuid) -> Result<Self, String> {
        let subject = match auth.subject.as_deref() {
            Some(subject) => subject,
            None if allow_local_system_identity() => "local-system",
            None => {
                return Err(
                    "missing auth subject; set MINDVAULT_ALLOW_LOCAL_SYSTEM_IDENTITY=1 to allow the local-system transition identity".into(),
                );
            }
        };
        let principal_id = Uuid::new_v5(&local_node_id, subject.as_bytes());
        let principal = StableUri::principal(local_node_id, principal_id);
        Ok(Self {
            actor: principal.clone(),
            principal,
        })
    }
}

fn allow_local_system_identity() -> bool {
    std::env::var("MINDVAULT_ALLOW_LOCAL_SYSTEM_IDENTITY")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
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

/// Build the admission question for Work Order admit (SPACE-002).
///
/// Same governing-node Tool Grant target as node create: the work order URI is
/// minted during admission, so the grant target is the local node.
pub(crate) fn work_order_admit_admission_request(
    identity: &CommandIdentity,
    local_node_id: Uuid,
    idempotency_key: IdempotencyKey,
    correlation_id: Uuid,
    causation_id: Option<Uuid>,
) -> CommandAdmissionRequest {
    node_command_admission_request(
        identity,
        local_node_id,
        StableUri::node(local_node_id),
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
        let identity = CommandIdentity::derive(&auth_with_subject(Some("owner")), node_id).unwrap();

        let expected = StableUri::principal(node_id, Uuid::new_v5(&node_id, b"owner"));
        assert_eq!(identity.principal, expected);
        assert_eq!(
            identity.actor, expected,
            "actor and principal are the same identity until a governed identity registry exists"
        );

        // Stable across calls.
        let again = CommandIdentity::derive(&auth_with_subject(Some("owner")), node_id).unwrap();
        assert_eq!(again.principal, expected);

        // A different subject is a different principal.
        let other = CommandIdentity::derive(&auth_with_subject(Some("delegate")), node_id).unwrap();
        assert_ne!(other.principal, expected);
    }

    /// An unauthenticated-subject caller still gets a deterministic principal.
    #[test]
    fn a_missing_subject_derives_the_local_system_principal() {
        let node_id = Uuid::now_v7();
        // Without the explicit transition flag, missing subjects are rejected.
        assert!(CommandIdentity::derive(&auth_with_subject(None), node_id).is_err());

        std::env::set_var("MINDVAULT_ALLOW_LOCAL_SYSTEM_IDENTITY", "1");
        let identity = CommandIdentity::derive(&auth_with_subject(None), node_id).unwrap();
        let expected = StableUri::principal(node_id, Uuid::new_v5(&node_id, b"local-system"));
        assert_eq!(identity.principal, expected);
        std::env::remove_var("MINDVAULT_ALLOW_LOCAL_SYSTEM_IDENTITY");
    }

    /// The admission question must mirror the node-create envelope exactly.
    #[test]
    fn a_node_create_request_targets_the_governing_node() {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let identity = CommandIdentity::derive(&auth_with_subject(Some("owner")), node_id).unwrap();
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
    pub purpose: String,
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
            purpose: grant.purpose.clone(),
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

    let grantee = match (&body.grantee, &body.grantee_subject) {
        (Some(uri), None) => StableUri::parse(uri).map_err(|err| (StatusCode::BAD_REQUEST, err))?,
        (None, Some(subject)) => {
            if subject.trim().is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "grantee_subject must not be empty".into(),
                ));
            }
            state.engine.principal_for_subject(local_node_id, subject)
        }
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "provide grantee or grantee_subject".into(),
            ));
        }
        (Some(_), Some(_)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "provide only one of grantee or grantee_subject".into(),
            ));
        }
    };

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
