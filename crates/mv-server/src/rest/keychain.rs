//! REST API handlers for the Sovereign Keychain system.
//! All endpoints require admin authorization.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use mv_core::model::keychain::*;
use mv_core::MvError;

use crate::auth::{authorize_write, AuthContext};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn map_keychain_error(err: MvError) -> (StatusCode, String) {
    match &err {
        MvError::VaultSealed => (StatusCode::LOCKED, err.to_string()),
        MvError::Keychain(msg) if msg.contains("not found") => (StatusCode::NOT_FOUND, err.to_string()),
        MvError::Keychain(msg) if msg.contains("not initialized") => {
            (StatusCode::PRECONDITION_FAILED, err.to_string())
        }
        MvError::Keychain(msg) if msg.contains("invalid password") => {
            (StatusCode::UNAUTHORIZED, err.to_string())
        }
        MvError::Keychain(msg) if msg.contains("already initialized") => {
            (StatusCode::CONFLICT, err.to_string())
        }
        MvError::Keychain(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn require_admin(auth: &AuthContext) -> Result<(), (StatusCode, String)> {
    authorize_write(auth)?;
    if !auth.is_admin() {
        return Err((StatusCode::FORBIDDEN, "admin access required".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct InitVaultRequest {
    pub password: String,
    #[serde(default)]
    pub macos_bridge: bool,
}

#[derive(Deserialize)]
pub struct UnsealRequest {
    pub password: Option<String>,
    #[serde(default)]
    pub from_macos_keychain: bool,
}

#[derive(Serialize)]
pub struct VaultStatusResponse {
    pub state: String,
    pub key_epoch: Option<u64>,
    pub credential_count: Option<usize>,
    pub domain_count: Option<usize>,
    pub created_at: Option<String>,
    pub last_rotated_at: Option<String>,
}

#[derive(Deserialize)]
pub struct RotateKeyRequest {
    pub new_password: String,
    #[serde(default = "default_grace_hours")]
    pub grace_hours: u32,
}

fn default_grace_hours() -> u32 {
    24
}

#[derive(Deserialize)]
pub struct CreateDomainRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct StoreCredentialRequest {
    pub domain_id: String,
    pub name: String,
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCredentialRequest {
    pub value: String,
}

#[derive(Deserialize)]
pub struct CreateDelegationRequest {
    pub credential_id: String,
    pub delegatee: String,
    #[serde(default)]
    pub can_read: bool,
    #[serde(default)]
    pub can_use: bool,
    #[serde(default)]
    pub can_delegate: bool,
    pub expires_at: Option<String>,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

fn default_max_depth() -> u32 {
    3
}

#[derive(Deserialize)]
pub struct SubDelegateRequest {
    pub delegatee: String,
    #[serde(default)]
    pub can_read: bool,
    #[serde(default)]
    pub can_use: bool,
    #[serde(default)]
    pub can_delegate: bool,
    pub expires_at: Option<String>,
}

#[derive(Deserialize)]
pub struct GenerateProofRequest {
    pub credential_id: String,
    pub challenge_nonce: String,
}

#[derive(Deserialize)]
pub struct VerifyProofRequest {
    pub credential_id: String,
    pub challenge_nonce: String,
    pub proof: String,
    pub generated_at: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    pub domain_id: Option<String>,
    pub state: Option<String>,
}

fn default_limit() -> usize {
    50
}

#[derive(Deserialize)]
pub struct DelegationListQuery {
    pub credential_id: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn init_vault(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<InitVaultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    state
        .engine
        .keychain
        .initialize_vault(&body.password, body.macos_bridge)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "initialized"})))
}

pub async fn unseal_vault(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<UnsealRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    if body.from_macos_keychain {
        state
            .engine
            .keychain
            .unseal_from_macos_keychain()
            .await
            .map_err(map_keychain_error)?;
    } else {
        let password = body
            .password
            .ok_or((StatusCode::BAD_REQUEST, "password required".into()))?;
        state
            .engine
            .keychain
            .unseal(&password)
            .await
            .map_err(map_keychain_error)?;
    }
    Ok(Json(serde_json::json!({"status": "unsealed"})))
}

pub async fn seal_vault(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    state
        .engine
        .keychain
        .seal()
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "sealed"})))
}

pub async fn vault_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<VaultStatusResponse>, (StatusCode, String)> {
    require_admin(&auth)?;
    let (vault_state, meta) = state
        .engine
        .keychain
        .vault_status()
        .await
        .map_err(map_keychain_error)?;

    let (cred_count, domain_count) = if vault_state != VaultState::Uninitialized {
        let cc = state
            .engine
            .keychain
            .store
            .count_credentials(None)
            .await
            .unwrap_or(0);
        let domains = state
            .engine
            .keychain
            .list_domains()
            .await
            .unwrap_or_default();
        (Some(cc), Some(domains.len()))
    } else {
        (None, None)
    };

    Ok(Json(VaultStatusResponse {
        state: vault_state.to_string(),
        key_epoch: meta.as_ref().map(|m| m.key_epoch),
        credential_count: cred_count,
        domain_count,
        created_at: meta.as_ref().map(|m| m.created_at.to_rfc3339()),
        last_rotated_at: meta
            .as_ref()
            .and_then(|m| m.last_rotated_at.map(|dt| dt.to_rfc3339())),
    }))
}

pub async fn rotate_key(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<RotateKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    state
        .engine
        .keychain
        .rotate_master_key(&body.new_password, body.grace_hours)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "rotated"})))
}

pub async fn list_epochs(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<KeyEpoch>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let epochs = state
        .engine
        .keychain
        .store
        .list_key_epochs()
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(epochs))
}

pub async fn create_domain(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateDomainRequest>,
) -> Result<(StatusCode, Json<DomainKey>), (StatusCode, String)> {
    require_admin(&auth)?;
    let domain = state
        .engine
        .keychain
        .create_domain(&body.name, body.description.as_deref())
        .await
        .map_err(map_keychain_error)?;
    Ok((StatusCode::CREATED, Json(domain)))
}

pub async fn list_domains(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<DomainKey>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let domains = state
        .engine
        .keychain
        .list_domains()
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(domains))
}

pub async fn revoke_domain(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state
        .engine
        .keychain
        .revoke_domain(uuid)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "revoked"})))
}

pub async fn store_credential(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<StoreCredentialRequest>,
) -> Result<(StatusCode, Json<StoredCredential>), (StatusCode, String)> {
    require_admin(&auth)?;
    let domain_id =
        Uuid::parse_str(&body.domain_id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let expires_at = match body.expires_at {
        Some(ref s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid expires_at: {e}")))?,
        ),
        None => None,
    };
    let cred = state
        .engine
        .keychain
        .store_credential(domain_id, &body.name, &body.kind, body.value.as_bytes(), body.tags, expires_at)
        .await
        .map_err(map_keychain_error)?;
    Ok((StatusCode::CREATED, Json(cred)))
}

pub async fn list_credentials(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<StoredCredential>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let domain_id = match q.domain_id {
        Some(ref s) => Some(Uuid::parse_str(s).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?),
        None => None,
    };
    let cred_state = match q.state {
        Some(ref s) => Some(
            s.parse::<CredentialState>()
                .map_err(|e| (StatusCode::BAD_REQUEST, e))?,
        ),
        None => None,
    };
    let creds = state
        .engine
        .keychain
        .list_credentials(domain_id, cred_state, q.limit, q.offset)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(creds))
}

pub async fn read_credential(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let (cred, plaintext) = state
        .engine
        .keychain
        .read_credential(uuid, &auth.subject)
        .await
        .map_err(map_keychain_error)?;
    let value = String::from_utf8_lossy(&plaintext).to_string();
    Ok(Json(serde_json::json!({
        "credential": cred,
        "value": value,
    })))
}

pub async fn update_credential(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCredentialRequest>,
) -> Result<Json<StoredCredential>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let cred = state
        .engine
        .keychain
        .update_credential_value(uuid, body.value.as_bytes())
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(cred))
}

pub async fn archive_credential(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state
        .engine
        .keychain
        .archive_credential(uuid)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "archived"})))
}

pub async fn destroy_credential(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state
        .engine
        .keychain
        .destroy_credential(uuid)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "destroyed"})))
}

pub async fn create_delegation(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateDelegationRequest>,
) -> Result<(StatusCode, Json<Delegation>), (StatusCode, String)> {
    require_admin(&auth)?;
    let cred_id = Uuid::parse_str(&body.credential_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let expires_at = match body.expires_at {
        Some(ref s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid expires_at: {e}")))?,
        ),
        None => None,
    };
    let perms = DelegationPermissions {
        can_read: body.can_read,
        can_use: body.can_use,
        can_delegate: body.can_delegate,
    };
    let delegation = state
        .engine
        .keychain
        .create_delegation(cred_id, &body.delegatee, perms, expires_at, body.max_depth)
        .await
        .map_err(map_keychain_error)?;
    Ok((StatusCode::CREATED, Json(delegation)))
}

pub async fn list_delegations(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(q): Query<DelegationListQuery>,
) -> Result<Json<Vec<Delegation>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let cred_id = Uuid::parse_str(&q.credential_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let delegations = state
        .engine
        .keychain
        .list_delegations(cred_id)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(delegations))
}

pub async fn revoke_delegation(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state
        .engine
        .keychain
        .revoke_delegation(uuid)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "revoked"})))
}

pub async fn sub_delegate(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<SubDelegateRequest>,
) -> Result<(StatusCode, Json<Delegation>), (StatusCode, String)> {
    require_admin(&auth)?;
    let parent_id = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let expires_at = match body.expires_at {
        Some(ref s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid expires_at: {e}")))?,
        ),
        None => None,
    };
    let perms = DelegationPermissions {
        can_read: body.can_read,
        can_use: body.can_use,
        can_delegate: body.can_delegate,
    };
    let delegation = state
        .engine
        .keychain
        .sub_delegate(parent_id, &body.delegatee, perms, expires_at)
        .await
        .map_err(map_keychain_error)?;
    Ok((StatusCode::CREATED, Json(delegation)))
}

pub async fn generate_proof(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<GenerateProofRequest>,
) -> Result<Json<ZkAccessProof>, (StatusCode, String)> {
    require_admin(&auth)?;
    let cred_id = Uuid::parse_str(&body.credential_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let proof = state
        .engine
        .keychain
        .generate_proof(cred_id, &body.challenge_nonce)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(proof))
}

pub async fn verify_proof(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<VerifyProofRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let cred_id = Uuid::parse_str(&body.credential_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let generated_at = chrono::DateTime::parse_from_rfc3339(&body.generated_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid generated_at: {e}")))?;
    let expires_at = chrono::DateTime::parse_from_rfc3339(&body.expires_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid expires_at: {e}")))?;

    let zk_proof = ZkAccessProof {
        credential_id: cred_id,
        challenge_nonce: body.challenge_nonce,
        proof: body.proof,
        generated_at,
        expires_at,
    };
    let valid = state
        .engine
        .keychain
        .verify_proof(&zk_proof)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"valid": valid})))
}

pub async fn list_audit(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<KeychainAuditEntry>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let entries = state
        .engine
        .keychain
        .list_audit_trail(q.limit, q.offset)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(entries))
}

pub async fn verify_audit_integrity(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let valid = state
        .engine
        .keychain
        .verify_audit_integrity()
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"valid": valid})))
}

pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<BreachAlert>>, (StatusCode, String)> {
    require_admin(&auth)?;
    let alerts = state
        .engine
        .keychain
        .list_breach_alerts(q.limit, q.offset)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(alerts))
}

pub async fn acknowledge_alert(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let uuid = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state
        .engine
        .keychain
        .acknowledge_alert(uuid)
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"status": "acknowledged"})))
}

pub async fn run_lifecycle(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_admin(&auth)?;
    let count = state
        .engine
        .keychain
        .run_lifecycle_transitions()
        .await
        .map_err(map_keychain_error)?;
    Ok(Json(serde_json::json!({"transitioned": count})))
}
