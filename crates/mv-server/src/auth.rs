use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use mv_core::{AccessKey, PermissionTemplate, PermissionTier};

use crate::state::AppState;

const AUTHORIZATION_BEARER_PREFIX: &str = "Bearer ";
const ENV_AUTH_TOKEN: &str = "MINDVAULT_AUTH_TOKEN";
const ENV_AUTH_ROLE: &str = "MINDVAULT_AUTH_ROLE";
const ENV_AUTH_NAMESPACE: &str = "MINDVAULT_AUTH_NAMESPACE";
const ENV_JWT_SECRET: &str = "MINDVAULT_JWT_SECRET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRole {
    Admin,
    Write,
    Read,
}

impl AuthRole {
    pub fn can_read(self) -> bool {
        true
    }

    pub fn can_write(self) -> bool {
        matches!(self, Self::Admin | Self::Write)
    }
}

impl FromStr for AuthRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "write" | "writer" => Ok(Self::Write),
            "read" | "reader" => Ok(Self::Read),
            _ => Err(format!("invalid auth role: {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub subject: Option<String>,
    pub role: AuthRole,
    pub namespace: Option<String>,
}

impl AuthContext {
    pub fn system_admin() -> Self {
        Self {
            subject: None,
            role: AuthRole::Admin,
            namespace: None,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == AuthRole::Admin
    }

    pub fn from_access_key(key: &AccessKey, template: &PermissionTemplate) -> Self {
        Self {
            subject: Some(format!("access-key:{}", key.id)),
            role: auth_role_from_tier(template.tier),
            namespace: template.scope_namespace.clone(),
        }
    }

    pub fn can_read(&self) -> bool {
        self.role.can_read()
    }

    pub fn can_write(&self) -> bool {
        self.role.can_write()
    }

    pub fn allows_namespace(&self, namespace: &str) -> bool {
        self.is_admin()
            || self
                .namespace
                .as_deref()
                .is_none_or(|allowed_ns| allowed_ns == namespace)
    }
}

fn auth_role_from_tier(tier: PermissionTier) -> AuthRole {
    match tier {
        PermissionTier::Admin => AuthRole::Admin,
        PermissionTier::View => AuthRole::Read,
        PermissionTier::Edit | PermissionTier::Action => AuthRole::Write,
    }
}

#[derive(Debug, Clone)]
struct AuthConfig {
    jwt_secret: Option<String>,
    shared_token: Option<String>,
    shared_role: AuthRole,
    shared_namespace: Option<String>,
}

impl AuthConfig {
    fn from_env() -> Self {
        Self {
            jwt_secret: read_non_empty_env(ENV_JWT_SECRET),
            shared_token: read_non_empty_env(ENV_AUTH_TOKEN),
            shared_role: read_non_empty_env(ENV_AUTH_ROLE)
                .and_then(|value| AuthRole::from_str(&value).ok())
                .unwrap_or(AuthRole::Admin),
            shared_namespace: read_non_empty_env(ENV_AUTH_NAMESPACE),
        }
    }

    fn is_enabled(&self) -> bool {
        self.jwt_secret.is_some() || self.shared_token.is_some()
    }
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    sub: String,
    exp: usize,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    namespace: Option<String>,
}

#[derive(Debug)]
pub(crate) enum AuthError {
    MissingAuthHeader,
    InvalidHeaderFormat,
    InvalidSharedToken,
    InvalidJwt,
}

/// Shared auth middleware for REST routes.
///
/// Auth behavior:
/// - If neither `MINDVAULT_JWT_SECRET` nor `MINDVAULT_AUTH_TOKEN` is set, auth is disabled.
/// - If `MINDVAULT_JWT_SECRET` is set, valid HS256 JWT bearer tokens are accepted.
/// - If `MINDVAULT_AUTH_TOKEN` is set, matching bearer token is accepted.
/// - If both are set, either mechanism is accepted.
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_context = auth_context_from_headers(request.headers())?;
    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}

pub async fn auth_middleware_with_state(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_context = auth_context_from_headers_with_state(request.headers(), &state).await?;
    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}

pub fn auth_context_from_headers(headers: &HeaderMap) -> Result<AuthContext, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    auth_context_from_authorization_header(auth_header).map_err(|_err| StatusCode::UNAUTHORIZED)
}

pub async fn auth_context_from_headers_with_state(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<AuthContext, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());

    auth_context_from_authorization_header_with_state(auth_header, state)
        .await
        .map_err(|_err| StatusCode::UNAUTHORIZED)
}

pub(crate) fn auth_context_from_authorization_header(
    auth_header: Option<&str>,
) -> Result<AuthContext, AuthError> {
    let config = AuthConfig::from_env();
    auth_context_from_authorization_header_with_config(auth_header, &config)
}

async fn auth_context_from_authorization_header_with_state(
    auth_header: Option<&str>,
    state: &AppState,
) -> Result<AuthContext, AuthError> {
    let config = AuthConfig::from_env();

    if !config.is_enabled() {
        return Ok(AuthContext::system_admin());
    }

    let token = extract_bearer_token(auth_header)?;

    if let Some((key, template)) = state
        .engine
        .resolve_access_key(token)
        .await
        .map_err(|_err| AuthError::InvalidSharedToken)?
    {
        return Ok(AuthContext::from_access_key(&key, &template));
    }

    auth_context_from_authorization_header_with_config(auth_header, &config)
}

fn auth_context_from_authorization_header_with_config(
    auth_header: Option<&str>,
    config: &AuthConfig,
) -> Result<AuthContext, AuthError> {
    if !config.is_enabled() {
        return Ok(AuthContext::system_admin());
    }

    let token = extract_bearer_token(auth_header)?;

    if let Some(shared_token) = &config.shared_token {
        if token == shared_token {
            return Ok(AuthContext {
                subject: Some("shared-token".into()),
                role: config.shared_role,
                namespace: config.shared_namespace.clone(),
            });
        }
    }

    if let Some(jwt_secret) = &config.jwt_secret {
        return validate_jwt(token, jwt_secret);
    }

    Err(AuthError::InvalidSharedToken)
}

fn extract_bearer_token(auth_header: Option<&str>) -> Result<&str, AuthError> {
    let header = auth_header.ok_or(AuthError::MissingAuthHeader)?;
    header
        .strip_prefix(AUTHORIZATION_BEARER_PREFIX)
        .ok_or(AuthError::InvalidHeaderFormat)
}

fn validate_jwt(token: &str, secret: &str) -> Result<AuthContext, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_err| AuthError::InvalidJwt)?;

    if token_data.claims.sub.trim().is_empty() || token_data.claims.exp == 0 {
        return Err(AuthError::InvalidJwt);
    }

    let role = token_data
        .claims
        .role
        .as_deref()
        .map(AuthRole::from_str)
        .transpose()
        .map_err(|_err| AuthError::InvalidJwt)?
        .unwrap_or(AuthRole::Write);

    Ok(AuthContext {
        subject: Some(token_data.claims.sub),
        role,
        namespace: token_data.claims.namespace,
    })
}

fn read_non_empty_env(key: &str) -> Option<String> {
    let value = std::env::var(key).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn authorize_read(auth: &AuthContext) -> Result<(), (StatusCode, String)> {
    if auth.can_read() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "read permission required".into()))
    }
}

pub fn authorize_write(auth: &AuthContext) -> Result<(), (StatusCode, String)> {
    if auth.can_write() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "write permission required".into()))
    }
}

pub fn authorize_namespace(
    auth: &AuthContext,
    namespace: &str,
) -> Result<(), (StatusCode, String)> {
    if auth.allows_namespace(namespace) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            format!("namespace '{namespace}' is not permitted"),
        ))
    }
}

pub fn scoped_namespace(
    auth: &AuthContext,
    requested_namespace: Option<String>,
) -> Result<Option<String>, (StatusCode, String)> {
    if auth.is_admin() {
        return Ok(requested_namespace);
    }

    match (&auth.namespace, requested_namespace) {
        (None, requested) => Ok(requested),
        (Some(allowed), Some(requested)) => {
            if requested == *allowed {
                Ok(Some(requested))
            } else {
                Err((
                    StatusCode::FORBIDDEN,
                    format!("namespace '{requested}' is not permitted"),
                ))
            }
        }
        (Some(allowed), None) => Ok(Some(allowed.clone())),
    }
}

pub fn namespace_for_create(
    auth: &AuthContext,
    requested_namespace: Option<String>,
    default_namespace: &str,
) -> Result<String, (StatusCode, String)> {
    let namespace = requested_namespace
        .or_else(|| auth.namespace.clone())
        .unwrap_or_else(|| default_namespace.to_string());

    authorize_namespace(auth, &namespace)?;
    Ok(namespace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[derive(Debug, serde::Serialize)]
    struct TestClaims {
        sub: String,
        exp: usize,
        role: Option<String>,
        namespace: Option<String>,
    }

    fn token_for(
        secret: &str,
        sub: &str,
        exp: usize,
        role: Option<&str>,
        namespace: Option<&str>,
    ) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            &TestClaims {
                sub: sub.to_string(),
                exp,
                role: role.map(|value| value.to_string()),
                namespace: namespace.map(|value| value.to_string()),
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("token creation should succeed")
    }

    #[test]
    fn auth_disabled_allows_missing_header() {
        let config = AuthConfig {
            jwt_secret: None,
            shared_token: None,
            shared_role: AuthRole::Admin,
            shared_namespace: None,
        };

        let result = auth_context_from_authorization_header_with_config(None, &config)
            .expect("auth should be disabled");
        assert!(result.is_admin());
    }

    #[test]
    fn shared_token_auth_accepts_matching_token() {
        let config = AuthConfig {
            jwt_secret: None,
            shared_token: Some("shared-token".into()),
            shared_role: AuthRole::Write,
            shared_namespace: Some("team-a".into()),
        };

        let result = auth_context_from_authorization_header_with_config(
            Some("Bearer shared-token"),
            &config,
        )
        .expect("shared token should be valid");

        assert_eq!(result.role, AuthRole::Write);
        assert_eq!(result.namespace.as_deref(), Some("team-a"));
    }

    #[test]
    fn shared_token_auth_rejects_wrong_token() {
        let config = AuthConfig {
            jwt_secret: None,
            shared_token: Some("shared-token".into()),
            shared_role: AuthRole::Admin,
            shared_namespace: None,
        };

        let result =
            auth_context_from_authorization_header_with_config(Some("Bearer wrong-token"), &config);
        assert!(matches!(result, Err(AuthError::InvalidSharedToken)));
    }

    #[test]
    fn jwt_auth_accepts_valid_token_with_role_and_namespace() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_secs() as usize;

        let secret = "jwt-secret";
        let jwt = token_for(
            secret,
            "demo-user",
            now + 3600,
            Some("read"),
            Some("team-a"),
        );

        let config = AuthConfig {
            jwt_secret: Some(secret.into()),
            shared_token: None,
            shared_role: AuthRole::Admin,
            shared_namespace: None,
        };

        let header = format!("Bearer {jwt}");
        let result = auth_context_from_authorization_header_with_config(Some(&header), &config)
            .expect("jwt should be valid");

        assert_eq!(result.role, AuthRole::Read);
        assert_eq!(result.namespace.as_deref(), Some("team-a"));
        assert_eq!(result.subject.as_deref(), Some("demo-user"));
    }

    #[test]
    fn jwt_auth_rejects_expired_token() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_secs() as usize;

        let secret = "jwt-secret";
        let jwt = token_for(secret, "demo-user", now.saturating_sub(120), None, None);

        let config = AuthConfig {
            jwt_secret: Some(secret.into()),
            shared_token: None,
            shared_role: AuthRole::Admin,
            shared_namespace: None,
        };

        let header = format!("Bearer {jwt}");
        let result = auth_context_from_authorization_header_with_config(Some(&header), &config);
        assert!(matches!(result, Err(AuthError::InvalidJwt)));
    }

    #[test]
    fn accepts_shared_token_when_both_auth_modes_enabled() {
        let config = AuthConfig {
            jwt_secret: Some("jwt-secret".into()),
            shared_token: Some("shared-token".into()),
            shared_role: AuthRole::Admin,
            shared_namespace: None,
        };

        let result = auth_context_from_authorization_header_with_config(
            Some("Bearer shared-token"),
            &config,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn scoped_namespace_limits_non_admin_requests() {
        let auth = AuthContext {
            subject: Some("user".into()),
            role: AuthRole::Write,
            namespace: Some("team-a".into()),
        };

        assert_eq!(
            scoped_namespace(&auth, None).expect("scope should default"),
            Some("team-a".into())
        );
        assert!(scoped_namespace(&auth, Some("team-b".into())).is_err());
    }
}
