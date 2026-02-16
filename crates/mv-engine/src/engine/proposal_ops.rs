use chrono::{DateTime, Utc};
use mv_core::*;
use uuid::Uuid;

use super::{glob_match_simple, proposal_node_payload, MindVaultEngine, ProposalActionResult, UndoActionResult};

impl MindVaultEngine {
    // ── Exchange Inbox ───────────────────────────────────────────────

    pub async fn submit_proposal(&self, proposal: &Proposal) -> MvResult<()> {
        self.store.nodes.submit_proposal(proposal).await
    }

    pub async fn get_proposal(&self, id: Uuid) -> MvResult<Option<Proposal>> {
        self.store.nodes.get_proposal(id).await
    }

    pub async fn list_proposals(
        &self,
        state: Option<ProposalState>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<Proposal>> {
        self.store.nodes.list_proposals(state, limit, offset).await
    }

    pub async fn resolve_proposal(&self, id: Uuid, state: ProposalState) -> MvResult<bool> {
        self.store.nodes.resolve_proposal(id, state).await
    }

    pub async fn count_proposals(&self, state: Option<ProposalState>) -> MvResult<usize> {
        self.store.nodes.count_proposals(state).await
    }

    pub async fn expire_proposals(&self, before: DateTime<Utc>) -> MvResult<usize> {
        self.store.nodes.expire_proposals(before).await
    }

    // ── Proposal Execution (business logic) ────────────────────────────────

    /// Check auto-approve rules against a proposal.
    /// Returns `true` if any enabled rule matches the given sender, action, and
    /// confidence level.
    pub async fn check_auto_approve_rules(
        &self,
        sender_name: &str,
        action: &ProposalAction,
        confidence: f32,
    ) -> MvResult<bool> {
        let rules = self.store.nodes.list_auto_approve_rules().await?;
        for rule in &rules {
            if !rule.enabled {
                continue;
            }
            if let Some(ref pattern) = rule.sender_pattern {
                if !glob_match_simple(pattern, sender_name) {
                    continue;
                }
            }
            if !rule.action_types.is_empty()
                && !rule.action_types.iter().any(|a| a == action.as_str())
            {
                continue;
            }
            if confidence < rule.min_confidence {
                continue;
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// Execute the action described by a proposal (create/update/delete node).
    ///
    /// This is pure business logic — authorization and HTTP mapping belong in the
    /// REST layer. The `namespace` parameter is the resolved namespace for creates.
    pub async fn execute_proposal_action(
        &self,
        proposal: &Proposal,
        namespace: &str,
    ) -> MvResult<ProposalActionResult> {
        let mut result = ProposalActionResult::default();

        match &proposal.action {
            ProposalAction::CreateNode => {
                let payload = proposal_node_payload(&proposal.payload)?;
                let content = payload.content.ok_or_else(|| {
                    MvError::InvalidInput("proposal payload missing content".into())
                })?;
                let kind_raw = payload.kind.unwrap_or_else(|| "fact".to_string());
                let kind: NodeKind = kind_raw
                    .parse()
                    .map_err(|e: String| MvError::InvalidInput(e))?;
                let tags = payload.tags.unwrap_or_default();

                let mut node =
                    KnowledgeNode::new(kind, content).with_namespace(namespace.to_string());
                if let Some(title) = payload.title {
                    node = node.with_title(title);
                }
                if let Some(source) = payload.source {
                    node = node.with_source(source);
                }
                if !tags.is_empty() {
                    node = node.with_tags(tags);
                }
                if let Some(importance) = payload.importance {
                    node = node.with_importance(importance);
                }
                if let Some(metadata) = payload.metadata {
                    node.metadata = metadata;
                }

                let stored = self.store_node(node).await?;
                result.created_node_id = Some(stored.id);
                result.affected_namespace = Some(stored.namespace);
            }
            ProposalAction::UpdateNode | ProposalAction::SuggestTag => {
                let target_id = proposal
                    .target_node_id
                    .ok_or_else(|| MvError::InvalidInput("proposal missing target_node_id".into()))?;
                let existing = self
                    .get_node(target_id)
                    .await?
                    .ok_or(MvError::NodeNotFound(target_id))?;

                let payload = proposal_node_payload(&proposal.payload)?;

                let mut updated = existing.clone();
                if let Some(kind) = payload.kind {
                    updated.kind = kind
                        .parse()
                        .map_err(|e: String| MvError::InvalidInput(e))?;
                }
                if let Some(content) = payload.content {
                    updated.content = content;
                }
                if let Some(title) = payload.title {
                    updated.title = Some(title);
                }
                if let Some(source) = payload.source {
                    updated.source = Some(source);
                }

                let mut tags = updated.tags.clone();
                if matches!(&proposal.action, ProposalAction::SuggestTag) {
                    if let Some(tag_val) = proposal.payload.get("tag").and_then(|v| v.as_str()) {
                        let tag = tag_val.trim();
                        if !tag.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                            tags.push(tag.to_string());
                        }
                    } else {
                        return Err(MvError::InvalidInput(
                            "proposal payload missing tag".into(),
                        ));
                    }
                } else if let Some(new_tags) = payload.tags {
                    tags = new_tags;
                }

                if let Some(importance) = payload.importance {
                    updated.importance = importance;
                }
                if let Some(metadata) = payload.metadata {
                    updated.metadata = metadata;
                }
                if let Some(ns) = payload.namespace {
                    updated.namespace = ns;
                }

                updated.tags = tags;
                let saved = self.update_node(updated).await?;
                result.updated_node_id = Some(saved.id);
                result.affected_namespace = Some(saved.namespace);
            }
            ProposalAction::DeleteNode => {
                let target_id = proposal
                    .target_node_id
                    .ok_or_else(|| MvError::InvalidInput("proposal missing target_node_id".into()))?;
                let existing = self
                    .get_node(target_id)
                    .await?
                    .ok_or(MvError::NodeNotFound(target_id))?;

                let deleted = self.delete_node(target_id).await?;
                if deleted {
                    result.deleted_node_id = Some(target_id);
                    result.affected_namespace = Some(existing.namespace);
                } else {
                    return Err(MvError::NodeNotFound(target_id));
                }
            }
            _ => {
                return Err(MvError::InvalidInput(
                    "proposal action not supported for execution".into(),
                ));
            }
        }

        Ok(result)
    }

    /// Build the undo snapshot data for a proposal before execution.
    ///
    /// Returns `None` for actions that do not support undo.
    pub async fn build_undo_snapshot(
        &self,
        proposal: &Proposal,
    ) -> MvResult<Option<serde_json::Value>> {
        match proposal.action {
            ProposalAction::CreateNode => {
                Ok(Some(serde_json::json!({ "action": "create_node" })))
            }
            ProposalAction::UpdateNode | ProposalAction::SuggestTag => {
                let target_id = proposal
                    .target_node_id
                    .ok_or_else(|| MvError::InvalidInput("missing target node id".into()))?;
                let existing = self
                    .get_node(target_id)
                    .await?
                    .ok_or(MvError::NodeNotFound(target_id))?;
                Ok(Some(serde_json::json!({
                    "action": "update_node",
                    "previous": existing
                })))
            }
            ProposalAction::DeleteNode => {
                let target_id = proposal
                    .target_node_id
                    .ok_or_else(|| MvError::InvalidInput("missing target node id".into()))?;
                let existing = self
                    .get_node(target_id)
                    .await?
                    .ok_or(MvError::NodeNotFound(target_id))?;
                Ok(Some(serde_json::json!({
                    "action": "delete_node",
                    "node": existing
                })))
            }
            _ => Ok(None),
        }
    }

    /// Save an undo snapshot for a completed proposal action.
    pub async fn save_proposal_undo(
        &self,
        proposal_id: Uuid,
        mut snapshot_data: serde_json::Value,
        created_node_id: Option<Uuid>,
    ) -> MvResult<()> {
        if let Some(node_id) = created_node_id {
            snapshot_data["node_id"] = serde_json::json!(node_id.to_string());
        }
        let now = Utc::now();
        let snapshot = UndoSnapshot {
            id: Uuid::now_v7(),
            proposal_id,
            snapshot_data,
            created_at: now,
            expires_at: now + chrono::Duration::days(7),
            used: false,
        };
        self.store.nodes.save_undo_snapshot(&snapshot).await
    }

    /// Apply an undo snapshot, reversing the original proposal action.
    pub async fn apply_undo_snapshot(
        &self,
        proposal_id: Uuid,
    ) -> MvResult<UndoActionResult> {
        let snapshot = self
            .store
            .nodes
            .get_undo_snapshot(proposal_id)
            .await?
            .ok_or_else(|| MvError::InvalidInput("no undo snapshot for this proposal".into()))?;

        if snapshot.used {
            return Err(MvError::InvalidInput(
                "undo already applied for this proposal".into(),
            ));
        }

        if Utc::now() > snapshot.expires_at {
            return Err(MvError::InvalidInput("undo window has expired".into()));
        }

        let action = snapshot
            .snapshot_data
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        match action.as_str() {
            "create_node" => {
                if let Some(node_id_str) =
                    snapshot.snapshot_data.get("node_id").and_then(|v| v.as_str())
                {
                    if let Ok(node_id) = Uuid::parse_str(node_id_str) {
                        self.delete_node(node_id).await?;
                    }
                }
            }
            "update_node" => {
                if let Some(previous) = snapshot.snapshot_data.get("previous") {
                    let node: KnowledgeNode = serde_json::from_value(previous.clone())
                        .map_err(|e| MvError::Internal(format!("failed to deserialize previous node: {e}")))?;
                    self.update_node(node).await?;
                }
            }
            "delete_node" => {
                if let Some(node_data) = snapshot.snapshot_data.get("node") {
                    let node: KnowledgeNode = serde_json::from_value(node_data.clone())
                        .map_err(|e| MvError::Internal(format!("failed to deserialize deleted node: {e}")))?;
                    self.store_node(node).await?;
                }
            }
            _ => {
                return Err(MvError::InvalidInput(format!(
                    "cannot undo action type: {action}"
                )));
            }
        }

        self.store.nodes.mark_undo_used(snapshot.id).await?;

        Ok(UndoActionResult { action })
    }
}
