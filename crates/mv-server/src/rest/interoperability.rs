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

use axum::http::StatusCode;
use mv_core::{
    AdmissionDecision, AuthorityGrantKind, CommandAdmissionRequest, ContextCapability,
    IdempotencyKey, MvError, RetentionClass, Sensitivity, StableUri,
};
use uuid::Uuid;

use crate::auth::AuthContext;
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
    pub(crate) fn derive(auth: &AuthContext, local_node_id: Uuid) -> Self {
        let subject = auth.subject.as_deref().unwrap_or("local-system");
        let principal_id = Uuid::new_v5(&local_node_id, subject.as_bytes());
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
pub(crate) fn node_create_admission_request(
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

/// Resolve admission according to the configured mode.
///
/// Returns `Ok(None)` when the mode is `Off`, so the caller emits an event that
/// is byte-identical to pre-admission behaviour. Returns `Err` only under
/// `Enforce`.
pub(crate) async fn admit_command(
    state: &AppState,
    request: &CommandAdmissionRequest,
) -> Result<Option<AdmissionDecision>, (StatusCode, String)> {
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
        Err(error) => {
            tracing::error!(%error, "command admission resolver failed");
            if enforcing {
                return Err((
                    StatusCode::FORBIDDEN,
                    COMMAND_ADMISSION_DENIED.to_string(),
                ));
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
            return Err((
                StatusCode::FORBIDDEN,
                COMMAND_ADMISSION_DENIED.to_string(),
            ));
        }
    }

    Ok(Some(decision))
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
        let identity = CommandIdentity::derive(&auth_with_subject(Some("owner")), node_id);

        let expected = StableUri::principal(node_id, Uuid::new_v5(&node_id, b"owner"));
        assert_eq!(identity.principal, expected);
        assert_eq!(
            identity.actor, expected,
            "actor and principal are the same identity until a governed identity registry exists"
        );

        // Stable across calls.
        let again = CommandIdentity::derive(&auth_with_subject(Some("owner")), node_id);
        assert_eq!(again.principal, expected);

        // A different subject is a different principal.
        let other = CommandIdentity::derive(&auth_with_subject(Some("delegate")), node_id);
        assert_ne!(other.principal, expected);
    }

    /// An unauthenticated-subject caller still gets a deterministic principal.
    #[test]
    fn a_missing_subject_derives_the_local_system_principal() {
        let node_id = Uuid::now_v7();
        let identity = CommandIdentity::derive(&auth_with_subject(None), node_id);
        let expected = StableUri::principal(node_id, Uuid::new_v5(&node_id, b"local-system"));
        assert_eq!(identity.principal, expected);
    }

    /// The admission question must mirror the node-create envelope exactly.
    #[test]
    fn a_node_create_request_targets_the_governing_node() {
        let node_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();
        let identity = CommandIdentity::derive(&auth_with_subject(Some("owner")), node_id);
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
