use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use mv_core::*;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::MindVaultEngine;

impl MindVaultEngine {
    // ── Consumer Profiles ────────────────────────────────────────────

    /// Create a new consumer profile with a random bearer token.
    ///
    /// Returns the profile and the raw token (shown once — only the hash is stored).
    pub async fn create_consumer(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> MvResult<(ConsumerProfile, String)> {
        // Check name uniqueness
        if let Some(_existing) = self.store.nodes.get_consumer_by_name(name).await? {
            return Err(MvError::InvalidInput(format!(
                "consumer with name '{name}' already exists"
            )));
        }

        // Generate 32 random bytes
        let mut raw_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut raw_bytes);

        // Base64url encode as the raw token
        let raw_token = format!("mvc_{}", URL_SAFE_NO_PAD.encode(raw_bytes));

        // Hash with SHA-256 for storage
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        let digest = hasher.finalize();
        let token_hash = URL_SAFE_NO_PAD.encode(digest);

        let now = Utc::now();
        let profile = ConsumerProfile {
            id: Uuid::now_v7(),
            name: name.to_string(),
            description: description.map(|d| d.to_string()),
            token_hash,
            created_at: now,
            last_used_at: None,
            revoked_at: None,
            metadata: std::collections::HashMap::new(),
        };

        self.store.nodes.create_consumer(&profile).await?;
        Ok((profile, raw_token))
    }

    /// Resolve a raw consumer token to the corresponding profile.
    ///
    /// Hashes the token, looks up by hash, returns None if not found or revoked.
    /// Touches `last_used_at` on success.
    pub async fn resolve_consumer_token(&self, token: &str) -> MvResult<Option<ConsumerProfile>> {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let digest = hasher.finalize();
        let token_hash = URL_SAFE_NO_PAD.encode(digest);

        let profile = match self
            .store
            .nodes
            .get_consumer_by_token_hash(&token_hash)
            .await?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        if profile.revoked_at.is_some() {
            return Ok(None);
        }

        // Touch last_used_at
        let _ = self.store.nodes.touch_consumer(profile.id).await;

        Ok(Some(profile))
    }

    /// List all consumer profiles.
    pub async fn list_consumers(&self) -> MvResult<Vec<ConsumerProfile>> {
        self.store.nodes.list_consumers().await
    }

    /// Get a consumer profile by ID.
    pub async fn get_consumer(&self, id: Uuid) -> MvResult<Option<ConsumerProfile>> {
        self.store.nodes.get_consumer(id).await
    }

    /// Revoke a consumer profile by ID.
    pub async fn revoke_consumer(&self, id: Uuid) -> MvResult<bool> {
        self.store.nodes.revoke_consumer(id).await
    }

    // ── Access Policies ─────────────────────────────────────────────

    /// Create or update an access policy.
    pub async fn set_policy(&self, policy: &AccessPolicy) -> MvResult<()> {
        self.store.nodes.set_policy(policy).await
    }

    /// Check whether a consumer is allowed access to a specific secret.
    ///
    /// Returns `PolicyDecision::Allow` with TTL/scopes or `PolicyDecision::Deny` with reason.
    /// Default deny: no matching policy means deny.
    pub(crate) async fn check_policy(
        &self,
        secret_key: &str,
        consumer: &str,
    ) -> MvResult<PolicyDecision> {
        let policy = self
            .store
            .nodes
            .get_policy_for(secret_key, consumer)
            .await?;

        match policy {
            None => Ok(PolicyDecision::Deny {
                reason: format!(
                    "no policy found for consumer '{consumer}' on secret '{secret_key}'"
                ),
            }),
            Some(p) => {
                if !p.allowed {
                    return Ok(PolicyDecision::Deny {
                        reason: "policy explicitly denies access".to_string(),
                    });
                }

                if p.is_expired() {
                    return Ok(PolicyDecision::Deny {
                        reason: "policy has expired".to_string(),
                    });
                }

                if p.require_approval {
                    return Ok(PolicyDecision::RequiresApproval {
                        ttl_seconds: p.max_ttl_seconds.unwrap_or(300),
                        scopes: p.scopes.clone(),
                    });
                }

                Ok(PolicyDecision::Allow {
                    ttl_seconds: p.max_ttl_seconds,
                    scopes: p.scopes.clone(),
                })
            }
        }
    }

    /// List access policies with optional filters.
    pub async fn list_policies(
        &self,
        secret_key: Option<&str>,
        consumer: Option<&str>,
    ) -> MvResult<Vec<AccessPolicy>> {
        self.store.nodes.list_policies(secret_key, consumer).await
    }

    /// Delete an access policy by ID.
    pub async fn delete_policy(&self, id: Uuid) -> MvResult<bool> {
        self.store.nodes.delete_policy(id).await
    }

    // ── Proxy Audit ─────────────────────────────────────────────────

    /// Log a proxy audit entry.
    pub(crate) async fn log_proxy_audit(&self, entry: &ProxyAuditEntry) -> MvResult<()> {
        self.store.nodes.log_proxy_audit(entry).await
    }

    /// Update a proxy audit entry with execution results.
    pub(crate) async fn update_proxy_audit(
        &self,
        id: Uuid,
        success: bool,
        sanitized: bool,
        error: Option<&str>,
        response_status: Option<i32>,
    ) -> MvResult<()> {
        self.store
            .nodes
            .update_proxy_audit(id, success, sanitized, error, response_status)
            .await
    }

    /// List proxy audit entries with optional consumer filter.
    pub async fn list_proxy_audit(
        &self,
        consumer: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ProxyAuditEntry>> {
        self.store
            .nodes
            .list_proxy_audit(consumer, limit, offset)
            .await
    }

    // ── Proxy Approvals ─────────────────────────────────────────────

    /// Create a new approval request.
    pub(crate) async fn create_approval(&self, request: &ApprovalRequest) -> MvResult<()> {
        self.store.nodes.create_approval(request).await
    }

    /// Get a specific approval by ID.
    pub async fn get_approval(&self, id: Uuid) -> MvResult<Option<ApprovalRequest>> {
        self.store.nodes.get_approval(id).await
    }

    /// List pending approvals, optionally filtered by consumer.
    pub async fn list_pending_approvals(
        &self,
        consumer: Option<&str>,
    ) -> MvResult<Vec<ApprovalRequest>> {
        self.store.nodes.list_pending_approvals(consumer).await
    }

    /// Approve or deny an approval request.
    pub async fn decide_approval(
        &self,
        id: Uuid,
        approved: bool,
        decided_by: Option<&str>,
        deny_reason: Option<&str>,
    ) -> MvResult<bool> {
        self.store
            .nodes
            .decide_approval(id, approved, decided_by, deny_reason)
            .await
    }

    /// Expire all past-due pending approvals.
    pub async fn expire_approvals(&self) -> MvResult<usize> {
        self.store.nodes.expire_approvals().await
    }

    /// Find an active (approved, non-expired) approval for a consumer+secret pair.
    pub(crate) async fn find_active_approval(
        &self,
        consumer: &str,
        secret_key: &str,
    ) -> MvResult<Option<ApprovalRequest>> {
        self.store
            .nodes
            .find_active_approval(consumer, secret_key)
            .await
    }
}
