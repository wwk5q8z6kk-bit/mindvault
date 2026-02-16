use chrono::Utc;
use mv_core::*;
use uuid::Uuid;

use super::MindVaultEngine;

impl MindVaultEngine {
    // ── Node Comments ────────────────────────────────────────────────

    pub async fn create_node_comment(
        &self,
        node_id: Uuid,
        author: Option<String>,
        body: String,
    ) -> MvResult<NodeComment> {
        let _ = self
            .store
            .nodes
            .get(node_id)
            .await?
            .ok_or_else(|| MvError::InvalidInput("node not found".to_string()))?;

        let now = Utc::now();
        let comment = NodeComment {
            id: Uuid::now_v7(),
            node_id,
            author,
            body,
            created_at: now,
            updated_at: now,
            resolved_at: None,
        };

        self.store.nodes.insert_comment(&comment).await?;
        Ok(comment)
    }

    pub async fn list_node_comments(
        &self,
        node_id: Uuid,
        include_resolved: bool,
    ) -> MvResult<Vec<NodeComment>> {
        self.store
            .nodes
            .list_comments(node_id, include_resolved)
            .await
    }

    pub async fn get_node_comment(&self, comment_id: Uuid) -> MvResult<Option<NodeComment>> {
        self.store.nodes.get_comment(comment_id).await
    }

    pub async fn resolve_node_comment(&self, comment_id: Uuid) -> MvResult<bool> {
        self.store
            .nodes
            .resolve_comment(comment_id, Utc::now())
            .await
    }

    pub async fn delete_node_comment(&self, comment_id: Uuid) -> MvResult<bool> {
        self.store.nodes.delete_comment(comment_id).await
    }

    // ── Contact Identity & Trust ───────────────────────────────────

    /// Add an identity to a relay contact.
    pub async fn add_contact_identity(&self, identity: &ContactIdentity) -> MvResult<()> {
        self.store.nodes.add_contact_identity(identity).await
    }

    /// List identities for a contact.
    pub async fn list_contact_identities(
        &self,
        contact_id: Uuid,
    ) -> MvResult<Vec<ContactIdentity>> {
        self.store.nodes.list_contact_identities(contact_id).await
    }

    /// Delete a contact identity.
    pub async fn delete_contact_identity(&self, id: Uuid) -> MvResult<bool> {
        self.store.nodes.delete_contact_identity(id).await
    }

    /// Verify a contact identity.
    pub async fn verify_contact_identity(&self, id: Uuid) -> MvResult<bool> {
        self.store.nodes.verify_contact_identity(id).await
    }

    /// Get trust model for a contact.
    pub async fn get_trust_model(&self, contact_id: Uuid) -> MvResult<Option<TrustModel>> {
        self.store.nodes.get_trust_model(contact_id).await
    }

    /// Set trust model for a contact.
    pub async fn set_trust_model(&self, model: &TrustModel) -> MvResult<()> {
        self.store.nodes.set_trust_model(model).await
    }

    // ── Chronicle & Audit Trail ───────────────────────────────────────

    /// List chronicle entries with optional node filter.
    pub async fn list_chronicles(
        &self,
        node_id: Option<Uuid>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ChronicleEntry>> {
        self.store
            .nodes
            .list_chronicles(node_id, limit, offset)
            .await
    }

    /// Log a chronicle entry for transparency.
    pub async fn log_chronicle(&self, entry: &ChronicleEntry) -> MvResult<()> {
        self.store.nodes.log_chronicle(entry).await
    }

    // ── Feedback / Learning ───────────────────────────────────────────

    /// Record feedback for an intent action (apply/dismiss).
    pub async fn record_feedback(&self, fb: &AgentFeedback) -> MvResult<()> {
        self.store.nodes.record_feedback(fb).await
    }

    /// Recalculate and store a confidence override based on accumulated feedback.
    pub async fn recalculate_confidence(&self, intent_type: &str) -> MvResult<()> {
        let (total, applied) = self.store.nodes.get_acceptance_rate(intent_type).await?;

        // Need at least 5 data points to start adjusting
        if total < 5 {
            return Ok(());
        }

        let rate = applied as f32 / total as f32;

        // base_adjustment: -0.2 to +0.2 based on acceptance rate
        // 50% → 0.0, 100% → +0.2, 0% → -0.2
        let base_adjustment = (rate - 0.5) * 0.4;

        // auto_apply_threshold: lower if acceptance rate is high
        let auto_apply_threshold = if rate > 0.9 && total >= 20 {
            0.9 // auto-apply above 0.9 confidence
        } else {
            0.95 // default: very high threshold
        };

        // suppress_below: raise if acceptance rate is very low
        let suppress_below = if rate < 0.1 && total >= 10 {
            0.5 // suppress weak suggestions for disliked intent types
        } else if rate < 0.3 {
            0.3
        } else {
            0.1 // default
        };

        let override_ = ConfidenceOverride {
            intent_type: intent_type.to_string(),
            base_adjustment,
            auto_apply_threshold,
            suppress_below,
            updated_at: chrono::Utc::now(),
        };

        self.store.nodes.set_confidence_override(&override_).await
    }
}
