use std::sync::Arc;

use mv_core::*;
use uuid::Uuid;

use super::MindVaultEngine;

impl MindVaultEngine {
    // ── Agentic Intelligence Methods ─────────────────────────────────

    /// List captured intents with optional filters.
    pub async fn list_intents(
        &self,
        node_id: Option<Uuid>,
        status: Option<IntentStatus>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<CapturedIntent>> {
        self.store
            .nodes
            .list_intents(node_id, status, limit, offset)
            .await
    }

    /// Update the status of an intent (apply/dismiss).
    pub async fn update_intent_status(&self, id: Uuid, status: IntentStatus) -> MvResult<bool> {
        self.store.nodes.update_intent_status(id, status).await
    }

    /// Get a single intent by ID.
    pub async fn get_intent(&self, id: Uuid) -> MvResult<Option<CapturedIntent>> {
        self.store.nodes.get_intent(id).await
    }

    /// Apply an intent that the owner explicitly authorized.
    ///
    /// This is the interactive path: the owner acted on the intent in the
    /// inbox, so the human — the ultimate anchor in a single-owner vault — is
    /// the authorizing party. The admission is recorded rather than inferred.
    ///
    /// Automated callers must use [`Self::apply_intent_autonomously`], which
    /// consults the autonomy gate and defers by default.
    pub async fn apply_intent(
        self: &Arc<Self>,
        id: Uuid,
    ) -> MvResult<crate::intent_executor::ExecutionResult> {
        let intent = self.require_intent(id).await?;
        let admission =
            crate::admission::admit_owner_authorized(format!("intent.{}", intent.intent_type));
        self.execute_admitted_intent(id, &intent, &admission).await
    }

    /// Apply an intent without a human in the loop.
    ///
    /// Every effect passes the autonomy gate first. A refusal is a normal
    /// outcome — deferral is the safe default under System Principle 1 — so it
    /// is returned rather than raised, and the intent stays pending for the
    /// owner to act on.
    pub async fn apply_intent_autonomously(
        self: &Arc<Self>,
        id: Uuid,
        confidence: f32,
    ) -> MvResult<Result<crate::intent_executor::ExecutionResult, crate::admission::EffectRefusal>>
    {
        let intent = self.require_intent(id).await?;
        let request = crate::admission::EffectRequest::new(
            format!("intent.{}", intent.intent_type),
            confidence,
        )
        .with_scope("domain", "agentic");

        let outcome = crate::admission::admit_effect(&self.autonomy, &request).await?;
        let admission = match outcome {
            crate::admission::EffectOutcome::Admitted(admission) => admission,
            crate::admission::EffectOutcome::Refused(refusal) => return Ok(Err(refusal)),
        };

        self.execute_admitted_intent(id, &intent, &admission)
            .await
            .map(Ok)
    }

    async fn require_intent(&self, id: Uuid) -> MvResult<CapturedIntent> {
        self.store
            .nodes
            .get_intent(id)
            .await?
            .ok_or_else(|| MvError::InvalidInput(format!("Intent {} not found", id)))
    }

    async fn execute_admitted_intent(
        self: &Arc<Self>,
        id: Uuid,
        intent: &CapturedIntent,
        admission: &crate::admission::EffectAdmission,
    ) -> MvResult<crate::intent_executor::ExecutionResult> {
        let executor = crate::intent_executor::IntentExecutor::new(Arc::clone(self));
        let result = executor.execute(intent, admission).await?;

        if result.success {
            self.store
                .nodes
                .update_intent_status(id, IntentStatus::Applied)
                .await?;
        }

        Ok(result)
    }

    /// List proactive insights.
    pub async fn list_insights(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ProactiveInsight>> {
        self.store.nodes.list_insights(limit, offset).await
    }

    /// Delete (dismiss) an insight.
    pub async fn delete_insight(&self, id: Uuid) -> MvResult<bool> {
        self.store.nodes.delete_insight(id).await
    }

    // --- Conflict Detection ---

    /// List conflict alerts.
    pub async fn list_conflicts(
        &self,
        resolved: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ConflictAlert>> {
        self.store
            .nodes
            .list_conflicts(resolved, limit, offset)
            .await
    }

    /// Get a single conflict alert.
    pub async fn get_conflict(&self, id: Uuid) -> MvResult<Option<ConflictAlert>> {
        self.store.nodes.get_conflict(id).await
    }

    /// Resolve (dismiss) a conflict alert.
    pub async fn resolve_conflict(&self, id: Uuid) -> MvResult<bool> {
        self.store.nodes.resolve_conflict(id).await
    }
}
