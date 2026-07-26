use mv_core::*;

use chrono::Utc;

use super::{
    event_source, event_times, google_export_events, google_list_events,
    google_refresh_access_token, GoogleCalendarFetchError, GoogleCalendarSyncReport,
    MindVaultEngine,
};

impl MindVaultEngine {
    // ── Google Calendar Sync ──────────────────────────────────────────

    pub async fn sync_google_calendar(&self) -> MvResult<GoogleCalendarSyncReport> {
        let config = &self.config.google_calendar;
        if !config.enabled {
            return Err(MvError::InvalidInput(
                "google calendar sync is disabled".to_string(),
            ));
        }

        let client_id = config
            .client_id
            .as_ref()
            .ok_or_else(|| MvError::InvalidInput("google calendar client_id missing".into()))?;
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| MvError::InvalidInput("google calendar client_secret missing".into()))?;
        let refresh_token = config
            .refresh_token
            .as_ref()
            .ok_or_else(|| MvError::InvalidInput("google calendar refresh_token missing".into()))?;

        let calendar_id = config.calendar_id.trim();
        if calendar_id.is_empty() {
            return Err(MvError::InvalidInput(
                "google calendar_id must not be empty".into(),
            ));
        }

        let access_token =
            google_refresh_access_token(client_id, client_secret, refresh_token).await?;
        let adapter_name = format!("google-calendar:{calendar_id}");
        let existing_sync = self
            .store
            .nodes
            .get_poll_state(&adapter_name)
            .await?
            .and_then(|state| {
                let cursor = state.cursor.trim().to_string();
                if cursor.is_empty() {
                    None
                } else {
                    Some(cursor)
                }
            });

        let mut report = GoogleCalendarSyncReport {
            calendar_id: calendar_id.to_string(),
            fetched: 0,
            created: 0,
            updated: 0,
            deleted: 0,
            skipped: 0,
            exported_created: 0,
            exported_updated: 0,
            next_sync_token: None,
        };

        let mut sync_token = existing_sync;
        for attempt in 0..2 {
            match google_list_events(&access_token, calendar_id, config, sync_token.as_deref())
                .await
            {
                Ok((events, next_sync_token)) => {
                    report.fetched = events.len();
                    report.next_sync_token = next_sync_token.clone();
                    let mut created = 0usize;
                    let mut updated = 0usize;
                    let mut deleted = 0usize;
                    let mut skipped = 0usize;

                    if config.import_events {
                        for event in events {
                            let Some(event_id) = event.id.as_ref() else {
                                skipped += 1;
                                continue;
                            };

                            if matches!(event.status.as_deref(), Some("cancelled")) {
                                if let Some(existing) = self
                                    .store
                                    .nodes
                                    .find_by_source(&event_source(calendar_id, event_id))
                                    .await?
                                {
                                    let _ = self.delete_node(existing.id).await?;
                                    deleted += 1;
                                }
                                continue;
                            }

                            let (start_at, end_at) = match event_times(&event) {
                                Some(times) => times,
                                None => {
                                    skipped += 1;
                                    continue;
                                }
                            };

                            let source = event_source(calendar_id, event_id);
                            let existing = self.store.nodes.find_by_source(&source).await?;
                            let was_existing = existing.is_some();
                            let mut node = if let Some(existing) = existing {
                                existing
                            } else {
                                KnowledgeNode::new(NodeKind::Event, "")
                                    .with_namespace(config.namespace.clone())
                                    .with_tags(vec!["calendar".into(), "google-calendar".into()])
                                    .with_source(source)
                            };

                            let title = event
                                .summary
                                .clone()
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or_else(|| "Google Calendar Event".to_string());
                            let content = event
                                .description
                                .clone()
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or_else(|| title.clone());

                            node.title = Some(title);
                            node.content = content;
                            node.metadata.insert(
                                "event_start_at".to_string(),
                                serde_json::Value::String(start_at.to_rfc3339()),
                            );
                            node.metadata.insert(
                                "event_end_at".to_string(),
                                serde_json::Value::String(end_at.to_rfc3339()),
                            );
                            node.metadata.insert(
                                "google_calendar_event_id".to_string(),
                                serde_json::Value::String(event_id.clone()),
                            );
                            node.metadata.insert(
                                "google_calendar_calendar_id".to_string(),
                                serde_json::Value::String(calendar_id.to_string()),
                            );
                            if let Some(updated_at) = event.updated.clone() {
                                node.metadata.insert(
                                    "google_calendar_updated_at".to_string(),
                                    serde_json::Value::String(updated_at),
                                );
                            }
                            if let Some(html_link) = event.html_link.clone() {
                                node.metadata.insert(
                                    "google_calendar_html_link".to_string(),
                                    serde_json::Value::String(html_link),
                                );
                            }

                            if was_existing {
                                node.temporal.updated_at = Utc::now();
                                let _ = self.update_node(node).await?;
                                updated += 1;
                            } else {
                                let _ = self.store_node(node).await?;
                                created += 1;
                            }
                        }
                    } else {
                        skipped = events.len();
                    }

                    report.created = created;
                    report.updated = updated;
                    report.deleted = deleted;
                    report.skipped = skipped;

                    if config.export_events {
                        let (exported_created, exported_updated) =
                            google_export_events(self, &access_token, calendar_id, config).await?;
                        report.exported_created = exported_created;
                        report.exported_updated = exported_updated;
                    }

                    if let Some(next) = next_sync_token {
                        let _ = self
                            .store
                            .nodes
                            .upsert_poll_state(&adapter_name, &next, report.fetched as u64)
                            .await;
                    }

                    return Ok(report);
                }
                Err(GoogleCalendarFetchError::SyncTokenExpired) => {
                    if attempt == 0 {
                        sync_token = None;
                        continue;
                    }
                    return Err(MvError::InvalidInput(
                        "google calendar sync token expired".into(),
                    ));
                }
                Err(GoogleCalendarFetchError::RequestFailed(err)) => {
                    return Err(MvError::Storage(format!(
                        "google calendar sync failed: {err}"
                    )));
                }
            }
        }

        Err(MvError::Storage(
            "google calendar sync failed unexpectedly".into(),
        ))
    }
}
