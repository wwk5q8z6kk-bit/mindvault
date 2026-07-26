use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use mv_core::*;

use crate::recurrence::{
    parse_optional_metadata_bool, parse_optional_metadata_datetime, TASK_COMPLETED_METADATA_KEY,
    TASK_DUE_AT_METADATA_KEY, TASK_REMINDER_SENT_AT_METADATA_KEY,
    TASK_REMINDER_STATUS_METADATA_KEY,
};

use super::{
    MindVaultEngine, PrioritizedTask, TaskPrioritizationOptions, TaskPriorityCandidate,
    TaskReminderDispatchStats, TASK_AI_PRIORITY_METADATA_KEY,
    TASK_ESTIMATE_MINUTES_ALT_METADATA_KEY, TASK_ESTIMATE_MINUTES_METADATA_KEY,
    TASK_ESTIMATE_MIN_METADATA_KEY, TASK_PRIORITY_ALT_METADATA_KEY, TASK_PRIORITY_METADATA_KEY,
    TASK_STATUS_ALT_METADATA_KEY, TASK_STATUS_METADATA_KEY,
};

impl MindVaultEngine {
    // ── Task Intelligence (Due Tasks, Prioritization, Reminders) ────

    /// List due tasks up to `due_before`.
    pub async fn list_due_tasks(
        &self,
        due_before: DateTime<Utc>,
        namespace: Option<String>,
        limit: usize,
        include_completed: bool,
    ) -> MvResult<Vec<KnowledgeNode>> {
        let capped_limit = limit.clamp(1, 1000);
        let page_size = 250;
        let mut offset = 0usize;
        let mut due = Vec::<(DateTime<Utc>, KnowledgeNode)>::new();

        loop {
            let filters = QueryFilters {
                namespace: namespace.clone(),
                kinds: Some(vec![NodeKind::Task]),
                ..Default::default()
            };
            let page = self.store.nodes.list(&filters, page_size, offset).await?;
            if page.is_empty() {
                break;
            }
            let page_len = page.len();

            for node in page {
                let due_at = match parse_optional_metadata_datetime(
                    &node.metadata,
                    TASK_DUE_AT_METADATA_KEY,
                ) {
                    Ok(Some(value)) => value,
                    Ok(None) => continue,
                    Err(err) => {
                        tracing::warn!(
                            node_id = %node.id,
                            namespace = %node.namespace,
                            error = %err,
                            "mindvault_due_task_parse_failed"
                        );
                        continue;
                    }
                };

                if due_at > due_before {
                    continue;
                }

                let is_completed =
                    parse_optional_metadata_bool(&node.metadata, TASK_COMPLETED_METADATA_KEY)
                        .unwrap_or(false);
                if !include_completed && is_completed {
                    continue;
                }

                due.push((due_at, node));
            }

            if page_size > 0 && page_len < page_size {
                break;
            }
            offset = offset.saturating_add(page_size);
        }

        due.sort_by(|(left_due, left_node), (right_due, right_node)| {
            left_due
                .cmp(right_due)
                .then_with(|| left_node.id.cmp(&right_node.id))
        });

        Ok(due
            .into_iter()
            .take(capped_limit)
            .map(|(_due_at, node)| node)
            .collect())
    }

    /// Prioritize tasks using deterministic heuristic scoring.
    pub async fn prioritize_tasks(
        &self,
        options: TaskPrioritizationOptions,
    ) -> MvResult<Vec<PrioritizedTask>> {
        let limit = options.limit.clamp(1, 200);
        let page_size = 250;
        let mut offset = 0usize;
        let mut candidates: Vec<TaskPriorityCandidate> = Vec::new();

        loop {
            let filters = QueryFilters {
                namespace: options.namespace.clone(),
                kinds: Some(vec![NodeKind::Task]),
                ..Default::default()
            };
            let page = self.store.nodes.list(&filters, page_size, offset).await?;
            if page.is_empty() {
                break;
            }
            let page_len = page.len();

            for node in page {
                let completed =
                    parse_optional_metadata_bool(&node.metadata, TASK_COMPLETED_METADATA_KEY)
                        .unwrap_or(false);
                if !options.include_completed && completed {
                    continue;
                }

                let due_at = match parse_optional_metadata_datetime(
                    &node.metadata,
                    TASK_DUE_AT_METADATA_KEY,
                ) {
                    Ok(value) => value,
                    Err(err) => {
                        tracing::warn!(
                            node_id = %node.id,
                            namespace = %node.namespace,
                            error = %err,
                            "mindvault_task_priority_due_at_parse_failed"
                        );
                        continue;
                    }
                };

                if due_at.is_none() && !options.include_without_due {
                    continue;
                }

                let (score, reason) = Self::score_task(&node, due_at, options.now);
                candidates.push(TaskPriorityCandidate {
                    task: node,
                    score,
                    reason,
                    due_at,
                });
            }

            if page_size > 0 && page_len < page_size {
                break;
            }
            offset = offset.saturating_add(page_size);
        }

        candidates.sort_by(|left, right| {
            let score_order = right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal);
            if score_order != Ordering::Equal {
                return score_order;
            }

            let due_order = match (left.due_at, right.due_at) {
                (Some(left_due), Some(right_due)) => left_due.cmp(&right_due),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            if due_order != Ordering::Equal {
                return due_order;
            }

            left.task.id.cmp(&right.task.id)
        });

        let mut prioritized = Vec::new();
        for (idx, candidate) in candidates.into_iter().take(limit).enumerate() {
            let rank = idx + 1;
            let mut task = candidate.task;
            let reason = candidate.reason;
            let score = candidate.score;

            if options.persist {
                task.metadata.insert(
                    TASK_AI_PRIORITY_METADATA_KEY.into(),
                    serde_json::json!({
                        "score": score,
                        "rank": rank,
                        "reason": reason,
                        "generated_at": options.now.to_rfc3339(),
                        "algorithm": "heuristic_v1",
                    }),
                );
                task.temporal.updated_at = options.now;
                task.temporal.version = task.temporal.version.saturating_add(1);
                task = self.ingest.update(task).await?;
            }

            prioritized.push(PrioritizedTask {
                task,
                score,
                rank,
                reason,
            });
        }

        Ok(prioritized)
    }

    pub(crate) fn score_task(
        task: &KnowledgeNode,
        due_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> (f64, String) {
        let priority_override = Self::parse_optional_metadata_f64(
            &task.metadata,
            TASK_PRIORITY_METADATA_KEY,
        )
        .or_else(|| {
            Self::parse_optional_metadata_f64(&task.metadata, TASK_PRIORITY_ALT_METADATA_KEY)
        });
        let priority_score = if let Some(priority_raw) = priority_override {
            let priority = priority_raw.round().clamp(1.0, 5.0);
            (6.0 - priority) / 5.0
        } else {
            task.importance.clamp(0.0, 1.0)
        };

        let mut due_score = 0.0;
        if let Some(due_at) = due_at {
            let hours = (due_at - now).num_seconds() as f64 / 3600.0;
            if hours <= 0.0 {
                due_score = 1.0;
            } else {
                let days = hours / 24.0;
                due_score = (1.0 - (days / 7.0).min(1.0)).max(0.0);
            }
        }

        let status = task
            .metadata
            .get(TASK_STATUS_METADATA_KEY)
            .or_else(|| task.metadata.get(TASK_STATUS_ALT_METADATA_KEY))
            .and_then(|value| value.as_str())
            .map(|value| value.to_ascii_lowercase());
        let status_score = match status.as_deref() {
            Some("in_progress") => 0.2,
            Some("planned") => 0.12,
            Some("review") => 0.1,
            Some("inbox") => 0.05,
            Some("waiting") => -0.05,
            Some("blocked") => -0.1,
            _ => 0.0,
        };

        let estimate =
            Self::parse_optional_metadata_f64(&task.metadata, TASK_ESTIMATE_MINUTES_METADATA_KEY)
                .or_else(|| {
                    Self::parse_optional_metadata_f64(
                        &task.metadata,
                        TASK_ESTIMATE_MINUTES_ALT_METADATA_KEY,
                    )
                })
                .or_else(|| {
                    Self::parse_optional_metadata_f64(
                        &task.metadata,
                        TASK_ESTIMATE_MIN_METADATA_KEY,
                    )
                });
        let estimate_score = match estimate {
            Some(minutes) => (1.0 - (minutes / 240.0).min(1.0)).max(0.0),
            None => 0.05,
        };

        let completed = parse_optional_metadata_bool(&task.metadata, TASK_COMPLETED_METADATA_KEY)
            .unwrap_or(false);
        let completion_penalty = if completed { -0.4 } else { 0.0 };

        let score = 0.45 * priority_score
            + 0.35 * due_score
            + 0.1 * status_score
            + 0.1 * estimate_score
            + completion_penalty;

        let mut reasons: Vec<String> = Vec::new();
        if let Some(priority_raw) = priority_override {
            let priority = priority_raw.round().clamp(1.0, 5.0) as i64;
            if priority <= 2 {
                reasons.push(format!("High priority (P{priority})"));
            } else if priority >= 4 {
                reasons.push(format!("Lower priority (P{priority})"));
            }
        } else if priority_score >= 0.8 {
            reasons.push("High importance".to_string());
        } else if priority_score <= 0.3 {
            reasons.push("Lower importance".to_string());
        }

        if let Some(due_at) = due_at {
            let delta = due_at - now;
            if delta.num_seconds() <= 0 {
                reasons.push("Overdue".to_string());
            } else {
                let days = delta.num_seconds() as f64 / 86_400.0;
                if days <= 1.0 {
                    reasons.push("Due within 24h".to_string());
                } else if days <= 3.0 {
                    reasons.push("Due soon".to_string());
                } else if days <= 7.0 {
                    reasons.push("Due this week".to_string());
                }
            }
        }

        match status.as_deref() {
            Some("in_progress") => reasons.push("In progress".to_string()),
            Some("planned") => reasons.push("Planned".to_string()),
            Some("waiting") => reasons.push("Waiting".to_string()),
            Some("review") => reasons.push("In review".to_string()),
            Some("blocked") => reasons.push("Blocked".to_string()),
            _ => {}
        }

        if let Some(minutes) = estimate {
            if minutes <= 30.0 {
                reasons.push("Quick win".to_string());
            }
        }

        if completed {
            reasons.push("Completed".to_string());
        }

        let reason = if reasons.is_empty() {
            "Balanced priority".to_string()
        } else {
            reasons.into_iter().take(3).collect::<Vec<_>>().join(", ")
        };

        (score, reason)
    }

    pub(crate) fn parse_optional_metadata_f64(
        metadata: &std::collections::HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Option<f64> {
        match metadata.get(key) {
            Some(serde_json::Value::Number(value)) => value.as_f64(),
            Some(serde_json::Value::String(value)) => value.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Mark due task reminders as sent by setting metadata fields on each task.
    pub async fn dispatch_due_task_reminders(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> MvResult<TaskReminderDispatchStats> {
        let due_tasks = self.list_due_tasks(now, None, limit, false).await?;
        let mut stats = TaskReminderDispatchStats {
            scanned_tasks: due_tasks.len(),
            due_tasks: due_tasks.len(),
            ..Default::default()
        };

        for mut task in due_tasks {
            let already_sent = match parse_optional_metadata_datetime(
                &task.metadata,
                TASK_REMINDER_SENT_AT_METADATA_KEY,
            ) {
                Ok(value) => value.is_some(),
                Err(err) => {
                    stats.errors += 1;
                    tracing::warn!(
                        node_id = %task.id,
                        namespace = %task.namespace,
                        error = %err,
                        "mindvault_task_reminder_sent_at_parse_failed"
                    );
                    continue;
                }
            };
            if already_sent {
                continue;
            }

            task.metadata.insert(
                TASK_REMINDER_STATUS_METADATA_KEY.into(),
                serde_json::Value::String("sent".to_string()),
            );
            task.metadata.insert(
                TASK_REMINDER_SENT_AT_METADATA_KEY.into(),
                serde_json::Value::String(now.to_rfc3339()),
            );
            task.temporal.updated_at = now;
            task.temporal.version = task.temporal.version.saturating_add(1);

            match self.ingest.update(task).await {
                Ok(_updated) => {
                    stats.reminders_marked_sent += 1;
                }
                Err(err) => {
                    stats.errors += 1;
                    tracing::warn!(
                        error = %err,
                        "mindvault_task_reminder_mark_sent_failed"
                    );
                }
            }
        }

        Ok(stats)
    }
}
