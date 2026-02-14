use mv_core::*;

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
}
