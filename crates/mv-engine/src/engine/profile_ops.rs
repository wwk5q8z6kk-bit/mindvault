use chrono::{DateTime, NaiveDate, Utc};
use mv_core::*;
use uuid::Uuid;

use crate::daily_notes::{daily_note_day_tag, daily_note_weekday_tag, render_daily_note_template};
use crate::recurrence::{
    collect_due_occurrences, parse_optional_metadata_datetime,
    parse_optional_metadata_u64, parse_task_recurrence_rule, previous_due_at,
    RECURRING_DUE_AT_METADATA_KEY, RECURRING_INSTANCE_METADATA_KEY,
    RECURRING_PARENT_ID_METADATA_KEY, TASK_DUE_AT_METADATA_KEY,
    TASK_RECURRENCE_GENERATED_COUNT_METADATA_KEY, TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY,
    TASK_RECURRENCE_METADATA_KEY,
};

use super::{
    is_recurring_instance, MindVaultEngine, TaskRecurrenceRollforwardStats,
    DAILY_NOTE_TAG, PROFILE_OWNER_CONTACT_NOTES, PROFILE_RELAY_CONTACT_ID_KEY,
};

impl MindVaultEngine {
    // ── Owner Profile ────────────────────────────────────────────────

    pub async fn get_profile(&self) -> MvResult<OwnerProfile> {
        self.store.nodes.get_profile().await
    }

    pub async fn update_profile(&self, req: &UpdateProfileRequest) -> MvResult<OwnerProfile> {
        let profile = self.store.nodes.update_profile(req).await?;
        self.sync_owner_relay_contact(profile).await
    }

    pub(crate) async fn sync_owner_relay_contact(&self, profile: OwnerProfile) -> MvResult<OwnerProfile> {
        let display_name = profile.display_name.trim();
        let display_name = if display_name.is_empty() {
            "MindVault Owner"
        } else {
            display_name
        };

        let email = profile.email.as_ref().map(|value| value.trim().to_string());
        let email = email.filter(|value| !value.is_empty());
        let vault_address = email.map(|value| format!("mailto:{value}"));

        let signature_key = profile
            .signature_public_key
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let contact_id = profile
            .metadata
            .get(PROFILE_RELAY_CONTACT_ID_KEY)
            .and_then(|value| value.as_str())
            .and_then(|value| Uuid::parse_str(value).ok());

        if contact_id.is_none() && signature_key.is_none() {
            return Ok(profile);
        }

        if let Some(contact_id) = contact_id {
            if let Some(mut contact) = self.relay.get_contact(contact_id).await? {
                contact.display_name = display_name.to_string();
                if let Some(key) = signature_key {
                    contact.public_key = key;
                }
                contact.vault_address = vault_address;
                if contact.notes.is_none() {
                    contact.notes = Some(PROFILE_OWNER_CONTACT_NOTES.to_string());
                }

                let _ = self.relay.update_contact(&contact).await?;
                return Ok(profile);
            }
        }

        let Some(signature_key) = signature_key else {
            return Ok(profile);
        };

        let mut contact =
            RelayContact::new(display_name, signature_key).with_trust(TrustLevel::Full);
        contact.vault_address = vault_address;
        contact.notes = Some(PROFILE_OWNER_CONTACT_NOTES.to_string());

        self.relay.add_contact(&contact).await?;

        let mut metadata = profile.metadata.clone();
        metadata.insert(
            PROFILE_RELAY_CONTACT_ID_KEY.to_string(),
            serde_json::Value::String(contact.id.to_string()),
        );

        let updated = self
            .store
            .nodes
            .update_profile(&UpdateProfileRequest {
                metadata: Some(metadata),
                ..Default::default()
            })
            .await?;

        Ok(updated)
    }

    // ── Daily Notes ──────────────────────────────────────────────────

    /// Return the daily note for a specific day/namespace if present.
    pub async fn find_daily_note(
        &self,
        date: NaiveDate,
        namespace: &str,
    ) -> MvResult<Option<KnowledgeNode>> {
        self.ensure_unsealed_for_node_io().await?;
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            tags: Some(vec![daily_note_day_tag(date)]),
            ..Default::default()
        };
        let mut existing = self.store.nodes.list(&filters, 1, 0).await?;
        Ok(existing.pop())
    }

    /// Ensure a daily note exists for a given date and namespace.
    /// Returns `(node, created)` where `created` is true only on first creation.
    pub async fn ensure_daily_note(
        &self,
        date: NaiveDate,
        namespace: Option<String>,
    ) -> MvResult<(KnowledgeNode, bool)> {
        if !self.config.daily_notes.enabled {
            return Err(MvError::InvalidInput(
                "daily notes are disabled".to_string(),
            ));
        }

        let daily_namespace =
            namespace.unwrap_or_else(|| self.config.daily_notes.namespace.clone());
        if let Some(existing) = self.find_daily_note(date, &daily_namespace).await? {
            return Ok((existing, false));
        }

        let mut node = KnowledgeNode::new(
            NodeKind::Fact,
            render_daily_note_template(&self.config.daily_notes.content_template, date),
        )
        .with_namespace(daily_namespace)
        .with_tags(vec![
            DAILY_NOTE_TAG.to_string(),
            daily_note_day_tag(date),
            daily_note_weekday_tag(date),
        ])
        .with_importance(self.config.daily_notes.default_importance);

        let title = render_daily_note_template(&self.config.daily_notes.title_template, date);
        if !title.trim().is_empty() {
            node = node.with_title(title);
        }

        node.metadata
            .insert("daily_note".to_string(), serde_json::Value::Bool(true));
        node.metadata.insert(
            "daily_note_date".to_string(),
            serde_json::Value::String(date.to_string()),
        );

        let stored = self.ingest.ingest(node).await?;
        Ok((stored, true))
    }

    /// List daily notes by namespace (or all namespaces when None).
    pub async fn list_daily_notes(
        &self,
        namespace: Option<String>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>> {
        let filters = QueryFilters {
            namespace,
            tags: Some(vec![DAILY_NOTE_TAG.to_string()]),
            ..Default::default()
        };
        self.store.nodes.list(&filters, limit, offset).await
    }

    // ── Recurring Tasks ──────────────────────────────────────────────

    /// Generate due instances for recurring task templates.
    pub async fn rollforward_recurring_tasks(
        &self,
        now: DateTime<Utc>,
        max_instances_per_template: usize,
    ) -> MvResult<TaskRecurrenceRollforwardStats> {
        if !self.config.recurrence.enabled {
            return Ok(TaskRecurrenceRollforwardStats::default());
        }

        let mut stats = TaskRecurrenceRollforwardStats::default();
        let page_size = 200;
        let mut offset = 0usize;

        loop {
            let filters = QueryFilters {
                kinds: Some(vec![NodeKind::Task]),
                ..Default::default()
            };
            let page = self.store.nodes.list(&filters, page_size, offset).await?;
            if page.is_empty() {
                break;
            }
            let page_len = page.len();

            for mut template in page {
                stats.scanned_tasks += 1;
                if is_recurring_instance(&template) {
                    continue;
                }

                let recurrence_rule = match parse_task_recurrence_rule(&template.metadata) {
                    Ok(Some(rule)) if rule.enabled => rule,
                    Ok(Some(_)) | Ok(None) => continue,
                    Err(err) => {
                        stats.errors += 1;
                        tracing::warn!(
                            node_id = %template.id,
                            namespace = %template.namespace,
                            error = %err,
                            "mindvault_recurrence_rule_parse_failed"
                        );
                        continue;
                    }
                };
                stats.recurring_templates += 1;

                let explicit_due_at = match parse_optional_metadata_datetime(
                    &template.metadata,
                    TASK_DUE_AT_METADATA_KEY,
                ) {
                    Ok(value) => value,
                    Err(err) => {
                        stats.errors += 1;
                        tracing::warn!(
                            node_id = %template.id,
                            namespace = %template.namespace,
                            error = %err,
                            "mindvault_recurrence_due_at_parse_failed"
                        );
                        continue;
                    }
                };
                let last_generated = match parse_optional_metadata_datetime(
                    &template.metadata,
                    TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY,
                ) {
                    Ok(Some(value)) => value,
                    Ok(None) => explicit_due_at
                        .map(|due| previous_due_at(due, &recurrence_rule))
                        .unwrap_or(template.temporal.created_at),
                    Err(err) => {
                        stats.errors += 1;
                        tracing::warn!(
                            node_id = %template.id,
                            namespace = %template.namespace,
                            error = %err,
                            "mindvault_recurrence_last_generated_parse_failed"
                        );
                        continue;
                    }
                };
                let generated_count = parse_optional_metadata_u64(
                    &template.metadata,
                    TASK_RECURRENCE_GENERATED_COUNT_METADATA_KEY,
                )
                .unwrap_or(0);
                let due_dates = collect_due_occurrences(
                    &recurrence_rule,
                    last_generated,
                    now,
                    max_instances_per_template,
                    generated_count,
                );
                if due_dates.is_empty() {
                    continue;
                }

                let mut created_for_template = 0usize;
                let mut latest_due = last_generated;
                for due_at in due_dates {
                    let mut instance = KnowledgeNode::new(NodeKind::Task, template.content.clone())
                        .with_namespace(template.namespace.clone())
                        .with_importance(template.importance);
                    if let Some(title) = template.title.as_deref() {
                        instance = instance.with_title(title);
                    }
                    if let Some(source) = template.source.as_deref() {
                        instance = instance.with_source(source);
                    }

                    let mut tags = template.tags.clone();
                    if !tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case("recurring-instance"))
                    {
                        tags.push("recurring-instance".to_string());
                    }
                    instance = instance.with_tags(tags);

                    for (key, value) in &template.metadata {
                        if matches!(
                            key.as_str(),
                            TASK_RECURRENCE_METADATA_KEY
                                | TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY
                                | TASK_RECURRENCE_GENERATED_COUNT_METADATA_KEY
                                | RECURRING_INSTANCE_METADATA_KEY
                                | RECURRING_PARENT_ID_METADATA_KEY
                                | RECURRING_DUE_AT_METADATA_KEY
                        ) {
                            continue;
                        }
                        instance.metadata.insert(key.clone(), value.clone());
                    }
                    instance.metadata.insert(
                        RECURRING_INSTANCE_METADATA_KEY.into(),
                        serde_json::Value::Bool(true),
                    );
                    instance.metadata.insert(
                        RECURRING_PARENT_ID_METADATA_KEY.into(),
                        serde_json::Value::String(template.id.to_string()),
                    );
                    instance.metadata.insert(
                        RECURRING_DUE_AT_METADATA_KEY.into(),
                        serde_json::Value::String(due_at.to_rfc3339()),
                    );
                    instance.metadata.insert(
                        TASK_DUE_AT_METADATA_KEY.into(),
                        serde_json::Value::String(due_at.to_rfc3339()),
                    );

                    match self.store_node(instance).await {
                        Ok(stored_instance) => {
                            created_for_template += 1;
                            latest_due = due_at;
                            stats.generated_instances += 1;

                            let rel = Relationship::new(
                                template.id,
                                stored_instance.id,
                                RelationKind::DerivedFrom,
                            );
                            if let Err(err) = self.graph.add_relationship(&rel).await {
                                stats.errors += 1;
                                tracing::warn!(
                                    template_id = %template.id,
                                    instance_id = %stored_instance.id,
                                    error = %err,
                                    "mindvault_recurrence_parent_instance_link_failed"
                                );
                            }
                        }
                        Err(err) => {
                            stats.errors += 1;
                            tracing::warn!(
                                node_id = %template.id,
                                namespace = %template.namespace,
                                error = %err,
                                "mindvault_recurrence_instance_create_failed"
                            );
                        }
                    }
                }

                if created_for_template > 0 {
                    template.metadata.insert(
                        TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY.into(),
                        serde_json::Value::String(latest_due.to_rfc3339()),
                    );
                    let total_generated_count =
                        generated_count.saturating_add(created_for_template as u64);
                    template.metadata.insert(
                        TASK_RECURRENCE_GENERATED_COUNT_METADATA_KEY.into(),
                        serde_json::Value::Number(serde_json::Number::from(total_generated_count)),
                    );
                    template.temporal.updated_at = now;
                    template.temporal.version = template.temporal.version.saturating_add(1);
                    if let Err(err) = self.ingest.update(template).await {
                        stats.errors += 1;
                        tracing::warn!(
                            error = %err,
                            "mindvault_recurrence_template_update_failed"
                        );
                    } else {
                        stats.updated_templates += 1;
                    }
                }
            }

            if page_size > 0 && page_len < page_size {
                break;
            }
            offset = offset.saturating_add(page_size);
        }

        Ok(stats)
    }
}
