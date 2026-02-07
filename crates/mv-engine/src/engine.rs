use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::Arc;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, NaiveDate, Utc};
use mv_core::*;
use mv_graph::store::SqliteGraphStore;
use mv_index::tantivy_index::TantivyFullTextIndex;
use mv_storage::unified::UnifiedStore;
use mv_storage::vector::{KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder, OpenAiEmbedder};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::backlinks::{
    extract_reference_targets_with_kind, resolve_reference_targets_with_kind,
    ContentReferenceSourceKind, KnowledgeVaultBacklinkResolutionIndex,
    ResolvedContentReferenceTarget,
};
use crate::config::EngineConfig;
use crate::daily_notes::{daily_note_day_tag, daily_note_weekday_tag, render_daily_note_template};
use crate::ingest::IngestPipeline;
use crate::recall::RecallPipeline;
use crate::recurrence::{
    collect_due_occurrences, parse_optional_metadata_bool, parse_optional_metadata_datetime,
    parse_optional_metadata_u64, parse_task_recurrence_rule, previous_due_at,
    RECURRING_DUE_AT_METADATA_KEY, RECURRING_INSTANCE_METADATA_KEY,
    RECURRING_PARENT_ID_METADATA_KEY, TASK_COMPLETED_METADATA_KEY, TASK_DUE_AT_METADATA_KEY,
    TASK_RECURRENCE_GENERATED_COUNT_METADATA_KEY, TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY,
    TASK_RECURRENCE_METADATA_KEY, TASK_REMINDER_SENT_AT_METADATA_KEY,
    TASK_REMINDER_STATUS_METADATA_KEY,
};

const DAILY_NOTE_TAG: &str = "daily-note";
const DAILY_LINK_CANDIDATE_TAGS: &[&str] = &[
    "task", "tasks", "todo", "to-do", "event", "events", "meeting", "reminder",
];
const AUTO_BACKLINK_METADATA_KEY: &str = "auto_backlink";
const AUTO_BACKLINK_SOURCE_METADATA_KEY: &str = "source";
const TASK_AI_PRIORITY_METADATA_KEY: &str = "ai_priority";
const TASK_PRIORITY_METADATA_KEY: &str = "task_priority";
const TASK_PRIORITY_ALT_METADATA_KEY: &str = "priority";
const TASK_STATUS_METADATA_KEY: &str = "task_status";
const TASK_STATUS_ALT_METADATA_KEY: &str = "status";
const TASK_ESTIMATE_MINUTES_METADATA_KEY: &str = "task_estimate_minutes";
const TASK_ESTIMATE_MINUTES_ALT_METADATA_KEY: &str = "task_estimate_min";
const TASK_ESTIMATE_MIN_METADATA_KEY: &str = "estimate_min";

#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus {
    pub configured_provider: String,
    pub configured_model: String,
    pub configured_dimensions: usize,
    pub effective_provider: String,
    pub effective_model: String,
    pub effective_dimensions: usize,
    pub fallback_to_noop: bool,
    pub reason: Option<String>,
    pub local_embeddings_feature_enabled: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TaskRecurrenceRollforwardStats {
    pub scanned_tasks: usize,
    pub recurring_templates: usize,
    pub generated_instances: usize,
    pub updated_templates: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TaskReminderDispatchStats {
    pub scanned_tasks: usize,
    pub due_tasks: usize,
    pub reminders_marked_sent: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrioritizedTask {
    pub task: KnowledgeNode,
    pub score: f64,
    pub rank: usize,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct TaskPrioritizationOptions {
    pub namespace: Option<String>,
    pub limit: usize,
    pub include_completed: bool,
    pub include_without_due: bool,
    pub persist: bool,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct TaskPriorityCandidate {
    task: KnowledgeNode,
    score: f64,
    reason: String,
    due_at: Option<DateTime<Utc>>,
}

struct EmbeddingProviderSelection {
    embedder: Option<Arc<dyn Embedder>>,
    vector_dimensions: usize,
    runtime_status: KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus,
}

/// The MindVault engine — orchestrates storage, indexing, and search.
pub struct MindVaultEngine {
    pub ingest: IngestPipeline,
    pub recall: RecallPipeline,
    pub store: Arc<UnifiedStore>,
    pub fts: Arc<TantivyFullTextIndex>,
    pub graph: Arc<SqliteGraphStore>,
    pub config: EngineConfig,
    embedding_runtime_status: KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus,
}

impl MindVaultEngine {
    /// Initialize the engine from configuration.
    pub async fn init(config: EngineConfig) -> MvResult<Self> {
        let data_dir = PathBuf::from(&config.data_dir);
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| MvError::Storage(format!("create data dir: {e}")))?;

        let selection = select_embedding_provider(&config);

        // Initialize unified store
        let mut store = UnifiedStore::open(&data_dir, selection.vector_dimensions).await?;
        if let Some(embedder) = selection.embedder {
            store = store.with_embedder(embedder);
        }

        let store = Arc::new(store);

        // Initialize Tantivy FTS
        let tantivy_path = data_dir.join("tantivy");
        let fts = Arc::new(TantivyFullTextIndex::open(&tantivy_path)?);

        // Initialize graph store (shares SQLite connection via separate connection)
        let graph_conn = rusqlite::Connection::open(data_dir.join("mindvault.sqlite"))
            .map_err(|e| MvError::Graph(format!("open graph db: {e}")))?;
        graph_conn
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
            )
            .map_err(|e| MvError::Graph(format!("graph pragma: {e}")))?;
        let graph = Arc::new(SqliteGraphStore::open(graph_conn)?);

        let ingest = IngestPipeline::new(
            Arc::clone(&store),
            Arc::clone(&fts),
            Arc::clone(&graph),
            config.clone(),
        );

        let recall = RecallPipeline::new(
            Arc::clone(&store),
            Arc::clone(&fts),
            Arc::clone(&graph),
            config.clone(),
        );

        let engine = Self {
            ingest,
            recall,
            store,
            fts,
            graph,
            config,
            embedding_runtime_status: selection.runtime_status,
        };

        engine.ensure_default_permission_templates().await?;

        Ok(engine)
    }

    async fn ensure_default_permission_templates(&self) -> MvResult<()> {
        let owner_exists = self
            .store
            .nodes
            .get_permission_template_by_name("Owner")
            .await?
            .is_some();
        let assistant_exists = self
            .store
            .nodes
            .get_permission_template_by_name("Assistant")
            .await?
            .is_some();

        if !owner_exists {
            let now = Utc::now();
            let owner = PermissionTemplate {
                id: Uuid::now_v7(),
                name: "Owner".into(),
                description: Some("Full access template".into()),
                tier: PermissionTier::Admin,
                scope_namespace: None,
                scope_tags: Vec::new(),
                allow_kinds: Vec::new(),
                allow_actions: Vec::new(),
                created_at: now,
                updated_at: now,
            };
            self.store.nodes.insert_permission_template(&owner).await?;
        }

        if !assistant_exists {
            let now = Utc::now();
            let assistant = PermissionTemplate {
                id: Uuid::now_v7(),
                name: "Assistant".into(),
                description: Some("Scoped assistant template".into()),
                tier: PermissionTier::Action,
                scope_namespace: Some("assistant".into()),
                scope_tags: Vec::new(),
                allow_kinds: Vec::new(),
                allow_actions: Vec::new(),
                created_at: now,
                updated_at: now,
            };
            self.store
                .nodes
                .insert_permission_template(&assistant)
                .await?;
        }

        Ok(())
    }

    pub async fn list_permission_templates(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<PermissionTemplate>> {
        self.store
            .nodes
            .list_permission_templates(limit, offset)
            .await
    }

    pub async fn create_permission_template(
        &self,
        name: String,
        description: Option<String>,
        tier: PermissionTier,
        scope_namespace: Option<String>,
        scope_tags: Vec<String>,
        allow_kinds: Vec<NodeKind>,
        allow_actions: Vec<String>,
    ) -> MvResult<PermissionTemplate> {
        let now = Utc::now();
        let template = PermissionTemplate {
            id: Uuid::now_v7(),
            name,
            description,
            tier,
            scope_namespace,
            scope_tags,
            allow_kinds,
            allow_actions,
            created_at: now,
            updated_at: now,
        };
        self.store
            .nodes
            .insert_permission_template(&template)
            .await?;
        Ok(template)
    }

    pub async fn update_permission_template(
        &self,
        template_id: Uuid,
        name: String,
        description: Option<String>,
        tier: PermissionTier,
        scope_namespace: Option<String>,
        scope_tags: Vec<String>,
        allow_kinds: Vec<NodeKind>,
        allow_actions: Vec<String>,
    ) -> MvResult<Option<PermissionTemplate>> {
        let mut existing = match self
            .store
            .nodes
            .get_permission_template(template_id)
            .await?
        {
            Some(template) => template,
            None => return Ok(None),
        };

        existing.name = name;
        existing.description = description;
        existing.tier = tier;
        existing.scope_namespace = scope_namespace;
        existing.scope_tags = scope_tags;
        existing.allow_kinds = allow_kinds;
        existing.allow_actions = allow_actions;
        existing.updated_at = Utc::now();

        self.store
            .nodes
            .update_permission_template(&existing)
            .await?;
        Ok(Some(existing))
    }

    pub async fn delete_permission_template(&self, template_id: Uuid) -> MvResult<bool> {
        self.store
            .nodes
            .delete_permission_template(template_id)
            .await
    }

    pub async fn create_access_key(
        &self,
        template_id: Uuid,
        name: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> MvResult<(AccessKey, String)> {
        let template = self
            .store
            .nodes
            .get_permission_template(template_id)
            .await?
            .ok_or_else(|| MvError::InvalidInput("permission template not found".into()))?;

        let token = generate_access_token();
        let key_hash = hash_access_token(&token);
        let now = Utc::now();

        let access_key = AccessKey {
            id: Uuid::now_v7(),
            name,
            template_id: template.id,
            key_hash,
            created_at: now,
            last_used_at: None,
            expires_at,
            revoked_at: None,
        };

        self.store.nodes.insert_access_key(&access_key).await?;

        Ok((access_key, token))
    }

    pub async fn list_access_keys(&self) -> MvResult<Vec<AccessKey>> {
        self.store.nodes.list_access_keys().await
    }

    pub async fn revoke_access_key(&self, key_id: Uuid) -> MvResult<bool> {
        self.store.nodes.revoke_access_key(key_id, Utc::now()).await
    }

    pub async fn resolve_access_key(
        &self,
        token: &str,
    ) -> MvResult<Option<(AccessKey, PermissionTemplate)>> {
        let key_hash = hash_access_token(token);
        let key = match self.store.nodes.get_access_key_by_hash(&key_hash).await? {
            Some(key) => key,
            None => return Ok(None),
        };

        if key.revoked_at.is_some() {
            return Ok(None);
        }

        if let Some(expires_at) = key.expires_at {
            if expires_at < Utc::now() {
                return Ok(None);
            }
        }

        let template = self
            .store
            .nodes
            .get_permission_template(key.template_id)
            .await?
            .ok_or_else(|| MvError::InvalidInput("permission template missing".into()))?;

        let _ = self
            .store
            .nodes
            .update_access_key_last_used(key.id, Utc::now())
            .await;

        Ok(Some((key, template)))
    }

    /// Store a knowledge node.
    pub async fn store_node(&self, node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        let stored = self.ingest.ingest(node).await?;
        self.auto_link_node_to_daily_note_best_effort(&stored).await;
        self.auto_backlink_node_references_best_effort(&stored)
            .await;
        Ok(stored)
    }

    /// Store a node with relationships.
    pub async fn store_with_relations(
        &self,
        node: KnowledgeNode,
        relations: Vec<Relationship>,
    ) -> MvResult<KnowledgeNode> {
        self.ingest.ingest_with_relations(node, relations).await
    }

    /// Recall knowledge matching a query.
    pub async fn recall(&self, query: &MemoryQuery) -> MvResult<Vec<SearchResult>> {
        self.recall.recall(query).await
    }

    /// Get a node by ID.
    pub async fn get_node(&self, id: uuid::Uuid) -> MvResult<Option<KnowledgeNode>> {
        self.store.nodes.get(id).await
    }

    /// Update an existing node.
    pub async fn update_node(&self, node: KnowledgeNode) -> MvResult<KnowledgeNode> {
        let updated = self.ingest.update(node).await?;
        self.auto_link_node_to_daily_note_best_effort(&updated)
            .await;
        self.auto_backlink_node_references_best_effort(&updated)
            .await;
        Ok(updated)
    }

    /// Delete a node.
    pub async fn delete_node(&self, id: uuid::Uuid) -> MvResult<bool> {
        self.ingest.delete(id).await
    }

    /// List nodes with filters.
    pub async fn list_nodes(
        &self,
        filters: &QueryFilters,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>> {
        self.store.nodes.list(filters, limit, offset).await
    }

    /// Return the daily note for a specific day/namespace if present.
    pub async fn find_daily_note(
        &self,
        date: NaiveDate,
        namespace: &str,
    ) -> MvResult<Option<KnowledgeNode>> {
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
            return Err(MvError::InvalidInput("daily notes are disabled".into()));
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
            .insert("daily_note".into(), serde_json::Value::Bool(true));
        node.metadata.insert(
            "daily_note_date".into(),
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

    fn score_task(
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

    fn parse_optional_metadata_f64(
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

    /// Add a relationship between nodes.
    pub async fn add_relationship(&self, rel: Relationship) -> MvResult<()> {
        self.graph.add_relationship(&rel).await
    }

    /// Get graph neighbors.
    pub async fn get_neighbors(
        &self,
        node_id: uuid::Uuid,
        depth: usize,
    ) -> MvResult<Vec<uuid::Uuid>> {
        self.graph.get_neighbors(node_id, depth).await
    }

    /// Get node count.
    pub async fn node_count(&self) -> MvResult<usize> {
        self.store.nodes.count(&QueryFilters::default()).await
    }

    /// Return the active embedding provider diagnostics for observability.
    pub fn embedding_runtime_status(
        &self,
    ) -> KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus {
        self.embedding_runtime_status.clone()
    }

    async fn auto_link_node_to_daily_note_best_effort(&self, node: &KnowledgeNode) {
        if !self.config.daily_notes.enabled {
            return;
        }
        if is_template_node(node) {
            return;
        }
        if is_daily_note(node) {
            return;
        }
        if !is_daily_link_candidate(node) {
            return;
        }

        let day = node.temporal.created_at.date_naive();
        let daily_note = match self.find_daily_note(day, &node.namespace).await {
            Ok(Some(existing)) => existing,
            Ok(None) => match self
                .ensure_daily_note(day, Some(node.namespace.clone()))
                .await
            {
                Ok((created, _created)) => created,
                Err(err) => {
                    tracing::warn!(
                        node_id = %node.id,
                        namespace = %node.namespace,
                        date = %day,
                        error = %err,
                        "mindvault_daily_note_auto_link_ensure_failed"
                    );
                    return;
                }
            },
            Err(err) => {
                tracing::warn!(
                    node_id = %node.id,
                    namespace = %node.namespace,
                    date = %day,
                    error = %err,
                    "mindvault_daily_note_auto_link_lookup_failed"
                );
                return;
            }
        };

        let existing_relationships = match self.graph.get_relationships_from(daily_note.id).await {
            Ok(relationships) => relationships,
            Err(err) => {
                tracing::warn!(
                    node_id = %node.id,
                    daily_note_id = %daily_note.id,
                    error = %err,
                    "mindvault_daily_note_auto_link_relationship_scan_failed"
                );
                return;
            }
        };

        let already_linked = existing_relationships.iter().any(|rel| {
            rel.to_node == node.id
                && matches!(
                    rel.kind,
                    RelationKind::Contains | RelationKind::References | RelationKind::PartOf
                )
        });
        if already_linked {
            return;
        }

        let relationship = Relationship::new(daily_note.id, node.id, RelationKind::Contains);
        if let Err(err) = self.graph.add_relationship(&relationship).await {
            tracing::warn!(
                node_id = %node.id,
                daily_note_id = %daily_note.id,
                error = %err,
                "mindvault_daily_note_auto_link_insert_failed"
            );
            return;
        }

        tracing::info!(
            node_id = %node.id,
            daily_note_id = %daily_note.id,
            namespace = %node.namespace,
            date = %day,
            "mindvault_daily_note_auto_linked"
        );
    }

    async fn auto_backlink_node_references_best_effort(&self, node: &KnowledgeNode) {
        if !self.config.linking.auto_backlinks_enabled {
            return;
        }
        if is_template_node(node) {
            return;
        }

        let link_targets = extract_reference_targets_with_kind(
            &node.content,
            self.config.linking.auto_backlinks_max_targets,
        );

        let resolved_target_ids = if link_targets.is_empty() {
            Vec::new()
        } else {
            let index = match self
                .build_backlink_resolution_index(
                    &node.namespace,
                    self.config.linking.auto_backlinks_scan_limit,
                )
                .await
            {
                Ok(index) => index,
                Err(err) => {
                    tracing::warn!(
                        node_id = %node.id,
                        namespace = %node.namespace,
                        error = %err,
                        "mindvault_backlink_auto_index_build_failed"
                    );
                    return;
                }
            };
            resolve_reference_targets_with_kind(&link_targets, node.id, &index)
        };

        if let Err(err) = self
            .sync_auto_backlink_references(node.id, &resolved_target_ids)
            .await
        {
            tracing::warn!(
                node_id = %node.id,
                namespace = %node.namespace,
                error = %err,
                "mindvault_backlink_auto_sync_failed"
            );
            return;
        }

        tracing::debug!(
            node_id = %node.id,
            namespace = %node.namespace,
            resolved_links = resolved_target_ids.len(),
            extracted_targets = link_targets.len(),
            "mindvault_backlink_auto_sync_applied"
        );
    }

    async fn build_backlink_resolution_index(
        &self,
        namespace: &str,
        scan_limit: usize,
    ) -> MvResult<KnowledgeVaultBacklinkResolutionIndex> {
        if scan_limit == 0 {
            return Ok(KnowledgeVaultBacklinkResolutionIndex::default());
        }

        let mut index = KnowledgeVaultBacklinkResolutionIndex::default();
        let filters = QueryFilters {
            namespace: Some(namespace.to_string()),
            ..Default::default()
        };
        let page_size = scan_limit.min(200);
        let mut offset = 0usize;
        let mut scanned = 0usize;

        while scanned < scan_limit {
            let remaining = scan_limit - scanned;
            let limit = remaining.min(page_size);
            let batch = self.store.nodes.list(&filters, limit, offset).await?;
            if batch.is_empty() {
                break;
            }
            offset += batch.len();
            scanned += batch.len();
            for node in batch {
                index.insert_node(node.id, node.title.as_deref(), node.source.as_deref());
            }
        }

        if scanned == scan_limit {
            tracing::debug!(
                namespace = %namespace,
                scan_limit,
                "mindvault_backlink_auto_index_scan_capped"
            );
        }

        Ok(index)
    }

    async fn sync_auto_backlink_references(
        &self,
        from_node_id: uuid::Uuid,
        desired_target_ids: &[ResolvedContentReferenceTarget],
    ) -> MvResult<()> {
        let mut desired =
            std::collections::HashMap::<uuid::Uuid, ContentReferenceSourceKind>::new();
        for target in desired_target_ids {
            desired.entry(target.node_id).or_insert(target.source_kind);
        }
        let desired_ids: std::collections::HashSet<uuid::Uuid> = desired.keys().copied().collect();
        let existing = self.graph.get_relationships_from(from_node_id).await?;

        let mut existing_auto_map = std::collections::HashMap::<uuid::Uuid, Relationship>::new();
        for rel in &existing {
            if rel.kind == RelationKind::References && is_auto_backlink_relationship(rel) {
                existing_auto_map.insert(rel.to_node, rel.clone());
            }
        }

        for (target_id, source_kind) in desired {
            let desired_source = source_kind.as_str();
            if let Some(existing_rel) = existing_auto_map.get(&target_id) {
                let existing_source = existing_rel
                    .metadata
                    .get(AUTO_BACKLINK_SOURCE_METADATA_KEY)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if existing_source == desired_source {
                    continue;
                }
                self.graph.remove_relationship(existing_rel.id).await?;
            }

            let mut relationship =
                Relationship::new(from_node_id, target_id, RelationKind::References);
            relationship.metadata.insert(
                AUTO_BACKLINK_METADATA_KEY.to_string(),
                serde_json::Value::Bool(true),
            );
            relationship.metadata.insert(
                AUTO_BACKLINK_SOURCE_METADATA_KEY.to_string(),
                serde_json::Value::String(desired_source.to_string()),
            );
            self.graph.add_relationship(&relationship).await?;
        }

        for rel in &existing {
            if rel.kind != RelationKind::References || !is_auto_backlink_relationship(rel) {
                continue;
            }
            if desired_ids.contains(&rel.to_node) {
                continue;
            }
            self.graph.remove_relationship(rel.id).await?;
        }

        Ok(())
    }
}

fn is_daily_note(node: &KnowledgeNode) -> bool {
    node.tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(DAILY_NOTE_TAG))
        || node
            .metadata
            .get("daily_note")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
}

fn is_template_node(node: &KnowledgeNode) -> bool {
    node.kind == NodeKind::Template
        || node
            .metadata
            .get("template")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        || node
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("template"))
}

fn is_recurring_instance(node: &KnowledgeNode) -> bool {
    node.metadata
        .get(RECURRING_INSTANCE_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn is_daily_link_candidate(node: &KnowledgeNode) -> bool {
    if matches!(node.kind, NodeKind::Task | NodeKind::Event) {
        return true;
    }

    node.tags.iter().any(|tag| {
        DAILY_LINK_CANDIDATE_TAGS
            .iter()
            .any(|candidate| tag.eq_ignore_ascii_case(candidate))
    })
}

fn is_auto_backlink_relationship(rel: &Relationship) -> bool {
    rel.metadata
        .get(AUTO_BACKLINK_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn select_embedding_provider(config: &EngineConfig) -> EmbeddingProviderSelection {
    let provider = config.embedding.provider.trim().to_ascii_lowercase();
    let configured_model = config.embedding.model.clone();
    let configured_dimensions = config.embedding.dimensions;

    let base_status =
        |effective_provider: &str,
         effective_model: String,
         effective_dimensions: usize,
         fallback_to_noop: bool,
         reason: Option<String>| KnowledgeVaultIndexNoteEmbeddingProviderRuntimeStatus {
            configured_provider: config.embedding.provider.clone(),
            configured_model: configured_model.clone(),
            configured_dimensions,
            effective_provider: effective_provider.to_string(),
            effective_model,
            effective_dimensions,
            fallback_to_noop,
            reason,
            local_embeddings_feature_enabled: cfg!(feature = "local-embeddings"),
        };

    match provider.as_str() {
        "openai" => match OpenAiEmbedder::from_env(
            config.embedding.model.clone(),
            config.embedding.dimensions,
        ) {
            Ok(embedder) => {
                tracing::info!(
                    provider = "openai",
                    dimensions = config.embedding.dimensions,
                    "mindvault_embedding_provider_initialized"
                );
                EmbeddingProviderSelection {
                    embedder: Some(Arc::new(embedder)),
                    vector_dimensions: config.embedding.dimensions,
                    runtime_status: base_status(
                        "openai",
                        configured_model.clone(),
                        config.embedding.dimensions,
                        false,
                        None,
                    ),
                }
            }
            Err(err) => {
                let reason = format!("openai initialization failed: {err}");
                tracing::warn!(
                    error = %err,
                    "mindvault_openai_embedder_unavailable_falling_back_to_noop"
                );
                EmbeddingProviderSelection {
                    embedder: None,
                    vector_dimensions: config.embedding.dimensions,
                    runtime_status: base_status(
                        "noop",
                        "noop".to_string(),
                        config.embedding.dimensions,
                        true,
                        Some(reason),
                    ),
                }
            }
        },
        "local_fastembed" | "fastembed" | "local" => {
            let local_model = default_local_model_if_needed(&config.embedding.model);
            match KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder::try_new(&local_model) {
                Ok(embedder) => {
                    let dimensions = embedder.dimensions();
                    tracing::info!(
                        provider = "local_fastembed",
                        model = %embedder.model_name(),
                        dimensions,
                        "mindvault_embedding_provider_initialized"
                    );
                    let reason = if local_model != configured_model {
                        Some(format!(
                            "model '{configured_model}' auto-mapped to '{local_model}' for local_fastembed"
                        ))
                    } else {
                        None
                    };
                    EmbeddingProviderSelection {
                        embedder: Some(Arc::new(embedder)),
                        vector_dimensions: dimensions,
                        runtime_status: base_status(
                            "local_fastembed",
                            local_model,
                            dimensions,
                            false,
                            reason,
                        ),
                    }
                }
                Err(err) => {
                    let reason = format!("local_fastembed initialization failed: {err}");
                    tracing::warn!(
                        provider = "local_fastembed",
                        model = %local_model,
                        error = %err,
                        "mindvault_local_embedder_unavailable_falling_back_to_noop"
                    );
                    EmbeddingProviderSelection {
                        embedder: None,
                        vector_dimensions: config.embedding.dimensions,
                        runtime_status: base_status(
                            "noop",
                            "noop".to_string(),
                            config.embedding.dimensions,
                            true,
                            Some(reason),
                        ),
                    }
                }
            }
        }
        other => {
            let reason = format!("unknown embedding provider '{other}'");
            tracing::warn!(
                provider = %other,
                "mindvault_unknown_embedding_provider_falling_back_to_noop"
            );
            EmbeddingProviderSelection {
                embedder: None,
                vector_dimensions: config.embedding.dimensions,
                runtime_status: base_status(
                    "noop",
                    "noop".to_string(),
                    config.embedding.dimensions,
                    true,
                    Some(reason),
                ),
            }
        }
    }
}

fn default_local_model_if_needed(config_model: &str) -> String {
    if config_model.starts_with("text-embedding-") {
        tracing::info!(
            configured_model = %config_model,
            fallback_model = "bge-small-en-v1.5",
            "mindvault_local_embedder_model_auto_selected"
        );
        "bge-small-en-v1.5".to_string()
    } else {
        config_model.to_string()
    }
}

fn generate_access_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    format!("mvk_{encoded}")
}

fn hash_access_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use mv_core::{GraphStore, KnowledgeNode, NodeKind, RelationKind, Relationship};
    use tempfile::TempDir;

    async fn create_test_engine() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_engine_with_ai_auto_tagging() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.ai.auto_tagging_enabled = true;
        config.ai.auto_tagging_similarity_seed_limit = 8;
        config.ai.auto_tagging_max_generated_tags = 6;
        config.ai.auto_tagging_max_total_tags = 12;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_engine_with_local_embedding_provider(
        provider: &str,
        model: &str,
    ) -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = provider.to_string();
        config.embedding.model = model.to_string();
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    async fn create_test_engine_with_backlinks_disabled() -> (MindVaultEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.linking.auto_backlinks_enabled = false;
        let engine = MindVaultEngine::init(config).await.unwrap();
        (engine, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve_node() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let node = KnowledgeNode::new(NodeKind::Fact, "Test content".to_string())
            .with_title("Test Title")
            .with_tags(vec!["test".into()]);

        let stored_node = engine.store_node(node.clone()).await.unwrap();
        assert_eq!(stored_node.content, "Test content");
        assert_eq!(stored_node.title, Some("Test Title".into()));

        let retrieved = engine.get_node(stored_node.id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Test content");
        assert_eq!(retrieved.tags, vec!["test"]);
    }

    #[tokio::test]
    async fn test_update_node() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let node = KnowledgeNode::new(NodeKind::Fact, "Original content".into());
        let stored = engine.store_node(node).await.unwrap();

        let mut updated = stored.clone();
        updated.content = "Updated content".into();
        updated.title = Some("Updated title".into());

        let updated_node = engine.update_node(updated).await.unwrap();
        assert_eq!(updated_node.content, "Updated content");
        assert_eq!(updated_node.title, Some("Updated title".into()));
    }

    #[tokio::test]
    async fn test_delete_node() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let node = KnowledgeNode::new(NodeKind::Fact, "To delete".into());
        let stored = engine.store_node(node).await.unwrap();

        assert!(engine.delete_node(stored.id).await.unwrap());
        assert!(engine.get_node(stored.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_node_count() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let count = engine.node_count().await.unwrap();
        assert_eq!(count, 0);

        engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "First".into()))
            .await
            .unwrap();
        engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Second".into()))
            .await
            .unwrap();

        let count = engine.node_count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_relationships() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let node1 = engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Node 1".into()))
            .await
            .unwrap();
        let node2 = engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Node 2".into()))
            .await
            .unwrap();

        let relation = Relationship {
            id: uuid::Uuid::new_v4(),
            from_node: node1.id,
            to_node: node2.id,
            kind: RelationKind::RelatesTo,
            weight: 1.0,
            metadata: Default::default(),
            created_at: chrono::Utc::now(),
        };

        engine.add_relationship(relation.clone()).await.unwrap();

        let neighbors = engine.get_neighbors(node1.id, 1).await.unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], node2.id);
    }

    #[tokio::test]
    async fn test_ai_auto_tagging_enriches_from_content_and_neighbors() {
        let (engine, _tmp_dir) = create_test_engine_with_ai_auto_tagging().await;

        let seed = KnowledgeNode::new(
            NodeKind::Fact,
            "Rust async tokio memory pipeline for background jobs".into(),
        )
        .with_tags(vec!["rust".into(), "async".into(), "tokio".into()]);
        let _seed_node = engine.store_node(seed).await.unwrap();

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Building a memory pipeline in Rust for async workers".into(),
            ))
            .await
            .unwrap();

        let tags_lower: Vec<String> = stored
            .tags
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect();
        assert!(!tags_lower.is_empty(), "auto-tagging should add tags");
        assert!(
            tags_lower.iter().any(|tag| tag == "rust"),
            "expected generated tags to include rust"
        );
        assert!(
            tags_lower
                .iter()
                .any(|tag| tag == "memory" || tag == "pipeline"),
            "expected lexical tags to include memory/pipeline"
        );
    }

    #[tokio::test]
    async fn test_ai_auto_tagging_disabled_keeps_empty_tag_list() {
        let (engine, _tmp_dir) = create_test_engine().await;

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Novel note without manual tags".into(),
            ))
            .await
            .unwrap();

        assert!(
            stored.tags.is_empty(),
            "tags should remain empty when auto-tagging is disabled"
        );
    }

    #[tokio::test]
    async fn test_unknown_embedding_provider_falls_back_without_breaking_ingest() {
        let (engine, _tmp_dir) =
            create_test_engine_with_local_embedding_provider("unknown-provider", "any").await;

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Provider fallback should keep ingest healthy".into(),
            ))
            .await
            .unwrap();

        assert_eq!(
            stored.content,
            "Provider fallback should keep ingest healthy"
        );

        let status = engine.embedding_runtime_status();
        assert_eq!(status.configured_provider, "unknown-provider");
        assert_eq!(status.effective_provider, "noop");
        assert!(status.fallback_to_noop);
        assert!(status
            .reason
            .as_deref()
            .is_some_and(|value| value.contains("unknown embedding provider")));
    }

    #[tokio::test]
    async fn test_local_fastembed_invalid_model_falls_back_without_breaking_ingest() {
        let (engine, _tmp_dir) =
            create_test_engine_with_local_embedding_provider("local_fastembed", "invalid-model")
                .await;

        let stored = engine
            .store_node(KnowledgeNode::new(
                NodeKind::Fact,
                "Invalid local model should not break ingest".into(),
            ))
            .await
            .unwrap();

        assert_eq!(
            stored.content,
            "Invalid local model should not break ingest"
        );

        let status = engine.embedding_runtime_status();
        assert_eq!(status.configured_provider, "local_fastembed");
        assert_eq!(status.effective_provider, "noop");
        assert!(status.fallback_to_noop);
        assert!(status
            .reason
            .as_deref()
            .is_some_and(|value| value.contains("local_fastembed initialization failed")));
    }

    #[tokio::test]
    async fn test_ensure_daily_note_is_idempotent() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let date = NaiveDate::from_ymd_opt(2026, 2, 6).expect("valid date");

        let (created_note, created) = engine.ensure_daily_note(date, None).await.unwrap();
        assert!(created);
        assert_eq!(created_note.namespace, engine.config.daily_notes.namespace);
        assert!(created_note.tags.iter().any(|tag| tag == DAILY_NOTE_TAG));
        assert!(created_note.tags.iter().any(|tag| tag == "day:2026-02-06"));
        assert_eq!(
            created_note.metadata.get("daily_note_date"),
            Some(&serde_json::Value::String("2026-02-06".into()))
        );

        let (existing_note, created_again) = engine.ensure_daily_note(date, None).await.unwrap();
        assert!(!created_again);
        assert_eq!(existing_note.id, created_note.id);
    }

    #[tokio::test]
    async fn test_list_daily_notes_filters_non_daily_nodes() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let date = NaiveDate::from_ymd_opt(2026, 2, 6).expect("valid date");
        let namespace = engine.config.daily_notes.namespace.clone();

        let (_daily, created) = engine
            .ensure_daily_note(date, Some(namespace.clone()))
            .await
            .unwrap();
        assert!(created);

        let non_daily = KnowledgeNode::new(NodeKind::Fact, "not a daily note".into())
            .with_namespace(namespace.clone())
            .with_tags(vec!["journal".into()]);
        let stored_non_daily = engine.store_node(non_daily).await.unwrap();

        let notes = engine
            .list_daily_notes(Some(namespace), 50, 0)
            .await
            .unwrap();

        assert!(!notes.is_empty());
        assert!(notes
            .iter()
            .all(|node| node.tags.iter().any(|tag| tag == DAILY_NOTE_TAG)));
        assert!(notes.iter().all(|node| node.id != stored_non_daily.id));
    }

    #[tokio::test]
    async fn test_store_node_auto_links_task_to_daily_note() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = engine.config.daily_notes.namespace.clone();
        let task_node = KnowledgeNode::new(
            NodeKind::Task,
            "Ship release checklist and verify deployment timeline".into(),
        )
        .with_namespace(namespace.clone())
        .with_tags(vec!["release".into()]);

        let stored_task = engine.store_node(task_node).await.unwrap();
        let day = stored_task.temporal.created_at.date_naive();
        let daily_note = engine
            .find_daily_note(day, &namespace)
            .await
            .unwrap()
            .expect("daily note should be created");

        let outgoing = engine
            .graph
            .get_relationships_from(daily_note.id)
            .await
            .unwrap();
        assert!(outgoing
            .iter()
            .any(|rel| { rel.to_node == stored_task.id && rel.kind == RelationKind::Contains }));
    }

    #[tokio::test]
    async fn test_store_node_auto_links_event_kind_without_tags() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = engine.config.daily_notes.namespace.clone();
        let event_node =
            KnowledgeNode::new(NodeKind::Event, "Team planning sync at 10:00 UTC".into())
                .with_namespace(namespace.clone());

        let stored_event = engine.store_node(event_node).await.unwrap();
        let day = stored_event.temporal.created_at.date_naive();
        let daily_note = engine
            .find_daily_note(day, &namespace)
            .await
            .unwrap()
            .expect("daily note should be created");

        let outgoing = engine
            .graph
            .get_relationships_from(daily_note.id)
            .await
            .unwrap();
        assert!(outgoing
            .iter()
            .any(|rel| rel.to_node == stored_event.id && rel.kind == RelationKind::Contains));
    }

    #[tokio::test]
    async fn test_update_node_auto_link_does_not_duplicate_daily_edge() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = engine.config.daily_notes.namespace.clone();
        let task_node = KnowledgeNode::new(NodeKind::Event, "Capture meeting action items".into())
            .with_namespace(namespace.clone());
        let stored_task = engine.store_node(task_node).await.unwrap();
        let day = stored_task.temporal.created_at.date_naive();
        let daily_note = engine
            .find_daily_note(day, &namespace)
            .await
            .unwrap()
            .expect("daily note should exist");

        let mut updated_task = stored_task.clone();
        updated_task.content = "Capture meeting action items and owner assignments".into();
        let _updated = engine.update_node(updated_task).await.unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(daily_note.id)
            .await
            .unwrap();
        let contains_edges = outgoing
            .iter()
            .filter(|rel| rel.to_node == stored_task.id && rel.kind == RelationKind::Contains)
            .count();
        assert_eq!(contains_edges, 1);
    }

    #[tokio::test]
    async fn test_store_node_auto_backlinks_from_wikilinks() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Launch scope and milestones".into())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Review [[Project Alpha]] and prep a kickoff checklist.".into(),
                )
                .with_namespace(namespace.clone())
                .with_title("Kickoff Brief"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        let reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(reference.is_some());
        assert_eq!(
            reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("wikilink")
        );
    }

    #[tokio::test]
    async fn test_store_node_auto_backlinks_from_markdown_and_source_urls() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let title_target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Project Beta release sequencing".into())
                    .with_namespace(namespace.clone())
                    .with_title("Project Beta"),
            )
            .await
            .unwrap();
        let source_target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Bookmark, "Spec reference".into())
                    .with_namespace(namespace.clone())
                    .with_title("Platform Spec")
                    .with_source("https://docs.example.com/spec"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Review [release scope](Project Beta#Milestones) and https://docs.example.com/spec#overview before kickoff.".into(),
                )
                .with_namespace(namespace.clone())
                .with_title("Kickoff Prep"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        let title_reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == title_target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(title_reference.is_some());
        assert_eq!(
            title_reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("markdown_link")
        );

        let source_reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == source_target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(source_reference.is_some());
        assert_eq!(
            source_reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("source_url")
        );
    }

    #[tokio::test]
    async fn test_store_node_auto_backlinks_from_mentions() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Project Alpha execution notes".into())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Align the launch checklist with @\"Project Alpha\" owners.".into(),
                )
                .with_namespace(namespace.clone())
                .with_title("Launch Checklist"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        let mention_reference = outgoing.iter().find(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target.id
                && is_auto_backlink_relationship(rel)
        });
        assert!(mention_reference.is_some());
        assert_eq!(
            mention_reference
                .and_then(|rel| rel.metadata.get(AUTO_BACKLINK_SOURCE_METADATA_KEY))
                .and_then(serde_json::Value::as_str),
            Some("mention")
        );
    }

    #[tokio::test]
    async fn test_update_node_auto_backlinks_removes_stale_targets() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "knowledge".to_string();
        let target_alpha = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Alpha details".into())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();
        let target_beta = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Beta details".into())
                    .with_namespace(namespace.clone())
                    .with_title("Project Beta"),
            )
            .await
            .unwrap();

        let mut source = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Draft [[Project Alpha]] notes".into())
                    .with_namespace(namespace.clone())
                    .with_title("Planning"),
            )
            .await
            .unwrap();

        source.content = "Finalize [[Project Beta]] notes".into();
        let source = engine.update_node(source).await.unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        assert!(outgoing.iter().any(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target_beta.id
                && is_auto_backlink_relationship(rel)
        }));
        assert!(!outgoing.iter().any(|rel| {
            rel.kind == RelationKind::References
                && rel.to_node == target_alpha.id
                && is_auto_backlink_relationship(rel)
        }));
    }

    #[tokio::test]
    async fn test_auto_backlinks_can_be_disabled() {
        let (engine, _tmp_dir) = create_test_engine_with_backlinks_disabled().await;
        let namespace = "knowledge".to_string();

        let _target = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Alpha details".into())
                    .with_namespace(namespace.clone())
                    .with_title("Project Alpha"),
            )
            .await
            .unwrap();

        let source = engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Draft [[Project Alpha]] notes".into())
                    .with_namespace(namespace.clone())
                    .with_title("Planning"),
            )
            .await
            .unwrap();

        let outgoing = engine
            .graph
            .get_relationships_from(source.id)
            .await
            .unwrap();
        assert!(!outgoing.iter().any(|rel| {
            rel.kind == RelationKind::References && is_auto_backlink_relationship(rel)
        }));
    }

    #[tokio::test]
    async fn test_rollforward_recurring_tasks_generates_due_instance() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let namespace = "ops".to_string();
        let mut template =
            KnowledgeNode::new(NodeKind::Task, "Daily standup action checklist".into())
                .with_namespace(namespace.clone())
                .with_tags(vec!["ops".into()]);

        let last_generated = Utc
            .with_ymd_and_hms(2026, 2, 5, 9, 0, 0)
            .single()
            .expect("valid datetime");
        template.metadata.insert(
            TASK_RECURRENCE_METADATA_KEY.into(),
            serde_json::json!({
                "frequency": "daily",
                "interval": 1,
                "enabled": true
            }),
        );
        template.metadata.insert(
            TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY.into(),
            serde_json::Value::String(last_generated.to_rfc3339()),
        );

        let stored_template = engine.store_node(template).await.unwrap();
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 9, 0, 0)
            .single()
            .expect("valid datetime");
        let stats = engine.rollforward_recurring_tasks(now, 4).await.unwrap();
        assert_eq!(stats.generated_instances, 1);

        let tasks = engine
            .list_nodes(
                &QueryFilters {
                    namespace: Some(namespace.clone()),
                    kinds: Some(vec![NodeKind::Task]),
                    ..Default::default()
                },
                50,
                0,
            )
            .await
            .unwrap();

        let instance = tasks
            .iter()
            .find(|node| {
                node.metadata
                    .get(RECURRING_INSTANCE_METADATA_KEY)
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            })
            .expect("recurring instance should exist");
        assert_eq!(
            instance
                .metadata
                .get(RECURRING_PARENT_ID_METADATA_KEY)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            Some(stored_template.id.to_string())
        );

        let rels = engine
            .graph
            .get_relationships_from(stored_template.id)
            .await
            .unwrap();
        assert!(rels
            .iter()
            .any(|rel| { rel.to_node == instance.id && rel.kind == RelationKind::DerivedFrom }));
    }

    #[tokio::test]
    async fn test_rollforward_recurring_tasks_is_idempotent_for_same_instant() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let mut template = KnowledgeNode::new(NodeKind::Task, "Weekly planning template".into())
            .with_namespace("ops");
        template.metadata.insert(
            TASK_RECURRENCE_METADATA_KEY.into(),
            serde_json::json!({
                "frequency": "weekly",
                "interval": 1,
                "enabled": true
            }),
        );
        template.metadata.insert(
            TASK_RECURRENCE_LAST_GENERATED_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 1, 30, 10, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let _stored = engine.store_node(template).await.unwrap();

        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 10, 0, 0)
            .single()
            .expect("valid datetime");
        let first = engine.rollforward_recurring_tasks(now, 4).await.unwrap();
        let second = engine.rollforward_recurring_tasks(now, 4).await.unwrap();

        assert!(first.generated_instances >= 1);
        assert_eq!(second.generated_instances, 0);
    }

    #[tokio::test]
    async fn test_list_due_tasks_filters_and_sorts() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let base = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut task_a = KnowledgeNode::new(NodeKind::Task, "A".into()).with_namespace("ops");
        task_a.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 10, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let mut task_b = KnowledgeNode::new(NodeKind::Task, "B".into()).with_namespace("ops");
        task_b.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 11, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let mut task_c = KnowledgeNode::new(NodeKind::Task, "C".into()).with_namespace("ops");
        task_c.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 9, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        task_c.metadata.insert(
            TASK_COMPLETED_METADATA_KEY.into(),
            serde_json::Value::Bool(true),
        );

        let _a = engine.store_node(task_a).await.unwrap();
        let _b = engine.store_node(task_b).await.unwrap();
        let _c = engine.store_node(task_c).await.unwrap();

        let due = engine
            .list_due_tasks(base, Some("ops".into()), 10, false)
            .await
            .unwrap();
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].content, "A");
        assert_eq!(due[1].content, "B");
    }

    #[tokio::test]
    async fn test_prioritize_tasks_ranks_by_due_and_importance() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut urgent = KnowledgeNode::new(NodeKind::Task, "Urgent".into())
            .with_namespace("ops")
            .with_importance(0.9);
        urgent.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 18, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );

        let important = KnowledgeNode::new(NodeKind::Task, "Important".into())
            .with_namespace("ops")
            .with_importance(0.8);

        let mut later = KnowledgeNode::new(NodeKind::Task, "Later".into())
            .with_namespace("ops")
            .with_importance(0.2);
        later.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );

        let _urgent = engine.store_node(urgent).await.unwrap();
        let _important = engine.store_node(important).await.unwrap();
        let _later = engine.store_node(later).await.unwrap();

        let prioritized = engine
            .prioritize_tasks(TaskPrioritizationOptions {
                namespace: Some("ops".into()),
                limit: 10,
                include_completed: false,
                include_without_due: true,
                persist: false,
                now,
            })
            .await
            .unwrap();

        assert_eq!(prioritized.len(), 3);
        assert_eq!(prioritized[0].task.content, "Urgent");
        assert_eq!(prioritized[1].task.content, "Important");
        assert_eq!(prioritized[2].task.content, "Later");
        assert_eq!(prioritized[0].rank, 1);
    }

    #[tokio::test]
    async fn test_dispatch_due_task_reminders_marks_once() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut task = KnowledgeNode::new(NodeKind::Task, "Follow up on incident".into())
            .with_namespace("ops");
        task.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(
                Utc.with_ymd_and_hms(2026, 2, 6, 8, 0, 0)
                    .single()
                    .expect("valid datetime")
                    .to_rfc3339(),
            ),
        );
        let stored = engine.store_node(task).await.unwrap();

        let first = engine.dispatch_due_task_reminders(now, 20).await.unwrap();
        assert_eq!(first.reminders_marked_sent, 1);

        let second = engine.dispatch_due_task_reminders(now, 20).await.unwrap();
        assert_eq!(second.reminders_marked_sent, 0);

        let refreshed = engine
            .get_node(stored.id)
            .await
            .unwrap()
            .expect("task exists");
        assert!(refreshed
            .metadata
            .contains_key(TASK_REMINDER_SENT_AT_METADATA_KEY));
    }

    #[tokio::test]
    async fn test_default_permission_templates_seeded() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let templates = engine.list_permission_templates(10, 0).await.unwrap();
        let names: Vec<String> = templates.into_iter().map(|t| t.name).collect();
        assert!(names.contains(&"Owner".to_string()));
        assert!(names.contains(&"Assistant".to_string()));
    }

    #[tokio::test]
    async fn test_access_key_round_trip() {
        let (engine, _tmp_dir) = create_test_engine().await;
        let template = engine
            .create_permission_template(
                "Scoped".into(),
                Some("Test".into()),
                PermissionTier::Edit,
                Some("ops".into()),
                vec!["shared".into()],
                vec![NodeKind::Fact],
                vec!["transform".into()],
            )
            .await
            .unwrap();

        let (_key, token) = engine
            .create_access_key(template.id, Some("Key".into()), None)
            .await
            .unwrap();

        let resolved = engine.resolve_access_key(&token).await.unwrap();
        assert!(resolved.is_some());
        let (_key, resolved_template) = resolved.unwrap();
        assert_eq!(resolved_template.id, template.id);
    }
}
