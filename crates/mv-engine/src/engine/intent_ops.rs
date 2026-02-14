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

    /// Apply an intent: execute the action and mark as applied.
    pub async fn apply_intent(
        self: &Arc<Self>,
        id: Uuid,
    ) -> MvResult<crate::intent_executor::ExecutionResult> {
        // Get the intent
        let intent = self
            .store
            .nodes
            .get_intent(id)
            .await?
            .ok_or_else(|| MvError::InvalidInput(format!("Intent {} not found", id)))?;

        // Execute the intent
        let executor = crate::intent_executor::IntentExecutor::new(Arc::clone(self));
        let result = executor.execute(&intent).await?;

        // If execution succeeded, mark as applied
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
