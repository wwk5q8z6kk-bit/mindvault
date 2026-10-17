use mv_core::*;
use uuid::Uuid;

use crate::llm;

use super::{MindVaultEngine, RelayInboundOutcome, RelayReplySuggestion};

impl MindVaultEngine {
    // ── Relay & Messaging ─────────────────────────────────────────────

    /// Receive a relay message and optionally generate an auto-reply or proposal.
    pub async fn receive_relay_message(
        &self,
        message: RelayMessage,
        namespace: &str,
    ) -> MvResult<RelayInboundOutcome> {
        let mut stored = self.relay.receive_message(message, namespace).await?;

        if stored.status == MessageStatus::Failed {
            return Ok(RelayInboundOutcome {
                message: stored,
                auto_reply: None,
                proposal_id: None,
            });
        }

        let Some(sender_id) = stored.sender_contact_id else {
            return Ok(RelayInboundOutcome {
                message: stored,
                auto_reply: None,
                proposal_id: None,
            });
        };

        let Some(contact) = self.relay.get_contact(sender_id).await? else {
            return Ok(RelayInboundOutcome {
                message: stored,
                auto_reply: None,
                proposal_id: None,
            });
        };

        if contact.trust_level == TrustLevel::RelayOnly {
            return Ok(RelayInboundOutcome {
                message: stored,
                auto_reply: None,
                proposal_id: None,
            });
        }

        if stored.content_type != ContentType::Text || stored.content.trim().is_empty() {
            return Ok(RelayInboundOutcome {
                message: stored,
                auto_reply: None,
                proposal_id: None,
            });
        }

        let thread_id = stored.thread_id.unwrap_or(stored.id);

        let subject = stored
            .metadata
            .get("subject")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                if value.to_ascii_lowercase().starts_with("re:") {
                    value.to_string()
                } else {
                    format!("Re: {value}")
                }
            });

        let query = MemoryQuery::new(&stored.content)
            .with_namespace(namespace.to_string())
            .with_limit(6)
            .with_min_score(0.0);

        let results = match self.recall(&query).await {
            Ok(results) => results,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "relay_reply_context_recall_failed"
                );
                return Ok(RelayInboundOutcome {
                    message: stored,
                    auto_reply: None,
                    proposal_id: None,
                });
            }
        };

        let context_snippets = llm::extract_context_snippets(&results, 4);
        if context_snippets.is_empty() {
            return Ok(RelayInboundOutcome {
                message: stored,
                auto_reply: None,
                proposal_id: None,
            });
        }

        let input = if let Some(ref subject) = subject {
            format!("Subject: {subject}\n\n{}", stored.content)
        } else {
            stored.content.clone()
        };

        let mut used_llm = false;
        let mut suggestion_text = None;
        if let Some(ref llm) = self.llm {
            match llm::llm_completion_suggestions(llm.as_ref(), &input, &context_snippets, 1).await
            {
                Ok(mut suggestions) => {
                    if let Some(first) = suggestions.pop() {
                        suggestion_text = Some(first);
                        used_llm = true;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        provider = %llm.name(),
                        "relay_reply_llm_suggestion_failed"
                    );
                }
            }
        }

        if suggestion_text.is_none() {
            let preview = context_snippets
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            suggestion_text = Some(format!(
                "I have related notes that might help:\n{preview}\n\nWant me to share details?"
            ));
        }

        let mut confidence: f32 = if used_llm { 0.6 } else { 0.4 };
        if context_snippets.len() >= 3 {
            confidence += 0.1;
        }
        if contact.trust_level == TrustLevel::Full {
            confidence += 0.1;
        }
        if contact.trust_level == TrustLevel::ContextInject {
            confidence -= 0.05;
        }
        confidence = confidence.clamp(0.0, 1.0);

        let suggestion = RelayReplySuggestion {
            content: suggestion_text.unwrap_or_default(),
            confidence,
            context_snippets,
        };

        let contact_scope = sender_id.to_string();
        let scope_hints = [("contact", contact_scope.as_str()), ("domain", "relay")];
        let mut decision = self
            .autonomy
            .evaluate("relay.reply", suggestion.confidence, &scope_hints)
            .await?;

        if contact.trust_level != TrustLevel::Full
            && matches!(decision, AutonomyDecision::AutoApply)
        {
            decision = AutonomyDecision::Defer;
        }

        let mut auto_reply = None;
        let mut proposal_id = None;

        match decision {
            AutonomyDecision::AutoApply => {
                let mut reply =
                    RelayMessage::outbound(stored.channel_id, suggestion.content.clone())
                        .with_thread(thread_id)
                        .with_content_type(ContentType::Text);
                reply.recipient_contact_id = Some(sender_id);
                reply
                    .metadata
                    .insert("auto_reply".to_string(), serde_json::Value::Bool(true));
                reply.metadata.insert(
                    "basis_message_id".to_string(),
                    serde_json::Value::String(stored.id.to_string()),
                );
                if let Some(ref subject) = subject {
                    reply.metadata.insert(
                        "subject".to_string(),
                        serde_json::Value::String(subject.clone()),
                    );
                }

                let stored_reply = self.relay.send_message(reply, namespace).await?;
                auto_reply = Some(stored_reply);

                if let Ok(true) = self
                    .relay
                    .update_status(stored.id, MessageStatus::AutoReplied)
                    .await
                {
                    stored.status = MessageStatus::AutoReplied;
                }
            }
            AutonomyDecision::Defer | AutonomyDecision::QueueForLater => {
                let mut payload = std::collections::HashMap::new();
                payload.insert(
                    "channel_id".to_string(),
                    serde_json::Value::String(stored.channel_id.to_string()),
                );
                payload.insert(
                    "content".to_string(),
                    serde_json::Value::String(suggestion.content.clone()),
                );
                payload.insert(
                    "content_type".to_string(),
                    serde_json::Value::String(ContentType::Text.to_string()),
                );
                payload.insert(
                    "basis_message_id".to_string(),
                    serde_json::Value::String(stored.id.to_string()),
                );
                payload.insert(
                    "context_snippets".to_string(),
                    serde_json::Value::Array(
                        suggestion
                            .context_snippets
                            .iter()
                            .map(|snippet| serde_json::Value::String(snippet.clone()))
                            .collect(),
                    ),
                );
                payload.insert(
                    "thread_id".to_string(),
                    serde_json::Value::String(thread_id.to_string()),
                );
                if let Some(recipient_id) = stored.sender_contact_id {
                    payload.insert(
                        "recipient_contact_id".to_string(),
                        serde_json::Value::String(recipient_id.to_string()),
                    );
                }
                if let Some(ref subject) = subject {
                    payload.insert(
                        "subject".to_string(),
                        serde_json::Value::String(subject.clone()),
                    );
                }

                let proposal = Proposal::new(
                    ProposalSender::Relay,
                    ProposalAction::Custom("relay.reply".to_string()),
                )
                .with_confidence(suggestion.confidence)
                .with_diff(suggestion.content.clone())
                .with_payload(payload);

                self.submit_proposal(&proposal).await?;
                proposal_id = Some(proposal.id);

                if let Ok(true) = self
                    .relay
                    .update_status(stored.id, MessageStatus::Deferred)
                    .await
                {
                    stored.status = MessageStatus::Deferred;
                }
            }
            AutonomyDecision::Block => {}
        }

        Ok(RelayInboundOutcome {
            message: stored,
            auto_reply,
            proposal_id,
        })
    }

    /// Promote a retained relay message into canonical knowledge with provenance.
    ///
    /// Communication remains the source of truth for the raw message. Promotion
    /// creates a Conversation knowledge node and binds `vault_node_id`.
    pub async fn promote_relay_message(
        &self,
        request: RelayPromotionRequest,
    ) -> MvResult<KnowledgeNode> {
        let Some(message) = self.relay.get_message(request.message_id).await? else {
            return Err(MvError::NotFound(format!(
                "relay message {}",
                request.message_id
            )));
        };

        if message.status == MessageStatus::Failed {
            return Err(MvError::InvalidInput(
                "blocked or failed relay messages cannot be promoted".into(),
            ));
        }

        if let Some(existing) = message.vault_node_id {
            if let Some(node) = self.get_node(existing).await? {
                return Ok(node);
            }
        }

        if request.extractor.trim().is_empty()
            || request.actor.trim().is_empty()
            || request.evidence.trim().is_empty()
        {
            return Err(MvError::InvalidInput(
                "promotion requires extractor, actor, and evidence".into(),
            ));
        }

        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "promotion".to_string(),
            serde_json::json!({
                "source_message_id": message.id.to_string(),
                "extractor": request.extractor,
                "actor": request.actor,
                "evidence": request.evidence,
                "confidence": request.confidence.clamp(0.0, 1.0),
                "policy": request.policy,
                "approval": request.approval,
            }),
        );
        metadata.insert(
            "relay_channel_id".to_string(),
            serde_json::Value::String(message.channel_id.to_string()),
        );

        let node = KnowledgeNode::new(NodeKind::Conversation, message.content.clone())
            .with_namespace(request.namespace)
            .with_source(format!("relay:{}", message.id))
            .with_tags(vec![
                "relay".to_string(),
                "promoted".to_string(),
                format!("channel:{}", message.channel_id),
            ]);
        let mut node = node;
        node.metadata = metadata;
        node.importance = request.confidence.clamp(0.0, 1.0);

        let stored = self.ingest.ingest(node).await?;
        self.store
            .nodes
            .bind_relay_message_vault_node(message.id, Some(stored.id))
            .await?;
        Ok(stored)
    }

    /// Retract a promotion: remove the knowledge node and indexes without
    /// rewriting the source communication record.
    pub async fn retract_relay_promotion(&self, message_id: Uuid) -> MvResult<RelayMessage> {
        let Some(message) = self.relay.get_message(message_id).await? else {
            return Err(MvError::NotFound(format!("relay message {message_id}")));
        };

        let original_content = message.content.clone();
        let original_status = message.status;

        if let Some(node_id) = message.vault_node_id {
            let _ = self.ingest.delete(node_id).await?;
            self.store
                .nodes
                .bind_relay_message_vault_node(message_id, None)
                .await?;
        }

        let Some(retained) = self.relay.get_message(message_id).await? else {
            return Err(MvError::NotFound(format!("relay message {message_id}")));
        };

        if retained.content != original_content {
            return Err(MvError::Storage(
                "retraction must not rewrite source communication content".into(),
            ));
        }
        if retained.status != original_status {
            return Err(MvError::Storage(
                "retraction must not rewrite source communication status".into(),
            ));
        }
        if retained.vault_node_id.is_some() {
            return Err(MvError::Storage(
                "retraction left vault_node_id bound".into(),
            ));
        }

        Ok(retained)
    }
}
