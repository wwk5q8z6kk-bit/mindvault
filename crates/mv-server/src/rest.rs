use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, Request, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, Method, StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use mv_core::*;
use mv_engine::engine::{PrioritizedTask, TaskPrioritizationOptions};
use mv_engine::recurrence::{
    parse_optional_metadata_bool, parse_optional_metadata_datetime, TASK_COMPLETED_AT_METADATA_KEY,
    TASK_COMPLETED_METADATA_KEY, TASK_DUE_AT_METADATA_KEY, TASK_REMINDER_SENT_AT_METADATA_KEY,
    TASK_REMINDER_STATUS_METADATA_KEY,
};

#[path = "rest/assist.rs"]
mod assist;
use assist::{
    collect_completion_sources, generate_action_items_transform, generate_autocomplete_completions,
    generate_completion_suggestions, generate_link_suggestions, generate_refine_transform,
    generate_summary_transform,
};
#[path = "rest/attachments.rs"]
mod attachments;
use attachments::{
    extract_attachment_search_text, normalize_attachment_search_blob,
    split_attachment_search_chunks, AttachmentTextExtractionOutcome,
};
#[path = "rest/saved_searches.rs"]
mod saved_searches;
use saved_searches::{
    apply_saved_search_definition, is_saved_search_node, saved_search_definition_from_node,
    SavedSearchDefinition, SAVED_SEARCH_TAG,
};
#[path = "rest/permissions.rs"]
mod permissions;
use permissions::{
    create_access_key, create_permission_template, delete_permission_template, list_access_keys,
    list_permission_templates, revoke_access_key, update_permission_template,
};
#[path = "rest/node_versions.rs"]
mod node_versions;
use node_versions::{
    apply_node_version, node_authored_fields_changed, node_version_detail_response,
    node_version_snapshot_from_node, node_version_summaries, parse_node_versions_from_metadata,
    push_node_version_snapshot, set_node_versions_in_metadata, NodeVersionDetailResponse,
    NodeVersionSummary,
};
#[path = "rest/voice.rs"]
mod voice;
use voice::{is_audio_file, transcribe_audio, transcribe_audio_api, WhisperConfig};

use crate::audit::{audit_middleware, list_audit_entries, AuditConfig, AuditEntry, AuditLogger};
use crate::auth::{
    auth_middleware_with_state, authorize_namespace, authorize_read, authorize_write,
    namespace_for_create, scoped_namespace, AuthContext,
};
use crate::limits::{enforce_namespace_quota, enforce_rate_limit, NamespaceQuotaError};
use crate::metrics::{init_metrics, metrics_handler, metrics_middleware};
use crate::openapi::{openapi_json, swagger_ui};
use crate::state::AppState;
use crate::validation::{
    validate_depth, validate_list_limit, validate_node_payload, validate_query_text,
    validate_recall_limit,
};

pub fn create_router(state: Arc<AppState>) -> Router {
    create_router_with_cors(state, &[])
}

/// Initialize audit and metrics systems.
pub fn init_observability() {
    // Initialize audit logger
    let audit_config = AuditConfig::from_env();
    if audit_config.enabled {
        AuditLogger::init(audit_config);
        tracing::info!("audit logging enabled");
    }

    // Initialize metrics
    init_metrics();
    tracing::info!("prometheus metrics initialized");
}

pub fn create_router_with_cors(state: Arc<AppState>, cors_allowed_origins: &[String]) -> Router {
    let router = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/diagnostics/embedding", get(embedding_diagnostics))
        .route("/api/v1/assist/completion", post(assist_completion))
        .route("/api/v1/assist/autocomplete", post(assist_autocomplete))
        .route("/api/v1/assist/links", post(assist_links))
        .route("/api/v1/assist/transform", post(assist_transform))
        .route("/api/v1/daily-notes", get(list_daily_notes))
        .route("/api/v1/daily-notes/ensure", post(ensure_daily_note))
        .route("/api/v1/calendar/items", get(list_calendar_items))
        .route("/api/v1/calendar/ical", get(export_calendar_ical))
        .route("/api/v1/calendar/ical/import", post(import_calendar_ical))
        .route("/api/v1/tasks/due", get(list_due_tasks))
        .route("/api/v1/tasks/prioritize", post(prioritize_tasks))
        .route("/api/v1/tasks/{id}/complete", post(complete_task))
        .route("/api/v1/tasks/{id}/reopen", post(reopen_task))
        .route("/api/v1/tasks/{id}/snooze", post(snooze_task_reminder))
        .route("/api/v1/template-packs", get(list_template_packs))
        .route(
            "/api/v1/template-packs/{pack_id}/install",
            post(install_template_pack),
        )
        .route(
            "/api/v1/templates",
            post(create_template).get(list_templates),
        )
        .route(
            "/api/v1/templates/{id}",
            delete(delete_template).patch(update_template),
        )
        .route("/api/v1/templates/{id}/duplicate", post(duplicate_template))
        .route(
            "/api/v1/templates/{id}/instantiate",
            post(instantiate_template),
        )
        .route("/api/v1/templates/{id}/apply", post(apply_template))
        .route(
            "/api/v1/templates/{id}/versions",
            get(list_template_versions),
        )
        .route(
            "/api/v1/templates/{id}/versions/{version_id}",
            get(get_template_version),
        )
        .route(
            "/api/v1/templates/{id}/versions/{version_id}/restore",
            post(restore_template_version),
        )
        .route(
            "/api/v1/permission-templates",
            get(list_permission_templates).post(create_permission_template),
        )
        .route(
            "/api/v1/permission-templates/{id}",
            put(update_permission_template).delete(delete_permission_template),
        )
        .route(
            "/api/v1/access-keys",
            get(list_access_keys).post(create_access_key),
        )
        .route("/api/v1/access-keys/{id}", delete(revoke_access_key))
        .route("/api/v1/export", get(export_bundle))
        .route("/api/v1/import", post(import_bundle))
        .route(
            "/api/v1/search/saved",
            get(list_saved_searches).post(create_saved_search),
        )
        .route(
            "/api/v1/search/saved/{id}",
            put(update_saved_search).delete(delete_saved_search),
        )
        .route("/api/v1/search/saved/{id}/run", post(run_saved_search))
        .route(
            "/api/v1/saved_views",
            get(list_saved_views).post(create_saved_view),
        )
        .route(
            "/api/v1/saved_views/{id}",
            patch(update_saved_view).delete(delete_saved_view),
        )
        .route("/api/v1/clips/import", post(import_clip))
        .route("/api/v1/clips/enrich", post(enrich_clip))
        .route("/api/v1/files/upload", post(upload_file))
        .route("/api/v1/voice/upload", post(upload_voice_note))
        .route("/api/v1/files/{node_id}", get(list_node_attachments))
        .route(
            "/api/v1/files/{node_id}/{attachment_id}/chunks",
            get(get_attachment_chunks),
        )
        .route(
            "/api/v1/files/{node_id}/{attachment_id}/reindex",
            post(reindex_attachment),
        )
        .route(
            "/api/v1/files/{node_id}/{attachment_id}",
            get(download_attachment).delete(delete_attachment),
        )
        .route("/api/v1/nodes", post(store_node))
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/nodes/{id}/versions", get(list_node_versions))
        .route(
            "/api/v1/nodes/{id}/versions/{version_id}",
            get(get_node_version),
        )
        .route(
            "/api/v1/nodes/{id}/versions/{version_id}/restore",
            post(restore_node_version),
        )
        .route("/api/v1/nodes/{id}", get(get_node))
        .route("/api/v1/nodes/{id}", put(update_node))
        .route("/api/v1/nodes/{id}", delete(delete_node))
        .route("/api/v1/recall", post(recall))
        .route("/api/v1/search", get(search))
        .route("/api/v1/graph/relationships", post(add_relationship))
        .route(
            "/api/v1/graph/relationships/{id}",
            get(get_node_relationships),
        )
        .route("/api/v1/graph/neighbors/{id}", get(get_neighbors))
        .route("/api/v1/audit", get(list_audit_logs))
        .route("/metrics", get(metrics_handler))
        .route("/api/openapi.json", get(openapi_spec_handler))
        .merge(swagger_ui())
        .layer(middleware::from_fn(audit_middleware))
        .layer(middleware::from_fn(metrics_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware_with_state,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    if cors_allowed_origins.is_empty() {
        router
    } else {
        router.layer(build_cors_layer(cors_allowed_origins))
    }
}

fn build_cors_layer(cors_allowed_origins: &[String]) -> CorsLayer {
    let mut parsed = Vec::new();
    for origin in cors_allowed_origins {
        match HeaderValue::from_str(origin) {
            Ok(value) => parsed.push(value),
            Err(err) => tracing::warn!("ignoring invalid CORS origin '{origin}': {err}"),
        }
    }

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_origin(parsed)
}

// --- DTOs ---

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    node_count: usize,
    version: String,
}

#[derive(Serialize)]
struct EmbeddingProviderDiagnosticsResponse {
    configured_provider: String,
    configured_model: String,
    configured_dimensions: usize,
    effective_provider: String,
    effective_model: String,
    effective_dimensions: usize,
    fallback_to_noop: bool,
    reason: Option<String>,
    local_embeddings_feature_enabled: bool,
}

#[derive(Deserialize)]
struct AssistCompletionRequest {
    text: String,
    limit: Option<usize>,
    namespace: Option<String>,
}

#[derive(Deserialize)]
struct AssistAutocompleteRequest {
    text: String,
    limit: Option<usize>,
    namespace: Option<String>,
}

#[derive(Deserialize)]
struct AssistLinkSuggestionsRequest {
    text: String,
    limit: Option<usize>,
    namespace: Option<String>,
    exclude_node_id: Option<String>,
}

#[derive(Deserialize)]
struct AssistTransformRequest {
    text: String,
    mode: Option<String>,
    limit: Option<usize>,
    namespace: Option<String>,
}

#[derive(Serialize)]
struct AssistCompletionResponse {
    suggestions: Vec<String>,
    sources: Vec<AssistSuggestionSourceDto>,
    source_nodes: usize,
    strategy: String,
}

#[derive(Serialize)]
struct AssistAutocompleteResponse {
    completions: Vec<String>,
    source_nodes: usize,
    strategy: String,
}

#[derive(Serialize)]
struct AssistLinkSuggestionDto {
    node_id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    heading: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<String>,
    namespace: String,
    score: f64,
    reason: String,
}

#[derive(Serialize)]
struct AssistSuggestionSourceDto {
    node_id: String,
    title: String,
    namespace: String,
    score: f64,
}

#[derive(Serialize)]
struct AssistLinkSuggestionsResponse {
    suggestions: Vec<AssistLinkSuggestionDto>,
    source_nodes: usize,
    strategy: String,
}

#[derive(Serialize)]
struct AssistTransformResponse {
    transformed_text: String,
    mode: String,
    source_nodes: usize,
    strategy: String,
}

#[derive(Deserialize)]
struct StoreNodeRequest {
    kind: String,
    content: String,
    title: Option<String>,
    source: Option<String>,
    namespace: Option<String>,
    tags: Option<Vec<String>>,
    importance: Option<f64>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct ClipImportRequest {
    url: String,
    title: Option<String>,
    #[serde(alias = "text", alias = "selection")]
    excerpt: Option<String>,
    tags: Option<Vec<String>>,
    namespace: Option<String>,
    clip_source: Option<String>,
    dedupe: Option<bool>,
    create_note: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ClipEnrichRequest {
    url: String,
    html: Option<String>,
}

#[derive(Deserialize)]
struct RecallRequest {
    text: String,
    strategy: Option<String>,
    limit: Option<usize>,
    min_score: Option<f64>,
    namespace: Option<String>,
    kinds: Option<Vec<String>>,
    tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
    #[serde(rename = "type")]
    search_type: Option<String>,
}

#[derive(Deserialize)]
struct SavedSearchListQuery {
    namespace: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct AuditListQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    subject: Option<String>,
    action: Option<String>,
    since: Option<String>,
}

#[derive(Deserialize)]
struct CreateSavedSearchRequest {
    name: String,
    description: Option<String>,
    query: String,
    #[serde(alias = "strategy")]
    search_type: Option<String>,
    limit: Option<usize>,
    namespace: Option<String>,
    target_namespace: Option<String>,
    kinds: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    min_score: Option<f64>,
    min_importance: Option<f64>,
}

#[derive(Deserialize)]
struct UpdateSavedSearchRequest {
    name: Option<String>,
    description: Option<Option<String>>,
    query: Option<String>,
    #[serde(alias = "strategy")]
    search_type: Option<String>,
    limit: Option<usize>,
    target_namespace: Option<Option<String>>,
    kinds: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    min_score: Option<Option<f64>>,
    min_importance: Option<Option<f64>>,
}

#[derive(Deserialize)]
struct ListQuery {
    namespace: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct DailyNotesListQuery {
    namespace: Option<String>,
    date: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct DailyNotesEnsureRequest {
    namespace: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
struct DueTasksQuery {
    namespace: Option<String>,
    before: Option<String>,
    limit: Option<usize>,
    include_completed: Option<bool>,
}

#[derive(Deserialize)]
struct PrioritizeTasksRequest {
    namespace: Option<String>,
    limit: Option<usize>,
    include_completed: Option<bool>,
    include_without_due: Option<bool>,
    persist: Option<bool>,
    now: Option<String>,
}

#[derive(Serialize)]
struct PrioritizeTasksResponse {
    generated_at: String,
    items: Vec<PrioritizedTask>,
    count: usize,
    limit: usize,
    strategy: String,
}

#[derive(Deserialize)]
struct CalendarItemsQuery {
    namespace: Option<String>,
    view: Option<String>,
    anchor: Option<String>,
    start: Option<String>,
    end: Option<String>,
    limit: Option<usize>,
    include_completed: Option<bool>,
}

#[derive(Deserialize)]
struct TemplateListQuery {
    namespace: Option<String>,
    kind: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedViewType {
    List,
    Kanban,
    Calendar,
}

impl SavedViewType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Kanban => "kanban",
            Self::Calendar => "calendar",
        }
    }
}

impl std::str::FromStr for SavedViewType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "list" => Ok(Self::List),
            "kanban" => Ok(Self::Kanban),
            "calendar" => Ok(Self::Calendar),
            _ => Err(format!("invalid view_type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedViewSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedViewSort {
    field: String,
    direction: SavedViewSortDirection,
}

#[derive(Debug, Clone)]
struct SavedViewDefinition {
    name: String,
    view_type: SavedViewType,
    filters: serde_json::Value,
    sort: Option<SavedViewSort>,
    group_by: Option<String>,
    query: Option<String>,
}

#[derive(Deserialize)]
struct SavedViewListQuery {
    namespace: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct CreateSavedViewRequest {
    name: String,
    view_type: String,
    filters: Option<serde_json::Value>,
    sort: Option<serde_json::Value>,
    group_by: Option<String>,
    query: Option<String>,
    namespace: Option<String>,
}

#[derive(Deserialize)]
struct UpdateSavedViewRequest {
    name: Option<String>,
    view_type: Option<String>,
    filters: Option<serde_json::Value>,
    sort: Option<serde_json::Value>,
    group_by: Option<String>,
    query: Option<String>,
}

#[derive(Serialize)]
struct SavedViewDto {
    id: String,
    name: String,
    namespace: String,
    view_type: String,
    group_by: Option<String>,
    query: Option<String>,
    filters: serde_json::Value,
    sort: Option<SavedViewSort>,
    updated_at: String,
}

#[derive(Deserialize)]
struct CreateTemplateRequest {
    kind: String,
    content: String,
    title: Option<String>,
    source: Option<String>,
    namespace: Option<String>,
    tags: Option<Vec<String>>,
    importance: Option<f64>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    template_key: Option<String>,
    template_variables: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct InstantiateTemplateRequest {
    namespace: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    values: Option<std::collections::HashMap<String, String>>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
struct UpdateTemplateRequest {
    title: Option<String>,
    content: Option<String>,
    source: Option<String>,
    tags: Option<Vec<String>>,
    importance: Option<f64>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    template_key: Option<String>,
    template_variables: Option<Vec<String>>,
    target_kind: Option<String>,
}

#[derive(Deserialize)]
struct ApplyTemplateRequest {
    target_node_id: Option<String>,
    target_kind: Option<String>,
    overwrite: Option<bool>,
}

#[derive(Serialize)]
struct TemplateApplyResponse {
    node: KnowledgeNode,
    created: bool,
    filled_fields: Vec<String>,
    overwritten_fields: Vec<String>,
}

#[derive(Deserialize)]
struct DuplicateTemplateRequest {
    namespace: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    template_key: Option<String>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TemplateVersionRecord {
    version_id: String,
    captured_at: String,
    kind: String,
    namespace: String,
    title: Option<String>,
    content: String,
    source: Option<String>,
    tags: Vec<String>,
    importance: f64,
    metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
struct TemplateVersionSummary {
    version_id: String,
    captured_at: String,
    kind: String,
    namespace: String,
    title: Option<String>,
    source: Option<String>,
    importance: f64,
    tag_count: usize,
    content_preview: String,
}

#[derive(Debug, Clone, Serialize)]
struct TemplateVersionCurrentSnapshot {
    kind: String,
    namespace: String,
    title: Option<String>,
    content: String,
    source: Option<String>,
    tags: Vec<String>,
    importance: f64,
}

#[derive(Debug, Clone, Serialize)]
struct TemplateVersionDiffSummary {
    version_line_count: usize,
    current_line_count: usize,
    added_line_count: usize,
    removed_line_count: usize,
    added_line_samples: Vec<String>,
    removed_line_samples: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct TemplateVersionDetailResponse {
    version: TemplateVersionRecord,
    current: TemplateVersionCurrentSnapshot,
    diff: TemplateVersionDiffSummary,
    field_changes: Vec<TemplateVersionFieldChange>,
}

#[derive(Debug, Clone, Serialize)]
struct TemplateVersionFieldChange {
    field: String,
    changed: bool,
    version_value: String,
    current_value: String,
}

#[derive(Deserialize)]
struct InstallTemplatePackRequest {
    namespace: Option<String>,
    overwrite_existing: Option<bool>,
    additional_tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SnoozeTaskReminderRequest {
    /// Duration to snooze in minutes. If not provided, resets reminder to unsent.
    snooze_minutes: Option<u64>,
}

#[derive(Serialize)]
struct SnoozeTaskReminderResponse {
    node_id: String,
    reminder_cleared: bool,
    new_due_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct VoiceNoteUploadResponse {
    node_id: String,
    title: Option<String>,
    transcription: String,
    transcription_chars: usize,
    audio_attachment_id: String,
    linked_to_daily_note: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct TemplatePackSummary {
    pack_id: String,
    name: String,
    description: String,
    template_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct InstallTemplatePackResponse {
    pack_id: String,
    namespace: String,
    installed_templates: usize,
    updated_templates: usize,
    skipped_templates: usize,
    template_ids: Vec<String>,
}

#[derive(Deserialize)]
struct ExportQuery {
    namespace: Option<String>,
    include_relationships: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultTransferBundle {
    format_version: String,
    exported_at: DateTime<Utc>,
    scope_namespace: Option<String>,
    nodes: Vec<KnowledgeNode>,
    relationships: Vec<Relationship>,
}

#[derive(Debug, Deserialize)]
struct ImportRequest {
    namespace_override: Option<String>,
    overwrite_existing: Option<bool>,
    include_relationships: Option<bool>,
    nodes: Vec<KnowledgeNode>,
    relationships: Option<Vec<Relationship>>,
}

#[derive(Debug, Serialize)]
struct ImportResponse {
    imported_nodes: usize,
    updated_nodes: usize,
    skipped_nodes: usize,
    imported_relationships: usize,
    skipped_relationships: usize,
}

#[derive(Debug, Serialize)]
struct ClipImportResponse {
    bookmark: KnowledgeNode,
    created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<KnowledgeNode>,
}

#[derive(Debug, Serialize)]
struct ClipEnrichResponse {
    normalized_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    site_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_reading_minutes: Option<u16>,
    suggested_tags: Vec<String>,
    fetched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileUploadResponse {
    attachment_id: String,
    node_id: String,
    file_name: String,
    content_type: Option<String>,
    size_bytes: usize,
    stored_path: String,
    extraction_status: String,
    extracted_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeAttachmentRecord {
    id: String,
    file_name: String,
    content_type: Option<String>,
    size_bytes: usize,
    stored_path: String,
    uploaded_at: Option<String>,
    extraction_status: Option<String>,
    extracted_chars: Option<usize>,
}

#[derive(Debug, Serialize)]
struct AttachmentListItemResponse {
    attachment_id: String,
    file_name: String,
    content_type: Option<String>,
    size_bytes: usize,
    uploaded_at: Option<String>,
    extraction_status: Option<String>,
    extracted_chars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_chunk_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_preview: Option<String>,
    download_url: String,
}

#[derive(Debug, Deserialize)]
struct AttachmentChunkQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AttachmentDownloadQuery {
    inline: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AttachmentChunkItemResponse {
    index: usize,
    text: String,
    char_count: usize,
}

#[derive(Debug, Serialize)]
struct AttachmentChunkListResponse {
    node_id: String,
    attachment_id: String,
    extraction_status: Option<String>,
    extracted_chars: Option<usize>,
    total_chunks: usize,
    offset: usize,
    limit: usize,
    returned_chunks: usize,
    chunks: Vec<AttachmentChunkItemResponse>,
}

#[derive(Debug, Serialize)]
struct AttachmentReindexResponse {
    node_id: String,
    attachment_id: String,
    extraction_status: String,
    extracted_chars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_chunk_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_preview: Option<String>,
}

#[derive(Debug, Serialize)]
struct AttachmentDeleteResponse {
    attachment_id: String,
    file_deleted: bool,
    remaining_attachments: usize,
}

#[derive(Debug, Serialize)]
struct DailyNotesEnsureResponse {
    node: KnowledgeNode,
    created: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CalendarItemResponse {
    node: KnowledgeNode,
    scheduled_at: DateTime<Utc>,
    scheduled_end_at: Option<DateTime<Utc>>,
    schedule_source: String,
    completed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CalendarItemsResponse {
    view: String,
    anchor: DateTime<Utc>,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    total_items: usize,
    returned_items: usize,
    items: Vec<CalendarItemResponse>,
}

#[derive(Debug, Serialize)]
struct CalendarIcalImportResponse {
    imported_nodes: usize,
    updated_nodes: usize,
    skipped_events: usize,
    parse_errors: usize,
    namespace: String,
}

#[derive(Serialize)]
struct SearchResultDto {
    node: mv_core::KnowledgeNode,
    score: f64,
    match_source: String,
}

#[derive(Debug, Clone, Serialize)]
struct SavedSearchDto {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    query: String,
    search_type: String,
    limit: usize,
    namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_namespace: Option<String>,
    kinds: Vec<String>,
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_importance: Option<f64>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct SavedSearchRunResponse {
    saved_search: SavedSearchDto,
    executed_at: String,
    results: Vec<SearchResultDto>,
}

#[derive(Deserialize)]
struct AddRelationshipRequest {
    from_node: String,
    to_node: String,
    kind: String,
    weight: Option<f64>,
}

#[derive(Deserialize)]
struct NeighborsQuery {
    depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct NodeRelationshipEdgeResponse {
    relationship_id: String,
    relation_kind: String,
    direction: String,
    related_node_id: String,
    related_node_title: Option<String>,
    related_node_kind: String,
    related_node_namespace: String,
    weight: f64,
    created_at: DateTime<Utc>,
    auto_managed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct NodeRelationshipOverviewResponse {
    node_id: String,
    outgoing: Vec<NodeRelationshipEdgeResponse>,
    incoming: Vec<NodeRelationshipEdgeResponse>,
}

const EXPORT_FORMAT_VERSION_V1: &str = "mindvault.export.v1";
const IMPORT_MAX_NODES: usize = 5000;
const IMPORT_MAX_RELATIONSHIPS: usize = 20_000;
const MAX_ICAL_IMPORT_BYTES: usize = 2 * 1024 * 1024;
const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
const MAX_ATTACHMENT_EXTRACTED_TEXT_CHARS: usize = 12_000;
const MAX_ATTACHMENT_SEARCH_BLOB_CHARS: usize = 64_000;
const MAX_ATTACHMENT_SEARCH_CHUNK_CHARS: usize = 420;
const MAX_ATTACHMENT_SEARCH_CHUNK_COUNT: usize = 32;
const MAX_ATTACHMENT_SEARCH_PREVIEW_CHARS: usize = 180;
const DEFAULT_ATTACHMENT_CHUNK_PAGE_SIZE: usize = 8;
const MAX_ATTACHMENT_CHUNK_PAGE_SIZE: usize = 64;
const ATTACHMENT_TEXT_INDEX_METADATA_KEY: &str = "attachment_text_index";
const ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY: &str = "attachment_text_chunks";
const ATTACHMENT_SEARCH_BLOB_METADATA_KEY: &str = "attachment_search_text";
const AUTO_BACKLINK_METADATA_KEY: &str = "auto_backlink";
const AUTO_BACKLINK_SOURCE_METADATA_KEY: &str = "source";
const CLIP_READ_METADATA_KEY: &str = "read";
const CLIP_SOURCE_METADATA_KEY: &str = "clip_source";
const CLIP_NORMALIZED_URL_METADATA_KEY: &str = "normalized_url";
const CLIP_CAPTURED_AT_METADATA_KEY: &str = "captured_at";
const CLIP_LINKED_BOOKMARK_ID_METADATA_KEY: &str = "clip_bookmark_id";
const CLIP_SCAN_PAGE_SIZE: usize = 250;
const CLIP_ENRICH_MAX_HTML_CHARS: usize = 180_000;
const CLIP_ENRICH_HTTP_TIMEOUT_SECS: u64 = 6;
const MAX_SAVED_SEARCH_NAME_LEN: usize = 120;
const MAX_SAVED_SEARCH_DESCRIPTION_LEN: usize = 2048;
const MAX_SAVED_SEARCH_FILTER_TAGS: usize = 32;
const MAX_SAVED_SEARCH_FILTER_TAG_LEN: usize = 64;
const DEFAULT_SAVED_SEARCH_LIMIT: usize = 10;
const MAX_SAVED_VIEW_NAME_LEN: usize = 160;
const MAX_SAVED_VIEW_GROUP_BY_LEN: usize = 120;
const EVENT_START_AT_METADATA_KEY: &str = "event_start_at";
const EVENT_END_AT_METADATA_KEY: &str = "event_end_at";
const ICAL_UID_METADATA_KEY: &str = "ical_uid";
const ICAL_STATUS_METADATA_KEY: &str = "ical_status";
const ICAL_IMPORTED_AT_METADATA_KEY: &str = "ical_imported_at";
const TEMPLATE_METADATA_KEY: &str = "template";
const TEMPLATE_KEY_METADATA_KEY: &str = "template_key";
const TEMPLATE_VARIABLES_METADATA_KEY: &str = "template_variables";
const TEMPLATE_TARGET_KIND_METADATA_KEY: &str = "template_target_kind";
const TEMPLATE_VERSIONS_METADATA_KEY: &str = "template_versions";
const TEMPLATE_SOURCE_ID_METADATA_KEY: &str = "instantiated_from_template_id";
const TEMPLATE_INSTANTIATED_AT_METADATA_KEY: &str = "instantiated_at";
const TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY: &str = "template_last_instantiated_at";
const TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY: &str = "template_instantiation_count";
const TEMPLATE_RESTORED_FROM_VERSION_METADATA_KEY: &str = "template_restored_from_version_id";
const TEMPLATE_RESTORED_AT_METADATA_KEY: &str = "template_restored_at";
const TEMPLATE_PACK_ID_METADATA_KEY: &str = "template_pack_id";
const TEMPLATE_PACK_NAME_METADATA_KEY: &str = "template_pack_name";
const TEMPLATE_TAG: &str = "template";
const SAVED_VIEW_TAG: &str = "saved-view";
const SAVED_VIEW_MARKER_METADATA_KEY: &str = "saved_view";
const SAVED_VIEW_FILTERS_METADATA_KEY: &str = "saved_view_filters";
const SAVED_VIEW_SORT_METADATA_KEY: &str = "saved_view_sort";
const SAVED_VIEW_VIEW_TYPE_METADATA_KEY: &str = "saved_view_view_type";
const SAVED_VIEW_GROUP_BY_METADATA_KEY: &str = "saved_view_group_by";
const SAVED_VIEW_QUERY_METADATA_KEY: &str = "saved_view_query";
const TEMPLATE_VERSION_MAX_ENTRIES: usize = 30;
const CALENDAR_SCAN_PAGE_SIZE: usize = 250;

#[derive(Debug, Clone, Copy)]
enum CalendarView {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy)]
struct BuiltinTemplateDefinition {
    key: &'static str,
    kind: &'static str,
    title: &'static str,
    content: &'static str,
    tags: &'static [&'static str],
    variables: &'static [&'static str],
    importance: f64,
    source: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct BuiltinTemplatePackDefinition {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    templates: &'static [BuiltinTemplateDefinition],
}

const BUILTIN_TEMPLATE_PACK_DAILY_FLOW: [BuiltinTemplateDefinition; 3] = [
    BuiltinTemplateDefinition {
        key: "daily.review",
        kind: "fact",
        title: "Daily Review {{date}}",
        content: "## Wins\n- {{win_1}}\n- {{win_2}}\n\n## Blockers\n- {{blocker_1}}\n\n## Focus For Tomorrow\n- {{focus_1}}\n- {{focus_2}}",
        tags: &["daily", "review", "journal"],
        variables: &["date", "win_1", "win_2", "blocker_1", "focus_1", "focus_2"],
        importance: 0.7,
        source: Some("builtin:daily-flow"),
    },
    BuiltinTemplateDefinition {
        key: "daily.priorities",
        kind: "task",
        title: "Top Priorities {{date}}",
        content: "- [ ] {{priority_1}}\n- [ ] {{priority_2}}\n- [ ] {{priority_3}}\n\n## Success Criteria\n- {{success_criteria}}",
        tags: &["daily", "planning", "execution"],
        variables: &[
            "date",
            "priority_1",
            "priority_2",
            "priority_3",
            "success_criteria",
        ],
        importance: 0.8,
        source: Some("builtin:daily-flow"),
    },
    BuiltinTemplateDefinition {
        key: "daily.shutdown",
        kind: "fact",
        title: "Shutdown Note {{date}}",
        content: "## Completed\n- {{completed_1}}\n\n## Carried Forward\n- {{carry_forward_1}}\n\n## Notes To Future Me\n- {{future_note}}",
        tags: &["daily", "reflection", "handoff"],
        variables: &["date", "completed_1", "carry_forward_1", "future_note"],
        importance: 0.6,
        source: Some("builtin:daily-flow"),
    },
];

const BUILTIN_TEMPLATE_PACK_PROJECT_EXECUTION: [BuiltinTemplateDefinition; 3] = [
    BuiltinTemplateDefinition {
        key: "project.brief",
        kind: "entity",
        title: "Project Brief: {{project_name}}",
        content: "## Outcome\n{{project_outcome}}\n\n## Scope\n{{project_scope}}\n\n## Stakeholders\n- {{stakeholder_1}}\n- {{stakeholder_2}}\n\n## Constraints\n- {{constraint_1}}",
        tags: &["project", "planning", "brief"],
        variables: &[
            "project_name",
            "project_outcome",
            "project_scope",
            "stakeholder_1",
            "stakeholder_2",
            "constraint_1",
        ],
        importance: 0.8,
        source: Some("builtin:project-execution"),
    },
    BuiltinTemplateDefinition {
        key: "project.weekly-status",
        kind: "fact",
        title: "Weekly Status: {{project_name}} - {{week_label}}",
        content: "## Progress\n- {{progress_1}}\n- {{progress_2}}\n\n## Risks\n- {{risk_1}}\n\n## Next Week\n- {{next_step_1}}\n- {{next_step_2}}",
        tags: &["project", "status", "weekly"],
        variables: &[
            "project_name",
            "week_label",
            "progress_1",
            "progress_2",
            "risk_1",
            "next_step_1",
            "next_step_2",
        ],
        importance: 0.7,
        source: Some("builtin:project-execution"),
    },
    BuiltinTemplateDefinition {
        key: "project.milestone-check",
        kind: "decision",
        title: "Milestone Decision: {{milestone_name}}",
        content: "## Context\n{{decision_context}}\n\n## Decision\n{{decision_outcome}}\n\n## Tradeoffs\n- {{tradeoff_1}}\n- {{tradeoff_2}}\n\n## Follow-up\n- {{follow_up_1}}",
        tags: &["project", "milestone", "decision"],
        variables: &[
            "milestone_name",
            "decision_context",
            "decision_outcome",
            "tradeoff_1",
            "tradeoff_2",
            "follow_up_1",
        ],
        importance: 0.75,
        source: Some("builtin:project-execution"),
    },
];

const BUILTIN_TEMPLATE_PACK_MEETING_SYSTEM: [BuiltinTemplateDefinition; 3] = [
    BuiltinTemplateDefinition {
        key: "meeting.agenda",
        kind: "event",
        title: "Meeting Agenda: {{meeting_name}}",
        content: "## Objective\n{{meeting_objective}}\n\n## Topics\n1. {{topic_1}}\n2. {{topic_2}}\n3. {{topic_3}}\n\n## Desired Outcome\n{{desired_outcome}}",
        tags: &["meeting", "agenda"],
        variables: &[
            "meeting_name",
            "meeting_objective",
            "topic_1",
            "topic_2",
            "topic_3",
            "desired_outcome",
        ],
        importance: 0.7,
        source: Some("builtin:meeting-system"),
    },
    BuiltinTemplateDefinition {
        key: "meeting.notes",
        kind: "fact",
        title: "Meeting Notes: {{meeting_name}}",
        content: "## Summary\n{{summary}}\n\n## Decisions\n- {{decision_1}}\n\n## Action Items\n- [ ] {{action_item_1}} (@{{owner_1}})",
        tags: &["meeting", "notes", "action-items"],
        variables: &[
            "meeting_name",
            "summary",
            "decision_1",
            "action_item_1",
            "owner_1",
        ],
        importance: 0.7,
        source: Some("builtin:meeting-system"),
    },
    BuiltinTemplateDefinition {
        key: "meeting.retrospective",
        kind: "decision",
        title: "Retrospective: {{team_name}} - {{period}}",
        content: "## What Went Well\n- {{good_1}}\n\n## What Needs Improvement\n- {{improve_1}}\n\n## Experiments\n- {{experiment_1}}",
        tags: &["meeting", "retrospective", "continuous-improvement"],
        variables: &["team_name", "period", "good_1", "improve_1", "experiment_1"],
        importance: 0.65,
        source: Some("builtin:meeting-system"),
    },
];

const BUILTIN_TEMPLATE_PACKS: &[BuiltinTemplatePackDefinition] = &[
    BuiltinTemplatePackDefinition {
        id: "daily-flow",
        name: "Daily Flow",
        description: "Daily planning, review, and shutdown templates for consistent execution.",
        templates: &BUILTIN_TEMPLATE_PACK_DAILY_FLOW,
    },
    BuiltinTemplatePackDefinition {
        id: "project-execution",
        name: "Project Execution",
        description: "Project brief, weekly status, and milestone decision templates.",
        templates: &BUILTIN_TEMPLATE_PACK_PROJECT_EXECUTION,
    },
    BuiltinTemplatePackDefinition {
        id: "meeting-system",
        name: "Meeting System",
        description: "Agenda, notes, and retrospective templates for meeting workflows.",
        templates: &BUILTIN_TEMPLATE_PACK_MEETING_SYSTEM,
    },
];

fn map_mv_error(err: MvError) -> (StatusCode, String) {
    match err {
        MvError::NodeNotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        MvError::InvalidInput(_) => (StatusCode::BAD_REQUEST, err.to_string()),
        MvError::DuplicateNode(_) => (StatusCode::CONFLICT, err.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn map_namespace_quota_error(err: NamespaceQuotaError) -> (StatusCode, String) {
    match err {
        NamespaceQuotaError::Exceeded {
            namespace,
            quota,
            count,
        } => (
            StatusCode::TOO_MANY_REQUESTS,
            format!("namespace '{namespace}' quota exceeded ({count}/{quota} nodes)"),
        ),
        NamespaceQuotaError::Backend(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    let Some(auth) = request.extensions().get::<AuthContext>().cloned() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    if let Err(rate) = enforce_rate_limit(&auth) {
        let body = serde_json::json!({
            "error": "rate limit exceeded",
            "retry_after_seconds": rate.retry_after_secs,
            "limit": rate.max_requests,
            "window_seconds": rate.window_secs,
        });
        let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
        if let Ok(value) = HeaderValue::from_str(&rate.retry_after_secs.to_string()) {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, value);
        }
        return response;
    }

    next.run(request).await
}

fn parse_kind_list(
    kinds: Option<Vec<String>>,
) -> Result<Option<Vec<NodeKind>>, (StatusCode, String)> {
    match kinds {
        Some(raw_kinds) => {
            if raw_kinds.is_empty() {
                Ok(None)
            } else {
                let mut parsed = Vec::with_capacity(raw_kinds.len());
                for kind in raw_kinds {
                    parsed.push(
                        kind.parse()
                            .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?,
                    );
                }
                Ok(Some(parsed))
            }
        }
        None => Ok(None),
    }
}

fn parse_saved_search_kind_filters(
    raw: Option<Vec<String>>,
) -> Result<Option<Vec<NodeKind>>, (StatusCode, String)> {
    match raw {
        Some(values) => {
            if values.is_empty() {
                return Ok(Some(Vec::new()));
            }
            let mut parsed = Vec::with_capacity(values.len());
            let mut seen = std::collections::HashSet::new();
            for value in values {
                let kind = value
                    .parse::<NodeKind>()
                    .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?;
                if seen.insert(kind.as_str()) {
                    parsed.push(kind);
                }
            }
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

fn parse_saved_search_strategy(
    raw: Option<String>,
    default: SearchStrategy,
) -> Result<SearchStrategy, (StatusCode, String)> {
    match raw {
        Some(value) => value
            .trim()
            .to_ascii_lowercase()
            .parse::<SearchStrategy>()
            .map_err(|err: String| (StatusCode::BAD_REQUEST, err)),
        None => Ok(default),
    }
}

fn normalize_saved_search_name(raw: &str) -> Result<String, (StatusCode, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "saved search name is required".into(),
        ));
    }
    if trimmed.len() > MAX_SAVED_SEARCH_NAME_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("saved search name exceeds max length of {MAX_SAVED_SEARCH_NAME_LEN}"),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_saved_search_description(
    raw: Option<String>,
) -> Result<Option<String>, (StatusCode, String)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() > MAX_SAVED_SEARCH_DESCRIPTION_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "saved search description exceeds max length of {MAX_SAVED_SEARCH_DESCRIPTION_LEN}"
            ),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn normalize_optional_namespace_value(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_clip_source(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_display_chars(value, 64))
        .unwrap_or_else(|| "manual".to_string())
}

fn normalize_clip_url(raw: &str) -> Option<String> {
    let input = raw.trim();
    if input.is_empty() {
        return None;
    }
    let lowered = input.to_ascii_lowercase();
    if lowered.contains("://")
        && !lowered.starts_with("http://")
        && !lowered.starts_with("https://")
    {
        return None;
    }
    if !lowered.starts_with("http://") && !lowered.starts_with("https://") {
        let authority_candidate = input.split('/').next().unwrap_or(input);
        if authority_candidate.contains(':') {
            let Some((host_part, port_part)) = authority_candidate.rsplit_once(':') else {
                return None;
            };
            if host_part.trim().is_empty()
                || port_part.is_empty()
                || !port_part.chars().all(|ch| ch.is_ascii_digit())
            {
                return None;
            }
        }
    }
    let with_protocol = if lowered.starts_with("http://") || lowered.starts_with("https://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let mut parsed = reqwest::Url::parse(&with_protocol).ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return None,
    }
    parsed.set_fragment(None);
    if let Some(host) = parsed.host_str().map(|value| value.to_ascii_lowercase()) {
        let _ = parsed.set_host(Some(&host));
    }
    let current_path = parsed.path().to_string();
    if current_path.len() > 1 && current_path.ends_with('/') {
        parsed.set_path(&current_path[..current_path.len() - 1]);
    }
    Some(parsed.to_string())
}

fn clip_title_from_url(normalized_url: &str) -> String {
    reqwest::Url::parse(normalized_url)
        .ok()
        .and_then(|parsed| {
            parsed
                .host_str()
                .map(|value| value.trim_start_matches("www.").to_string())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| truncate_display_chars(normalized_url, 80))
}

fn normalize_clip_tag(raw: &str) -> Option<String> {
    let normalized = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
        .collect::<String>();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_clip_tags(raw: Vec<String>) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in std::iter::once("web-clip".to_string()).chain(raw.into_iter()) {
        let Some(normalized) = normalize_clip_tag(&value) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            tags.push(normalized);
        }
    }
    tags
}

fn clip_normalized_url_for_node(node: &KnowledgeNode) -> Option<String> {
    if let Some(metadata_url) = node
        .metadata
        .get(CLIP_NORMALIZED_URL_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .and_then(normalize_clip_url)
    {
        return Some(metadata_url);
    }
    node.source.as_deref().and_then(normalize_clip_url)
}

fn clip_note_content(bookmark: &KnowledgeNode, excerpt: Option<&str>) -> String {
    let title = bookmark
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Web clip");
    let source = bookmark.source.as_deref().unwrap_or("unknown");

    let mut lines = vec![
        format!("# Clip: {title}"),
        String::new(),
        format!("Source: {source}"),
        String::new(),
    ];
    if let Some(body) = excerpt.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(body.to_string());
        lines.push(String::new());
    }
    lines.push(format!("Bookmark ID: {}", bookmark.id));
    lines.join("\n")
}

async fn find_existing_clip_by_url(
    state: &AppState,
    namespace: &str,
    normalized_url: &str,
) -> Result<Option<KnowledgeNode>, (StatusCode, String)> {
    let mut offset = 0usize;
    loop {
        let page = state
            .engine
            .list_nodes(
                &QueryFilters {
                    namespace: Some(namespace.to_string()),
                    kinds: Some(vec![NodeKind::Bookmark]),
                    ..Default::default()
                },
                CLIP_SCAN_PAGE_SIZE,
                offset,
            )
            .await
            .map_err(map_mv_error)?;
        if page.is_empty() {
            break;
        }
        let page_len = page.len();
        if let Some(existing) = page.into_iter().find(|node| {
            !is_saved_search_node(node)
                && clip_normalized_url_for_node(node)
                    .is_some_and(|candidate| candidate == normalized_url)
        }) {
            return Ok(Some(existing));
        }
        if page_len < CLIP_SCAN_PAGE_SIZE {
            break;
        }
        offset = offset.saturating_add(CLIP_SCAN_PAGE_SIZE);
    }
    Ok(None)
}

fn clip_text_value(raw: &str, max_chars: usize) -> Option<String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(truncate_display_chars(&normalized, max_chars))
    }
}

fn decode_basic_html_entities(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_html_attribute(tag: &str, attribute: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{attribute}=");
    let attr_pos = lower.find(&needle)?;
    let mut value = tag[attr_pos + needle.len()..].trim_start();
    if value.is_empty() {
        return None;
    }
    let extracted = if value.starts_with('"') {
        value = &value[1..];
        let end = value.find('"')?;
        &value[..end]
    } else if value.starts_with('\'') {
        value = &value[1..];
        let end = value.find('\'')?;
        &value[..end]
    } else {
        let end = value
            .find(|ch: char| ch.is_ascii_whitespace() || ch == '>')
            .unwrap_or(value.len());
        &value[..end]
    };
    clip_text_value(&decode_basic_html_entities(extracted), 300)
}

fn extract_html_tag_content(html: &str, tag_name: &str, max_chars: usize) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag_name}");
    let close = format!("</{tag_name}>");
    let start_tag = lower.find(&open)?;
    let start_content = lower[start_tag..].find('>')? + start_tag + 1;
    let end_content = lower[start_content..].find(&close)? + start_content;
    let raw = &html[start_content..end_content];
    clip_text_value(&decode_basic_html_entities(raw), max_chars)
}

fn extract_all_html_tag_contents(
    html: &str,
    tag_name: &str,
    max_chars_per_item: usize,
    max_items: usize,
) -> Vec<String> {
    if max_items == 0 {
        return Vec::new();
    }

    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag_name}");
    let close = format!("</{tag_name}>");
    let mut cursor = 0usize;
    let mut items = Vec::new();

    while cursor < html.len() && items.len() < max_items {
        let Some(start_rel) = lower[cursor..].find(&open) else {
            break;
        };
        let start_tag = cursor + start_rel;
        let Some(open_end_rel) = lower[start_tag..].find('>') else {
            break;
        };
        let start_content = start_tag + open_end_rel + 1;
        let Some(close_rel) = lower[start_content..].find(&close) else {
            break;
        };
        let end_content = start_content + close_rel;
        let raw = &html[start_content..end_content];
        if let Some(text) = clip_text_value(&decode_basic_html_entities(raw), max_chars_per_item) {
            items.push(text);
        }
        cursor = end_content + close.len();
    }

    items
}

fn extract_html_meta_content(html: &str, keys: &[&str]) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0usize;
    while let Some(rel) = lower[cursor..].find("<meta") {
        let start = cursor + rel;
        let Some(close_rel) = lower[start..].find('>') else {
            break;
        };
        let end = start + close_rel + 1;
        let tag_lower = &lower[start..end];
        let matches = keys.iter().any(|key| {
            let key = key.to_ascii_lowercase();
            [
                format!("name=\"{key}\""),
                format!("name='{key}'"),
                format!("property=\"{key}\""),
                format!("property='{key}'"),
                format!("itemprop=\"{key}\""),
                format!("itemprop='{key}'"),
            ]
            .iter()
            .any(|needle| tag_lower.contains(needle))
        });
        if matches {
            let tag_raw = &html[start..end];
            if let Some(content) = extract_html_attribute(tag_raw, "content") {
                return Some(content);
            }
        }
        cursor = end;
    }
    None
}

fn strip_html_tags(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut inside_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn strip_html_element_blocks(html: &str, tag_name: &str) -> String {
    let open_pattern = format!("<{}", tag_name.to_ascii_lowercase());
    let close_pattern = format!("</{}>", tag_name.to_ascii_lowercase());
    let lower = html.to_ascii_lowercase();
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while cursor < html.len() {
        let remaining = &lower[cursor..];
        let Some(open_rel) = remaining.find(&open_pattern) else {
            output.push_str(&html[cursor..]);
            break;
        };
        let open_idx = cursor + open_rel;
        output.push_str(&html[cursor..open_idx]);

        let after_open = &lower[open_idx..];
        let Some(close_rel) = after_open.find(&close_pattern) else {
            break;
        };
        let close_idx = open_idx + close_rel + close_pattern.len();
        cursor = close_idx;
    }

    output
}

fn extract_body_text_from_html(html: &str, max_chars: usize) -> Option<String> {
    let without_scripts = strip_html_element_blocks(html, "script");
    let without_styles = strip_html_element_blocks(&without_scripts, "style");
    let without_noscript = strip_html_element_blocks(&without_styles, "noscript");

    for container_tag in ["article", "main"] {
        if let Some(raw) = extract_html_tag_content(
            &without_noscript,
            container_tag,
            max_chars.saturating_mul(3),
        ) {
            let plain = strip_html_tags(&raw);
            if let Some(value) = clip_text_value(&decode_basic_html_entities(&plain), max_chars) {
                if value.split_whitespace().count() >= 40 {
                    return Some(value);
                }
            }
        }
    }

    let paragraphs = extract_all_html_tag_contents(&without_noscript, "p", 900, 24);
    if !paragraphs.is_empty() {
        let mut selected = Vec::new();
        for paragraph in paragraphs {
            let plain = strip_html_tags(&paragraph);
            if plain.split_whitespace().count() < 8 {
                continue;
            }
            selected.push(plain);
            if selected.len() >= 8 {
                break;
            }
        }
        if !selected.is_empty() {
            let joined = selected.join("\n\n");
            if let Some(value) = clip_text_value(&decode_basic_html_entities(&joined), max_chars) {
                return Some(value);
            }
        }
    }

    let body_raw = extract_html_tag_content(&without_noscript, "body", max_chars.saturating_mul(2))
        .unwrap_or_else(|| without_noscript.clone());
    let plain = strip_html_tags(&body_raw);
    clip_text_value(&decode_basic_html_entities(&plain), max_chars)
}

fn estimate_reading_minutes(text: &str) -> Option<u16> {
    let word_count = text.split_whitespace().count();
    if word_count == 0 {
        return None;
    }
    let minutes = ((word_count as f64) / 200.0).ceil() as u16;
    Some(minutes.max(1))
}

fn extract_clip_preview_from_html(html: &str) -> Option<String> {
    if let Some(paragraph) = extract_html_tag_content(html, "p", 700) {
        let stripped = strip_html_tags(&paragraph);
        if let Some(value) = clip_text_value(&stripped, 320) {
            return Some(value);
        }
    }
    None
}

fn clip_stop_words() -> &'static [&'static str] {
    &[
        "about", "after", "also", "because", "from", "have", "into", "just", "more", "only",
        "over", "that", "their", "there", "these", "this", "those", "with", "your", "guide",
    ]
}

fn push_unique_clip_tag(
    raw: &str,
    tags: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
) {
    let Some(tag) = normalize_clip_tag(raw) else {
        return;
    };
    if seen.insert(tag.clone()) {
        tags.push(tag);
    }
}

fn build_clip_suggested_tags(
    normalized_url: &str,
    title: Option<&str>,
    description: Option<&str>,
    content_preview: Option<&str>,
    keywords: Option<&str>,
) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Ok(parsed) = reqwest::Url::parse(normalized_url) {
        if let Some(host) = parsed.host_str() {
            for segment in host.split('.') {
                let lowered = segment.to_ascii_lowercase();
                if lowered.len() < 3 {
                    continue;
                }
                if [
                    "www", "com", "org", "net", "io", "dev", "app", "co", "ai", "blog",
                ]
                .contains(&lowered.as_str())
                {
                    continue;
                }
                push_unique_clip_tag(&lowered, &mut tags, &mut seen);
            }
        }
    }

    let mut seed_text = String::new();
    if let Some(title) = title {
        seed_text.push_str(title);
        seed_text.push(' ');
    }
    if let Some(description) = description {
        seed_text.push_str(description);
        seed_text.push(' ');
    }
    if let Some(content_preview) = content_preview {
        seed_text.push_str(&truncate_display_chars(content_preview, 1000));
    }
    for token in seed_text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 4)
    {
        let lowered = token.to_ascii_lowercase();
        if clip_stop_words().contains(&lowered.as_str()) {
            continue;
        }
        push_unique_clip_tag(&lowered, &mut tags, &mut seen);
        if tags.len() >= 8 {
            break;
        }
    }

    if let Some(keywords) = keywords {
        for token in keywords
            .split(|ch: char| ch == ',' || ch == ';' || ch == '|')
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
            push_unique_clip_tag(token, &mut tags, &mut seen);
            if tags.len() >= 10 {
                break;
            }
        }
    }

    tags.truncate(10);
    tags
}

async fn fetch_clip_html(url: &str) -> Result<(String, Option<String>), (StatusCode, String)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            CLIP_ENRICH_HTTP_TIMEOUT_SECS,
        ))
        .user_agent("MindVaultClipper/0.2")
        .build()
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to build HTTP client: {err}"),
            )
        })?;

    let response = client
        .get(url)
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml;q=0.9,*/*;q=0.2",
        )
        .send()
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_GATEWAY,
                format!("failed to fetch URL for enrichment: {err}"),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "failed to fetch URL for enrichment: status {}",
                response.status()
            ),
        ));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let body = response.text().await.map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!("failed to read fetched URL content: {err}"),
        )
    })?;

    Ok((
        truncate_display_chars(&body, CLIP_ENRICH_MAX_HTML_CHARS),
        content_type,
    ))
}

fn normalize_saved_search_filter_tags(
    raw: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, (StatusCode, String)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.len() > MAX_SAVED_SEARCH_FILTER_TAGS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("saved search tags cannot exceed {MAX_SAVED_SEARCH_FILTER_TAGS} items"),
        ));
    }

    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in raw {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.len() > MAX_SAVED_SEARCH_FILTER_TAG_LEN {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("saved search tag exceeds max length of {MAX_SAVED_SEARCH_FILTER_TAG_LEN}"),
            ));
        }
        let normalized_key = trimmed.to_ascii_lowercase();
        if seen.insert(normalized_key) {
            normalized.push(trimmed.to_string());
        }
    }
    Ok(Some(normalized))
}

fn normalize_saved_search_limit(limit: Option<usize>) -> Result<usize, (StatusCode, String)> {
    let normalized_limit = limit.unwrap_or(DEFAULT_SAVED_SEARCH_LIMIT);
    validate_recall_limit(normalized_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    Ok(normalized_limit)
}

fn normalize_unit_interval(
    field_name: &str,
    value: Option<f64>,
) -> Result<Option<f64>, (StatusCode, String)> {
    let Some(value) = value else {
        return Ok(None);
    };
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{field_name} must be a finite value between 0.0 and 1.0"),
        ));
    }
    Ok(Some(value))
}

fn saved_search_dto_from_node(
    node: &KnowledgeNode,
    definition: SavedSearchDefinition,
) -> SavedSearchDto {
    SavedSearchDto {
        id: node.id.to_string(),
        name: definition.name,
        description: definition.description,
        query: definition.query,
        search_type: search_strategy_to_str(definition.strategy).to_string(),
        limit: definition.limit,
        namespace: node.namespace.clone(),
        target_namespace: definition.target_namespace,
        kinds: definition
            .kinds
            .into_iter()
            .map(|kind| kind.to_string())
            .collect(),
        tags: definition.tags,
        min_score: definition.min_score,
        min_importance: definition.min_importance,
        created_at: node.temporal.created_at.to_rfc3339(),
        updated_at: node.temporal.updated_at.to_rfc3339(),
    }
}

fn match_source_to_str(source: MatchSource) -> &'static str {
    match source {
        MatchSource::Vector => "vector",
        MatchSource::FullText => "full_text",
        MatchSource::Hybrid => "hybrid",
        MatchSource::Graph => "graph",
    }
}

fn search_strategy_to_str(strategy: SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::Vector => "vector",
        SearchStrategy::FullText => "fulltext",
        SearchStrategy::Hybrid => "hybrid",
        SearchStrategy::Graph => "graph",
    }
}

#[derive(Debug, Clone)]
struct CalendarScheduleWindow {
    start_at: DateTime<Utc>,
    end_at: Option<DateTime<Utc>>,
    source: &'static str,
}

#[derive(Debug, Clone)]
struct ResolvedCalendarWindow {
    view_label: String,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CalendarCollectionResult {
    total_items: usize,
    items: Vec<CalendarItemResponse>,
}

#[derive(Debug, Clone)]
struct ParsedIcalEvent {
    uid: Option<String>,
    node_id: Option<uuid::Uuid>,
    kind: Option<NodeKind>,
    namespace: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    categories: Vec<String>,
    status: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
}

impl CalendarView {
    fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
        }
    }

    fn from_query(raw: Option<String>) -> Result<Self, (StatusCode, String)> {
        let Some(value) = raw else {
            return Ok(Self::Week);
        };
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(Self::Week);
        }
        match normalized.as_str() {
            "day" => Ok(Self::Day),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            _ => Err((
                StatusCode::BAD_REQUEST,
                "view must be one of day|week|month".into(),
            )),
        }
    }

    fn range_for_anchor(self, anchor: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let anchor_day = anchor.date_naive();
        match self {
            Self::Day => {
                let start = start_of_day_utc(anchor_day);
                (start, start + Duration::days(1))
            }
            Self::Week => {
                let weekday_offset = i64::from(anchor_day.weekday().num_days_from_monday());
                let week_start_day = anchor_day - Duration::days(weekday_offset);
                let start = start_of_day_utc(week_start_day);
                (start, start + Duration::days(7))
            }
            Self::Month => {
                let month_start_day =
                    NaiveDate::from_ymd_opt(anchor_day.year(), anchor_day.month(), 1)
                        .unwrap_or(anchor_day);
                let (next_year, next_month) = if month_start_day.month() == 12 {
                    (month_start_day.year() + 1, 1)
                } else {
                    (month_start_day.year(), month_start_day.month() + 1)
                };
                let next_month_start = NaiveDate::from_ymd_opt(next_year, next_month, 1)
                    .unwrap_or(month_start_day + Duration::days(31));
                let start = start_of_day_utc(month_start_day);
                let end = start_of_day_utc(next_month_start);
                (start, end)
            }
        }
    }
}

fn start_of_day_utc(day: NaiveDate) -> DateTime<Utc> {
    let naive = day
        .and_hms_opt(0, 0, 0)
        .expect("midnight should always be a valid NaiveDateTime");
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
}

fn resolve_calendar_window(
    anchor: DateTime<Utc>,
    view: CalendarView,
    explicit_start: Option<DateTime<Utc>>,
    explicit_end: Option<DateTime<Utc>>,
) -> Result<ResolvedCalendarWindow, (StatusCode, String)> {
    match (explicit_start, explicit_end) {
        (Some(start), Some(end)) => {
            if start >= end {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "start must be earlier than end".into(),
                ));
            }
            Ok(ResolvedCalendarWindow {
                view_label: "custom".to_string(),
                range_start: start,
                range_end: end,
            })
        }
        (None, None) => {
            let (start, end) = view.range_for_anchor(anchor);
            Ok(ResolvedCalendarWindow {
                view_label: view.as_str().to_string(),
                range_start: start,
                range_end: end,
            })
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            "start and end must both be set when using a custom range".into(),
        )),
    }
}

fn parse_calendar_datetime_metadata(node: &KnowledgeNode, key: &str) -> Option<DateTime<Utc>> {
    match parse_optional_metadata_datetime(&node.metadata, key) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(
                node_id = %node.id,
                namespace = %node.namespace,
                metadata_key = key,
                error = %err,
                "mindvault_calendar_metadata_datetime_parse_failed"
            );
            None
        }
    }
}

fn calendar_schedule_window_for_node(node: &KnowledgeNode) -> Option<CalendarScheduleWindow> {
    match node.kind {
        NodeKind::Task => {
            let due_at = parse_calendar_datetime_metadata(node, TASK_DUE_AT_METADATA_KEY)?;
            Some(CalendarScheduleWindow {
                start_at: due_at,
                end_at: None,
                source: TASK_DUE_AT_METADATA_KEY,
            })
        }
        NodeKind::Event => {
            let explicit_start =
                parse_calendar_datetime_metadata(node, EVENT_START_AT_METADATA_KEY);
            let fallback_due = parse_calendar_datetime_metadata(node, TASK_DUE_AT_METADATA_KEY);
            let (start_at, source) = if let Some(start) = explicit_start {
                (start, EVENT_START_AT_METADATA_KEY)
            } else if let Some(start) = fallback_due {
                (start, TASK_DUE_AT_METADATA_KEY)
            } else {
                return None;
            };

            let raw_end = parse_calendar_datetime_metadata(node, EVENT_END_AT_METADATA_KEY);
            let end_at = raw_end.filter(|value| value >= &start_at);
            if raw_end.is_some() && end_at.is_none() {
                tracing::warn!(
                    node_id = %node.id,
                    namespace = %node.namespace,
                    "mindvault_calendar_event_end_before_start_ignored"
                );
            }

            Some(CalendarScheduleWindow {
                start_at,
                end_at,
                source,
            })
        }
        _ => None,
    }
}

fn is_completed_calendar_task(node: &KnowledgeNode) -> bool {
    if node.kind != NodeKind::Task {
        return false;
    }
    parse_optional_metadata_bool(&node.metadata, TASK_COMPLETED_METADATA_KEY).unwrap_or(false)
}

async fn collect_calendar_items(
    state: &AppState,
    namespace: Option<String>,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    include_completed: bool,
    limit: usize,
) -> Result<CalendarCollectionResult, (StatusCode, String)> {
    let capped_limit = limit.clamp(1, 500);
    let mut offset = 0usize;
    let mut items = Vec::new();
    loop {
        let page = state
            .engine
            .list_nodes(
                &QueryFilters {
                    namespace: namespace.clone(),
                    kinds: Some(vec![NodeKind::Task, NodeKind::Event]),
                    ..Default::default()
                },
                CALENDAR_SCAN_PAGE_SIZE,
                offset,
            )
            .await
            .map_err(map_mv_error)?;
        if page.is_empty() {
            break;
        }
        let page_len = page.len();

        for node in page {
            let Some(schedule) = calendar_schedule_window_for_node(&node) else {
                continue;
            };
            if schedule.start_at < range_start || schedule.start_at >= range_end {
                continue;
            }

            let completed = is_completed_calendar_task(&node);
            if node.kind == NodeKind::Task && completed && !include_completed {
                continue;
            }

            items.push(CalendarItemResponse {
                node,
                scheduled_at: schedule.start_at,
                scheduled_end_at: schedule.end_at,
                schedule_source: schedule.source.to_string(),
                completed,
            });
        }

        if page_len < CALENDAR_SCAN_PAGE_SIZE {
            break;
        }
        offset = offset.saturating_add(CALENDAR_SCAN_PAGE_SIZE);
    }

    items.sort_by(|left, right| {
        left.scheduled_at
            .cmp(&right.scheduled_at)
            .then_with(|| left.node.id.cmp(&right.node.id))
    });

    let total_items = items.len();
    items.truncate(capped_limit);
    Ok(CalendarCollectionResult { total_items, items })
}

async fn find_existing_node_by_ical_uid(
    state: &AppState,
    namespace: &str,
    uid: &str,
) -> Result<Option<KnowledgeNode>, (StatusCode, String)> {
    let mut offset = 0usize;
    loop {
        let page = state
            .engine
            .list_nodes(
                &QueryFilters {
                    namespace: Some(namespace.to_string()),
                    kinds: Some(vec![NodeKind::Task, NodeKind::Event]),
                    ..Default::default()
                },
                CALENDAR_SCAN_PAGE_SIZE,
                offset,
            )
            .await
            .map_err(map_mv_error)?;
        if page.is_empty() {
            break;
        }
        let page_len = page.len();

        if let Some(found) = page.into_iter().find(|node| {
            node.metadata
                .get(ICAL_UID_METADATA_KEY)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|existing| existing == uid)
        }) {
            return Ok(Some(found));
        }

        if page_len < CALENDAR_SCAN_PAGE_SIZE {
            break;
        }
        offset = offset.saturating_add(CALENDAR_SCAN_PAGE_SIZE);
    }

    Ok(None)
}

fn format_ical_datetime_utc(value: DateTime<Utc>) -> String {
    value.format("%Y%m%dT%H%M%SZ").to_string()
}

fn escape_ical_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
}

fn calendar_item_summary(node: &KnowledgeNode) -> String {
    node.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            let preview = trim_content_preview(&node.content, 100);
            if preview.is_empty() {
                "Untitled item".to_string()
            } else {
                preview
            }
        })
}

fn calendar_item_description(node: &KnowledgeNode) -> String {
    let preview = trim_content_preview(&node.content, 4000);
    format!(
        "Kind: {}\nNamespace: {}\nImportance: {:.3}\n\n{}",
        node.kind, node.namespace, node.importance, preview
    )
}

fn calendar_filename_component(raw: &str) -> String {
    let mut sanitized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    sanitized.trim_matches('_').to_string()
}

fn build_calendar_ical_document(
    items: &[CalendarItemResponse],
    generated_at: DateTime<Utc>,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> String {
    let mut output = String::new();
    output.push_str("BEGIN:VCALENDAR\r\n");
    output.push_str("VERSION:2.0\r\n");
    output.push_str("PRODID:-//MindVault//Calendar Export//EN\r\n");
    output.push_str("CALSCALE:GREGORIAN\r\n");
    output.push_str("METHOD:PUBLISH\r\n");
    output.push_str(&format!(
        "X-WR-CALDESC:{}\r\n",
        escape_ical_text(&format!(
            "MindVault export {} to {}",
            range_start.to_rfc3339(),
            range_end.to_rfc3339()
        ))
    ));

    let dtstamp = format_ical_datetime_utc(generated_at);
    for item in items {
        let summary = escape_ical_text(&calendar_item_summary(&item.node));
        let description = escape_ical_text(&calendar_item_description(&item.node));
        let categories = if item.node.tags.is_empty() {
            item.node.kind.to_string()
        } else {
            item.node
                .tags
                .iter()
                .map(|tag| escape_ical_text(tag))
                .collect::<Vec<_>>()
                .join(",")
        };
        let status = match item.node.kind {
            NodeKind::Task if item.completed => "COMPLETED",
            NodeKind::Task => "NEEDS-ACTION",
            _ => "CONFIRMED",
        };
        let event_end = item.scheduled_end_at.or_else(|| {
            if item.node.kind == NodeKind::Event {
                Some(item.scheduled_at + Duration::minutes(30))
            } else {
                None
            }
        });

        output.push_str("BEGIN:VEVENT\r\n");
        output.push_str(&format!("UID:{}@mindvault.local\r\n", item.node.id));
        output.push_str(&format!("DTSTAMP:{}\r\n", dtstamp));
        output.push_str(&format!(
            "DTSTART:{}\r\n",
            format_ical_datetime_utc(item.scheduled_at)
        ));
        if let Some(end_at) = event_end {
            output.push_str(&format!("DTEND:{}\r\n", format_ical_datetime_utc(end_at)));
        }
        output.push_str(&format!("SUMMARY:{}\r\n", summary));
        output.push_str(&format!("DESCRIPTION:{}\r\n", description));
        output.push_str(&format!("CATEGORIES:{}\r\n", categories));
        output.push_str(&format!("STATUS:{}\r\n", status));
        output.push_str(&format!(
            "X-MINDVAULT-NODE-ID:{}\r\n",
            escape_ical_text(&item.node.id.to_string())
        ));
        output.push_str(&format!(
            "X-MINDVAULT-KIND:{}\r\n",
            escape_ical_text(&item.node.kind.to_string())
        ));
        output.push_str(&format!(
            "X-MINDVAULT-NAMESPACE:{}\r\n",
            escape_ical_text(&item.node.namespace)
        ));
        output.push_str(&format!(
            "X-MINDVAULT-SCHEDULE-SOURCE:{}\r\n",
            escape_ical_text(&item.schedule_source)
        ));
        output.push_str("END:VEVENT\r\n");
    }

    output.push_str("END:VCALENDAR\r\n");
    output
}

fn parse_bool_flag(raw: Option<&str>) -> bool {
    let Some(value) = raw else {
        return false;
    };
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn unescape_ical_text(raw: &str) -> String {
    let mut output = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' | 'N' => output.push('\n'),
                    '\\' => output.push('\\'),
                    ';' => output.push(';'),
                    ',' => output.push(','),
                    _ => {
                        output.push(next);
                    }
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn unfold_ical_lines(raw: &str) -> Vec<String> {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = Vec::new();
    for line in normalized.lines() {
        if let Some(last) = lines.last_mut() {
            if line.starts_with(' ') || line.starts_with('\t') {
                last.push_str(line.trim_start());
                continue;
            }
        }
        lines.push(line.to_string());
    }
    lines
}

fn parse_ical_datetime(raw: &str) -> Option<DateTime<Utc>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() == 8 {
        if let Ok(day) = NaiveDate::parse_from_str(trimmed, "%Y%m%d") {
            return Some(start_of_day_utc(day));
        }
        return None;
    }
    if trimmed.ends_with('Z') {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(trimmed, "%Y%m%dT%H%M%SZ") {
            return Some(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
        }
        return None;
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y%m%dT%H%M%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    None
}

fn parse_ical_property(line: &str) -> Option<(String, String)> {
    let (name_with_params, value) = line.split_once(':')?;
    let property_name = name_with_params
        .split(';')
        .next()
        .unwrap_or(name_with_params)
        .trim()
        .to_ascii_uppercase();
    Some((property_name, unescape_ical_text(value.trim())))
}

fn maybe_uuid_from_uid(raw_uid: &str) -> Option<uuid::Uuid> {
    let candidate = raw_uid.split('@').next().unwrap_or(raw_uid).trim();
    if candidate.is_empty() {
        return None;
    }
    uuid::Uuid::parse_str(candidate).ok()
}

fn parse_ical_events(raw: &str) -> (Vec<ParsedIcalEvent>, usize) {
    let lines = unfold_ical_lines(raw);
    let mut events = Vec::new();
    let mut current: Option<ParsedIcalEvent> = None;
    let mut parse_errors = 0usize;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("BEGIN:VEVENT") {
            current = Some(ParsedIcalEvent {
                uid: None,
                node_id: None,
                kind: None,
                namespace: None,
                summary: None,
                description: None,
                categories: Vec::new(),
                status: None,
                starts_at: None,
                ends_at: None,
            });
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VEVENT") {
            if let Some(event) = current.take() {
                if event.starts_at.is_some() {
                    events.push(event);
                } else {
                    parse_errors += 1;
                }
            }
            continue;
        }

        let Some(event) = current.as_mut() else {
            continue;
        };
        let Some((property, value)) = parse_ical_property(trimmed) else {
            continue;
        };
        match property.as_str() {
            "UID" => {
                event.uid = Some(value.clone());
                if event.node_id.is_none() {
                    event.node_id = maybe_uuid_from_uid(&value);
                }
            }
            "SUMMARY" => {
                if !value.trim().is_empty() {
                    event.summary = Some(value);
                }
            }
            "DESCRIPTION" => {
                if !value.trim().is_empty() {
                    event.description = Some(value);
                }
            }
            "CATEGORIES" => {
                event.categories = value
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect();
            }
            "STATUS" => {
                if !value.trim().is_empty() {
                    event.status = Some(value.to_ascii_uppercase());
                }
            }
            "DTSTART" => {
                let parsed = parse_ical_datetime(&value);
                if parsed.is_none() {
                    parse_errors += 1;
                }
                event.starts_at = parsed;
            }
            "DTEND" => {
                event.ends_at = parse_ical_datetime(&value);
            }
            "X-MINDVAULT-NODE-ID" => {
                event.node_id = uuid::Uuid::parse_str(value.trim()).ok();
            }
            "X-MINDVAULT-KIND" => {
                event.kind = value.trim().to_ascii_lowercase().parse::<NodeKind>().ok();
            }
            "X-MINDVAULT-NAMESPACE" => {
                if !value.trim().is_empty() {
                    event.namespace = Some(value.trim().to_string());
                }
            }
            _ => {}
        }
    }

    (events, parse_errors)
}

fn infer_kind_for_ical_event(event: &ParsedIcalEvent, default_kind: NodeKind) -> NodeKind {
    if let Some(kind) = event.kind {
        if matches!(kind, NodeKind::Task | NodeKind::Event) {
            return kind;
        }
    }
    if event
        .status
        .as_deref()
        .is_some_and(|status| matches!(status, "NEEDS-ACTION" | "IN-PROCESS" | "COMPLETED"))
    {
        return NodeKind::Task;
    }
    if event.categories.iter().any(|tag| {
        let lowered = tag.to_ascii_lowercase();
        matches!(lowered.as_str(), "task" | "todo" | "to-do" | "action")
    }) {
        return NodeKind::Task;
    }
    default_kind
}

fn node_from_ical_event(
    event: &ParsedIcalEvent,
    namespace: &str,
    default_kind: NodeKind,
    now: DateTime<Utc>,
) -> Option<KnowledgeNode> {
    let starts_at = event.starts_at?;
    let kind = infer_kind_for_ical_event(event, default_kind);
    let summary = event
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Imported calendar item")
        .to_string();
    let content = event
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(summary.as_str())
        .to_string();

    let mut node = KnowledgeNode::new(kind, content).with_namespace(namespace.to_string());
    node = node.with_title(summary);
    node = node.with_source("import:calendar-ical");
    node.tags = merge_tags_case_insensitive(&event.categories, &["imported-ical".to_string()]);

    if let Some(existing_id) = event.node_id {
        node.id = existing_id;
    }

    node.metadata.insert(
        ICAL_IMPORTED_AT_METADATA_KEY.to_string(),
        serde_json::Value::String(now.to_rfc3339()),
    );
    if let Some(uid) = event.uid.as_deref() {
        node.metadata.insert(
            ICAL_UID_METADATA_KEY.to_string(),
            serde_json::Value::String(uid.to_string()),
        );
    }
    if let Some(status) = event.status.as_deref() {
        node.metadata.insert(
            ICAL_STATUS_METADATA_KEY.to_string(),
            serde_json::Value::String(status.to_string()),
        );
    }

    match kind {
        NodeKind::Task => {
            node.metadata.insert(
                TASK_DUE_AT_METADATA_KEY.to_string(),
                serde_json::Value::String(starts_at.to_rfc3339()),
            );
            if event.status.as_deref() == Some("COMPLETED") {
                node.metadata.insert(
                    TASK_COMPLETED_METADATA_KEY.to_string(),
                    serde_json::Value::Bool(true),
                );
                node.metadata.insert(
                    TASK_COMPLETED_AT_METADATA_KEY.to_string(),
                    serde_json::Value::String(now.to_rfc3339()),
                );
            }
        }
        NodeKind::Event => {
            node.metadata.insert(
                EVENT_START_AT_METADATA_KEY.to_string(),
                serde_json::Value::String(starts_at.to_rfc3339()),
            );
            if let Some(end_at) = event.ends_at.filter(|end| end >= &starts_at) {
                node.metadata.insert(
                    EVENT_END_AT_METADATA_KEY.to_string(),
                    serde_json::Value::String(end_at.to_rfc3339()),
                );
            }
        }
        _ => {}
    }
    Some(node)
}

fn parse_uuid_param(raw_id: &str, field_name: &str) -> Result<uuid::Uuid, (StatusCode, String)> {
    uuid::Uuid::parse_str(raw_id).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid {field_name} UUID: {err}"),
        )
    })
}

fn sanitize_attachment_filename(raw_name: &str) -> String {
    let base_name = raw_name.rsplit(['/', '\\']).next().unwrap_or(raw_name);
    let mut sanitized = base_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.starts_with('.') {
        sanitized.remove(0);
    }
    while sanitized.contains("..") {
        sanitized = sanitized.replace("..", "_");
    }

    let trimmed = sanitized.trim_matches(|ch| ch == '_' || ch == '.');
    if trimmed.is_empty() {
        "attachment.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_node_attachments(node: &KnowledgeNode) -> Vec<NodeAttachmentRecord> {
    let Some(items) = node
        .metadata
        .get("attachments")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| serde_json::from_value::<NodeAttachmentRecord>(item.clone()).ok())
        .collect()
}

fn is_template_node(node: &KnowledgeNode) -> bool {
    node.kind == NodeKind::Template
        || node
            .metadata
            .get(TEMPLATE_METADATA_KEY)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        || node
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(TEMPLATE_TAG))
}

#[derive(Clone)]
struct TemplatePayload {
    title: Option<String>,
    content: String,
    source: Option<String>,
    tags: Vec<String>,
    importance: f64,
    metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Default)]
struct TemplateMergeSummary {
    filled_fields: Vec<String>,
    overwritten_fields: Vec<String>,
}

fn record_template_field(summary: &mut TemplateMergeSummary, field: &str, overwritten: bool) {
    if overwritten {
        summary.overwritten_fields.push(field.to_string());
    } else {
        summary.filled_fields.push(field.to_string());
    }
}

fn template_payload_from_node(template: &KnowledgeNode) -> TemplatePayload {
    let mut metadata = template.metadata.clone();
    for key in [
        TEMPLATE_METADATA_KEY,
        TEMPLATE_KEY_METADATA_KEY,
        TEMPLATE_VARIABLES_METADATA_KEY,
        TEMPLATE_TARGET_KIND_METADATA_KEY,
        TEMPLATE_VERSIONS_METADATA_KEY,
        TEMPLATE_SOURCE_ID_METADATA_KEY,
        TEMPLATE_INSTANTIATED_AT_METADATA_KEY,
        TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY,
        TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY,
        TEMPLATE_RESTORED_FROM_VERSION_METADATA_KEY,
        TEMPLATE_RESTORED_AT_METADATA_KEY,
        TEMPLATE_PACK_ID_METADATA_KEY,
        TEMPLATE_PACK_NAME_METADATA_KEY,
    ] {
        metadata.remove(key);
    }

    let tags = template
        .tags
        .iter()
        .filter(|tag| !tag.eq_ignore_ascii_case(TEMPLATE_TAG))
        .cloned()
        .collect::<Vec<_>>();

    TemplatePayload {
        title: template.title.clone(),
        content: template.content.clone(),
        source: template.source.clone(),
        tags,
        importance: template.importance,
        metadata,
    }
}

fn merge_template_payload(
    target: &KnowledgeNode,
    payload: &TemplatePayload,
    overwrite: bool,
) -> (KnowledgeNode, TemplateMergeSummary) {
    let mut updated = target.clone();
    let mut summary = TemplateMergeSummary::default();

    let title_empty = updated
        .title
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true);
    if let Some(title) = payload.title.clone() {
        if overwrite || title_empty {
            updated.title = Some(title);
            record_template_field(&mut summary, "title", overwrite);
        }
    }

    if overwrite || updated.content.trim().is_empty() {
        if !payload.content.trim().is_empty() {
            updated.content = payload.content.clone();
            record_template_field(&mut summary, "content", overwrite);
        }
    }

    if overwrite || updated.tags.is_empty() {
        if !payload.tags.is_empty() {
            updated.tags = payload.tags.clone();
            record_template_field(&mut summary, "tags", overwrite);
        }
    }

    let source_empty = updated
        .source
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true);
    if let Some(source) = payload.source.clone() {
        if overwrite || source_empty {
            updated.source = Some(source);
            record_template_field(&mut summary, "source", overwrite);
        }
    }

    if overwrite {
        if (updated.importance - payload.importance).abs() > f64::EPSILON {
            updated.importance = payload.importance;
            record_template_field(&mut summary, "importance", true);
        }
    }

    for (key, value) in payload.metadata.iter() {
        let should_set = overwrite
            || !updated.metadata.contains_key(key)
            || updated
                .metadata
                .get(key)
                .map(|value| value.is_null())
                .unwrap_or(true);
        if should_set {
            updated.metadata.insert(key.clone(), value.clone());
            record_template_field(&mut summary, &format!("metadata.{key}"), overwrite);
        }
    }

    (updated, summary)
}

fn parse_template_target_kind(raw: &str) -> Result<NodeKind, (StatusCode, String)> {
    let parsed: NodeKind = raw
        .trim()
        .parse()
        .map_err(|err: String| (StatusCode::BAD_REQUEST, err))?;
    if matches!(parsed, NodeKind::Template | NodeKind::SavedView) {
        return Err((
            StatusCode::BAD_REQUEST,
            "template target kind cannot be template or saved_view".into(),
        ));
    }
    Ok(parsed)
}

fn resolve_template_target_kind(
    template: &KnowledgeNode,
) -> Result<NodeKind, (StatusCode, String)> {
    if template.kind != NodeKind::Template {
        return Ok(template.kind);
    }
    let raw = template
        .metadata
        .get(TEMPLATE_TARGET_KIND_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .ok_or((
            StatusCode::BAD_REQUEST,
            "template_target_kind is missing".into(),
        ))?;
    parse_template_target_kind(raw)
}

fn normalize_saved_view_name(raw: &str) -> Result<String, (StatusCode, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name is required".into()));
    }
    if trimmed.chars().count() > MAX_SAVED_VIEW_NAME_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("name exceeds {MAX_SAVED_VIEW_NAME_LEN} characters"),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_saved_view_group_by(
    raw: Option<String>,
) -> Result<Option<String>, (StatusCode, String)> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_SAVED_VIEW_GROUP_BY_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("group_by exceeds {MAX_SAVED_VIEW_GROUP_BY_LEN} characters"),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn parse_saved_view_filters(
    raw: Option<serde_json::Value>,
) -> Result<serde_json::Value, (StatusCode, String)> {
    match raw {
        None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        Some(serde_json::Value::Null) => Ok(serde_json::Value::Object(serde_json::Map::new())),
        Some(serde_json::Value::Object(map)) => Ok(serde_json::Value::Object(map)),
        Some(_) => Err((StatusCode::BAD_REQUEST, "filters must be an object".into())),
    }
}

fn parse_saved_view_sort(
    raw: Option<serde_json::Value>,
) -> Result<Option<SavedViewSort>, (StatusCode, String)> {
    let Some(value) = raw else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let sort: SavedViewSort = serde_json::from_value(value).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "sort must include field/direction".into(),
        )
    })?;
    if sort.field.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "sort.field is required".into()));
    }
    Ok(Some(sort))
}

fn parse_saved_view_view_type(raw: &str) -> Result<SavedViewType, (StatusCode, String)> {
    raw.parse()
        .map_err(|err: String| (StatusCode::BAD_REQUEST, err))
}

fn parse_saved_view_query(raw: Option<String>) -> Result<Option<String>, (StatusCode, String)> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_query_text("query", trimmed).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    Ok(Some(trimmed.to_string()))
}

fn is_saved_view_node(node: &KnowledgeNode) -> bool {
    node.kind == NodeKind::SavedView
        || node
            .metadata
            .get(SAVED_VIEW_MARKER_METADATA_KEY)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        || node
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(SAVED_VIEW_TAG))
}

fn saved_view_definition_from_node(node: &KnowledgeNode) -> Option<SavedViewDefinition> {
    if !is_saved_view_node(node) {
        return None;
    }

    let name = node
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Saved View")
        .to_string();

    let view_type = node
        .metadata
        .get(SAVED_VIEW_VIEW_TYPE_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse::<SavedViewType>().ok())?;

    let filters = node
        .metadata
        .get(SAVED_VIEW_FILTERS_METADATA_KEY)
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    let sort = node
        .metadata
        .get(SAVED_VIEW_SORT_METADATA_KEY)
        .and_then(|value| serde_json::from_value::<SavedViewSort>(value.clone()).ok());

    let group_by = node
        .metadata
        .get(SAVED_VIEW_GROUP_BY_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let query = node
        .metadata
        .get(SAVED_VIEW_QUERY_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Some(SavedViewDefinition {
        name,
        view_type,
        filters,
        sort,
        group_by,
        query,
    })
}

fn apply_saved_view_definition(node: &mut KnowledgeNode, definition: &SavedViewDefinition) {
    node.kind = NodeKind::SavedView;
    node.title = Some(definition.name.clone());
    let content = definition
        .query
        .clone()
        .unwrap_or_else(|| definition.name.clone());
    node.content = content;

    if !node
        .tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(SAVED_VIEW_TAG))
    {
        node.tags.push(SAVED_VIEW_TAG.to_string());
    }

    node.metadata.insert(
        SAVED_VIEW_MARKER_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
    node.metadata.insert(
        SAVED_VIEW_VIEW_TYPE_METADATA_KEY.to_string(),
        serde_json::Value::String(definition.view_type.as_str().to_string()),
    );
    node.metadata.insert(
        SAVED_VIEW_FILTERS_METADATA_KEY.to_string(),
        definition.filters.clone(),
    );

    if let Some(sort) = &definition.sort {
        if let Ok(value) = serde_json::to_value(sort) {
            node.metadata
                .insert(SAVED_VIEW_SORT_METADATA_KEY.to_string(), value);
        }
    } else {
        node.metadata.remove(SAVED_VIEW_SORT_METADATA_KEY);
    }

    if let Some(group_by) = &definition.group_by {
        node.metadata.insert(
            SAVED_VIEW_GROUP_BY_METADATA_KEY.to_string(),
            serde_json::Value::String(group_by.clone()),
        );
    } else {
        node.metadata.remove(SAVED_VIEW_GROUP_BY_METADATA_KEY);
    }

    if let Some(query) = &definition.query {
        node.metadata.insert(
            SAVED_VIEW_QUERY_METADATA_KEY.to_string(),
            serde_json::Value::String(query.clone()),
        );
    } else {
        node.metadata.remove(SAVED_VIEW_QUERY_METADATA_KEY);
    }
}

fn saved_view_dto_from_node(node: &KnowledgeNode, definition: SavedViewDefinition) -> SavedViewDto {
    SavedViewDto {
        id: node.id.to_string(),
        name: definition.name,
        namespace: node.namespace.clone(),
        view_type: definition.view_type.as_str().to_string(),
        group_by: definition.group_by,
        query: definition.query,
        filters: definition.filters,
        sort: definition.sort,
        updated_at: node.temporal.updated_at.to_rfc3339(),
    }
}

fn normalize_template_variable_name(raw: &str) -> Option<String> {
    let cleaned = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn normalize_template_variable_list(raw: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for variable in raw {
        let Some(item) = normalize_template_variable_name(variable) else {
            continue;
        };
        if seen.insert(item.clone()) {
            normalized.push(item);
        }
    }
    normalized
}

fn extract_template_placeholders(raw: &str, limit: usize) -> Vec<String> {
    if raw.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut extracted = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut start = 0usize;

    while start < raw.len() && extracted.len() < limit {
        let Some(open_rel) = raw[start..].find("{{") else {
            break;
        };
        let begin = start + open_rel + 2;
        let Some(close_rel) = raw[begin..].find("}}") else {
            break;
        };
        let end = begin + close_rel;
        start = end + 2;

        let candidate = &raw[begin..end];
        let Some(normalized) = normalize_template_variable_name(candidate) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            extracted.push(normalized);
        }
    }

    extracted
}

fn render_template_value_with_values(
    template: &str,
    values: &std::collections::HashMap<String, String>,
) -> String {
    if template.is_empty() || values.is_empty() {
        return template.to_string();
    }

    let mut rendered = template.to_string();
    for (key, value) in values {
        let Some(normalized_key) = normalize_template_variable_name(key) else {
            continue;
        };
        let canonical = format!("{{{{{normalized_key}}}}}");
        let spaced = format!("{{{{ {normalized_key} }}}}");
        rendered = rendered.replace(&canonical, value);
        rendered = rendered.replace(&spaced, value);
    }
    rendered
}

fn merge_tags_case_insensitive(base: &[String], extra: &[String]) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for tag in base.iter().chain(extra.iter()) {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.to_ascii_lowercase();
        if seen.insert(normalized) {
            merged.push(trimmed.to_string());
        }
    }

    merged
}

fn trim_content_preview(raw: &str, max_chars: usize) -> String {
    let mut preview = String::new();
    for ch in raw.chars() {
        if preview.chars().count() >= max_chars {
            break;
        }
        preview.push(ch);
    }
    preview
}

fn template_metadata_for_version_snapshot(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut sanitized = metadata.clone();
    sanitized.remove(TEMPLATE_VERSIONS_METADATA_KEY);
    sanitized.remove(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY);
    sanitized.remove(TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY);
    sanitized.remove(TEMPLATE_RESTORED_FROM_VERSION_METADATA_KEY);
    sanitized.remove(TEMPLATE_RESTORED_AT_METADATA_KEY);
    sanitized
}

fn template_version_snapshot_from_node(node: &KnowledgeNode) -> TemplateVersionRecord {
    TemplateVersionRecord {
        version_id: uuid::Uuid::now_v7().to_string(),
        captured_at: Utc::now().to_rfc3339(),
        kind: node.kind.to_string(),
        namespace: node.namespace.clone(),
        title: node.title.clone(),
        content: node.content.clone(),
        source: node.source.clone(),
        tags: node.tags.clone(),
        importance: node.importance,
        metadata: template_metadata_for_version_snapshot(&node.metadata),
    }
}

fn parse_template_versions_from_metadata(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> Vec<TemplateVersionRecord> {
    let Some(raw_versions) = metadata
        .get(TEMPLATE_VERSIONS_METADATA_KEY)
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };

    raw_versions
        .iter()
        .filter_map(|item| serde_json::from_value::<TemplateVersionRecord>(item.clone()).ok())
        .collect()
}

fn set_template_versions_in_metadata(
    metadata: &mut std::collections::HashMap<String, serde_json::Value>,
    versions: &[TemplateVersionRecord],
) {
    if versions.is_empty() {
        metadata.remove(TEMPLATE_VERSIONS_METADATA_KEY);
        return;
    }
    metadata.insert(
        TEMPLATE_VERSIONS_METADATA_KEY.to_string(),
        serde_json::Value::Array(
            versions
                .iter()
                .filter_map(|item| serde_json::to_value(item).ok())
                .collect(),
        ),
    );
}

fn push_template_version_snapshot(
    versions: &mut Vec<TemplateVersionRecord>,
    snapshot: TemplateVersionRecord,
) {
    versions.push(snapshot);
    if versions.len() > TEMPLATE_VERSION_MAX_ENTRIES {
        let to_drop = versions.len() - TEMPLATE_VERSION_MAX_ENTRIES;
        versions.drain(0..to_drop);
    }
}

fn template_authored_fields_changed(existing: &KnowledgeNode, incoming: &KnowledgeNode) -> bool {
    existing.kind != incoming.kind
        || existing.namespace != incoming.namespace
        || existing.title != incoming.title
        || existing.content != incoming.content
        || existing.source != incoming.source
        || existing.tags != incoming.tags
        || (existing.importance - incoming.importance).abs() > f64::EPSILON
        || template_metadata_for_version_snapshot(&existing.metadata)
            != template_metadata_for_version_snapshot(&incoming.metadata)
}

fn ensure_template_markers(node: &mut KnowledgeNode) {
    node.tags = merge_tags_case_insensitive(&node.tags, &[TEMPLATE_TAG.to_string()]);
    node.metadata.insert(
        TEMPLATE_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
}

fn builtin_template_pack_by_id(pack_id: &str) -> Option<&'static BuiltinTemplatePackDefinition> {
    BUILTIN_TEMPLATE_PACKS
        .iter()
        .find(|pack| pack.id.eq_ignore_ascii_case(pack_id.trim()))
}

fn pack_template_tags(
    definition: &BuiltinTemplateDefinition,
    additional_tags: &[String],
) -> Vec<String> {
    let base_tags = definition
        .tags
        .iter()
        .map(|tag| (*tag).to_string())
        .collect::<Vec<_>>();
    merge_tags_case_insensitive(
        &merge_tags_case_insensitive(&base_tags, additional_tags),
        &[TEMPLATE_TAG.to_string()],
    )
}

fn pack_template_variables(definition: &BuiltinTemplateDefinition) -> Vec<String> {
    definition
        .variables
        .iter()
        .map(|item| (*item).to_string())
        .collect()
}

fn apply_template_pack_metadata(
    metadata: &mut std::collections::HashMap<String, serde_json::Value>,
    pack: &BuiltinTemplatePackDefinition,
    definition: &BuiltinTemplateDefinition,
) {
    metadata.insert(
        TEMPLATE_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
    metadata.insert(
        TEMPLATE_TARGET_KIND_METADATA_KEY.to_string(),
        serde_json::Value::String(definition.kind.to_string()),
    );
    metadata.insert(
        TEMPLATE_KEY_METADATA_KEY.to_string(),
        serde_json::Value::String(definition.key.to_string()),
    );
    metadata.insert(
        TEMPLATE_PACK_ID_METADATA_KEY.to_string(),
        serde_json::Value::String(pack.id.to_string()),
    );
    metadata.insert(
        TEMPLATE_PACK_NAME_METADATA_KEY.to_string(),
        serde_json::Value::String(pack.name.to_string()),
    );

    let variables = pack_template_variables(definition);
    if variables.is_empty() {
        metadata.remove(TEMPLATE_VARIABLES_METADATA_KEY);
    } else {
        metadata.insert(
            TEMPLATE_VARIABLES_METADATA_KEY.to_string(),
            serde_json::Value::Array(
                variables
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect::<Vec<_>>(),
            ),
        );
    }
}

fn template_line_items(raw: &str) -> Vec<String> {
    raw.lines()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn trim_preview_line(raw: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for ch in raw.chars() {
        if output.chars().count() >= max_chars {
            break;
        }
        output.push(ch);
    }
    output
}

fn template_version_diff_summary(
    version_content: &str,
    current_content: &str,
) -> TemplateVersionDiffSummary {
    const MAX_DIFF_SAMPLES: usize = 12;
    const MAX_DIFF_SAMPLE_CHARS: usize = 180;

    let version_lines = template_line_items(version_content);
    let current_lines = template_line_items(current_content);
    let version_set = version_lines
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let current_set = current_lines
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();

    let removed_line_samples = version_lines
        .iter()
        .filter(|line| !current_set.contains(*line))
        .take(MAX_DIFF_SAMPLES)
        .map(|line| trim_preview_line(line, MAX_DIFF_SAMPLE_CHARS))
        .collect::<Vec<_>>();
    let added_line_samples = current_lines
        .iter()
        .filter(|line| !version_set.contains(*line))
        .take(MAX_DIFF_SAMPLES)
        .map(|line| trim_preview_line(line, MAX_DIFF_SAMPLE_CHARS))
        .collect::<Vec<_>>();

    TemplateVersionDiffSummary {
        version_line_count: version_lines.len(),
        current_line_count: current_lines.len(),
        added_line_count: current_lines
            .iter()
            .filter(|line| !version_set.contains(*line))
            .count(),
        removed_line_count: version_lines
            .iter()
            .filter(|line| !current_set.contains(*line))
            .count(),
        added_line_samples,
        removed_line_samples,
    }
}

fn display_optional_text(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| "<empty>".to_string())
}

fn normalize_tags_for_compare(tags: &[String]) -> Vec<String> {
    let mut normalized = tags
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn display_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        "<none>".to_string()
    } else {
        tags.join(", ")
    }
}

fn metadata_string_value(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    metadata
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn metadata_string_list(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
    key: &str,
) -> Vec<String> {
    let Some(values) = metadata.get(key).and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn normalize_string_list_for_compare(raw: &[String]) -> Vec<String> {
    let mut normalized = raw
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn template_version_field_changes(
    version: &TemplateVersionRecord,
    current: &KnowledgeNode,
) -> Vec<TemplateVersionFieldChange> {
    let version_title = display_optional_text(version.title.as_deref());
    let current_title = display_optional_text(current.title.as_deref());
    let version_source = display_optional_text(version.source.as_deref());
    let current_source = display_optional_text(current.source.as_deref());
    let version_tags = display_tags(&version.tags);
    let current_tags = display_tags(&current.tags);
    let version_template_key = display_optional_text(
        metadata_string_value(&version.metadata, TEMPLATE_KEY_METADATA_KEY).as_deref(),
    );
    let current_template_key = display_optional_text(
        metadata_string_value(&current.metadata, TEMPLATE_KEY_METADATA_KEY).as_deref(),
    );
    let version_pack_id = display_optional_text(
        metadata_string_value(&version.metadata, TEMPLATE_PACK_ID_METADATA_KEY).as_deref(),
    );
    let current_pack_id = display_optional_text(
        metadata_string_value(&current.metadata, TEMPLATE_PACK_ID_METADATA_KEY).as_deref(),
    );
    let current_kind = current.kind.to_string();
    let version_variables =
        metadata_string_list(&version.metadata, TEMPLATE_VARIABLES_METADATA_KEY);
    let current_variables =
        metadata_string_list(&current.metadata, TEMPLATE_VARIABLES_METADATA_KEY);
    let version_variables_display = if version_variables.is_empty() {
        "<none>".to_string()
    } else {
        version_variables.join(", ")
    };
    let current_variables_display = if current_variables.is_empty() {
        "<none>".to_string()
    } else {
        current_variables.join(", ")
    };

    vec![
        TemplateVersionFieldChange {
            field: "kind".to_string(),
            changed: !version.kind.eq_ignore_ascii_case(&current_kind),
            version_value: version.kind.clone(),
            current_value: current_kind.clone(),
        },
        TemplateVersionFieldChange {
            field: "namespace".to_string(),
            changed: version.namespace != current.namespace,
            version_value: version.namespace.clone(),
            current_value: current.namespace.clone(),
        },
        TemplateVersionFieldChange {
            field: "title".to_string(),
            changed: version_title != current_title,
            version_value: version_title,
            current_value: current_title,
        },
        TemplateVersionFieldChange {
            field: "source".to_string(),
            changed: version_source != current_source,
            version_value: version_source,
            current_value: current_source,
        },
        TemplateVersionFieldChange {
            field: "importance".to_string(),
            changed: (version.importance - current.importance).abs() > f64::EPSILON,
            version_value: format!("{:.3}", version.importance),
            current_value: format!("{:.3}", current.importance),
        },
        TemplateVersionFieldChange {
            field: "tags".to_string(),
            changed: normalize_tags_for_compare(&version.tags)
                != normalize_tags_for_compare(&current.tags),
            version_value: version_tags,
            current_value: current_tags,
        },
        TemplateVersionFieldChange {
            field: "template_key".to_string(),
            changed: version_template_key != current_template_key,
            version_value: version_template_key,
            current_value: current_template_key,
        },
        TemplateVersionFieldChange {
            field: "template_variables".to_string(),
            changed: normalize_string_list_for_compare(&version_variables)
                != normalize_string_list_for_compare(&current_variables),
            version_value: version_variables_display,
            current_value: current_variables_display,
        },
        TemplateVersionFieldChange {
            field: "template_pack_id".to_string(),
            changed: version_pack_id != current_pack_id,
            version_value: version_pack_id,
            current_value: current_pack_id,
        },
    ]
}

fn upsert_attachment_text_index_entry(
    node: &mut KnowledgeNode,
    attachment_id: &str,
    extracted_text: Option<&str>,
) {
    let mut index_map = node
        .metadata
        .get(ATTACHMENT_TEXT_INDEX_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();

    match extracted_text {
        Some(value) if !value.trim().is_empty() => {
            index_map.insert(
                attachment_id.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
        _ => {
            index_map.remove(attachment_id);
        }
    }

    if index_map.is_empty() {
        node.metadata.remove(ATTACHMENT_TEXT_INDEX_METADATA_KEY);
    } else {
        node.metadata.insert(
            ATTACHMENT_TEXT_INDEX_METADATA_KEY.to_string(),
            serde_json::Value::Object(index_map),
        );
    }
}

fn upsert_attachment_text_chunk_index_entry(
    node: &mut KnowledgeNode,
    attachment_id: &str,
    extracted_text: Option<&str>,
) {
    let mut chunk_index = node
        .metadata
        .get(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();

    let chunks = extracted_text
        .map(|value| {
            split_attachment_search_chunks(
                value,
                MAX_ATTACHMENT_SEARCH_CHUNK_CHARS,
                MAX_ATTACHMENT_SEARCH_CHUNK_COUNT,
            )
        })
        .unwrap_or_default();

    if chunks.is_empty() {
        chunk_index.remove(attachment_id);
    } else {
        let json_chunks = chunks
            .into_iter()
            .map(serde_json::Value::String)
            .collect::<Vec<_>>();
        chunk_index.insert(
            attachment_id.to_string(),
            serde_json::Value::Array(json_chunks),
        );
    }

    if chunk_index.is_empty() {
        node.metadata
            .remove(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY);
    } else {
        node.metadata.insert(
            ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY.to_string(),
            serde_json::Value::Object(chunk_index),
        );
    }
}

fn remove_attachment_text_index_entry(node: &mut KnowledgeNode, attachment_id: &str) {
    upsert_attachment_text_index_entry(node, attachment_id, None);
}

fn remove_attachment_text_chunk_index_entry(node: &mut KnowledgeNode, attachment_id: &str) {
    upsert_attachment_text_chunk_index_entry(node, attachment_id, None);
}

fn truncate_display_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect::<String>()
}

fn attachment_chunks_from_chunk_index(node: &KnowledgeNode, attachment_id: &str) -> Vec<String> {
    let Some(chunk_index) = node
        .metadata
        .get(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };

    let Some(raw_chunks) = chunk_index
        .get(attachment_id)
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };

    raw_chunks
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|chunk| !chunk.is_empty())
        .map(str::to_string)
        .collect()
}

fn attachment_chunks_from_text_index(node: &KnowledgeNode, attachment_id: &str) -> Vec<String> {
    let extracted_text = node
        .metadata
        .get(ATTACHMENT_TEXT_INDEX_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
        .and_then(|index| index.get(attachment_id))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if extracted_text.trim().is_empty() {
        return Vec::new();
    }
    split_attachment_search_chunks(
        extracted_text,
        MAX_ATTACHMENT_SEARCH_CHUNK_CHARS,
        MAX_ATTACHMENT_SEARCH_CHUNK_COUNT,
    )
}

fn resolve_attachment_search_chunks(node: &KnowledgeNode, attachment_id: &str) -> Vec<String> {
    let chunk_indexed = attachment_chunks_from_chunk_index(node, attachment_id);
    if !chunk_indexed.is_empty() {
        return chunk_indexed;
    }
    attachment_chunks_from_text_index(node, attachment_id)
}

fn attachment_chunk_summary(
    node: &KnowledgeNode,
    attachment_id: &str,
) -> (Option<usize>, Option<String>) {
    let chunks = resolve_attachment_search_chunks(node, attachment_id);
    if chunks.is_empty() {
        return (None, None);
    }

    let preview = truncate_display_chars(&chunks[0], MAX_ATTACHMENT_SEARCH_PREVIEW_CHARS);
    (Some(chunks.len()), Some(preview))
}

fn sync_attachment_search_blob_metadata(node: &mut KnowledgeNode) {
    let Some(index_map) = node
        .metadata
        .get(ATTACHMENT_TEXT_INDEX_METADATA_KEY)
        .and_then(serde_json::Value::as_object)
    else {
        node.metadata.remove(ATTACHMENT_SEARCH_BLOB_METADATA_KEY);
        return;
    };

    let mut keys: Vec<&String> = index_map.keys().collect();
    keys.sort_unstable();

    let mut combined_segments = Vec::new();
    for key in keys {
        if let Some(value) = index_map.get(key).and_then(serde_json::Value::as_str) {
            if value.trim().is_empty() {
                continue;
            }
            combined_segments.push(value);
        }
    }

    let combined = normalize_attachment_search_blob(
        &combined_segments.join("\n"),
        MAX_ATTACHMENT_SEARCH_BLOB_CHARS,
    );
    if combined.is_empty() {
        node.metadata.remove(ATTACHMENT_SEARCH_BLOB_METADATA_KEY);
        return;
    }

    node.metadata.insert(
        ATTACHMENT_SEARCH_BLOB_METADATA_KEY.to_string(),
        serde_json::Value::String(combined),
    );
}

async fn resolve_attachment_path(
    state: &AppState,
    node_id: uuid::Uuid,
    attachment: &NodeAttachmentRecord,
) -> Result<PathBuf, (StatusCode, String)> {
    let candidate = PathBuf::from(&attachment.stored_path);
    let canonical_candidate = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "attachment file not found".into()))?;
    let expected_base = PathBuf::from(&state.engine.config.data_dir)
        .join("blobs")
        .join(node_id.to_string());
    let canonical_base = tokio::fs::canonicalize(expected_base).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            "attachment directory not found".into(),
        )
    })?;
    if !canonical_candidate.starts_with(&canonical_base) {
        return Err((
            StatusCode::FORBIDDEN,
            "attachment path outside permitted storage scope".into(),
        ));
    }
    Ok(canonical_candidate)
}

async fn list_nodes_for_export(
    state: &AppState,
    namespace: Option<String>,
) -> Result<Vec<KnowledgeNode>, (StatusCode, String)> {
    let page_size = 500usize;
    let mut offset = 0usize;
    let mut nodes = Vec::new();

    loop {
        let page = state
            .engine
            .list_nodes(
                &QueryFilters {
                    namespace: namespace.clone(),
                    ..Default::default()
                },
                page_size,
                offset,
            )
            .await
            .map_err(map_mv_error)?;

        if page.is_empty() {
            break;
        }
        let page_len = page.len();
        nodes.extend(page);
        if page_len < page_size {
            break;
        }
        offset = offset.saturating_add(page_size);
    }

    Ok(nodes)
}

async fn collect_relationships_for_nodes(
    state: &AppState,
    nodes: &[KnowledgeNode],
) -> Result<Vec<Relationship>, (StatusCode, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut relationships = Vec::new();

    for node in nodes {
        let outgoing = state
            .engine
            .graph
            .get_relationships_from(node.id)
            .await
            .map_err(map_mv_error)?;

        for relationship in outgoing {
            if seen.insert(relationship.id) {
                relationships.push(relationship);
            }
        }
    }

    Ok(relationships)
}

fn parse_optional_iso_date(
    raw: Option<String>,
    field_name: &str,
) -> Result<Option<NaiveDate>, (StatusCode, String)> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map(Some)
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("{field_name} must be YYYY-MM-DD: {err}"),
            )
        })
}

fn parse_optional_rfc3339_datetime(
    raw: Option<String>,
    field_name: &str,
) -> Result<Option<DateTime<Utc>>, (StatusCode, String)> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    DateTime::parse_from_rfc3339(trimmed)
        .map(|value| Some(value.with_timezone(&Utc)))
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("{field_name} must be RFC3339 datetime: {err}"),
            )
        })
}

#[derive(Clone, Copy)]
enum AssistTransformMode {
    Summarize,
    ActionItems,
    Refine,
}

impl AssistTransformMode {
    fn parse(raw_mode: Option<&str>) -> Result<Self, (StatusCode, String)> {
        let normalized = raw_mode.unwrap_or("summarize").trim().to_ascii_lowercase();
        match normalized.as_str() {
            "summarize" | "summary" => Ok(Self::Summarize),
            "action_items" | "action-items" | "actions" | "tasks" => Ok(Self::ActionItems),
            "refine" | "rewrite" | "clarify" => Ok(Self::Refine),
            _ => Err((
                StatusCode::BAD_REQUEST,
                "mode must be one of summarize|action_items|refine".into(),
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Summarize => "summarize",
            Self::ActionItems => "action_items",
            Self::Refine => "refine",
        }
    }
}

// --- Handlers ---

async fn openapi_spec_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        openapi_json(),
    )
}

async fn health(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let count = state.engine.node_count().await.unwrap_or(0);
    Ok(Json(HealthResponse {
        status: "ok".into(),
        node_count: count,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}

async fn embedding_diagnostics(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<EmbeddingProviderDiagnosticsResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;

    let diagnostics = state.engine.embedding_runtime_status();
    Ok(Json(EmbeddingProviderDiagnosticsResponse {
        configured_provider: diagnostics.configured_provider,
        configured_model: diagnostics.configured_model,
        configured_dimensions: diagnostics.configured_dimensions,
        effective_provider: diagnostics.effective_provider,
        effective_model: diagnostics.effective_model,
        effective_dimensions: diagnostics.effective_dimensions,
        fallback_to_noop: diagnostics.fallback_to_noop,
        reason: diagnostics.reason,
        local_embeddings_feature_enabled: diagnostics.local_embeddings_feature_enabled,
    }))
}

async fn assist_completion(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<AssistCompletionRequest>,
) -> Result<Json<AssistCompletionResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    validate_query_text("text", &req.text).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let requested_limit = req.limit.unwrap_or(4);
    validate_recall_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let suggestion_limit = requested_limit.clamp(1, 8);
    let recall_limit = (suggestion_limit * 5).clamp(10, 40);

    let query = MemoryQuery {
        text: req.text.clone(),
        strategy: SearchStrategy::Hybrid,
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, req.namespace.take())?,
            ..Default::default()
        },
        limit: recall_limit,
        min_score: 0.0,
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;
    let suggestions = generate_completion_suggestions(&req.text, &results, suggestion_limit);
    let sources = collect_completion_sources(&results, 5)
        .into_iter()
        .map(|source| AssistSuggestionSourceDto {
            node_id: source.node_id.to_string(),
            title: source.title,
            namespace: source.namespace,
            score: source.score,
        })
        .collect();

    Ok(Json(AssistCompletionResponse {
        suggestions,
        sources,
        source_nodes: results.len(),
        strategy: "retrieval_heuristic_v1".to_string(),
    }))
}

async fn assist_autocomplete(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<AssistAutocompleteRequest>,
) -> Result<Json<AssistAutocompleteResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    validate_query_text("text", &req.text).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let requested_limit = req.limit.unwrap_or(5);
    validate_recall_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let completion_limit = requested_limit.clamp(1, 8);
    let recall_limit = (completion_limit * 5).clamp(10, 40);

    let query = MemoryQuery {
        text: req.text.clone(),
        strategy: SearchStrategy::Hybrid,
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, req.namespace.take())?,
            ..Default::default()
        },
        limit: recall_limit,
        min_score: 0.0,
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;
    let completions = generate_autocomplete_completions(&req.text, &results, completion_limit);

    Ok(Json(AssistAutocompleteResponse {
        completions,
        source_nodes: results.len(),
        strategy: "retrieval_autocomplete_v1".to_string(),
    }))
}

async fn assist_links(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<AssistLinkSuggestionsRequest>,
) -> Result<Json<AssistLinkSuggestionsResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    validate_query_text("text", &req.text).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let requested_limit = req.limit.unwrap_or(6);
    validate_recall_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let suggestion_limit = requested_limit.clamp(1, 10);
    let recall_limit = (suggestion_limit * 6).clamp(24, 80);
    let exclude_node_id = match req.exclude_node_id.take() {
        Some(raw) if !raw.trim().is_empty() => {
            Some(parse_uuid_param(raw.trim(), "exclude_node_id")?)
        }
        _ => None,
    };

    let query = MemoryQuery {
        text: req.text.clone(),
        strategy: SearchStrategy::Hybrid,
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, req.namespace.take())?,
            ..Default::default()
        },
        limit: recall_limit,
        min_score: 0.0,
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;
    let suggestions =
        generate_link_suggestions(&req.text, &results, suggestion_limit, exclude_node_id);
    let suggestion_dtos = suggestions
        .into_iter()
        .map(|candidate| AssistLinkSuggestionDto {
            node_id: candidate.node_id.to_string(),
            title: candidate.title,
            heading: candidate.heading,
            preview: candidate.preview,
            namespace: candidate.namespace,
            score: candidate.score,
            reason: candidate.reason,
        })
        .collect();

    Ok(Json(AssistLinkSuggestionsResponse {
        suggestions: suggestion_dtos,
        source_nodes: results.len(),
        strategy: "retrieval_link_suggestion_v1".to_string(),
    }))
}

async fn assist_transform(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<AssistTransformRequest>,
) -> Result<Json<AssistTransformResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    validate_query_text("text", &req.text).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let mode = AssistTransformMode::parse(req.mode.as_deref())?;
    let requested_limit = req.limit.unwrap_or(4);
    validate_recall_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let transform_limit = requested_limit.clamp(1, 8);
    let recall_limit = (transform_limit * 6).clamp(20, 64);

    let query = MemoryQuery {
        text: req.text.clone(),
        strategy: SearchStrategy::Hybrid,
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, req.namespace.take())?,
            ..Default::default()
        },
        limit: recall_limit,
        min_score: 0.0,
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;
    let transformed_text = match mode {
        AssistTransformMode::Summarize => {
            generate_summary_transform(&req.text, &results, transform_limit.min(6))
        }
        AssistTransformMode::ActionItems => {
            let items =
                generate_action_items_transform(&req.text, &results, transform_limit.min(8));
            items
                .into_iter()
                .map(|item| format!("- [ ] {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        }
        AssistTransformMode::Refine => {
            generate_refine_transform(&req.text, &results, transform_limit.min(6))
        }
    };

    Ok(Json(AssistTransformResponse {
        transformed_text,
        mode: mode.as_str().to_string(),
        source_nodes: results.len(),
        strategy: "retrieval_transform_v1".to_string(),
    }))
}

async fn list_daily_notes(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(mut params): Query<DailyNotesListQuery>,
) -> Result<Json<Vec<KnowledgeNode>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let limit = params.limit.unwrap_or(30);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let offset = params.offset.unwrap_or(0);
    let requested_date = parse_optional_iso_date(params.date.take(), "date")?;

    let requested_namespace = if params.namespace.is_some() {
        params.namespace.take()
    } else if auth.is_admin() {
        Some(state.engine.config.daily_notes.namespace.clone())
    } else {
        None
    };
    let namespace = scoped_namespace(&auth, requested_namespace)?;

    if let Some(date) = requested_date {
        if let Some(ns) = namespace {
            if let Some(note) = state
                .engine
                .find_daily_note(date, &ns)
                .await
                .map_err(map_mv_error)?
            {
                return Ok(Json(vec![note]));
            }
            return Ok(Json(Vec::new()));
        }
        return Err((
            StatusCode::BAD_REQUEST,
            "namespace is required when querying by exact date".into(),
        ));
    }

    let notes = state
        .engine
        .list_daily_notes(namespace, limit, offset)
        .await
        .map_err(map_mv_error)?;

    Ok(Json(notes))
}

async fn ensure_daily_note(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<DailyNotesEnsureRequest>,
) -> Result<(StatusCode, Json<DailyNotesEnsureResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let date = parse_optional_iso_date(req.date.take(), "date")?
        .unwrap_or_else(|| Utc::now().date_naive());
    let namespace = namespace_for_create(
        &auth,
        req.namespace.take(),
        &state.engine.config.daily_notes.namespace,
    )?;

    let existing = state
        .engine
        .find_daily_note(date, &namespace)
        .await
        .map_err(map_mv_error)?;
    let should_check_quota = existing.is_none();
    if should_check_quota {
        enforce_namespace_quota(&state.engine, &namespace)
            .await
            .map_err(map_namespace_quota_error)?;
    }

    let (node, created) = state
        .engine
        .ensure_daily_note(date, Some(namespace.clone()))
        .await
        .map_err(map_mv_error)?;

    if created {
        state.notify_change(&node.id.to_string(), "create", Some(&node.namespace));
    }

    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((status, Json(DailyNotesEnsureResponse { node, created })))
}

async fn list_calendar_items(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(mut params): Query<CalendarItemsQuery>,
) -> Result<Json<CalendarItemsResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let requested_limit = params.limit.unwrap_or(200);
    validate_list_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let include_completed = params.include_completed.unwrap_or(false);
    let namespace = scoped_namespace(&auth, params.namespace.take())?;

    let view = CalendarView::from_query(params.view.take())?;
    let anchor =
        parse_optional_rfc3339_datetime(params.anchor.take(), "anchor")?.unwrap_or_else(Utc::now);
    let explicit_start = parse_optional_rfc3339_datetime(params.start.take(), "start")?;
    let explicit_end = parse_optional_rfc3339_datetime(params.end.take(), "end")?;
    let resolved_window = resolve_calendar_window(anchor, view, explicit_start, explicit_end)?;
    let collected = collect_calendar_items(
        &state,
        namespace,
        resolved_window.range_start,
        resolved_window.range_end,
        include_completed,
        requested_limit,
    )
    .await?;
    let returned_items = collected.items.len();

    Ok(Json(CalendarItemsResponse {
        view: resolved_window.view_label,
        anchor,
        range_start: resolved_window.range_start,
        range_end: resolved_window.range_end,
        total_items: collected.total_items,
        returned_items,
        items: collected.items,
    }))
}

async fn export_calendar_ical(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(mut params): Query<CalendarItemsQuery>,
) -> Result<Response, (StatusCode, String)> {
    authorize_read(&auth)?;
    let requested_limit = params.limit.unwrap_or(500);
    validate_list_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let include_completed = params.include_completed.unwrap_or(false);
    let namespace = scoped_namespace(&auth, params.namespace.take())?;

    let view = CalendarView::from_query(params.view.take())?;
    let anchor =
        parse_optional_rfc3339_datetime(params.anchor.take(), "anchor")?.unwrap_or_else(Utc::now);
    let explicit_start = parse_optional_rfc3339_datetime(params.start.take(), "start")?;
    let explicit_end = parse_optional_rfc3339_datetime(params.end.take(), "end")?;
    let resolved_window = resolve_calendar_window(anchor, view, explicit_start, explicit_end)?;
    let collected = collect_calendar_items(
        &state,
        namespace.clone(),
        resolved_window.range_start,
        resolved_window.range_end,
        include_completed,
        requested_limit,
    )
    .await?;

    let now = Utc::now();
    let document = build_calendar_ical_document(
        &collected.items,
        now,
        resolved_window.range_start,
        resolved_window.range_end,
    );
    let namespace_part = namespace
        .as_deref()
        .map(calendar_filename_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "all".to_string());
    let filename = format!(
        "mindvault-calendar-{}-{}.ics",
        namespace_part,
        now.format("%Y%m%d-%H%M%S")
    );

    let mut response = Response::new(Body::from(document));
    if let Ok(content_type) = HeaderValue::from_str("text/calendar; charset=utf-8") {
        response.headers_mut().insert(CONTENT_TYPE, content_type);
    }
    if let Ok(disposition) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
    {
        response
            .headers_mut()
            .insert(CONTENT_DISPOSITION, disposition);
    }
    Ok(response)
}

async fn import_calendar_ical(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<CalendarIcalImportResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;

    let mut namespace_raw: Option<String> = None;
    let mut overwrite_existing = false;
    let mut default_kind = NodeKind::Event;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid multipart payload: {err}"),
        )
    })? {
        match field.name() {
            Some("namespace") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid namespace field: {err}"),
                    )
                })?;
                if !value.trim().is_empty() {
                    namespace_raw = Some(value.trim().to_string());
                }
            }
            Some("overwrite_existing") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid overwrite_existing field: {err}"),
                    )
                })?;
                overwrite_existing = parse_bool_flag(Some(&value));
            }
            Some("default_kind") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid default_kind field: {err}"),
                    )
                })?;
                default_kind = value
                    .trim()
                    .to_ascii_lowercase()
                    .parse::<NodeKind>()
                    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
                if !matches!(default_kind, NodeKind::Task | NodeKind::Event) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "default_kind must be task or event".into(),
                    ));
                }
            }
            Some("file") => {
                let bytes = field.bytes().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid file field: {err}"),
                    )
                })?;
                if bytes.len() > MAX_ICAL_IMPORT_BYTES {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("file exceeds max size of {} bytes", MAX_ICAL_IMPORT_BYTES),
                    ));
                }
                file_bytes = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let file_bytes = file_bytes.ok_or((StatusCode::BAD_REQUEST, "file is required".into()))?;
    let namespace_was_provided = namespace_raw.is_some();
    let fallback_namespace = namespace_for_create(&auth, namespace_raw.take(), "default")?;
    let raw_ical = String::from_utf8_lossy(&file_bytes);
    let (events, mut parse_errors) = parse_ical_events(&raw_ical);
    if events.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "no valid VEVENT entries found in iCal payload".into(),
        ));
    }

    let mut imported_nodes = 0usize;
    let mut updated_nodes = 0usize;
    let mut skipped_events = 0usize;
    let now = Utc::now();

    for event in events {
        let event_namespace = if namespace_was_provided || !auth.is_admin() {
            fallback_namespace.clone()
        } else {
            namespace_for_create(&auth, event.namespace.clone(), fallback_namespace.as_str())?
        };
        authorize_namespace(&auth, &event_namespace)?;

        let Some(mut node) = node_from_ical_event(&event, &event_namespace, default_kind, now)
        else {
            skipped_events += 1;
            parse_errors += 1;
            continue;
        };

        validate_node_payload(
            node.kind,
            node.title.as_deref(),
            &node.content,
            node.source.as_deref(),
            Some(&node.namespace),
            &node.tags,
            Some(node.importance),
            Some(&node.metadata),
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

        let existing_by_id = state.engine.get_node(node.id).await.map_err(map_mv_error)?;
        let existing = if existing_by_id.is_some() {
            existing_by_id
        } else if let Some(uid) = event.uid.as_deref() {
            find_existing_node_by_ical_uid(&state, &event_namespace, uid).await?
        } else {
            None
        };

        if let Some(existing) = existing {
            if !overwrite_existing {
                skipped_events += 1;
                continue;
            }
            node.id = existing.id;
            node.temporal = existing.temporal;
            node.temporal.updated_at = now;
            node.temporal.last_accessed_at = now;
            node.temporal.version = node.temporal.version.saturating_add(1);

            let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
            state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));
            updated_nodes += 1;
            continue;
        }

        enforce_namespace_quota(&state.engine, &event_namespace)
            .await
            .map_err(map_namespace_quota_error)?;
        let created = state.engine.store_node(node).await.map_err(map_mv_error)?;
        state.notify_change(&created.id.to_string(), "create", Some(&created.namespace));
        imported_nodes += 1;
    }

    Ok(Json(CalendarIcalImportResponse {
        imported_nodes,
        updated_nodes,
        skipped_events,
        parse_errors,
        namespace: fallback_namespace,
    }))
}

async fn list_due_tasks(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(mut params): Query<DueTasksQuery>,
) -> Result<Json<Vec<KnowledgeNode>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let limit = params.limit.unwrap_or(50);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let due_before =
        parse_optional_rfc3339_datetime(params.before.take(), "before")?.unwrap_or_else(Utc::now);
    let namespace = scoped_namespace(&auth, params.namespace.take())?;
    let include_completed = params.include_completed.unwrap_or(false);

    let tasks = state
        .engine
        .list_due_tasks(due_before, namespace, limit, include_completed)
        .await
        .map_err(map_mv_error)?;

    Ok(Json(tasks))
}

async fn prioritize_tasks(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<PrioritizeTasksRequest>,
) -> Result<Json<PrioritizeTasksResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let persist = req.persist.unwrap_or(false);
    if persist {
        authorize_write(&auth)?;
    }

    let requested_limit = req.limit.unwrap_or(20);
    validate_recall_limit(requested_limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let limit = requested_limit.clamp(1, 200);
    let namespace = scoped_namespace(&auth, req.namespace.take())?;
    let now = parse_optional_rfc3339_datetime(req.now.take(), "now")?.unwrap_or_else(Utc::now);

    let options = TaskPrioritizationOptions {
        namespace,
        limit,
        include_completed: req.include_completed.unwrap_or(false),
        include_without_due: req.include_without_due.unwrap_or(true),
        persist,
        now,
    };

    let items = state
        .engine
        .prioritize_tasks(options)
        .await
        .map_err(map_mv_error)?;

    if persist {
        for item in &items {
            state.notify_change(
                &item.task.id.to_string(),
                "update",
                Some(&item.task.namespace),
            );
        }
    }

    Ok(Json(PrioritizeTasksResponse {
        generated_at: now.to_rfc3339(),
        count: items.len(),
        limit,
        strategy: "heuristic_v1".to_string(),
        items,
    }))
}

async fn export_bundle(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(mut params): Query<ExportQuery>,
) -> Result<Json<VaultTransferBundle>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let namespace = scoped_namespace(&auth, params.namespace.take())?;
    let nodes = list_nodes_for_export(&state, namespace.clone()).await?;
    let include_relationships = params.include_relationships.unwrap_or(true);
    let relationships = if include_relationships {
        collect_relationships_for_nodes(&state, &nodes).await?
    } else {
        Vec::new()
    };

    Ok(Json(VaultTransferBundle {
        format_version: EXPORT_FORMAT_VERSION_V1.to_string(),
        exported_at: Utc::now(),
        scope_namespace: namespace,
        nodes,
        relationships,
    }))
}

async fn import_bundle(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<ImportRequest>,
) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;
    if request.nodes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "nodes cannot be empty".into()));
    }
    if request.nodes.len() > IMPORT_MAX_NODES {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("nodes cannot exceed {IMPORT_MAX_NODES}"),
        ));
    }
    let relationships = request.relationships.take().unwrap_or_default();
    if relationships.len() > IMPORT_MAX_RELATIONSHIPS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("relationships cannot exceed {IMPORT_MAX_RELATIONSHIPS}"),
        ));
    }

    let namespace_override = scoped_namespace(&auth, request.namespace_override.take())?;
    let overwrite_existing = request.overwrite_existing.unwrap_or(false);
    let include_relationships = request.include_relationships.unwrap_or(true);

    let mut imported_nodes = 0usize;
    let mut updated_nodes = 0usize;
    let mut skipped_nodes = 0usize;
    let mut imported_relationships = 0usize;
    let mut skipped_relationships = 0usize;

    for mut node in request.nodes {
        if let Some(namespace) = namespace_override.as_ref() {
            node.namespace = namespace.clone();
        }
        authorize_namespace(&auth, &node.namespace)?;
        validate_node_payload(
            node.kind,
            node.title.as_deref(),
            &node.content,
            node.source.as_deref(),
            Some(&node.namespace),
            &node.tags,
            Some(node.importance),
            Some(&node.metadata),
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

        let exists = state
            .engine
            .get_node(node.id)
            .await
            .map_err(map_mv_error)?
            .is_some();
        if exists {
            if overwrite_existing {
                let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
                state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));
                updated_nodes += 1;
            } else {
                skipped_nodes += 1;
            }
        } else {
            let created = state.engine.store_node(node).await.map_err(map_mv_error)?;
            state.notify_change(&created.id.to_string(), "create", Some(&created.namespace));
            imported_nodes += 1;
        }
    }

    if include_relationships {
        for relationship in relationships {
            let from = state
                .engine
                .get_node(relationship.from_node)
                .await
                .map_err(map_mv_error)?;
            let to = state
                .engine
                .get_node(relationship.to_node)
                .await
                .map_err(map_mv_error)?;
            let (Some(from_node), Some(to_node)) = (from, to) else {
                skipped_relationships += 1;
                continue;
            };

            authorize_namespace(&auth, &from_node.namespace)?;
            authorize_namespace(&auth, &to_node.namespace)?;

            let exists = state
                .engine
                .graph
                .get_relationship(relationship.id)
                .await
                .map_err(map_mv_error)?
                .is_some();
            if exists && !overwrite_existing {
                skipped_relationships += 1;
                continue;
            }

            state
                .engine
                .add_relationship(relationship)
                .await
                .map_err(map_mv_error)?;
            imported_relationships += 1;
        }
    }

    Ok(Json(ImportResponse {
        imported_nodes,
        updated_nodes,
        skipped_nodes,
        imported_relationships,
        skipped_relationships,
    }))
}

async fn upload_file(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<FileUploadResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let mut node_id_raw: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid multipart payload: {err}"),
        )
    })? {
        match field.name() {
            Some("node_id") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid node_id field: {err}"),
                    )
                })?;
                node_id_raw = Some(value);
            }
            Some("file") => {
                file_name = Some(sanitize_attachment_filename(
                    field.file_name().unwrap_or("attachment.bin"),
                ));
                content_type = field.content_type().map(str::to_string);
                let bytes = field.bytes().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid file field: {err}"),
                    )
                })?;
                if bytes.len() > MAX_ATTACHMENT_BYTES {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("file exceeds max size of {} bytes", MAX_ATTACHMENT_BYTES),
                    ));
                }
                file_bytes = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let node_id_raw = node_id_raw.ok_or((StatusCode::BAD_REQUEST, "node_id is required".into()))?;
    let node_id = parse_uuid_param(&node_id_raw, "node_id")?;
    let file_name = file_name.ok_or((StatusCode::BAD_REQUEST, "file is required".into()))?;
    let file_bytes = file_bytes.ok_or((StatusCode::BAD_REQUEST, "file is required".into()))?;

    let mut node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let attachment_id = uuid::Uuid::now_v7().to_string();
    let scoped_dir: PathBuf = PathBuf::from(&state.engine.config.data_dir)
        .join("blobs")
        .join(node_id.to_string());
    tokio::fs::create_dir_all(&scoped_dir)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create attachment directory: {err}"),
            )
        })?;

    let stored_file_name = format!("{attachment_id}-{file_name}");
    let stored_path = scoped_dir.join(&stored_file_name);
    tokio::fs::write(&stored_path, &file_bytes)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to persist file: {err}"),
            )
        })?;

    let file_name_for_extraction = file_name.clone();
    let content_type_for_extraction = content_type.clone();
    let bytes_for_extraction = file_bytes.clone();
    let extraction: AttachmentTextExtractionOutcome = tokio::task::spawn_blocking(move || {
        extract_attachment_search_text(
            &file_name_for_extraction,
            content_type_for_extraction.as_deref(),
            &bytes_for_extraction,
            MAX_ATTACHMENT_EXTRACTED_TEXT_CHARS,
        )
    })
    .await
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("attachment extraction task failed: {err}"),
        )
    })?;

    let attachment_record = NodeAttachmentRecord {
        id: attachment_id.clone(),
        file_name: file_name.clone(),
        content_type: content_type.clone(),
        size_bytes: file_bytes.len(),
        stored_path: stored_path.to_string_lossy().to_string(),
        uploaded_at: Some(Utc::now().to_rfc3339()),
        extraction_status: Some(extraction.status.clone()),
        extracted_chars: Some(extraction.extracted_chars),
    };
    let attachment_metadata = serde_json::to_value(&attachment_record).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to serialize attachment metadata: {err}"),
        )
    })?;
    let attachments = node
        .metadata
        .entry("attachments".to_string())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    match attachments {
        serde_json::Value::Array(items) => items.push(attachment_metadata.clone()),
        _ => {
            node.metadata.insert(
                "attachments".to_string(),
                serde_json::Value::Array(vec![attachment_metadata.clone()]),
            );
        }
    }
    upsert_attachment_text_index_entry(
        &mut node,
        &attachment_id,
        extraction.extracted_text.as_deref(),
    );
    upsert_attachment_text_chunk_index_entry(
        &mut node,
        &attachment_id,
        extraction.extracted_text.as_deref(),
    );
    sync_attachment_search_blob_metadata(&mut node);

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));

    Ok((
        StatusCode::CREATED,
        Json(FileUploadResponse {
            attachment_id,
            node_id: node_id.to_string(),
            file_name,
            content_type,
            size_bytes: file_bytes.len(),
            stored_path: stored_path.to_string_lossy().to_string(),
            extraction_status: extraction.status,
            extracted_chars: extraction.extracted_chars,
        }),
    ))
}

/// Upload a voice note with automatic transcription.
///
/// Creates a new node with the transcribed content and attaches the audio file.
/// Optionally links to today's daily note if enabled.
async fn upload_voice_note(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<VoiceNoteUploadResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut namespace: Option<String> = None;
    let mut title: Option<String> = None;
    let mut link_to_daily: bool = true;

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid multipart payload: {err}"),
        )
    })? {
        match field.name() {
            Some("namespace") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid namespace field: {err}"),
                    )
                })?;
                namespace = Some(value);
            }
            Some("title") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid title field: {err}"),
                    )
                })?;
                title = Some(value);
            }
            Some("link_to_daily") => {
                let value = field.text().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid link_to_daily field: {err}"),
                    )
                })?;
                link_to_daily = value.eq_ignore_ascii_case("true") || value == "1";
            }
            Some("file") => {
                file_name = Some(sanitize_attachment_filename(
                    field.file_name().unwrap_or("audio.wav"),
                ));
                content_type = field.content_type().map(str::to_string);
                let bytes = field.bytes().await.map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("invalid file field: {err}"),
                    )
                })?;
                if bytes.len() > MAX_ATTACHMENT_BYTES {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("file exceeds max size of {} bytes", MAX_ATTACHMENT_BYTES),
                    ));
                }
                file_bytes = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let file_name = file_name.ok_or((StatusCode::BAD_REQUEST, "file is required".into()))?;
    let file_bytes = file_bytes.ok_or((StatusCode::BAD_REQUEST, "file is required".into()))?;

    // Validate it's an audio file
    if !is_audio_file(&file_name, content_type.as_deref()) {
        return Err((
            StatusCode::BAD_REQUEST,
            "file must be an audio file (wav, mp3, m4a, webm, etc.)".into(),
        ));
    }

    // Get namespace
    let target_namespace = namespace_for_create(&auth, namespace, "default")?;
    enforce_namespace_quota(&state.engine, &target_namespace)
        .await
        .map_err(|err| match err {
            NamespaceQuotaError::Exceeded {
                namespace,
                quota,
                count,
            } => (
                StatusCode::FORBIDDEN,
                format!("namespace '{namespace}' quota exceeded ({count}/{quota} nodes)"),
            ),
            NamespaceQuotaError::Backend(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        })?;

    // Transcribe audio
    let whisper_config = WhisperConfig::from_env();
    let transcription = if whisper_config.enabled {
        // Use local whisper
        transcribe_audio(&file_name, &file_bytes, &whisper_config).map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("transcription failed: {err}"),
            )
        })?
    } else {
        // Try OpenAI API if OPENAI_API_KEY is set
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .ok_or((
                StatusCode::SERVICE_UNAVAILABLE,
                "transcription not available: whisper binary not found and OPENAI_API_KEY not set"
                    .into(),
            ))?;

        transcribe_audio_api(&file_name, &file_bytes, &api_key)
            .await
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("transcription failed: {err}"),
                )
            })?
    };

    // Generate title if not provided
    let node_title = title.or_else(|| {
        let preview: String = transcription
            .text
            .chars()
            .take(60)
            .collect::<String>()
            .split_whitespace()
            .take(8)
            .collect::<Vec<_>>()
            .join(" ");
        if preview.is_empty() {
            None
        } else {
            Some(format!("Voice: {preview}..."))
        }
    });

    // Create the node.
    let now = Utc::now();
    let mut node = KnowledgeNode::new(NodeKind::Observation, transcription.text.clone())
        .with_namespace(target_namespace.clone())
        .with_tags(vec!["voice-note".into(), "transcription".into()])
        .with_source("capture:voice");

    if let Some(ref node_title_value) = node_title {
        node = node.with_title(node_title_value.clone());
    }

    let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
    let node_id = stored.id;

    // Persist the audio payload as an attachment under the node blob directory.
    let attachment_id = uuid::Uuid::now_v7().to_string();
    let scoped_dir: PathBuf = PathBuf::from(&state.engine.config.data_dir)
        .join("blobs")
        .join(node_id.to_string());
    tokio::fs::create_dir_all(&scoped_dir)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create attachment directory: {err}"),
            )
        })?;

    let stored_file_name = format!("{attachment_id}-{file_name}");
    let stored_path = scoped_dir.join(&stored_file_name);
    tokio::fs::write(&stored_path, &file_bytes)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to write attachment file: {err}"),
            )
        })?;

    // Update node metadata with canonical attachment record + transcription index blob.
    let mut node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;

    let attachment_record = NodeAttachmentRecord {
        id: attachment_id.clone(),
        file_name: file_name.clone(),
        content_type: content_type.clone(),
        size_bytes: file_bytes.len(),
        stored_path: stored_path.to_string_lossy().to_string(),
        uploaded_at: Some(now.to_rfc3339()),
        extraction_status: Some("transcribed".to_string()),
        extracted_chars: Some(transcription.text.chars().count()),
    };
    let attachment_metadata = serde_json::to_value(&attachment_record).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to serialize attachment metadata: {err}"),
        )
    })?;
    let attachments = node
        .metadata
        .entry("attachments".to_string())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    match attachments {
        serde_json::Value::Array(items) => items.push(attachment_metadata),
        _ => {
            node.metadata.insert(
                "attachments".to_string(),
                serde_json::Value::Array(vec![attachment_metadata]),
            );
        }
    }
    upsert_attachment_text_index_entry(
        &mut node,
        &attachment_id,
        Some(transcription.text.as_str()),
    );
    upsert_attachment_text_chunk_index_entry(
        &mut node,
        &attachment_id,
        Some(transcription.text.as_str()),
    );
    sync_attachment_search_blob_metadata(&mut node);

    node.metadata
        .insert("voice_note".into(), serde_json::Value::Bool(true));
    node.metadata.insert(
        "transcription_source".into(),
        serde_json::Value::String(file_name.clone()),
    );

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));

    // Optionally link to daily note in the same namespace.
    let linked_to_daily_note = if link_to_daily && state.engine.config.daily_notes.enabled {
        let today = now.date_naive();
        match state
            .engine
            .ensure_daily_note(today, Some(target_namespace.clone()))
            .await
        {
            Ok((daily_node, _created)) => {
                let relationship =
                    Relationship::new(daily_node.id, updated.id, RelationKind::Contains);
                if let Err(err) = state.engine.add_relationship(relationship).await {
                    tracing::warn!(
                        error = %err,
                        daily_note_id = %daily_node.id,
                        node_id = %updated.id,
                        "mindvault_voice_note_daily_link_failed"
                    );
                    false
                } else {
                    true
                }
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    node_id = %updated.id,
                    namespace = %target_namespace,
                    "mindvault_voice_note_daily_note_ensure_failed"
                );
                false
            }
        }
    } else {
        false
    };

    Ok((
        StatusCode::CREATED,
        Json(VoiceNoteUploadResponse {
            node_id: node_id.to_string(),
            title: node_title,
            transcription: transcription.text.clone(),
            transcription_chars: transcription.text.chars().count(),
            audio_attachment_id: attachment_id,
            linked_to_daily_note,
            created_at: now,
        }),
    ))
}

async fn list_node_attachments(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(node_id_raw): Path<String>,
) -> Result<Json<Vec<AttachmentListItemResponse>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node_id")?;
    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let attachments = parse_node_attachments(&node)
        .into_iter()
        .map(|attachment| {
            let (search_chunk_count, search_preview) =
                attachment_chunk_summary(&node, &attachment.id);
            AttachmentListItemResponse {
                attachment_id: attachment.id.clone(),
                file_name: attachment.file_name.clone(),
                content_type: attachment.content_type.clone(),
                size_bytes: attachment.size_bytes,
                uploaded_at: attachment.uploaded_at.clone(),
                extraction_status: attachment.extraction_status.clone(),
                extracted_chars: attachment.extracted_chars,
                search_chunk_count,
                search_preview,
                download_url: format!("/api/v1/files/{}/{}", node.id, attachment.id),
            }
        })
        .collect();

    Ok(Json(attachments))
}

async fn get_attachment_chunks(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((node_id_raw, attachment_id)): Path<(String, String)>,
    Query(query): Query<AttachmentChunkQuery>,
) -> Result<Json<AttachmentChunkListResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node_id")?;
    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let attachments = parse_node_attachments(&node);
    let attachment = attachments
        .iter()
        .find(|item| item.id == attachment_id)
        .ok_or((StatusCode::NOT_FOUND, "attachment not found".into()))?;

    let chunks = resolve_attachment_search_chunks(&node, &attachment_id);
    let total_chunks = chunks.len();
    let limit = query
        .limit
        .unwrap_or(DEFAULT_ATTACHMENT_CHUNK_PAGE_SIZE)
        .clamp(1, MAX_ATTACHMENT_CHUNK_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0);
    let end = offset.saturating_add(limit).min(total_chunks);

    let items = if offset >= total_chunks {
        Vec::new()
    } else {
        chunks[offset..end]
            .iter()
            .enumerate()
            .map(|(position, text)| AttachmentChunkItemResponse {
                index: offset + position,
                text: text.to_string(),
                char_count: text.chars().count(),
            })
            .collect::<Vec<_>>()
    };

    Ok(Json(AttachmentChunkListResponse {
        node_id: node.id.to_string(),
        attachment_id,
        extraction_status: attachment.extraction_status.clone(),
        extracted_chars: attachment.extracted_chars,
        total_chunks,
        offset,
        limit,
        returned_chunks: items.len(),
        chunks: items,
    }))
}

async fn reindex_attachment(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((node_id_raw, attachment_id)): Path<(String, String)>,
) -> Result<Json<AttachmentReindexResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node_id")?;
    let mut node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let mut attachments = parse_node_attachments(&node);
    let Some(index) = attachments.iter().position(|item| item.id == attachment_id) else {
        return Err((StatusCode::NOT_FOUND, "attachment not found".into()));
    };
    let mut attachment = attachments[index].clone();

    if attachment
        .extraction_status
        .as_deref()
        .is_some_and(|status| status.eq_ignore_ascii_case("transcribed"))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "transcribed attachments are indexed from voice transcription".into(),
        ));
    }

    let safe_path = resolve_attachment_path(&state, node_id, &attachment).await?;
    let file_bytes = tokio::fs::read(&safe_path).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read attachment file: {err}"),
        )
    })?;
    let file_name_for_extraction = attachment.file_name.clone();
    let content_type_for_extraction = attachment.content_type.clone();
    let extraction: AttachmentTextExtractionOutcome = tokio::task::spawn_blocking(move || {
        extract_attachment_search_text(
            &file_name_for_extraction,
            content_type_for_extraction.as_deref(),
            &file_bytes,
            MAX_ATTACHMENT_EXTRACTED_TEXT_CHARS,
        )
    })
    .await
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("attachment extraction task failed: {err}"),
        )
    })?;

    attachment.extraction_status = Some(extraction.status.clone());
    attachment.extracted_chars = Some(extraction.extracted_chars);
    attachments[index] = attachment;
    let attachment_metadata = attachments
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to serialize attachment metadata: {err}"),
            )
        })?;
    node.metadata.insert(
        "attachments".to_string(),
        serde_json::Value::Array(attachment_metadata),
    );

    upsert_attachment_text_index_entry(
        &mut node,
        &attachment_id,
        extraction.extracted_text.as_deref(),
    );
    upsert_attachment_text_chunk_index_entry(
        &mut node,
        &attachment_id,
        extraction.extracted_text.as_deref(),
    );
    sync_attachment_search_blob_metadata(&mut node);

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));
    let (search_chunk_count, search_preview) = attachment_chunk_summary(&updated, &attachment_id);

    Ok(Json(AttachmentReindexResponse {
        node_id: updated.id.to_string(),
        attachment_id,
        extraction_status: extraction.status,
        extracted_chars: extraction.extracted_chars,
        search_chunk_count,
        search_preview,
    }))
}

async fn download_attachment(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((node_id_raw, attachment_id)): Path<(String, String)>,
    Query(query): Query<AttachmentDownloadQuery>,
) -> Result<Response, (StatusCode, String)> {
    authorize_read(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node_id")?;
    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let attachments = parse_node_attachments(&node);
    let attachment = attachments
        .iter()
        .find(|item| item.id == attachment_id)
        .ok_or((StatusCode::NOT_FOUND, "attachment not found".into()))?;
    let safe_path = resolve_attachment_path(&state, node_id, attachment).await?;
    let bytes = tokio::fs::read(&safe_path).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read file: {err}"),
        )
    })?;

    let mut response = Response::new(Body::from(bytes));
    let content_type = attachment
        .content_type
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("application/octet-stream");
    if let Ok(value) = HeaderValue::from_str(content_type) {
        response.headers_mut().insert(CONTENT_TYPE, value);
    }
    let file_name = sanitize_attachment_filename(&attachment.file_name).replace('"', "_");
    let disposition = if query.inline.unwrap_or(false) {
        "inline"
    } else {
        "attachment"
    };
    if let Ok(value) = HeaderValue::from_str(&format!("{disposition}; filename=\"{file_name}\"")) {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }

    Ok(response)
}

async fn delete_attachment(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((node_id_raw, attachment_id)): Path<(String, String)>,
) -> Result<Json<AttachmentDeleteResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node_id")?;
    let mut node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let mut attachments = parse_node_attachments(&node);
    let Some(index) = attachments.iter().position(|item| item.id == attachment_id) else {
        return Err((StatusCode::NOT_FOUND, "attachment not found".into()));
    };
    let removed = attachments.remove(index);

    let file_deleted = match resolve_attachment_path(&state, node_id, &removed).await {
        Ok(path) => match tokio::fs::remove_file(path).await {
            Ok(_) => true,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
            Err(err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to delete attachment file: {err}"),
                ));
            }
        },
        Err((StatusCode::NOT_FOUND, _)) => false,
        Err(other) => return Err(other),
    };

    if attachments.is_empty() {
        node.metadata.remove("attachments");
    } else {
        let items: Vec<serde_json::Value> = attachments
            .iter()
            .filter_map(|item| serde_json::to_value(item).ok())
            .collect();
        node.metadata
            .insert("attachments".to_string(), serde_json::Value::Array(items));
    }
    remove_attachment_text_index_entry(&mut node, &attachment_id);
    remove_attachment_text_chunk_index_entry(&mut node, &attachment_id);
    sync_attachment_search_blob_metadata(&mut node);

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));

    Ok(Json(AttachmentDeleteResponse {
        attachment_id,
        file_deleted,
        remaining_attachments: attachments.len(),
    }))
}

async fn set_task_completion_status(
    auth: AuthContext,
    state: Arc<AppState>,
    id: String,
    completed: bool,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let uuid = parse_uuid_param(&id, "task id")?;

    let mut node = state
        .engine
        .get_node(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "task not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    if node.kind != NodeKind::Task {
        return Err((
            StatusCode::BAD_REQUEST,
            "completion status can only be updated for kind=task".into(),
        ));
    }

    node.metadata.insert(
        TASK_COMPLETED_METADATA_KEY.into(),
        serde_json::Value::Bool(completed),
    );

    if completed {
        node.metadata.insert(
            TASK_COMPLETED_AT_METADATA_KEY.into(),
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );
        node.metadata.insert(
            TASK_REMINDER_STATUS_METADATA_KEY.into(),
            serde_json::Value::String("acknowledged".to_string()),
        );
    } else {
        node.metadata.remove(TASK_COMPLETED_AT_METADATA_KEY);
        node.metadata.remove(TASK_REMINDER_STATUS_METADATA_KEY);
        node.metadata.remove(TASK_REMINDER_SENT_AT_METADATA_KEY);
    }

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&id, "update", Some(&updated.namespace));

    Ok(Json(updated))
}

async fn complete_task(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    set_task_completion_status(auth, state, id, true).await
}

async fn reopen_task(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    set_task_completion_status(auth, state, id, false).await
}

async fn snooze_task_reminder(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SnoozeTaskReminderRequest>,
) -> Result<Json<SnoozeTaskReminderResponse>, (StatusCode, String)> {
    authorize_write(&auth)?;

    let node_id = id
        .parse::<uuid::Uuid>()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid node ID".into()))?;

    let mut node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;

    authorize_namespace(&auth, &node.namespace)?;

    if node.kind != NodeKind::Task {
        return Err((StatusCode::BAD_REQUEST, "node is not a task".into()));
    }

    // Clear the reminder sent flag so it can be sent again
    node.metadata.remove(TASK_REMINDER_SENT_AT_METADATA_KEY);
    node.metadata.remove(TASK_REMINDER_STATUS_METADATA_KEY);

    let mut new_due_at: Option<DateTime<Utc>> = None;

    // If snooze_minutes is provided, update the due_at time
    if let Some(minutes) = req.snooze_minutes {
        let now = Utc::now();
        let snoozed_due = now + Duration::minutes(minutes as i64);
        node.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(snoozed_due.to_rfc3339()),
        );
        new_due_at = Some(snoozed_due);
    }

    node.temporal.updated_at = Utc::now();
    node.temporal.version = node.temporal.version.saturating_add(1);

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&id, "update", Some(&updated.namespace));

    Ok(Json(SnoozeTaskReminderResponse {
        node_id: updated.id.to_string(),
        reminder_cleared: true,
        new_due_at,
    }))
}

async fn list_template_packs(
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<TemplatePackSummary>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let packs = BUILTIN_TEMPLATE_PACKS
        .iter()
        .map(|pack| TemplatePackSummary {
            pack_id: pack.id.to_string(),
            name: pack.name.to_string(),
            description: pack.description.to_string(),
            template_count: pack.templates.len(),
        })
        .collect::<Vec<_>>();
    Ok(Json(packs))
}

async fn install_template_pack(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(pack_id_raw): Path<String>,
    Json(mut req): Json<InstallTemplatePackRequest>,
) -> Result<(StatusCode, Json<InstallTemplatePackResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;
    let pack = builtin_template_pack_by_id(&pack_id_raw)
        .ok_or((StatusCode::NOT_FOUND, "template pack not found".into()))?;

    let target_namespace = namespace_for_create(&auth, req.namespace.take(), "default")?;
    let overwrite_existing = req.overwrite_existing.unwrap_or(false);
    let additional_tags = req.additional_tags.take().unwrap_or_default();

    let template_filters = QueryFilters {
        namespace: Some(target_namespace.clone()),
        tags: Some(vec![TEMPLATE_TAG.to_string()]),
        ..Default::default()
    };

    let mut existing_by_key = std::collections::HashMap::new();
    let mut offset = 0usize;
    loop {
        let batch = state
            .engine
            .list_nodes(&template_filters, 500, offset)
            .await
            .map_err(map_mv_error)?;
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        for node in batch {
            if !is_template_node(&node) {
                continue;
            }
            let Some(template_key) = node
                .metadata
                .get(TEMPLATE_KEY_METADATA_KEY)
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            existing_by_key.insert(template_key.to_ascii_lowercase(), node);
        }
        if batch_len < 500 {
            break;
        }
        offset += batch_len;
    }

    let mut installed_templates = 0usize;
    let mut updated_templates = 0usize;
    let mut skipped_templates = 0usize;
    let mut template_ids = Vec::new();

    for definition in pack.templates {
        let template_key_normalized = definition.key.to_ascii_lowercase();
        let tags = pack_template_tags(definition, &additional_tags);

        if let Some(existing_template) = existing_by_key.get(&template_key_normalized).cloned() {
            if !overwrite_existing {
                skipped_templates += 1;
                continue;
            }

            let mut updated_payload = existing_template.clone();
            updated_payload.kind = NodeKind::Template;
            updated_payload.title = Some(definition.title.to_string());
            updated_payload.content = definition.content.to_string();
            updated_payload.source = definition.source.map(|source| source.to_string());
            updated_payload.namespace = target_namespace.clone();
            updated_payload.tags = tags;
            updated_payload.importance = definition.importance;
            apply_template_pack_metadata(&mut updated_payload.metadata, pack, definition);

            let Json(updated) = update_node(
                Extension(auth.clone()),
                State(Arc::clone(&state)),
                Path(existing_template.id.to_string()),
                Json(updated_payload),
            )
            .await?;
            template_ids.push(updated.id.to_string());
            existing_by_key.insert(template_key_normalized, updated);
            updated_templates += 1;
        } else {
            let mut metadata = std::collections::HashMap::new();
            apply_template_pack_metadata(&mut metadata, pack, definition);
            let (_status, Json(created)) = create_template(
                Extension(auth.clone()),
                State(Arc::clone(&state)),
                Json(CreateTemplateRequest {
                    kind: definition.kind.to_string(),
                    content: definition.content.to_string(),
                    title: Some(definition.title.to_string()),
                    source: definition.source.map(|source| source.to_string()),
                    namespace: Some(target_namespace.clone()),
                    tags: Some(tags),
                    importance: Some(definition.importance),
                    metadata: Some(metadata),
                    template_key: Some(definition.key.to_string()),
                    template_variables: Some(pack_template_variables(definition)),
                }),
            )
            .await?;
            template_ids.push(created.id.to_string());
            existing_by_key.insert(template_key_normalized, created);
            installed_templates += 1;
        }
    }

    Ok((
        StatusCode::OK,
        Json(InstallTemplatePackResponse {
            pack_id: pack.id.to_string(),
            namespace: target_namespace,
            installed_templates,
            updated_templates,
            skipped_templates,
            template_ids,
        }),
    ))
}

async fn list_templates(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<TemplateListQuery>,
) -> Result<Json<Vec<KnowledgeNode>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let namespace = scoped_namespace(&auth, params.namespace)?;
    let target_kind = match params.kind {
        Some(raw) => Some(parse_template_target_kind(&raw)?),
        None => None,
    };
    let limit = params.limit.unwrap_or(100);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let offset = params.offset.unwrap_or(0);

    let filters = QueryFilters {
        namespace,
        tags: Some(vec![TEMPLATE_TAG.to_string()]),
        ..Default::default()
    };
    let templates = state
        .engine
        .list_nodes(&filters, limit, offset)
        .await
        .map_err(map_mv_error)?
        .into_iter()
        .filter(is_template_node)
        .filter(|node| {
            if let Some(target_kind) = target_kind {
                resolve_template_target_kind(node)
                    .map(|kind| kind == target_kind)
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();
    Ok(Json(templates))
}

async fn create_template(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<KnowledgeNode>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let tags = req.tags.take().unwrap_or_default();
    let target_kind = parse_template_target_kind(&req.kind)?;

    validate_node_payload(
        NodeKind::Template,
        req.title.as_deref(),
        &req.content,
        req.source.as_deref(),
        req.namespace.as_deref(),
        &tags,
        req.importance,
        req.metadata.as_ref(),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let namespace = namespace_for_create(&auth, req.namespace.take(), "default")?;
    enforce_namespace_quota(&state.engine, &namespace)
        .await
        .map_err(map_namespace_quota_error)?;

    let mut template =
        KnowledgeNode::new(NodeKind::Template, req.content).with_namespace(namespace);
    if let Some(title) = req.title {
        template = template.with_title(title);
    }
    if let Some(source) = req.source {
        template = template.with_source(source);
    }
    if let Some(importance) = req.importance {
        template = template.with_importance(importance);
    }

    let template_tags = merge_tags_case_insensitive(&tags, &[TEMPLATE_TAG.to_string()]);
    template = template.with_tags(template_tags);

    let mut metadata = req.metadata.take().unwrap_or_default();
    metadata.insert(
        TEMPLATE_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
    metadata.insert(
        TEMPLATE_TARGET_KIND_METADATA_KEY.to_string(),
        serde_json::Value::String(target_kind.as_str().to_string()),
    );
    if let Some(template_key) = req.template_key.take() {
        let key = template_key.trim();
        if !key.is_empty() {
            metadata.insert(
                TEMPLATE_KEY_METADATA_KEY.to_string(),
                serde_json::Value::String(key.to_string()),
            );
        }
    }

    let template_variables = if let Some(raw_variables) = req.template_variables.take() {
        normalize_template_variable_list(&raw_variables)
    } else {
        let mut discovered = extract_template_placeholders(&template.content, 128);
        if let Some(title) = template.title.as_deref() {
            let mut from_title = extract_template_placeholders(title, 128);
            discovered.append(&mut from_title);
        }
        normalize_template_variable_list(&discovered)
    };
    if !template_variables.is_empty() {
        metadata.insert(
            TEMPLATE_VARIABLES_METADATA_KEY.to_string(),
            serde_json::Value::Array(
                template_variables
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    template.metadata = metadata;

    let stored = state
        .engine
        .store_node(template)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
    Ok((StatusCode::CREATED, Json(stored)))
}

async fn update_template(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(template_id_raw): Path<String>,
    Json(mut req): Json<UpdateTemplateRequest>,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let existing = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &existing.namespace)?;

    if !is_template_node(&existing) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let mut updated = existing.clone();
    updated.kind = NodeKind::Template;

    let content_changed = req.content.is_some();
    let title_changed = req.title.is_some();

    if let Some(title) = req.title.take() {
        updated.title = Some(title);
    }
    if let Some(content) = req.content.take() {
        updated.content = content;
    }
    if let Some(source) = req.source.take() {
        updated.source = Some(source);
    }
    if let Some(tags) = req.tags.take() {
        updated.tags = merge_tags_case_insensitive(&tags, &[TEMPLATE_TAG.to_string()]);
    } else {
        ensure_template_markers(&mut updated);
    }
    if let Some(importance) = req.importance.take() {
        updated.importance = importance;
    }

    if let Some(metadata) = req.metadata.take() {
        for (key, value) in metadata {
            if value.is_null() {
                updated.metadata.remove(&key);
            } else {
                updated.metadata.insert(key, value);
            }
        }
    }

    if let Some(raw_target_kind) = req.target_kind.take() {
        let target_kind = parse_template_target_kind(&raw_target_kind)?;
        updated.metadata.insert(
            TEMPLATE_TARGET_KIND_METADATA_KEY.to_string(),
            serde_json::Value::String(target_kind.as_str().to_string()),
        );
    } else if !updated
        .metadata
        .contains_key(TEMPLATE_TARGET_KIND_METADATA_KEY)
    {
        let fallback_kind = if existing.kind == NodeKind::Template {
            None
        } else {
            Some(existing.kind)
        };
        if let Some(kind) = fallback_kind {
            updated.metadata.insert(
                TEMPLATE_TARGET_KIND_METADATA_KEY.to_string(),
                serde_json::Value::String(kind.as_str().to_string()),
            );
        }
    }

    if let Some(template_key) = req.template_key.take() {
        let trimmed = template_key.trim();
        if trimmed.is_empty() {
            updated.metadata.remove(TEMPLATE_KEY_METADATA_KEY);
        } else {
            updated.metadata.insert(
                TEMPLATE_KEY_METADATA_KEY.to_string(),
                serde_json::Value::String(trimmed.to_string()),
            );
        }
    }

    if let Some(raw_variables) = req.template_variables.take() {
        let normalized = normalize_template_variable_list(&raw_variables);
        if normalized.is_empty() {
            updated.metadata.remove(TEMPLATE_VARIABLES_METADATA_KEY);
        } else {
            updated.metadata.insert(
                TEMPLATE_VARIABLES_METADATA_KEY.to_string(),
                serde_json::Value::Array(
                    normalized
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
    } else if content_changed || title_changed {
        let mut discovered = extract_template_placeholders(&updated.content, 128);
        if let Some(title) = updated.title.as_deref() {
            let mut from_title = extract_template_placeholders(title, 128);
            discovered.append(&mut from_title);
        }
        let normalized = normalize_template_variable_list(&discovered);
        if normalized.is_empty() {
            updated.metadata.remove(TEMPLATE_VARIABLES_METADATA_KEY);
        } else {
            updated.metadata.insert(
                TEMPLATE_VARIABLES_METADATA_KEY.to_string(),
                serde_json::Value::Array(
                    normalized
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
    }

    validate_node_payload(
        NodeKind::Template,
        updated.title.as_deref(),
        &updated.content,
        updated.source.as_deref(),
        Some(&updated.namespace),
        &updated.tags,
        Some(updated.importance),
        Some(&updated.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let updated = state
        .engine
        .update_node(updated)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));
    Ok(Json(updated))
}

async fn instantiate_template(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(template_id_raw): Path<String>,
    Json(mut req): Json<InstantiateTemplateRequest>,
) -> Result<(StatusCode, Json<KnowledgeNode>), (StatusCode, String)> {
    authorize_write(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let target_kind = resolve_template_target_kind(&template)?;

    let target_namespace =
        namespace_for_create(&auth, req.namespace.take(), template.namespace.as_str())?;
    enforce_namespace_quota(&state.engine, &target_namespace)
        .await
        .map_err(map_namespace_quota_error)?;

    let values = req.values.take().unwrap_or_default();
    let mut normalized_values = std::collections::HashMap::new();
    for (key, value) in values {
        if let Some(normalized_key) = normalize_template_variable_name(&key) {
            normalized_values.insert(normalized_key, value);
        }
    }

    let rendered_content = render_template_value_with_values(&template.content, &normalized_values);
    let rendered_title = req.title.take().or_else(|| {
        template
            .title
            .as_deref()
            .map(|title| render_template_value_with_values(title, &normalized_values))
    });

    let base_tags: Vec<String> = template
        .tags
        .iter()
        .filter(|tag| !tag.eq_ignore_ascii_case(TEMPLATE_TAG))
        .cloned()
        .collect();
    let request_tags = req.tags.take().unwrap_or_default();
    let merged_tags = merge_tags_case_insensitive(&base_tags, &request_tags);

    let mut metadata = template.metadata.clone();
    metadata.remove(TEMPLATE_METADATA_KEY);
    metadata.remove(TEMPLATE_KEY_METADATA_KEY);
    metadata.remove(TEMPLATE_VARIABLES_METADATA_KEY);
    metadata.remove(TEMPLATE_TARGET_KIND_METADATA_KEY);
    metadata.insert(
        TEMPLATE_SOURCE_ID_METADATA_KEY.to_string(),
        serde_json::Value::String(template.id.to_string()),
    );
    metadata.insert(
        TEMPLATE_INSTANTIATED_AT_METADATA_KEY.to_string(),
        serde_json::Value::String(Utc::now().to_rfc3339()),
    );
    if let Some(extra_metadata) = req.metadata.take() {
        for (key, value) in extra_metadata {
            metadata.insert(key, value);
        }
    }

    validate_node_payload(
        target_kind,
        rendered_title.as_deref(),
        &rendered_content,
        template.source.as_deref(),
        Some(&target_namespace),
        &merged_tags,
        Some(template.importance),
        Some(&metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let mut instance =
        KnowledgeNode::new(target_kind, rendered_content).with_namespace(target_namespace);
    if let Some(title) = rendered_title {
        instance = instance.with_title(title);
    }
    if let Some(source) = template.source.clone() {
        instance = instance.with_source(source);
    }
    instance = instance
        .with_tags(merged_tags)
        .with_importance(template.importance);
    instance.metadata = metadata;

    let stored = state
        .engine
        .store_node(instance)
        .await
        .map_err(map_mv_error)?;
    state
        .engine
        .add_relationship(Relationship::new(
            template.id,
            stored.id,
            RelationKind::DerivedFrom,
        ))
        .await
        .map_err(map_mv_error)?;

    // Track template usage for ranking and UX hints without requiring external analytics.
    let mut template_usage = template.clone();
    let next_count = template_usage
        .metadata
        .get(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        + 1;
    template_usage.metadata.insert(
        TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY.to_string(),
        serde_json::Value::from(next_count),
    );
    template_usage.metadata.insert(
        TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY.to_string(),
        serde_json::Value::String(Utc::now().to_rfc3339()),
    );
    if let Ok(updated_template) = state.engine.update_node(template_usage).await {
        state.notify_change(
            &updated_template.id.to_string(),
            "update",
            Some(&updated_template.namespace),
        );
    }

    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
    Ok((StatusCode::CREATED, Json(stored)))
}

async fn apply_template(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(template_id_raw): Path<String>,
    Json(req): Json<ApplyTemplateRequest>,
) -> Result<(StatusCode, Json<TemplateApplyResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let overwrite = req.overwrite.unwrap_or(false);
    let now = Utc::now().to_rfc3339();

    let mut payload = template_payload_from_node(&template);
    payload.metadata.insert(
        TEMPLATE_SOURCE_ID_METADATA_KEY.to_string(),
        serde_json::Value::String(template.id.to_string()),
    );
    payload.metadata.insert(
        TEMPLATE_INSTANTIATED_AT_METADATA_KEY.to_string(),
        serde_json::Value::String(now),
    );

    let target_kind_override = match req.target_kind {
        Some(raw) => Some(parse_template_target_kind(&raw)?),
        None => None,
    };

    let (status, response) = if let Some(target_id_raw) = req.target_node_id {
        let target_id = parse_uuid_param(&target_id_raw, "target node id")?;
        let target = state
            .engine
            .get_node(target_id)
            .await
            .map_err(map_mv_error)?
            .ok_or((StatusCode::NOT_FOUND, "target node not found".into()))?;
        authorize_namespace(&auth, &target.namespace)?;

        if matches!(target.kind, NodeKind::Template | NodeKind::SavedView) {
            return Err((
                StatusCode::BAD_REQUEST,
                "cannot apply template to template/saved_view".into(),
            ));
        }

        let template_kind =
            target_kind_override.unwrap_or(resolve_template_target_kind(&template)?);
        if template_kind != target.kind {
            return Err((
                StatusCode::BAD_REQUEST,
                "target_kind does not match existing node kind".into(),
            ));
        }

        let (updated, mut summary) = merge_template_payload(&target, &payload, overwrite);

        validate_node_payload(
            updated.kind,
            updated.title.as_deref(),
            &updated.content,
            updated.source.as_deref(),
            Some(&updated.namespace),
            &updated.tags,
            Some(updated.importance),
            Some(&updated.metadata),
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

        let updated = state
            .engine
            .update_node(updated)
            .await
            .map_err(map_mv_error)?;

        let existing_relationships = state
            .engine
            .graph
            .get_relationships_from(updated.id)
            .await
            .map_err(map_mv_error)?;
        let template_relationships = state
            .engine
            .graph
            .get_relationships_from(template.id)
            .await
            .map_err(map_mv_error)?;

        let should_copy_relations = overwrite || existing_relationships.is_empty();
        if should_copy_relations {
            if overwrite {
                for rel in &existing_relationships {
                    state
                        .engine
                        .graph
                        .remove_relationship(rel.id)
                        .await
                        .map_err(map_mv_error)?;
                }
            }
            for rel in template_relationships {
                if rel.kind == RelationKind::DerivedFrom {
                    continue;
                }
                if rel.to_node == updated.id {
                    continue;
                }
                let mut cloned = Relationship::new(updated.id, rel.to_node, rel.kind);
                cloned.weight = rel.weight;
                cloned.metadata = rel.metadata.clone();
                state
                    .engine
                    .graph
                    .add_relationship(&cloned)
                    .await
                    .map_err(map_mv_error)?;
            }
            record_template_field(&mut summary, "relations", overwrite);
        }

        let derived_exists = state
            .engine
            .graph
            .get_relationships_from(template.id)
            .await
            .map_err(map_mv_error)?
            .iter()
            .any(|rel| rel.to_node == updated.id && rel.kind == RelationKind::DerivedFrom);
        if !derived_exists {
            state
                .engine
                .graph
                .add_relationship(&Relationship::new(
                    template.id,
                    updated.id,
                    RelationKind::DerivedFrom,
                ))
                .await
                .map_err(map_mv_error)?;
        }

        (
            StatusCode::OK,
            TemplateApplyResponse {
                node: updated,
                created: false,
                filled_fields: summary.filled_fields,
                overwritten_fields: summary.overwritten_fields,
            },
        )
    } else {
        let target_kind = target_kind_override.unwrap_or(resolve_template_target_kind(&template)?);
        let mut node = KnowledgeNode::new(target_kind, payload.content.clone())
            .with_namespace(template.namespace.clone());
        if let Some(title) = payload.title.clone() {
            node = node.with_title(title);
        }
        if let Some(source) = payload.source.clone() {
            node = node.with_source(source);
        }
        if !payload.tags.is_empty() {
            node = node.with_tags(payload.tags.clone());
        }
        node = node.with_importance(payload.importance);
        node.metadata = payload.metadata.clone();

        validate_node_payload(
            node.kind,
            node.title.as_deref(),
            &node.content,
            node.source.as_deref(),
            Some(&node.namespace),
            &node.tags,
            Some(node.importance),
            Some(&node.metadata),
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

        let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;

        let template_relationships = state
            .engine
            .graph
            .get_relationships_from(template.id)
            .await
            .map_err(map_mv_error)?;
        for rel in template_relationships {
            if rel.kind == RelationKind::DerivedFrom {
                continue;
            }
            let mut cloned = Relationship::new(stored.id, rel.to_node, rel.kind);
            cloned.weight = rel.weight;
            cloned.metadata = rel.metadata.clone();
            state
                .engine
                .graph
                .add_relationship(&cloned)
                .await
                .map_err(map_mv_error)?;
        }
        state
            .engine
            .graph
            .add_relationship(&Relationship::new(
                template.id,
                stored.id,
                RelationKind::DerivedFrom,
            ))
            .await
            .map_err(map_mv_error)?;

        let mut filled_fields = Vec::new();
        if payload.title.is_some() {
            filled_fields.push("title".to_string());
        }
        if !payload.content.trim().is_empty() {
            filled_fields.push("content".to_string());
        }
        if !payload.tags.is_empty() {
            filled_fields.push("tags".to_string());
        }
        if payload.source.is_some() {
            filled_fields.push("source".to_string());
        }
        filled_fields.push("importance".to_string());
        for key in payload.metadata.keys() {
            filled_fields.push(format!("metadata.{key}"));
        }
        filled_fields.push("relations".to_string());

        (
            StatusCode::CREATED,
            TemplateApplyResponse {
                node: stored,
                created: true,
                filled_fields,
                overwritten_fields: Vec::new(),
            },
        )
    };

    // Track template usage for ranking and UX hints without requiring external analytics.
    let mut template_usage = template.clone();
    let next_count = template_usage
        .metadata
        .get(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        + 1;
    template_usage.metadata.insert(
        TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY.to_string(),
        serde_json::Value::from(next_count),
    );
    template_usage.metadata.insert(
        TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY.to_string(),
        serde_json::Value::String(Utc::now().to_rfc3339()),
    );
    if let Ok(updated_template) = state.engine.update_node(template_usage).await {
        state.notify_change(
            &updated_template.id.to_string(),
            "update",
            Some(&updated_template.namespace),
        );
    }

    let operation = if response.created { "create" } else { "update" };
    state.notify_change(
        &response.node.id.to_string(),
        operation,
        Some(&response.node.namespace),
    );
    Ok((status, Json(response)))
}

async fn duplicate_template(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(template_id_raw): Path<String>,
    Json(mut req): Json<DuplicateTemplateRequest>,
) -> Result<(StatusCode, Json<KnowledgeNode>), (StatusCode, String)> {
    authorize_write(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let target_kind = resolve_template_target_kind(&template)?;

    let target_namespace =
        namespace_for_create(&auth, req.namespace.take(), template.namespace.as_str())?;
    enforce_namespace_quota(&state.engine, &target_namespace)
        .await
        .map_err(map_namespace_quota_error)?;

    let request_tags = req.tags.take().unwrap_or_default();
    let duplicated_tags = merge_tags_case_insensitive(
        &merge_tags_case_insensitive(&template.tags, &request_tags),
        &[TEMPLATE_TAG.to_string()],
    );

    let mut duplicated_metadata = template.metadata.clone();
    duplicated_metadata.remove(TEMPLATE_SOURCE_ID_METADATA_KEY);
    duplicated_metadata.remove(TEMPLATE_INSTANTIATED_AT_METADATA_KEY);
    duplicated_metadata.remove(TEMPLATE_VERSIONS_METADATA_KEY);
    duplicated_metadata.remove(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY);
    duplicated_metadata.remove(TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY);
    duplicated_metadata.remove(TEMPLATE_RESTORED_FROM_VERSION_METADATA_KEY);
    duplicated_metadata.remove(TEMPLATE_RESTORED_AT_METADATA_KEY);
    duplicated_metadata.insert(
        TEMPLATE_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
    duplicated_metadata.insert(
        TEMPLATE_TARGET_KIND_METADATA_KEY.to_string(),
        serde_json::Value::String(target_kind.as_str().to_string()),
    );
    if let Some(template_key) = req.template_key.take() {
        let trimmed = template_key.trim();
        if trimmed.is_empty() {
            duplicated_metadata.remove(TEMPLATE_KEY_METADATA_KEY);
        } else {
            duplicated_metadata.insert(
                TEMPLATE_KEY_METADATA_KEY.to_string(),
                serde_json::Value::String(trimmed.to_string()),
            );
        }
    }
    if let Some(extra_metadata) = req.metadata.take() {
        for (key, value) in extra_metadata {
            duplicated_metadata.insert(key, value);
        }
    }

    let duplicated_title = req.title.take().or_else(|| {
        template
            .title
            .as_deref()
            .map(|title| format!("{title} (copy)"))
    });

    validate_node_payload(
        NodeKind::Template,
        duplicated_title.as_deref(),
        &template.content,
        template.source.as_deref(),
        Some(&target_namespace),
        &duplicated_tags,
        Some(template.importance),
        Some(&duplicated_metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let mut duplicated = KnowledgeNode::new(NodeKind::Template, template.content.clone())
        .with_namespace(target_namespace);
    if let Some(title) = duplicated_title {
        duplicated = duplicated.with_title(title);
    }
    if let Some(source) = template.source.clone() {
        duplicated = duplicated.with_source(source);
    }
    duplicated = duplicated
        .with_tags(duplicated_tags)
        .with_importance(template.importance);
    duplicated.metadata = duplicated_metadata;

    let stored = state
        .engine
        .store_node(duplicated)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
    Ok((StatusCode::CREATED, Json(stored)))
}

async fn delete_template(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(template_id_raw): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let deleted = state
        .engine
        .delete_node(template_id)
        .await
        .map_err(map_mv_error)?;
    if deleted {
        state.notify_change(
            &template_id.to_string(),
            "delete",
            Some(&template.namespace),
        );
    }

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "template_id": template_id.to_string()
    })))
}

async fn list_template_versions(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(template_id_raw): Path<String>,
) -> Result<Json<Vec<TemplateVersionSummary>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let versions = parse_template_versions_from_metadata(&template.metadata);
    let summaries = versions
        .into_iter()
        .rev()
        .map(|item| TemplateVersionSummary {
            version_id: item.version_id,
            captured_at: item.captured_at,
            kind: item.kind,
            namespace: item.namespace,
            title: item.title,
            source: item.source,
            importance: item.importance,
            tag_count: item.tags.len(),
            content_preview: trim_content_preview(&item.content, 180),
        })
        .collect();

    Ok(Json(summaries))
}

async fn get_template_version(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((template_id_raw, version_id)): Path<(String, String)>,
) -> Result<Json<TemplateVersionDetailResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let versions = parse_template_versions_from_metadata(&template.metadata);
    let version = versions
        .into_iter()
        .find(|item| item.version_id == version_id)
        .ok_or((StatusCode::NOT_FOUND, "template version not found".into()))?;
    let field_changes = template_version_field_changes(&version, &template);

    let current = TemplateVersionCurrentSnapshot {
        kind: template.kind.to_string(),
        namespace: template.namespace,
        title: template.title,
        content: template.content,
        source: template.source,
        tags: template.tags,
        importance: template.importance,
    };
    let diff = template_version_diff_summary(&version.content, &current.content);

    Ok(Json(TemplateVersionDetailResponse {
        version,
        current,
        diff,
        field_changes,
    }))
}

async fn restore_template_version(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((template_id_raw, version_id)): Path<(String, String)>,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let template_id = parse_uuid_param(&template_id_raw, "template id")?;
    let template = state
        .engine
        .get_node(template_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "template not found".into()))?;
    authorize_namespace(&auth, &template.namespace)?;

    if !is_template_node(&template) {
        return Err((StatusCode::BAD_REQUEST, "node is not a template".into()));
    }

    let mut versions = parse_template_versions_from_metadata(&template.metadata);
    let Some(version) = versions
        .iter()
        .find(|item| item.version_id == version_id)
        .cloned()
    else {
        return Err((StatusCode::NOT_FOUND, "template version not found".into()));
    };

    push_template_version_snapshot(
        &mut versions,
        template_version_snapshot_from_node(&template),
    );

    let mut restored = template.clone();
    if let Ok(kind) = version.kind.parse::<NodeKind>() {
        restored.kind = kind;
    }
    restored.content = version.content;
    restored.title = version.title;
    restored.source = version.source;
    restored.tags = version.tags;
    restored.importance = version.importance;
    restored.metadata = version.metadata;
    if auth.is_admin() {
        restored.namespace = version.namespace;
    }
    ensure_template_markers(&mut restored);
    if let Some(instantiation_count) = template
        .metadata
        .get(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY)
        .cloned()
    {
        restored.metadata.insert(
            TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY.to_string(),
            instantiation_count,
        );
    }
    if let Some(last_instantiated_at) = template
        .metadata
        .get(TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY)
        .cloned()
    {
        restored.metadata.insert(
            TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY.to_string(),
            last_instantiated_at,
        );
    }
    restored.metadata.insert(
        TEMPLATE_RESTORED_FROM_VERSION_METADATA_KEY.to_string(),
        serde_json::Value::String(version_id),
    );
    restored.metadata.insert(
        TEMPLATE_RESTORED_AT_METADATA_KEY.to_string(),
        serde_json::Value::String(Utc::now().to_rfc3339()),
    );
    set_template_versions_in_metadata(&mut restored.metadata, &versions);

    validate_node_payload(
        restored.kind,
        restored.title.as_deref(),
        &restored.content,
        restored.source.as_deref(),
        Some(&restored.namespace),
        &restored.tags,
        Some(restored.importance),
        Some(&restored.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    if restored.namespace != template.namespace {
        enforce_namespace_quota(&state.engine, &restored.namespace)
            .await
            .map_err(map_namespace_quota_error)?;
    }
    authorize_namespace(&auth, &restored.namespace)?;

    let updated = state
        .engine
        .update_node(restored)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));
    Ok(Json(updated))
}

async fn list_node_versions(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(node_id_raw): Path<String>,
) -> Result<Json<Vec<NodeVersionSummary>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node id")?;
    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    if is_template_node(&node) {
        return Err((
            StatusCode::BAD_REQUEST,
            "use template version endpoints for template nodes".into(),
        ));
    }

    let versions = parse_node_versions_from_metadata(&node.metadata);
    Ok(Json(node_version_summaries(versions)))
}

async fn get_node_version(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((node_id_raw, version_id)): Path<(String, String)>,
) -> Result<Json<NodeVersionDetailResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node id")?;
    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    if is_template_node(&node) {
        return Err((
            StatusCode::BAD_REQUEST,
            "use template version endpoints for template nodes".into(),
        ));
    }

    let version = parse_node_versions_from_metadata(&node.metadata)
        .into_iter()
        .find(|item| item.version_id == version_id)
        .ok_or((StatusCode::NOT_FOUND, "node version not found".into()))?;

    Ok(Json(node_version_detail_response(version, &node)))
}

async fn restore_node_version(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path((node_id_raw, version_id)): Path<(String, String)>,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let node_id = parse_uuid_param(&node_id_raw, "node id")?;
    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    if is_template_node(&node) {
        return Err((
            StatusCode::BAD_REQUEST,
            "use template version endpoints for template nodes".into(),
        ));
    }

    let mut versions = parse_node_versions_from_metadata(&node.metadata);
    let Some(version) = versions
        .iter()
        .find(|item| item.version_id == version_id)
        .cloned()
    else {
        return Err((StatusCode::NOT_FOUND, "node version not found".into()));
    };

    push_node_version_snapshot(&mut versions, node_version_snapshot_from_node(&node));

    let mut restored = apply_node_version(&node, &version);
    if !auth.is_admin() {
        restored.namespace = node.namespace.clone();
    }
    set_node_versions_in_metadata(&mut restored.metadata, &versions);

    validate_node_payload(
        restored.kind,
        restored.title.as_deref(),
        &restored.content,
        restored.source.as_deref(),
        Some(&restored.namespace),
        &restored.tags,
        Some(restored.importance),
        Some(&restored.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    if restored.namespace != node.namespace {
        enforce_namespace_quota(&state.engine, &restored.namespace)
            .await
            .map_err(map_namespace_quota_error)?;
    }
    authorize_namespace(&auth, &restored.namespace)?;

    let updated = state
        .engine
        .update_node(restored)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));
    Ok(Json(updated))
}

async fn enrich_clip(
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<ClipEnrichRequest>,
) -> Result<Json<ClipEnrichResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;

    let normalized_url = normalize_clip_url(&req.url).ok_or((
        StatusCode::BAD_REQUEST,
        "url must be a valid HTTP(S) URL".into(),
    ))?;

    let (html, fetched, content_type) = if let Some(provided_html) = req
        .html
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        (
            truncate_display_chars(provided_html, CLIP_ENRICH_MAX_HTML_CHARS),
            false,
            None,
        )
    } else {
        let (fetched_html, fetched_content_type) = fetch_clip_html(&normalized_url).await?;
        (fetched_html, true, fetched_content_type)
    };

    let title = extract_html_meta_content(&html, &["og:title", "twitter:title"])
        .or_else(|| extract_html_tag_content(&html, "title", 220))
        .or_else(|| Some(clip_title_from_url(&normalized_url)));
    let description = extract_html_meta_content(
        &html,
        &["description", "og:description", "twitter:description"],
    )
    .or_else(|| extract_clip_preview_from_html(&html));
    let content_preview = extract_body_text_from_html(&html, 1400);
    let estimated_reading_minutes = content_preview
        .as_deref()
        .and_then(estimate_reading_minutes);
    let site_name = extract_html_meta_content(&html, &["og:site_name", "application-name"]);
    let keywords = extract_html_meta_content(&html, &["keywords", "news_keywords"]);

    let suggested_tags = build_clip_suggested_tags(
        &normalized_url,
        title.as_deref(),
        description.as_deref(),
        content_preview.as_deref(),
        keywords.as_deref(),
    );

    Ok(Json(ClipEnrichResponse {
        normalized_url,
        title,
        description,
        site_name,
        content_preview,
        estimated_reading_minutes,
        suggested_tags,
        fetched,
        content_type,
    }))
}

async fn import_clip(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<ClipImportRequest>,
) -> Result<(StatusCode, Json<ClipImportResponse>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let normalized_url = normalize_clip_url(&req.url).ok_or((
        StatusCode::BAD_REQUEST,
        "url must be a valid HTTP(S) URL".into(),
    ))?;
    let namespace = namespace_for_create(
        &auth,
        normalize_optional_namespace_value(req.namespace.take()),
        "default",
    )?;

    let dedupe = req.dedupe.unwrap_or(true);
    let create_note = req.create_note.unwrap_or(false);
    let clip_source = normalize_clip_source(req.clip_source.as_deref());
    let excerpt = req
        .excerpt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_default();
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| clip_title_from_url(&normalized_url));
    let tags = normalize_clip_tags(req.tags.take().unwrap_or_default());

    let mut created = false;
    let bookmark = if dedupe {
        if let Some(existing) =
            find_existing_clip_by_url(state.as_ref(), &namespace, &normalized_url).await?
        {
            existing
        } else {
            enforce_namespace_quota(&state.engine, &namespace)
                .await
                .map_err(map_namespace_quota_error)?;
            let now = Utc::now().to_rfc3339();
            let mut metadata = std::collections::HashMap::new();
            metadata.insert(
                CLIP_READ_METADATA_KEY.to_string(),
                serde_json::Value::Bool(false),
            );
            metadata.insert(
                CLIP_SOURCE_METADATA_KEY.to_string(),
                serde_json::Value::String(clip_source.clone()),
            );
            metadata.insert(
                CLIP_NORMALIZED_URL_METADATA_KEY.to_string(),
                serde_json::Value::String(normalized_url.clone()),
            );
            metadata.insert(
                CLIP_CAPTURED_AT_METADATA_KEY.to_string(),
                serde_json::Value::String(now),
            );

            let mut node = KnowledgeNode::new(NodeKind::Bookmark, excerpt.clone());
            node = node
                .with_namespace(namespace.clone())
                .with_title(title.clone())
                .with_source(normalized_url.clone())
                .with_tags(tags.clone());
            node.metadata = metadata;

            validate_node_payload(
                node.kind,
                node.title.as_deref(),
                &node.content,
                node.source.as_deref(),
                Some(&node.namespace),
                &node.tags,
                Some(node.importance),
                Some(&node.metadata),
            )
            .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

            let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;
            state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
            created = true;
            stored
        }
    } else {
        enforce_namespace_quota(&state.engine, &namespace)
            .await
            .map_err(map_namespace_quota_error)?;
        let now = Utc::now().to_rfc3339();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            CLIP_READ_METADATA_KEY.to_string(),
            serde_json::Value::Bool(false),
        );
        metadata.insert(
            CLIP_SOURCE_METADATA_KEY.to_string(),
            serde_json::Value::String(clip_source.clone()),
        );
        metadata.insert(
            CLIP_NORMALIZED_URL_METADATA_KEY.to_string(),
            serde_json::Value::String(normalized_url.clone()),
        );
        metadata.insert(
            CLIP_CAPTURED_AT_METADATA_KEY.to_string(),
            serde_json::Value::String(now),
        );

        let mut node = KnowledgeNode::new(NodeKind::Bookmark, excerpt.clone());
        node = node
            .with_namespace(namespace.clone())
            .with_title(title.clone())
            .with_source(normalized_url.clone())
            .with_tags(tags.clone());
        node.metadata = metadata;

        validate_node_payload(
            node.kind,
            node.title.as_deref(),
            &node.content,
            node.source.as_deref(),
            Some(&node.namespace),
            &node.tags,
            Some(node.importance),
            Some(&node.metadata),
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

        let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;
        state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
        created = true;
        stored
    };

    let note = if create_note {
        enforce_namespace_quota(&state.engine, &namespace)
            .await
            .map_err(map_namespace_quota_error)?;
        let note_content = clip_note_content(
            &bookmark,
            if excerpt.is_empty() {
                None
            } else {
                Some(excerpt.as_str())
            },
        );
        let note_title = bookmark
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("Clip Note: {value}"))
            .unwrap_or_else(|| "Clip Note".to_string());
        let note_tags = merge_tags_case_insensitive(&tags, &["clip-note".to_string()]);
        let mut note = KnowledgeNode::new(NodeKind::Fact, note_content);
        note = note
            .with_namespace(namespace.clone())
            .with_title(note_title)
            .with_source("clip-import")
            .with_tags(note_tags);
        note.metadata.insert(
            CLIP_LINKED_BOOKMARK_ID_METADATA_KEY.to_string(),
            serde_json::Value::String(bookmark.id.to_string()),
        );

        validate_node_payload(
            note.kind,
            note.title.as_deref(),
            &note.content,
            note.source.as_deref(),
            Some(&note.namespace),
            &note.tags,
            Some(note.importance),
            Some(&note.metadata),
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

        let stored_note = state.engine.store_node(note).await.map_err(map_mv_error)?;
        state.notify_change(
            &stored_note.id.to_string(),
            "create",
            Some(&stored_note.namespace),
        );

        let relationship = Relationship::new(stored_note.id, bookmark.id, RelationKind::References);
        if let Err(err) = state.engine.add_relationship(relationship).await {
            tracing::warn!(
                error = %err,
                note_id = %stored_note.id,
                bookmark_id = %bookmark.id,
                "mindvault_clip_relationship_create_failed"
            );
        }
        Some(stored_note)
    } else {
        None
    };

    let status = if created || note.is_some() {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(ClipImportResponse {
            bookmark,
            created,
            note,
        }),
    ))
}

async fn store_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<StoreNodeRequest>,
) -> Result<(StatusCode, Json<KnowledgeNode>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let tags = req.tags.take().unwrap_or_default();
    let kind: NodeKind = req
        .kind
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    validate_node_payload(
        kind,
        req.title.as_deref(),
        &req.content,
        req.source.as_deref(),
        req.namespace.as_deref(),
        &tags,
        req.importance,
        req.metadata.as_ref(),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let mut node = KnowledgeNode::new(kind, req.content);
    if let Some(title) = req.title {
        node = node.with_title(title);
    }
    if let Some(source) = req.source {
        node = node.with_source(source);
    }
    let namespace = namespace_for_create(&auth, req.namespace.take(), "default")?;
    enforce_namespace_quota(&state.engine, &namespace)
        .await
        .map_err(map_namespace_quota_error)?;
    node = node.with_namespace(namespace);
    if !tags.is_empty() {
        node = node.with_tags(tags);
    }
    if let Some(imp) = req.importance {
        node = node.with_importance(imp);
    }
    if let Some(metadata) = req.metadata {
        node.metadata = metadata;
    }

    let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;

    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));
    Ok((StatusCode::CREATED, Json(stored)))
}

async fn get_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Option<KnowledgeNode>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let uuid = parse_uuid_param(&id, "node id")?;

    let node = state.engine.get_node(uuid).await.map_err(map_mv_error)?;

    if let Some(ref node) = node {
        authorize_namespace(&auth, &node.namespace)?;
    }

    Ok(Json(node))
}

async fn update_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(mut node): Json<KnowledgeNode>,
) -> Result<Json<KnowledgeNode>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let uuid = parse_uuid_param(&id, "node id")?;

    let existing = state
        .engine
        .get_node(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;

    authorize_namespace(&auth, &existing.namespace)?;

    node.id = uuid;
    if !auth.is_admin() {
        node.namespace = existing.namespace.clone();
    }
    if is_template_node(&existing) {
        let mut versions = parse_template_versions_from_metadata(&existing.metadata);
        if template_authored_fields_changed(&existing, &node) {
            push_template_version_snapshot(
                &mut versions,
                template_version_snapshot_from_node(&existing),
            );
        }
        ensure_template_markers(&mut node);
        set_template_versions_in_metadata(&mut node.metadata, &versions);
        if let Some(instantiation_count) = existing
            .metadata
            .get(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY)
            .cloned()
        {
            node.metadata.insert(
                TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY.to_string(),
                instantiation_count,
            );
        }
        if let Some(last_instantiated_at) = existing
            .metadata
            .get(TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY)
            .cloned()
        {
            node.metadata.insert(
                TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY.to_string(),
                last_instantiated_at,
            );
        }
    } else {
        let mut versions = parse_node_versions_from_metadata(&existing.metadata);
        if node_authored_fields_changed(&existing, &node) {
            push_node_version_snapshot(&mut versions, node_version_snapshot_from_node(&existing));
        }
        set_node_versions_in_metadata(&mut node.metadata, &versions);
    }
    validate_node_payload(
        node.kind,
        node.title.as_deref(),
        &node.content,
        node.source.as_deref(),
        Some(&node.namespace),
        &node.tags,
        Some(node.importance),
        Some(&node.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    if node.namespace != existing.namespace {
        enforce_namespace_quota(&state.engine, &node.namespace)
            .await
            .map_err(map_namespace_quota_error)?;
    }
    authorize_namespace(&auth, &node.namespace)?;

    let updated = state.engine.update_node(node).await.map_err(map_mv_error)?;

    state.notify_change(&id, "update", Some(&updated.namespace));
    Ok(Json(updated))
}

async fn delete_node(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let uuid = parse_uuid_param(&id, "node id")?;

    let existing = state.engine.get_node(uuid).await.map_err(map_mv_error)?;
    if let Some(node) = existing.as_ref() {
        authorize_namespace(&auth, &node.namespace)?;
    }

    let deleted = state.engine.delete_node(uuid).await.map_err(map_mv_error)?;

    if deleted {
        state.notify_change(&id, "delete", None);
    }
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

async fn recall(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<RecallRequest>,
) -> Result<Json<Vec<SearchResultDto>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    validate_query_text("text", &req.text).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let strategy = req
        .strategy
        .as_deref()
        .unwrap_or("hybrid")
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    let kinds = parse_kind_list(req.kinds)?;
    let limit = req.limit.unwrap_or(10);
    validate_recall_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let query = MemoryQuery {
        text: req.text,
        strategy,
        limit,
        min_score: req.min_score.unwrap_or(0.0),
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, req.namespace.take())?,
            kinds,
            tags: req.tags,
            ..Default::default()
        },
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;

    let dtos: Vec<SearchResultDto> = results
        .into_iter()
        .map(|r| SearchResultDto {
            node: r.node,
            score: r.score,
            match_source: match_source_to_str(r.match_source).to_string(),
        })
        .collect();

    Ok(Json(dtos))
}

async fn search(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResultDto>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    validate_query_text("q", &params.q).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let limit = params.limit.unwrap_or(10);
    validate_recall_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let strategy = match params.search_type.as_deref() {
        Some("fulltext") | Some("full_text") => SearchStrategy::FullText,
        Some("vector") => SearchStrategy::Vector,
        Some("graph") => SearchStrategy::Graph,
        _ => SearchStrategy::Hybrid,
    };

    let query = MemoryQuery::new(params.q)
        .with_strategy(strategy)
        .with_limit(limit);
    let query = if let Some(namespace) = scoped_namespace(&auth, None)? {
        query.with_namespace(namespace)
    } else {
        query
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;

    let dtos: Vec<SearchResultDto> = results
        .into_iter()
        .map(|r| SearchResultDto {
            node: r.node,
            score: r.score,
            match_source: match_source_to_str(r.match_source).to_string(),
        })
        .collect();

    Ok(Json(dtos))
}

async fn list_saved_views(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<SavedViewListQuery>,
) -> Result<Json<Vec<SavedViewDto>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let namespace = scoped_namespace(&auth, params.namespace)?;
    let limit = params.limit.unwrap_or(100);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let offset = params.offset.unwrap_or(0);

    let filters = QueryFilters {
        namespace,
        kinds: Some(vec![NodeKind::SavedView]),
        ..Default::default()
    };

    let mut views: Vec<SavedViewDto> = state
        .engine
        .list_nodes(&filters, limit, offset)
        .await
        .map_err(map_mv_error)?
        .into_iter()
        .filter_map(|node| {
            saved_view_definition_from_node(&node)
                .map(|definition| saved_view_dto_from_node(&node, definition))
        })
        .collect();
    views.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    Ok(Json(views))
}

async fn create_saved_view(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSavedViewRequest>,
) -> Result<(StatusCode, Json<SavedViewDto>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let name = normalize_saved_view_name(&req.name)?;
    let view_type = parse_saved_view_view_type(&req.view_type)?;
    let filters = parse_saved_view_filters(req.filters)?;
    let sort = parse_saved_view_sort(req.sort)?;
    let group_by = normalize_saved_view_group_by(req.group_by)?;
    let query = parse_saved_view_query(req.query)?;

    let namespace = namespace_for_create(&auth, req.namespace, "default")?;
    enforce_namespace_quota(&state.engine, &namespace)
        .await
        .map_err(map_namespace_quota_error)?;

    let definition = SavedViewDefinition {
        name,
        view_type,
        filters,
        sort,
        group_by,
        query,
    };

    let mut node =
        KnowledgeNode::new(NodeKind::SavedView, "saved view".to_string()).with_namespace(namespace);
    apply_saved_view_definition(&mut node, &definition);

    validate_node_payload(
        node.kind,
        node.title.as_deref(),
        &node.content,
        node.source.as_deref(),
        Some(&node.namespace),
        &node.tags,
        Some(node.importance),
        Some(&node.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));

    let definition = saved_view_definition_from_node(&stored).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "stored saved view metadata is invalid".into(),
    ))?;
    let dto = saved_view_dto_from_node(&stored, definition);
    Ok((StatusCode::CREATED, Json(dto)))
}

async fn update_saved_view(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(saved_view_id_raw): Path<String>,
    Json(req): Json<UpdateSavedViewRequest>,
) -> Result<Json<SavedViewDto>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let saved_view_id = parse_uuid_param(&saved_view_id_raw, "saved view id")?;
    let existing = state
        .engine
        .get_node(saved_view_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "saved view not found".into()))?;
    authorize_namespace(&auth, &existing.namespace)?;

    if !is_saved_view_node(&existing) {
        return Err((StatusCode::BAD_REQUEST, "node is not a saved view".into()));
    }

    let mut definition = saved_view_definition_from_node(&existing).ok_or((
        StatusCode::BAD_REQUEST,
        "saved view metadata is invalid".into(),
    ))?;

    if let Some(name) = req.name {
        definition.name = normalize_saved_view_name(&name)?;
    }
    if let Some(view_type) = req.view_type {
        definition.view_type = parse_saved_view_view_type(&view_type)?;
    }
    if let Some(filters) = req.filters {
        definition.filters = parse_saved_view_filters(Some(filters))?;
    }
    if let Some(sort) = req.sort {
        definition.sort = parse_saved_view_sort(Some(sort))?;
    }
    if req.group_by.is_some() {
        definition.group_by = normalize_saved_view_group_by(req.group_by)?;
    }
    if req.query.is_some() {
        definition.query = parse_saved_view_query(req.query)?;
    }

    let mut updated = existing.clone();
    apply_saved_view_definition(&mut updated, &definition);

    validate_node_payload(
        updated.kind,
        updated.title.as_deref(),
        &updated.content,
        updated.source.as_deref(),
        Some(&updated.namespace),
        &updated.tags,
        Some(updated.importance),
        Some(&updated.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let updated = state
        .engine
        .update_node(updated)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));

    let definition = saved_view_definition_from_node(&updated).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "stored saved view metadata is invalid".into(),
    ))?;
    Ok(Json(saved_view_dto_from_node(&updated, definition)))
}

async fn delete_saved_view(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(saved_view_id_raw): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let saved_view_id = parse_uuid_param(&saved_view_id_raw, "saved view id")?;

    let existing = state
        .engine
        .get_node(saved_view_id)
        .await
        .map_err(map_mv_error)?;
    if let Some(node) = existing.as_ref() {
        authorize_namespace(&auth, &node.namespace)?;
        if !is_saved_view_node(node) {
            return Err((StatusCode::BAD_REQUEST, "node is not a saved view".into()));
        }
    }

    let deleted = state
        .engine
        .delete_node(saved_view_id)
        .await
        .map_err(map_mv_error)?;

    if deleted {
        state.notify_change(&saved_view_id_raw, "delete", None);
    }

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

async fn list_saved_searches(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<SavedSearchListQuery>,
) -> Result<Json<Vec<SavedSearchDto>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let namespace = scoped_namespace(&auth, normalize_optional_namespace_value(params.namespace))?;
    let limit = params.limit.unwrap_or(100);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let offset = params.offset.unwrap_or(0);

    let filters = QueryFilters {
        namespace,
        kinds: Some(vec![NodeKind::Bookmark]),
        tags: Some(vec![SAVED_SEARCH_TAG.to_string()]),
        ..Default::default()
    };

    let mut searches: Vec<SavedSearchDto> = state
        .engine
        .list_nodes(&filters, limit, offset)
        .await
        .map_err(map_mv_error)?
        .into_iter()
        .filter_map(|node| {
            saved_search_definition_from_node(&node)
                .map(|definition| saved_search_dto_from_node(&node, definition))
        })
        .collect();
    searches.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    Ok(Json(searches))
}

async fn create_saved_search(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSavedSearchRequest>,
) -> Result<(StatusCode, Json<SavedSearchDto>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let name = normalize_saved_search_name(&req.name)?;
    validate_query_text("query", &req.query).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let query = req.query.trim().to_string();
    let description = normalize_saved_search_description(req.description)?;
    let strategy = parse_saved_search_strategy(req.search_type, SearchStrategy::Hybrid)?;
    let limit = normalize_saved_search_limit(req.limit)?;
    let target_namespace = scoped_namespace(
        &auth,
        normalize_optional_namespace_value(req.target_namespace),
    )?;
    let kinds = parse_saved_search_kind_filters(req.kinds)?.unwrap_or_default();
    let tags = normalize_saved_search_filter_tags(req.tags)?.unwrap_or_default();
    let min_score = normalize_unit_interval("min_score", req.min_score)?;
    let min_importance = normalize_unit_interval("min_importance", req.min_importance)?;

    let storage_default_namespace = target_namespace.as_deref().unwrap_or("default");
    let namespace = namespace_for_create(
        &auth,
        normalize_optional_namespace_value(req.namespace),
        storage_default_namespace,
    )?;
    enforce_namespace_quota(&state.engine, &namespace)
        .await
        .map_err(map_namespace_quota_error)?;

    let definition = SavedSearchDefinition {
        name,
        description,
        query: query.clone(),
        strategy,
        limit,
        target_namespace: target_namespace.clone(),
        kinds,
        tags,
        min_score,
        min_importance,
    };

    let mut node = KnowledgeNode::new(NodeKind::Bookmark, query).with_namespace(namespace);
    node = node.with_tags(vec![SAVED_SEARCH_TAG.to_string()]);
    apply_saved_search_definition(&mut node, &definition);
    validate_node_payload(
        node.kind,
        node.title.as_deref(),
        &node.content,
        node.source.as_deref(),
        Some(&node.namespace),
        &node.tags,
        Some(node.importance),
        Some(&node.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let stored = state.engine.store_node(node).await.map_err(map_mv_error)?;
    state.notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));

    let definition = saved_search_definition_from_node(&stored).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "stored saved search metadata is invalid".into(),
    ))?;
    let dto = saved_search_dto_from_node(&stored, definition);
    Ok((StatusCode::CREATED, Json(dto)))
}

async fn update_saved_search(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSavedSearchRequest>,
) -> Result<Json<SavedSearchDto>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let uuid = parse_uuid_param(&id, "saved search id")?;

    let existing = state
        .engine
        .get_node(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "saved search not found".into()))?;
    authorize_namespace(&auth, &existing.namespace)?;

    if !is_saved_search_node(&existing) {
        return Err((StatusCode::BAD_REQUEST, "node is not a saved search".into()));
    }
    let existing_definition = saved_search_definition_from_node(&existing).ok_or((
        StatusCode::BAD_REQUEST,
        "saved search metadata is invalid".into(),
    ))?;

    let name = match req.name {
        Some(raw) => normalize_saved_search_name(&raw)?,
        None => existing_definition.name,
    };
    let query = match req.query {
        Some(raw) => {
            validate_query_text("query", &raw).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
            raw.trim().to_string()
        }
        None => existing_definition.query,
    };
    let strategy = parse_saved_search_strategy(req.search_type, existing_definition.strategy)?;
    let limit = if req.limit.is_some() {
        normalize_saved_search_limit(req.limit)?
    } else {
        existing_definition.limit
    };
    let target_namespace = match req.target_namespace {
        Some(raw) => scoped_namespace(&auth, normalize_optional_namespace_value(raw))?,
        None => existing_definition.target_namespace,
    };
    let kinds = match parse_saved_search_kind_filters(req.kinds)? {
        Some(parsed) => parsed,
        None => existing_definition.kinds,
    };
    let tags = match normalize_saved_search_filter_tags(req.tags)? {
        Some(parsed) => parsed,
        None => existing_definition.tags,
    };
    let min_score = match req.min_score {
        Some(value) => normalize_unit_interval("min_score", value)?,
        None => existing_definition.min_score,
    };
    let min_importance = match req.min_importance {
        Some(value) => normalize_unit_interval("min_importance", value)?,
        None => existing_definition.min_importance,
    };
    let description = match req.description {
        Some(raw) => normalize_saved_search_description(raw)?,
        None => existing_definition.description,
    };

    let definition = SavedSearchDefinition {
        name,
        description,
        query,
        strategy,
        limit,
        target_namespace,
        kinds,
        tags,
        min_score,
        min_importance,
    };

    let mut updated_payload = existing.clone();
    apply_saved_search_definition(&mut updated_payload, &definition);
    validate_node_payload(
        updated_payload.kind,
        updated_payload.title.as_deref(),
        &updated_payload.content,
        updated_payload.source.as_deref(),
        Some(&updated_payload.namespace),
        &updated_payload.tags,
        Some(updated_payload.importance),
        Some(&updated_payload.metadata),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let updated = state
        .engine
        .update_node(updated_payload)
        .await
        .map_err(map_mv_error)?;
    state.notify_change(&id, "update", Some(&updated.namespace));

    let definition = saved_search_definition_from_node(&updated).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "updated saved search metadata is invalid".into(),
    ))?;
    Ok(Json(saved_search_dto_from_node(&updated, definition)))
}

async fn delete_saved_search(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_write(&auth)?;
    let uuid = parse_uuid_param(&id, "saved search id")?;
    let existing = state
        .engine
        .get_node(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "saved search not found".into()))?;
    authorize_namespace(&auth, &existing.namespace)?;
    if !is_saved_search_node(&existing) {
        return Err((StatusCode::BAD_REQUEST, "node is not a saved search".into()));
    }

    let deleted = state.engine.delete_node(uuid).await.map_err(map_mv_error)?;
    if deleted {
        state.notify_change(&id, "delete", Some(&existing.namespace));
    }

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "saved_search_id": id
    })))
}

async fn run_saved_search(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SavedSearchRunResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let uuid = parse_uuid_param(&id, "saved search id")?;
    let node = state
        .engine
        .get_node(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "saved search not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;
    if !is_saved_search_node(&node) {
        return Err((StatusCode::BAD_REQUEST, "node is not a saved search".into()));
    }
    let definition = saved_search_definition_from_node(&node).ok_or((
        StatusCode::BAD_REQUEST,
        "saved search metadata is invalid".into(),
    ))?;

    let query = MemoryQuery {
        text: definition.query.clone(),
        strategy: definition.strategy,
        limit: definition.limit,
        min_score: definition.min_score.unwrap_or(0.0),
        filters: QueryFilters {
            namespace: scoped_namespace(&auth, definition.target_namespace.clone())?,
            kinds: if definition.kinds.is_empty() {
                None
            } else {
                Some(definition.kinds.clone())
            },
            tags: if definition.tags.is_empty() {
                None
            } else {
                Some(definition.tags.clone())
            },
            min_importance: definition.min_importance,
            ..Default::default()
        },
    };

    let results = state.engine.recall(&query).await.map_err(map_mv_error)?;
    let dtos = results
        .into_iter()
        .map(|result| SearchResultDto {
            node: result.node,
            score: result.score,
            match_source: match_source_to_str(result.match_source).to_string(),
        })
        .collect();

    Ok(Json(SavedSearchRunResponse {
        saved_search: saved_search_dto_from_node(&node, definition),
        executed_at: Utc::now().to_rfc3339(),
        results: dtos,
    }))
}

async fn list_audit_logs(
    Extension(auth): Extension<AuthContext>,
    Query(mut params): Query<AuditListQuery>,
) -> Result<Json<Vec<AuditEntry>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    if !auth.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            "audit log access requires admin role".into(),
        ));
    }

    let limit = params.limit.unwrap_or(100);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let offset = params.offset.unwrap_or(0);
    let since = parse_optional_rfc3339_datetime(params.since.take(), "since")?;
    let subject = params
        .subject
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let action = params
        .action
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    Ok(Json(list_audit_entries(
        limit, offset, subject, action, since,
    )))
}

async fn list_nodes(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListQuery>,
) -> Result<Json<Vec<KnowledgeNode>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let limit = params.limit.unwrap_or(50);
    validate_list_limit(limit).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let kinds = match params.kind {
        Some(kind) => Some(vec![kind
            .parse::<NodeKind>()
            .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?]),
        None => None,
    };

    let scoped_ns = scoped_namespace(&auth, params.namespace)?;

    let filters = QueryFilters {
        namespace: scoped_ns,
        kinds,
        ..Default::default()
    };

    let nodes = state
        .engine
        .list_nodes(&filters, limit, params.offset.unwrap_or(0))
        .await
        .map_err(map_mv_error)?;

    Ok(Json(nodes))
}

async fn add_relationship(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddRelationshipRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    authorize_write(&auth)?;

    let from = parse_uuid_param(&req.from_node, "from_node")?;
    let to = parse_uuid_param(&req.to_node, "to_node")?;
    let kind: RelationKind = req
        .kind
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    let mut rel = Relationship::new(from, to, kind);
    if let Some(w) = req.weight {
        rel = rel.with_weight(w);
    }

    let from_node = state
        .engine
        .get_node(from)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "from_node not found".into()))?;
    let to_node = state
        .engine
        .get_node(to)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "to_node not found".into()))?;

    authorize_namespace(&auth, &from_node.namespace)?;
    authorize_namespace(&auth, &to_node.namespace)?;

    let rel_id = rel.id;
    state
        .engine
        .add_relationship(rel)
        .await
        .map_err(map_mv_error)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": rel_id.to_string() })),
    ))
}

fn relationship_is_auto_managed(rel: &Relationship) -> bool {
    rel.metadata
        .get(AUTO_BACKLINK_METADATA_KEY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn relationship_auto_source(rel: &Relationship) -> Option<String> {
    if !relationship_is_auto_managed(rel) {
        return None;
    }
    rel.metadata
        .get(AUTO_BACKLINK_SOURCE_METADATA_KEY)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

async fn relationship_edge_from_direction(
    state: &Arc<AppState>,
    auth: &AuthContext,
    relationship: Relationship,
    direction: &'static str,
) -> Result<Option<NodeRelationshipEdgeResponse>, (StatusCode, String)> {
    let related_node_id = if direction == "outgoing" {
        relationship.to_node
    } else {
        relationship.from_node
    };

    let Some(related_node) = state
        .engine
        .get_node(related_node_id)
        .await
        .map_err(map_mv_error)?
    else {
        return Ok(None);
    };

    if !auth.allows_namespace(&related_node.namespace) {
        return Ok(None);
    }

    Ok(Some(NodeRelationshipEdgeResponse {
        relationship_id: relationship.id.to_string(),
        relation_kind: relationship.kind.to_string(),
        direction: direction.to_string(),
        related_node_id: related_node.id.to_string(),
        related_node_title: related_node.title,
        related_node_kind: related_node.kind.as_str().to_string(),
        related_node_namespace: related_node.namespace,
        weight: relationship.weight,
        created_at: relationship.created_at,
        auto_managed: relationship_is_auto_managed(&relationship),
        auto_source: relationship_auto_source(&relationship),
    }))
}

async fn get_node_relationships(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<NodeRelationshipOverviewResponse>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let node_id = parse_uuid_param(&id, "node id")?;

    let node = state
        .engine
        .get_node(node_id)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let outgoing_rels = state
        .engine
        .graph
        .get_relationships_from(node_id)
        .await
        .map_err(map_mv_error)?;
    let incoming_rels = state
        .engine
        .graph
        .get_relationships_to(node_id)
        .await
        .map_err(map_mv_error)?;

    let mut outgoing = Vec::new();
    for relationship in outgoing_rels {
        if let Some(edge) =
            relationship_edge_from_direction(&state, &auth, relationship, "outgoing").await?
        {
            outgoing.push(edge);
        }
    }

    let mut incoming = Vec::new();
    for relationship in incoming_rels {
        if let Some(edge) =
            relationship_edge_from_direction(&state, &auth, relationship, "incoming").await?
        {
            incoming.push(edge);
        }
    }

    outgoing.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    incoming.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    Ok(Json(NodeRelationshipOverviewResponse {
        node_id: node_id.to_string(),
        outgoing,
        incoming,
    }))
}

async fn get_neighbors(
    Extension(auth): Extension<AuthContext>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<NeighborsQuery>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    authorize_read(&auth)?;
    let depth = params.depth.unwrap_or(2);
    validate_depth(depth).map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let uuid = parse_uuid_param(&id, "node id")?;

    let node = state
        .engine
        .get_node(uuid)
        .await
        .map_err(map_mv_error)?
        .ok_or((StatusCode::NOT_FOUND, "node not found".into()))?;
    authorize_namespace(&auth, &node.namespace)?;

    let neighbors = state
        .engine
        .get_neighbors(uuid, depth)
        .await
        .map_err(map_mv_error)?;

    let mut visible_neighbors = Vec::new();
    for neighbor_id in neighbors {
        if let Some(neighbor_node) = state
            .engine
            .get_node(neighbor_id)
            .await
            .map_err(map_mv_error)?
        {
            if auth.allows_namespace(&neighbor_node.namespace) {
                visible_neighbors.push(neighbor_id.to_string());
            }
        }
    }

    Ok(Json(visible_neighbors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use mv_core::{KnowledgeNode, MatchSource, NodeKind, RelationKind, Relationship, SearchResult};
    use mv_engine::config::EngineConfig;
    use mv_engine::engine::MindVaultEngine;
    use mv_engine::recurrence::{
        TASK_COMPLETED_AT_METADATA_KEY, TASK_COMPLETED_METADATA_KEY, TASK_DUE_AT_METADATA_KEY,
    };
    use tempfile::TempDir;

    async fn create_state_with_embedding(provider: &str, model: &str) -> (Arc<AppState>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let mut config = EngineConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };
        config.embedding.provider = provider.to_string();
        config.embedding.model = model.to_string();
        let engine = MindVaultEngine::init(config)
            .await
            .expect("test engine should initialize");
        (Arc::new(AppState::new(Arc::new(engine))), temp_dir)
    }

    #[tokio::test]
    async fn embedding_diagnostics_reports_fallback_status_for_unknown_provider() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let Json(response) = embedding_diagnostics(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
        )
        .await
        .expect("diagnostics should succeed");

        assert_eq!(response.configured_provider, "unknown-provider");
        assert_eq!(response.effective_provider, "noop");
        assert!(response.fallback_to_noop);
        assert!(response
            .reason
            .as_deref()
            .is_some_and(|value| value.contains("unknown embedding provider")));
    }

    #[test]
    fn completion_suggestions_prioritize_sentence_overlap() {
        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Prepare a design review checklist for Thursday planning. Capture decisions and follow-up actions for the API migration.".to_string(),
        )
        .with_title("Design Review Prep");

        let results = vec![SearchResult {
            node,
            score: 0.82,
            match_source: MatchSource::Hybrid,
        }];

        let suggestions =
            generate_completion_suggestions("Need to prepare for design review", &results, 3);
        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|suggestion| suggestion.to_ascii_lowercase().contains("design review")));
    }

    #[test]
    fn completion_suggestions_fall_back_when_no_candidates() {
        let suggestions = generate_completion_suggestions("x", &[], 2);
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions[0].to_ascii_lowercase().contains("next step"));
    }

    #[test]
    fn summary_transform_uses_recalled_context() {
        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Prepare onboarding checklist for Atlas release. Confirm staging sign-off with security lead."
                .to_string(),
        )
        .with_title("Atlas Launch Checklist");
        let results = vec![SearchResult {
            node,
            score: 0.84,
            match_source: MatchSource::Hybrid,
        }];

        let summary = generate_summary_transform("Need launch prep summary", &results, 2);
        assert!(summary.to_ascii_lowercase().contains("checklist"));
    }

    #[test]
    fn action_items_transform_returns_structured_items() {
        let node = KnowledgeNode::new(
            NodeKind::Task,
            "Draft migration timeline for API cutover. Validate rollback playbook with SRE."
                .to_string(),
        )
        .with_title("Migration prep");
        let results = vec![SearchResult {
            node,
            score: 0.77,
            match_source: MatchSource::Hybrid,
        }];

        let actions = generate_action_items_transform("migration tasks", &results, 3);
        assert!(!actions.is_empty());
        assert!(actions
            .iter()
            .any(|item| item.to_ascii_lowercase().contains("migration timeline")));
    }

    #[test]
    fn refine_transform_contains_summary_and_next_steps_sections() {
        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Capture weekly planning highlights and decision owners for roadmap execution."
                .to_string(),
        )
        .with_title("Weekly planning");
        let results = vec![SearchResult {
            node,
            score: 0.7,
            match_source: MatchSource::Hybrid,
        }];

        let refined = generate_refine_transform("planning draft", &results, 4);
        assert!(refined.contains("### Refined Summary"));
        assert!(refined.contains("### Suggested Next Steps"));
    }

    #[test]
    fn assist_transform_mode_parse_supports_aliases() {
        assert!(matches!(
            AssistTransformMode::parse(Some("summary")),
            Ok(AssistTransformMode::Summarize)
        ));
        assert!(matches!(
            AssistTransformMode::parse(Some("tasks")),
            Ok(AssistTransformMode::ActionItems)
        ));
        assert!(matches!(
            AssistTransformMode::parse(Some("clarify")),
            Ok(AssistTransformMode::Refine)
        ));
        assert!(AssistTransformMode::parse(Some("unsupported")).is_err());
    }

    #[test]
    fn autocomplete_completions_prioritize_prefix_candidates() {
        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Prepare migration checklist for API rollout and verify rollout notes.".to_string(),
        )
        .with_title("Prepare migration checklist");
        let results = vec![SearchResult {
            node,
            score: 0.8,
            match_source: MatchSource::Hybrid,
        }];

        let completions = generate_autocomplete_completions("prepare migration", &results, 3);
        assert!(!completions.is_empty());
        assert!(completions[0]
            .to_ascii_lowercase()
            .contains("prepare migration checklist"));
    }

    #[test]
    fn autocomplete_completions_fallback_when_none_match() {
        let completions = generate_autocomplete_completions("nonexistent phrase", &[], 2);
        assert_eq!(completions.len(), 2);
    }

    #[test]
    fn completion_sources_prioritize_high_score_and_dedupe_node_ids() {
        let duplicate_id = uuid::Uuid::now_v7();
        let mut duplicate_low = KnowledgeNode::new(
            NodeKind::Fact,
            "Atlas migration baseline and sequencing details.".to_string(),
        )
        .with_title("Atlas Migration");
        duplicate_low.id = duplicate_id;
        duplicate_low.namespace = "ops".to_string();

        let mut duplicate_high = KnowledgeNode::new(
            NodeKind::Fact,
            "Latest Atlas migration timeline with ownership and blockers.".to_string(),
        )
        .with_title("Atlas Migration v2");
        duplicate_high.id = duplicate_id;
        duplicate_high.namespace = "ops".to_string();

        let unique = KnowledgeNode::new(
            NodeKind::Procedure,
            "Runbook for rollback validation and post-deploy checks.".to_string(),
        )
        .with_title("Rollback Runbook")
        .with_namespace("ops");

        let results = vec![
            SearchResult {
                node: duplicate_low,
                score: 0.41,
                match_source: MatchSource::Hybrid,
            },
            SearchResult {
                node: unique,
                score: 0.76,
                match_source: MatchSource::Hybrid,
            },
            SearchResult {
                node: duplicate_high,
                score: 0.93,
                match_source: MatchSource::Hybrid,
            },
        ];

        let sources = collect_completion_sources(&results, 4);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].node_id, duplicate_id);
        assert_eq!(sources[0].title, "Atlas Migration v2");
        assert_eq!(sources[0].namespace, "ops");
        assert!(sources[0].score >= sources[1].score);
        assert_eq!(sources[1].title, "Rollback Runbook");
    }

    #[test]
    fn completion_sources_use_kind_fallback_for_untitled_nodes() {
        let untitled = KnowledgeNode::new(
            NodeKind::Task,
            "Coordinate release handoff and assign post-launch owners.".to_string(),
        )
        .with_namespace("ops");
        let results = vec![SearchResult {
            node: untitled,
            score: 0.6,
            match_source: MatchSource::Hybrid,
        }];

        let sources = collect_completion_sources(&results, 2);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "Untitled task");
        assert_eq!(sources[0].namespace, "ops");
    }

    #[tokio::test]
    async fn assist_completion_returns_grounding_sources() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        state
            .engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Fact,
                    "Atlas launch brief with release checklist and dependency map.".to_string(),
                )
                .with_title("Atlas Launch Brief")
                .with_namespace("ops"),
            )
            .await
            .expect("first node should store");
        state
            .engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Task,
                    "Prepare rollback checklist and schedule validation window.".to_string(),
                )
                .with_title("Rollback Checklist")
                .with_namespace("ops"),
            )
            .await
            .expect("second node should store");

        let Json(response) = assist_completion(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(AssistCompletionRequest {
                text: "Need atlas rollout guidance".to_string(),
                limit: Some(4),
                namespace: Some("ops".to_string()),
            }),
        )
        .await
        .expect("assist completion should succeed");

        assert!(!response.suggestions.is_empty());
        assert!(!response.sources.is_empty());
        assert!(response.sources.len() <= 5);
        assert!(response
            .sources
            .iter()
            .all(|source| !source.node_id.is_empty() && !source.title.is_empty()));
    }

    #[test]
    fn link_suggestions_prioritize_prefix_matches_and_exclude_current_node() {
        let excluded_id = uuid::Uuid::now_v7();
        let mut excluded = KnowledgeNode::new(
            NodeKind::Fact,
            "Current working draft for Project Atlas roadmap.".to_string(),
        )
        .with_title("Project Atlas Roadmap");
        excluded.id = excluded_id;

        let candidate_prefix = KnowledgeNode::new(
            NodeKind::Fact,
            "Milestone sequencing and scope decisions for the next release.".to_string(),
        )
        .with_title("Project Atlas Milestones");
        let candidate_contains = KnowledgeNode::new(
            NodeKind::Fact,
            "Atlas ownership updates and risk tracking notes.".to_string(),
        )
        .with_title("Weekly Atlas Review");

        let results = vec![
            SearchResult {
                node: excluded,
                score: 0.95,
                match_source: MatchSource::Hybrid,
            },
            SearchResult {
                node: candidate_contains,
                score: 0.76,
                match_source: MatchSource::Hybrid,
            },
            SearchResult {
                node: candidate_prefix,
                score: 0.71,
                match_source: MatchSource::Hybrid,
            },
        ];

        let suggestions =
            generate_link_suggestions("project atlas", &results, 3, Some(excluded_id));
        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .all(|candidate| candidate.node_id != excluded_id));
        assert_eq!(suggestions[0].title, "Project Atlas Milestones");
        assert_eq!(suggestions[0].reason, "title_prefix_match");
    }

    #[test]
    fn link_suggestions_fall_back_to_semantic_titles_when_lexical_match_is_missing() {
        let candidate = KnowledgeNode::new(
            NodeKind::Fact,
            "Operational status digest with highlights from async updates.".to_string(),
        )
        .with_title("Quarterly Ops Digest");
        let results = vec![SearchResult {
            node: candidate,
            score: 0.88,
            match_source: MatchSource::Hybrid,
        }];

        let suggestions = generate_link_suggestions("unrelated-token", &results, 2, None);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].title, "Quarterly Ops Digest");
        assert_eq!(suggestions[0].reason, "semantic_match");
    }

    #[test]
    fn link_suggestions_support_heading_targets_for_anchor_queries() {
        let candidate = KnowledgeNode::new(
            NodeKind::Fact,
            "# Risks\n- Database migration rollback path\n## Mitigations\n- Shadow deploy strategy\n"
                .to_string(),
        )
        .with_title("Project Atlas Roadmap");

        let results = vec![SearchResult {
            node: candidate,
            score: 0.83,
            match_source: MatchSource::Hybrid,
        }];

        let suggestions = generate_link_suggestions("project atlas#ri", &results, 4, None);
        let heading_match = suggestions
            .iter()
            .find(|candidate| candidate.heading.as_deref() == Some("Risks"))
            .expect("expected heading-level suggestion");

        assert_eq!(heading_match.title, "Project Atlas Roadmap");
        assert!(heading_match.reason.starts_with("heading_"));
        assert!(heading_match
            .preview
            .as_deref()
            .is_some_and(|preview| preview.to_ascii_lowercase().contains("rollback")));
    }

    #[test]
    fn sanitize_attachment_filename_replaces_unsafe_chars() {
        let sanitized = sanitize_attachment_filename("../my report?.pdf");
        assert_eq!(sanitized, "my_report_.pdf");
    }

    #[test]
    fn parse_node_attachments_skips_invalid_items() {
        let mut node = KnowledgeNode::new(NodeKind::Fact, "x".into());
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": "a-1",
                    "file_name": "demo.txt",
                    "content_type": "text/plain",
                    "size_bytes": 10,
                    "stored_path": "/tmp/demo.txt",
                    "uploaded_at": "2026-02-06T00:00:00Z"
                },
                {
                    "not_attachment": true
                }
            ]),
        );

        let parsed = parse_node_attachments(&node);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "a-1");
    }

    #[test]
    fn attachment_search_blob_sync_aggregates_index_entries() {
        let mut node = KnowledgeNode::new(NodeKind::Fact, "x".into());
        upsert_attachment_text_index_entry(&mut node, "att-a", Some("alpha planning notes"));
        upsert_attachment_text_index_entry(&mut node, "att-b", Some("beta incident summary"));
        sync_attachment_search_blob_metadata(&mut node);

        let combined = node
            .metadata
            .get(ATTACHMENT_SEARCH_BLOB_METADATA_KEY)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        assert!(combined.contains("alpha planning notes"));
        assert!(combined.contains("beta incident summary"));
    }

    #[test]
    fn attachment_chunk_index_tracks_chunked_entries() {
        let mut node = KnowledgeNode::new(NodeKind::Fact, "x".into());
        upsert_attachment_text_chunk_index_entry(
            &mut node,
            "att-a",
            Some("alpha planning notes for sprint 14. capture owner updates and blockers."),
        );
        upsert_attachment_text_chunk_index_entry(
            &mut node,
            "att-b",
            Some("beta incident summary with timeline and rollback mitigation actions."),
        );

        let chunk_index = node
            .metadata
            .get(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY)
            .and_then(serde_json::Value::as_object)
            .expect("chunk metadata should be present");
        assert!(chunk_index.contains_key("att-a"));
        assert!(chunk_index.contains_key("att-b"));

        remove_attachment_text_chunk_index_entry(&mut node, "att-a");
        let chunk_index_after = node
            .metadata
            .get(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY)
            .and_then(serde_json::Value::as_object)
            .expect("chunk metadata should remain for att-b");
        assert!(!chunk_index_after.contains_key("att-a"));
        assert!(chunk_index_after.contains_key("att-b"));
    }

    #[test]
    fn render_template_value_with_values_replaces_supported_placeholder_shapes() {
        let mut values = std::collections::HashMap::new();
        values.insert("project".to_string(), "Atlas".to_string());
        values.insert("owner".to_string(), "Morgan".to_string());

        let rendered = render_template_value_with_values(
            "Build {{project}} roadmap for {{ owner }} by Friday",
            &values,
        );
        assert_eq!(rendered, "Build Atlas roadmap for Morgan by Friday");
    }

    #[tokio::test]
    async fn create_and_instantiate_template_round_trip() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: "Ship {{project}} plan with {{owner}}".into(),
                title: Some("Plan {{project}}".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["planning".into()]),
                importance: Some(0.7),
                metadata: None,
                template_key: Some("task.plan".into()),
                template_variables: None,
            }),
        )
        .await
        .expect("template creation should succeed");
        assert_eq!(status, StatusCode::CREATED);
        assert!(is_template_node(&template));
        assert_eq!(template.kind, NodeKind::Template);
        assert!(template
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("template")));
        assert_eq!(
            template
                .metadata
                .get(TEMPLATE_TARGET_KIND_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some("task")
        );

        let Json(templates) = list_templates(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(TemplateListQuery {
                namespace: Some("ops".into()),
                kind: Some("task".into()),
                limit: Some(20),
                offset: Some(0),
            }),
        )
        .await
        .expect("template list should succeed");
        assert!(templates.iter().any(|item| item.id == template.id));

        let mut values = std::collections::HashMap::new();
        values.insert("project".to_string(), "Atlas".to_string());
        values.insert("owner".to_string(), "Morgan".to_string());
        let (status, Json(instance)) = instantiate_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
            Json(InstantiateTemplateRequest {
                namespace: Some("ops".into()),
                title: None,
                tags: Some(vec!["urgent".into()]),
                values: Some(values),
                metadata: None,
            }),
        )
        .await
        .expect("template instantiation should succeed");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(instance.kind, NodeKind::Task);
        assert_eq!(instance.title.as_deref(), Some("Plan Atlas"));
        assert_eq!(instance.content, "Ship Atlas plan with Morgan");
        assert!(!instance
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("template")));
        assert!(instance.tags.iter().any(|tag| tag == "planning"));
        assert!(instance.tags.iter().any(|tag| tag == "urgent"));
        let expected_template_id = template.id.to_string();
        assert_eq!(
            instance
                .metadata
                .get(TEMPLATE_SOURCE_ID_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some(expected_template_id.as_str())
        );

        let derived = state
            .engine
            .graph
            .get_relationships_from(template.id)
            .await
            .expect("relationship lookup should succeed");
        assert!(derived
            .iter()
            .any(|rel| { rel.to_node == instance.id && rel.kind == RelationKind::DerivedFrom }));

        let refreshed_template = state
            .engine
            .get_node(template.id)
            .await
            .expect("template lookup should succeed")
            .expect("template should still exist");
        assert_eq!(
            refreshed_template
                .metadata
                .get(TEMPLATE_INSTANTIATION_COUNT_METADATA_KEY)
                .and_then(serde_json::Value::as_u64),
            Some(1)
        );
        assert!(refreshed_template
            .metadata
            .contains_key(TEMPLATE_LAST_INSTANTIATED_AT_METADATA_KEY));
    }

    #[tokio::test]
    async fn duplicate_and_delete_template_round_trip() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: "Run {{workflow}} checklist".into(),
                title: Some("Ops Checklist".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["runbook".into()]),
                importance: Some(0.8),
                metadata: Some(std::collections::HashMap::from([(
                    "scope".to_string(),
                    serde_json::Value::String("oncall".to_string()),
                )])),
                template_key: Some("ops.checklist".into()),
                template_variables: Some(vec!["workflow".into()]),
            }),
        )
        .await
        .expect("template creation should succeed");
        assert_eq!(status, StatusCode::CREATED);

        let (status, Json(duplicate)) = duplicate_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
            Json(DuplicateTemplateRequest {
                namespace: Some("ops".into()),
                title: None,
                tags: Some(vec!["copy".into()]),
                template_key: Some("ops.checklist.copy".into()),
                metadata: Some(std::collections::HashMap::from([(
                    "scope".to_string(),
                    serde_json::Value::String("ops-automation".to_string()),
                )])),
            }),
        )
        .await
        .expect("template duplication should succeed");
        assert_eq!(status, StatusCode::CREATED);
        assert_ne!(duplicate.id, template.id);
        assert!(is_template_node(&duplicate));
        assert_eq!(duplicate.kind, NodeKind::Template);
        assert_eq!(
            duplicate
                .metadata
                .get(TEMPLATE_TARGET_KIND_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some("task")
        );
        assert!(duplicate
            .title
            .as_deref()
            .unwrap_or_default()
            .contains("Ops Checklist"));
        assert!(duplicate
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("copy")));
        assert_eq!(
            duplicate
                .metadata
                .get(TEMPLATE_KEY_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some("ops.checklist.copy")
        );
        assert_eq!(
            duplicate
                .metadata
                .get("scope")
                .and_then(serde_json::Value::as_str),
            Some("ops-automation")
        );

        let Json(delete_response) = delete_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(duplicate.id.to_string()),
        )
        .await
        .expect("template delete should succeed");
        assert_eq!(
            delete_response
                .get("deleted")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );

        let deleted_node = state
            .engine
            .get_node(duplicate.id)
            .await
            .expect("template lookup should succeed");
        assert!(deleted_node.is_none());
    }

    #[tokio::test]
    async fn template_version_history_tracks_updates_and_restore() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let original_content = "Run {{workflow}} checklist".to_string();
        let original_title = "Ops Checklist".to_string();

        let (status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: original_content.clone(),
                title: Some(original_title.clone()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["runbook".into()]),
                importance: Some(0.8),
                metadata: None,
                template_key: Some("ops.checklist".into()),
                template_variables: Some(vec!["workflow".into()]),
            }),
        )
        .await
        .expect("template creation should succeed");
        assert_eq!(status, StatusCode::CREATED);

        let mut edited_template = template.clone();
        edited_template.content = "Run {{workflow}} checklist with incident follow-ups".into();
        edited_template.title = Some("Ops Checklist v2".into());
        let Json(updated) = update_node(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
            Json(edited_template),
        )
        .await
        .expect("template update should succeed");
        assert_eq!(
            updated.content,
            "Run {{workflow}} checklist with incident follow-ups"
        );

        let Json(versions) = list_template_versions(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
        )
        .await
        .expect("version list should succeed");
        assert_eq!(versions.len(), 1);
        let restore_version_id = versions[0].version_id.clone();

        let Json(restored) = restore_template_version(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((template.id.to_string(), restore_version_id.clone())),
        )
        .await
        .expect("version restore should succeed");
        assert_eq!(restored.content, original_content);
        assert_eq!(restored.title.as_deref(), Some(original_title.as_str()));
        assert_eq!(
            restored
                .metadata
                .get(TEMPLATE_RESTORED_FROM_VERSION_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some(restore_version_id.as_str())
        );

        let Json(versions_after_restore) = list_template_versions(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
        )
        .await
        .expect("version list after restore should succeed");
        assert_eq!(versions_after_restore.len(), 2);
    }

    #[tokio::test]
    async fn get_template_version_returns_diff_summary() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: "Line A\nLine B".into(),
                title: Some("Template Diff Test".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["diff".into()]),
                importance: Some(0.7),
                metadata: None,
                template_key: Some("ops.diff.test".into()),
                template_variables: None,
            }),
        )
        .await
        .expect("template creation should succeed");
        assert_eq!(status, StatusCode::CREATED);

        let mut edited_template = template.clone();
        edited_template.content = "Line A\nLine C".into();
        edited_template.title = Some("Template Diff Test v2".into());
        let _updated = update_node(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
            Json(edited_template),
        )
        .await
        .expect("template update should succeed");

        let Json(versions) = list_template_versions(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
        )
        .await
        .expect("version list should succeed");
        assert_eq!(versions.len(), 1);
        let version_id = versions[0].version_id.clone();

        let Json(detail) = get_template_version(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((template.id.to_string(), version_id)),
        )
        .await
        .expect("version detail should succeed");

        assert_eq!(
            detail.current.title.as_deref(),
            Some("Template Diff Test v2")
        );
        assert!(detail.diff.added_line_count >= 1);
        assert!(detail.diff.removed_line_count >= 1);
        assert!(detail
            .diff
            .added_line_samples
            .iter()
            .any(|line| line.contains("Line C")));
        assert!(detail
            .diff
            .removed_line_samples
            .iter()
            .any(|line| line.contains("Line B")));
        assert!(detail.field_changes.iter().any(|change| {
            change.field == "title"
                && change.changed
                && change.version_value.contains("Template Diff Test")
                && change.current_value.contains("Template Diff Test v2")
        }));
    }

    #[tokio::test]
    async fn list_template_packs_returns_curated_entries() {
        let Json(packs) = list_template_packs(Extension(AuthContext::system_admin()))
            .await
            .expect("template packs should list");

        assert!(packs.len() >= 3);
        assert!(packs.iter().any(|pack| pack.pack_id == "daily-flow"));
        assert!(packs.iter().any(|pack| pack.pack_id == "project-execution"));
        assert!(packs.iter().any(|pack| pack.pack_id == "meeting-system"));
    }

    #[tokio::test]
    async fn install_template_pack_is_idempotent_and_supports_overwrite() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (_status, Json(first_install)) = install_template_pack(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path("daily-flow".to_string()),
            Json(InstallTemplatePackRequest {
                namespace: Some("ops".into()),
                overwrite_existing: Some(false),
                additional_tags: None,
            }),
        )
        .await
        .expect("first template-pack install should succeed");

        assert_eq!(first_install.installed_templates, 3);
        assert_eq!(first_install.updated_templates, 0);
        assert_eq!(first_install.skipped_templates, 0);

        let (_status, Json(second_install)) = install_template_pack(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path("daily-flow".to_string()),
            Json(InstallTemplatePackRequest {
                namespace: Some("ops".into()),
                overwrite_existing: Some(false),
                additional_tags: None,
            }),
        )
        .await
        .expect("second template-pack install should succeed");

        assert_eq!(second_install.installed_templates, 0);
        assert_eq!(second_install.updated_templates, 0);
        assert_eq!(second_install.skipped_templates, 3);

        let (_status, Json(overwrite_install)) = install_template_pack(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path("daily-flow".to_string()),
            Json(InstallTemplatePackRequest {
                namespace: Some("ops".into()),
                overwrite_existing: Some(true),
                additional_tags: Some(vec!["autopack".into()]),
            }),
        )
        .await
        .expect("overwrite install should succeed");

        assert_eq!(overwrite_install.installed_templates, 0);
        assert_eq!(overwrite_install.updated_templates, 3);
        assert_eq!(overwrite_install.skipped_templates, 0);

        let Json(installed_templates) = list_templates(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(TemplateListQuery {
                namespace: Some("ops".into()),
                kind: None,
                limit: Some(50),
                offset: Some(0),
            }),
        )
        .await
        .expect("list templates should succeed");

        assert_eq!(installed_templates.len(), 3);
        assert!(installed_templates.iter().all(|node| node
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("autopack"))));
    }

    #[tokio::test]
    async fn install_template_pack_rejects_unknown_pack() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let err = install_template_pack(
            Extension(AuthContext::system_admin()),
            State(state),
            Path("unknown-pack".to_string()),
            Json(InstallTemplatePackRequest {
                namespace: Some("ops".into()),
                overwrite_existing: Some(false),
                additional_tags: None,
            }),
        )
        .await
        .expect_err("unknown pack should fail");

        assert_eq!(err.0, StatusCode::NOT_FOUND);
        assert!(err.1.contains("template pack not found"));
    }

    #[tokio::test]
    async fn ensure_daily_note_handler_is_idempotent() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let request = DailyNotesEnsureRequest {
            namespace: Some("journal".into()),
            date: Some("2026-02-06".into()),
        };

        let (first_status, Json(first)) = ensure_daily_note(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(request),
        )
        .await
        .expect("first ensure should succeed");

        assert_eq!(first_status, StatusCode::CREATED);
        assert!(first.created);

        let (second_status, Json(second)) = ensure_daily_note(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(DailyNotesEnsureRequest {
                namespace: Some("journal".into()),
                date: Some("2026-02-06".into()),
            }),
        )
        .await
        .expect("second ensure should succeed");

        assert_eq!(second_status, StatusCode::OK);
        assert!(!second.created);
        assert_eq!(second.node.id, first.node.id);

        let Json(notes) = list_daily_notes(
            Extension(AuthContext::system_admin()),
            State(state),
            Query(DailyNotesListQuery {
                namespace: Some("journal".into()),
                date: None,
                limit: Some(10),
                offset: Some(0),
            }),
        )
        .await
        .expect("list should succeed");

        assert!(!notes.is_empty());
        assert!(notes.iter().any(|node| node.id == first.node.id));
    }

    #[tokio::test]
    async fn ensure_daily_note_rejects_invalid_date() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let err = ensure_daily_note(
            Extension(AuthContext::system_admin()),
            State(state),
            Json(DailyNotesEnsureRequest {
                namespace: Some("journal".into()),
                date: Some("2026-99-99".into()),
            }),
        )
        .await
        .expect_err("invalid date should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err.1.contains("YYYY-MM-DD"));
    }

    #[test]
    fn parse_ical_events_parses_unfolded_entries_and_extensions() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:123e4567-e89b-12d3-a456-426614174000@mindvault.local\r\nSUMMARY:Weekly\\, Planning\r\nDESCRIPTION:Line one\\n line two\r\nCATEGORIES:task,planning\r\nDTSTART:20260206T090000Z\r\nDTEND:20260206T093000Z\r\nSTATUS:COMPLETED\r\nX-MINDVAULT-KIND:task\r\nX-MINDVAULT-NAMESPACE:ops\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let (events, parse_errors) = parse_ical_events(ical);
        assert_eq!(parse_errors, 0);
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(
            event.uid.as_deref(),
            Some("123e4567-e89b-12d3-a456-426614174000@mindvault.local")
        );
        assert_eq!(event.kind, Some(NodeKind::Task));
        assert_eq!(event.namespace.as_deref(), Some("ops"));
        assert_eq!(event.summary.as_deref(), Some("Weekly, Planning"));
        assert_eq!(
            event.categories,
            vec!["task".to_string(), "planning".to_string()]
        );
        assert_eq!(event.status.as_deref(), Some("COMPLETED"));
        assert!(event.starts_at.is_some());
        assert!(event.ends_at.is_some());
    }

    #[test]
    fn node_from_ical_event_maps_completed_task_metadata() {
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 8, 0, 0)
            .single()
            .expect("valid datetime");
        let starts_at = Utc
            .with_ymd_and_hms(2026, 2, 6, 9, 0, 0)
            .single()
            .expect("valid datetime");

        let event = ParsedIcalEvent {
            uid: Some("test-uid".into()),
            node_id: None,
            kind: Some(NodeKind::Task),
            namespace: Some("ops".into()),
            summary: Some("Prepare weekly review".into()),
            description: Some("Outline the release notes".into()),
            categories: vec!["task".into(), "planning".into()],
            status: Some("COMPLETED".into()),
            starts_at: Some(starts_at),
            ends_at: None,
        };
        let node = node_from_ical_event(&event, "ops", NodeKind::Event, now)
            .expect("node conversion should succeed");

        assert_eq!(node.kind, NodeKind::Task);
        assert_eq!(node.namespace, "ops");
        assert_eq!(node.title.as_deref(), Some("Prepare weekly review"));
        assert_eq!(
            node.metadata
                .get(TASK_DUE_AT_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some(starts_at.to_rfc3339().as_str())
        );
        assert_eq!(
            node.metadata
                .get(TASK_COMPLETED_METADATA_KEY)
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            node.metadata
                .get(ICAL_UID_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some("test-uid")
        );
    }

    #[tokio::test]
    async fn list_calendar_items_handler_filters_and_sorts_scheduled_nodes() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let due_task_at = Utc
            .with_ymd_and_hms(2026, 2, 4, 9, 0, 0)
            .single()
            .expect("valid datetime");
        let event_start_at = Utc
            .with_ymd_and_hms(2026, 2, 5, 15, 30, 0)
            .single()
            .expect("valid datetime");
        let outside_range_due_at = Utc
            .with_ymd_and_hms(2026, 3, 2, 12, 0, 0)
            .single()
            .expect("valid datetime");

        let mut task = KnowledgeNode::new(NodeKind::Task, "Finish weekly planning".into())
            .with_namespace("ops");
        task.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(due_task_at.to_rfc3339()),
        );

        let mut event =
            KnowledgeNode::new(NodeKind::Event, "Leadership sync".into()).with_namespace("ops");
        event.metadata.insert(
            EVENT_START_AT_METADATA_KEY.into(),
            serde_json::Value::String(event_start_at.to_rfc3339()),
        );

        let mut future_task =
            KnowledgeNode::new(NodeKind::Task, "Quarter planning".into()).with_namespace("ops");
        future_task.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(outside_range_due_at.to_rfc3339()),
        );

        state
            .engine
            .store_node(event)
            .await
            .expect("event should store");
        state
            .engine
            .store_node(future_task)
            .await
            .expect("future task should store");
        state
            .engine
            .store_node(task)
            .await
            .expect("task should store");

        let Json(calendar) = list_calendar_items(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(CalendarItemsQuery {
                namespace: Some("ops".into()),
                view: Some("week".into()),
                anchor: Some("2026-02-06T10:00:00Z".into()),
                start: None,
                end: None,
                limit: Some(25),
                include_completed: Some(false),
            }),
        )
        .await
        .expect("calendar list should succeed");

        assert_eq!(calendar.view, "week");
        assert_eq!(calendar.total_items, 2);
        assert_eq!(calendar.returned_items, 2);
        assert_eq!(calendar.items[0].node.kind, NodeKind::Task);
        assert_eq!(calendar.items[0].schedule_source, TASK_DUE_AT_METADATA_KEY);
        assert_eq!(calendar.items[1].node.kind, NodeKind::Event);
        assert_eq!(
            calendar.items[1].schedule_source,
            EVENT_START_AT_METADATA_KEY
        );
        assert!(calendar.items[0].scheduled_at < calendar.items[1].scheduled_at);
    }

    #[tokio::test]
    async fn list_calendar_items_handler_hides_completed_tasks_unless_requested() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let task_due_at = Utc
            .with_ymd_and_hms(2026, 2, 6, 8, 0, 0)
            .single()
            .expect("valid datetime");
        let event_start_at = Utc
            .with_ymd_and_hms(2026, 2, 6, 11, 0, 0)
            .single()
            .expect("valid datetime");

        let mut completed_task =
            KnowledgeNode::new(NodeKind::Task, "Submit invoice".into()).with_namespace("ops");
        completed_task.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(task_due_at.to_rfc3339()),
        );
        completed_task.metadata.insert(
            TASK_COMPLETED_METADATA_KEY.into(),
            serde_json::Value::Bool(true),
        );

        let mut event =
            KnowledgeNode::new(NodeKind::Event, "Client kickoff".into()).with_namespace("ops");
        event.metadata.insert(
            EVENT_START_AT_METADATA_KEY.into(),
            serde_json::Value::String(event_start_at.to_rfc3339()),
        );

        state
            .engine
            .store_node(completed_task)
            .await
            .expect("completed task should store");
        state
            .engine
            .store_node(event)
            .await
            .expect("event should store");

        let Json(default_view) = list_calendar_items(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(CalendarItemsQuery {
                namespace: Some("ops".into()),
                view: Some("day".into()),
                anchor: Some("2026-02-06T00:00:00Z".into()),
                start: None,
                end: None,
                limit: Some(20),
                include_completed: Some(false),
            }),
        )
        .await
        .expect("calendar list should succeed");
        assert_eq!(default_view.items.len(), 1);
        assert_eq!(default_view.items[0].node.kind, NodeKind::Event);

        let Json(include_completed_view) = list_calendar_items(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(CalendarItemsQuery {
                namespace: Some("ops".into()),
                view: Some("day".into()),
                anchor: Some("2026-02-06T00:00:00Z".into()),
                start: None,
                end: None,
                limit: Some(20),
                include_completed: Some(true),
            }),
        )
        .await
        .expect("calendar list with completed should succeed");
        assert_eq!(include_completed_view.items.len(), 2);
    }

    #[tokio::test]
    async fn list_calendar_items_handler_rejects_partial_custom_range() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let err = list_calendar_items(
            Extension(AuthContext::system_admin()),
            State(state),
            Query(CalendarItemsQuery {
                namespace: None,
                view: Some("week".into()),
                anchor: Some("2026-02-06T00:00:00Z".into()),
                start: Some("2026-02-01T00:00:00Z".into()),
                end: None,
                limit: Some(10),
                include_completed: Some(false),
            }),
        )
        .await
        .expect_err("partial range should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err.1.contains("start and end must both be set"));
    }

    #[test]
    fn build_calendar_ical_document_escapes_text_and_includes_metadata_extensions() {
        let start = Utc
            .with_ymd_and_hms(2026, 2, 6, 9, 30, 0)
            .single()
            .expect("valid datetime");
        let end = Utc
            .with_ymd_and_hms(2026, 2, 6, 10, 0, 0)
            .single()
            .expect("valid datetime");
        let generated_at = Utc
            .with_ymd_and_hms(2026, 2, 6, 8, 0, 0)
            .single()
            .expect("valid datetime");

        let mut node = KnowledgeNode::new(NodeKind::Event, "Line one,\nline two".into())
            .with_namespace("ops")
            .with_title("Team sync; roadmap");
        node.tags = vec!["meeting".into(), "weekly".into()];

        let document = build_calendar_ical_document(
            &[CalendarItemResponse {
                node,
                scheduled_at: start,
                scheduled_end_at: Some(end),
                schedule_source: EVENT_START_AT_METADATA_KEY.to_string(),
                completed: false,
            }],
            generated_at,
            start - Duration::days(1),
            start + Duration::days(1),
        );

        assert!(document.contains("BEGIN:VCALENDAR\r\n"));
        assert!(document.contains("BEGIN:VEVENT\r\n"));
        assert!(document.contains("SUMMARY:Team sync\\; roadmap\r\n"));
        assert!(document.contains("DESCRIPTION:Kind: event\\nNamespace: ops"));
        assert!(document.contains("X-MINDVAULT-SCHEDULE-SOURCE:event_start_at\r\n"));
        assert!(document.contains("CATEGORIES:meeting,weekly\r\n"));
        assert!(document.contains("DTSTART:20260206T093000Z\r\n"));
        assert!(document.contains("DTEND:20260206T100000Z\r\n"));
    }

    #[tokio::test]
    async fn export_calendar_ical_handler_returns_calendar_headers() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut task = KnowledgeNode::new(NodeKind::Task, "Prepare weekly report".into())
            .with_namespace("ops");
        task.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String("2026-02-06T09:00:00Z".to_string()),
        );
        state
            .engine
            .store_node(task)
            .await
            .expect("task should store");

        let response = export_calendar_ical(
            Extension(AuthContext::system_admin()),
            State(state),
            Query(CalendarItemsQuery {
                namespace: Some("ops".into()),
                view: Some("day".into()),
                anchor: Some("2026-02-06T00:00:00Z".into()),
                start: None,
                end: None,
                limit: Some(50),
                include_completed: Some(false),
            }),
        )
        .await
        .expect("ical export should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/calendar; charset=utf-8")
        );
        assert!(response
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("attachment; filename=\"mindvault-calendar-")));
    }

    #[tokio::test]
    async fn list_due_tasks_handler_filters_and_sorts_tasks() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let due_a = Utc
            .with_ymd_and_hms(2026, 2, 6, 9, 0, 0)
            .single()
            .expect("valid datetime");
        let due_b = Utc
            .with_ymd_and_hms(2026, 2, 6, 11, 0, 0)
            .single()
            .expect("valid datetime");

        let mut task_a = KnowledgeNode::new(NodeKind::Task, "Finish incident review".into())
            .with_namespace("ops");
        task_a.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(due_a.to_rfc3339()),
        );
        let mut task_b =
            KnowledgeNode::new(NodeKind::Task, "Send retro summary".into()).with_namespace("ops");
        task_b.metadata.insert(
            TASK_DUE_AT_METADATA_KEY.into(),
            serde_json::Value::String(due_b.to_rfc3339()),
        );
        task_b.metadata.insert(
            TASK_COMPLETED_METADATA_KEY.into(),
            serde_json::Value::Bool(true),
        );

        state
            .engine
            .store_node(task_b)
            .await
            .expect("task b should store");
        state
            .engine
            .store_node(task_a)
            .await
            .expect("task a should store");

        let Json(tasks) = list_due_tasks(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(DueTasksQuery {
                namespace: Some("ops".into()),
                before: Some("2026-02-06T12:00:00Z".into()),
                limit: Some(20),
                include_completed: Some(false),
            }),
        )
        .await
        .expect("due task list should succeed");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].content, "Finish incident review");

        let Json(all_tasks) = list_due_tasks(
            Extension(AuthContext::system_admin()),
            State(state),
            Query(DueTasksQuery {
                namespace: Some("ops".into()),
                before: Some("2026-02-06T12:00:00Z".into()),
                limit: Some(20),
                include_completed: Some(true),
            }),
        )
        .await
        .expect("due task list with completed should succeed");

        assert_eq!(all_tasks.len(), 2);
        assert_eq!(all_tasks[0].content, "Finish incident review");
        assert_eq!(all_tasks[1].content, "Send retro summary");
    }

    #[tokio::test]
    async fn prioritize_tasks_handler_returns_ranked_focus_list() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
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

        state
            .engine
            .store_node(urgent)
            .await
            .expect("urgent task should store");
        state
            .engine
            .store_node(important)
            .await
            .expect("important task should store");
        state
            .engine
            .store_node(later)
            .await
            .expect("later task should store");

        let Json(response) = prioritize_tasks(
            Extension(AuthContext::system_admin()),
            State(state),
            Json(PrioritizeTasksRequest {
                namespace: Some("ops".into()),
                limit: Some(5),
                include_completed: Some(false),
                include_without_due: Some(true),
                persist: Some(false),
                now: Some(now.to_rfc3339()),
            }),
        )
        .await
        .expect("prioritize tasks should succeed");

        assert_eq!(response.items.len(), 3);
        assert_eq!(response.items[0].task.content, "Urgent");
        assert_eq!(response.items[0].rank, 1);
        assert_eq!(response.generated_at, now.to_rfc3339());
    }

    #[tokio::test]
    async fn list_due_tasks_handler_rejects_invalid_before_datetime() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let err = list_due_tasks(
            Extension(AuthContext::system_admin()),
            State(state),
            Query(DueTasksQuery {
                namespace: None,
                before: Some("invalid".into()),
                limit: Some(10),
                include_completed: Some(false),
            }),
        )
        .await
        .expect_err("invalid datetime should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err.1.contains("RFC3339"));
    }

    #[tokio::test]
    async fn complete_and_reopen_task_updates_completion_metadata() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let stored = state
            .engine
            .store_node(
                KnowledgeNode::new(NodeKind::Task, "Review on-call checklist".into())
                    .with_namespace("ops"),
            )
            .await
            .expect("task should store");

        let Json(completed) = complete_task(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(stored.id.to_string()),
        )
        .await
        .expect("complete endpoint should succeed");

        assert_eq!(
            completed
                .metadata
                .get(TASK_COMPLETED_METADATA_KEY)
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert!(completed
            .metadata
            .contains_key(TASK_COMPLETED_AT_METADATA_KEY));

        let Json(reopened) = reopen_task(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(stored.id.to_string()),
        )
        .await
        .expect("reopen endpoint should succeed");

        assert_eq!(
            reopened
                .metadata
                .get(TASK_COMPLETED_METADATA_KEY)
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert!(!reopened
            .metadata
            .contains_key(TASK_COMPLETED_AT_METADATA_KEY));
    }

    #[tokio::test]
    async fn complete_task_rejects_non_task_node() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let stored = state
            .engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "This is a fact node".into())
                    .with_namespace("ops"),
            )
            .await
            .expect("fact should store");

        let err = complete_task(
            Extension(AuthContext::system_admin()),
            State(state),
            Path(stored.id.to_string()),
        )
        .await
        .expect_err("complete endpoint should reject non-task");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err.1.contains("kind=task"));
    }

    #[tokio::test]
    async fn saved_search_lifecycle_create_run_update_delete() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let matching = state
            .engine
            .store_node(
                KnowledgeNode::new(
                    NodeKind::Task,
                    "Finalize the incident report before deadline".into(),
                )
                .with_namespace("ops")
                .with_tags(vec!["urgent".into(), "incident".into()]),
            )
            .await
            .expect("matching task should store");
        state
            .engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Unrelated retrospective note".into())
                    .with_namespace("ops")
                    .with_tags(vec!["retrospective".into()]),
            )
            .await
            .expect("non-matching note should store");

        let (_status, Json(created)) = create_saved_search(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateSavedSearchRequest {
                name: "Urgent Deadline Tasks".into(),
                description: Some("Tracks urgent work".into()),
                query: "deadline".into(),
                search_type: Some("hybrid".into()),
                limit: Some(20),
                namespace: Some("ops".into()),
                target_namespace: Some("ops".into()),
                kinds: Some(vec!["task".into()]),
                tags: Some(vec!["urgent".into()]),
                min_score: Some(0.0),
                min_importance: None,
            }),
        )
        .await
        .expect("saved search should create");

        assert_eq!(created.name, "Urgent Deadline Tasks");
        assert_eq!(created.namespace, "ops");
        assert_eq!(created.search_type, "hybrid");
        assert_eq!(created.kinds, vec!["task".to_string()]);
        assert_eq!(created.tags, vec!["urgent".to_string()]);

        let Json(run_response) = run_saved_search(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(created.id.clone()),
        )
        .await
        .expect("saved search should run");
        assert!(!run_response.results.is_empty());
        assert!(run_response
            .results
            .iter()
            .any(|item| item.node.id == matching.id));
        assert!(run_response
            .results
            .iter()
            .all(|item| item.node.kind == NodeKind::Task));

        let Json(updated) = update_saved_search(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(created.id.clone()),
            Json(UpdateSavedSearchRequest {
                name: Some("Urgent Tasks".into()),
                description: Some(Some("".into())),
                query: None,
                search_type: Some("vector".into()),
                limit: Some(5),
                target_namespace: None,
                kinds: Some(vec![]),
                tags: Some(vec!["incident".into()]),
                min_score: Some(Some(0.0)),
                min_importance: Some(Some(0.0)),
            }),
        )
        .await
        .expect("saved search should update");
        assert_eq!(updated.name, "Urgent Tasks");
        assert_eq!(updated.search_type, "vector");
        assert_eq!(updated.limit, 5);
        assert!(updated.kinds.is_empty());
        assert_eq!(updated.tags, vec!["incident".to_string()]);

        let Json(listed) = list_saved_searches(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(SavedSearchListQuery {
                namespace: Some("ops".into()),
                limit: Some(100),
                offset: Some(0),
            }),
        )
        .await
        .expect("saved searches should list");
        assert!(listed.iter().any(|item| item.id == created.id));

        let Json(delete_response) = delete_saved_search(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(created.id.clone()),
        )
        .await
        .expect("saved search should delete");
        assert_eq!(
            delete_response
                .get("deleted")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[tokio::test]
    async fn saved_search_create_rejects_forbidden_target_namespace() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let auth = AuthContext {
            subject: Some("user-a".into()),
            role: crate::auth::AuthRole::Write,
            namespace: Some("ops".into()),
        };

        let err = create_saved_search(
            Extension(auth),
            State(state),
            Json(CreateSavedSearchRequest {
                name: "Cross Namespace".into(),
                description: None,
                query: "deadline".into(),
                search_type: Some("hybrid".into()),
                limit: Some(10),
                namespace: Some("ops".into()),
                target_namespace: Some("finance".into()),
                kinds: None,
                tags: None,
                min_score: None,
                min_importance: None,
            }),
        )
        .await
        .expect_err("target namespace should be denied");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn saved_search_update_allows_clearing_nullable_fields() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let (_status, Json(created)) = create_saved_search(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateSavedSearchRequest {
                name: "Initial".into(),
                description: Some("desc".into()),
                query: "deadline".into(),
                search_type: Some("hybrid".into()),
                limit: Some(10),
                namespace: Some("ops".into()),
                target_namespace: Some("ops".into()),
                kinds: Some(vec!["task".into()]),
                tags: Some(vec!["urgent".into()]),
                min_score: Some(0.3),
                min_importance: Some(0.4),
            }),
        )
        .await
        .expect("saved search should create");

        let Json(updated) = update_saved_search(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(created.id.clone()),
            Json(UpdateSavedSearchRequest {
                name: None,
                description: Some(None),
                query: None,
                search_type: None,
                limit: None,
                target_namespace: Some(None),
                kinds: Some(vec![]),
                tags: Some(vec![]),
                min_score: Some(None),
                min_importance: Some(None),
            }),
        )
        .await
        .expect("saved search should update");

        assert!(updated.description.is_none());
        assert!(updated.target_namespace.is_none());
        assert!(updated.kinds.is_empty());
        assert!(updated.tags.is_empty());
        assert!(updated.min_score.is_none());
        assert!(updated.min_importance.is_none());
    }

    #[tokio::test]
    async fn export_bundle_includes_nodes_and_relationships() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let from = state
            .engine
            .store_node(
                KnowledgeNode::new(NodeKind::Task, "Prepare launch checklist".into())
                    .with_namespace("ops"),
            )
            .await
            .expect("from task should store");
        let to = state
            .engine
            .store_node(
                KnowledgeNode::new(NodeKind::Task, "Run production validation".into())
                    .with_namespace("ops"),
            )
            .await
            .expect("to task should store");
        state
            .engine
            .add_relationship(Relationship::new(from.id, to.id, RelationKind::DependsOn))
            .await
            .expect("relationship should store");

        let Json(bundle) = export_bundle(
            Extension(AuthContext::system_admin()),
            State(state),
            Query(ExportQuery {
                namespace: Some("ops".into()),
                include_relationships: Some(true),
            }),
        )
        .await
        .expect("export should succeed");

        assert_eq!(bundle.format_version, EXPORT_FORMAT_VERSION_V1);
        assert_eq!(bundle.scope_namespace.as_deref(), Some("ops"));
        assert!(bundle.nodes.len() >= 2);
        assert!(!bundle.relationships.is_empty());
        assert!(bundle
            .relationships
            .iter()
            .any(|relationship| relationship.kind == RelationKind::DependsOn));
    }

    #[tokio::test]
    async fn import_bundle_creates_nodes_and_relationships() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut node_a = KnowledgeNode::new(NodeKind::Task, "A".into()).with_namespace("ops");
        let mut node_b = KnowledgeNode::new(NodeKind::Task, "B".into()).with_namespace("ops");
        node_a.id = uuid::Uuid::now_v7();
        node_b.id = uuid::Uuid::now_v7();
        let relationship = Relationship::new(node_a.id, node_b.id, RelationKind::DependsOn);

        let Json(response) = import_bundle(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(ImportRequest {
                namespace_override: None,
                overwrite_existing: Some(false),
                include_relationships: Some(true),
                nodes: vec![node_a.clone(), node_b.clone()],
                relationships: Some(vec![relationship.clone()]),
            }),
        )
        .await
        .expect("import should succeed");

        assert_eq!(response.imported_nodes, 2);
        assert_eq!(response.imported_relationships, 1);

        let loaded_a = state.engine.get_node(node_a.id).await.unwrap();
        let loaded_b = state.engine.get_node(node_b.id).await.unwrap();
        assert!(loaded_a.is_some());
        assert!(loaded_b.is_some());

        let loaded_relationship = state
            .engine
            .graph
            .get_relationship(relationship.id)
            .await
            .unwrap();
        assert!(loaded_relationship.is_some());
    }

    #[tokio::test]
    async fn get_node_relationships_returns_incoming_and_outgoing_edges() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let center = state
            .engine
            .store_node(KnowledgeNode::new(NodeKind::Fact, "Center".into()).with_namespace("ops"))
            .await
            .expect("center should store");
        let outbound_target = state
            .engine
            .store_node(KnowledgeNode::new(NodeKind::Task, "Target".into()).with_namespace("ops"))
            .await
            .expect("target should store");
        let inbound_source = state
            .engine
            .store_node(KnowledgeNode::new(NodeKind::Event, "Source".into()).with_namespace("ops"))
            .await
            .expect("source should store");

        let mut references =
            Relationship::new(center.id, outbound_target.id, RelationKind::References);
        references.metadata.insert(
            AUTO_BACKLINK_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );
        references.metadata.insert(
            AUTO_BACKLINK_SOURCE_METADATA_KEY.to_string(),
            serde_json::Value::String("wikilink".to_string()),
        );
        state
            .engine
            .add_relationship(references)
            .await
            .expect("outgoing relationship should store");
        state
            .engine
            .add_relationship(Relationship::new(
                inbound_source.id,
                center.id,
                RelationKind::DependsOn,
            ))
            .await
            .expect("incoming relationship should store");

        let Json(overview) = get_node_relationships(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(center.id.to_string()),
        )
        .await
        .expect("overview should load");

        assert_eq!(overview.node_id, center.id.to_string());
        assert_eq!(overview.outgoing.len(), 1);
        assert_eq!(overview.incoming.len(), 1);

        let outgoing = &overview.outgoing[0];
        assert_eq!(outgoing.direction, "outgoing");
        assert_eq!(outgoing.related_node_id, outbound_target.id.to_string());
        assert_eq!(outgoing.relation_kind, "references");
        assert!(outgoing.auto_managed);
        assert_eq!(outgoing.auto_source.as_deref(), Some("wikilink"));

        let incoming = &overview.incoming[0];
        assert_eq!(incoming.direction, "incoming");
        assert_eq!(incoming.related_node_id, inbound_source.id.to_string());
        assert_eq!(incoming.relation_kind, "depends_on");
        assert!(!incoming.auto_managed);
        assert!(incoming.auto_source.is_none());
    }

    #[tokio::test]
    async fn list_node_attachments_returns_download_urls() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut node =
            KnowledgeNode::new(NodeKind::Fact, "Attachment node".into()).with_namespace("ops");
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": "att-1",
                    "file_name": "report.pdf",
                    "content_type": "application/pdf",
                    "size_bytes": 128,
                    "stored_path": "/tmp/report.pdf",
                    "uploaded_at": "2026-02-06T00:00:00Z"
                }
            ]),
        );
        node.metadata.insert(
            ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY.into(),
            serde_json::json!({
                "att-1": [
                    "report summary chunk one with mitigations",
                    "report summary chunk two with owners"
                ]
            }),
        );
        let stored = state
            .engine
            .store_node(node)
            .await
            .expect("node should store");

        let Json(items) = list_node_attachments(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(stored.id.to_string()),
        )
        .await
        .expect("list attachments should succeed");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].attachment_id, "att-1");
        assert_eq!(items[0].search_chunk_count, Some(2));
        assert!(items[0]
            .search_preview
            .as_deref()
            .is_some_and(|preview| preview.contains("chunk one")));
        assert!(items[0]
            .download_url
            .contains(&format!("/api/v1/files/{}/att-1", stored.id)));
    }

    #[tokio::test]
    async fn get_attachment_chunks_returns_paginated_chunks() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut node =
            KnowledgeNode::new(NodeKind::Fact, "Attachment node".into()).with_namespace("ops");
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": "att-1",
                    "file_name": "report.pdf",
                    "content_type": "application/pdf",
                    "size_bytes": 128,
                    "stored_path": "/tmp/report.pdf",
                    "uploaded_at": "2026-02-06T00:00:00Z",
                    "extraction_status": "indexed_pdf_text",
                    "extracted_chars": 96
                }
            ]),
        );
        node.metadata.insert(
            ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY.into(),
            serde_json::json!({
                "att-1": [
                    "chunk zero text",
                    "chunk one text",
                    "chunk two text"
                ]
            }),
        );
        let stored = state
            .engine
            .store_node(node)
            .await
            .expect("node should store");

        let Json(response) = get_attachment_chunks(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), "att-1".to_string())),
            Query(AttachmentChunkQuery {
                limit: Some(2),
                offset: Some(1),
            }),
        )
        .await
        .expect("chunk lookup should succeed");

        assert_eq!(response.attachment_id, "att-1");
        assert_eq!(response.total_chunks, 3);
        assert_eq!(response.returned_chunks, 2);
        assert_eq!(response.offset, 1);
        assert_eq!(response.limit, 2);
        assert_eq!(response.chunks[0].index, 1);
        assert_eq!(response.chunks[0].text, "chunk one text");
        assert_eq!(response.chunks[1].index, 2);
        assert_eq!(response.chunks[1].text, "chunk two text");
    }

    #[tokio::test]
    async fn get_attachment_chunks_falls_back_to_text_index_when_chunk_index_missing() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut node =
            KnowledgeNode::new(NodeKind::Fact, "Attachment node".into()).with_namespace("ops");
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": "att-legacy",
                    "file_name": "legacy.txt",
                    "content_type": "text/plain",
                    "size_bytes": 64,
                    "stored_path": "/tmp/legacy.txt",
                    "uploaded_at": "2026-02-06T00:00:00Z",
                    "extraction_status": "indexed_text",
                    "extracted_chars": 80
                }
            ]),
        );
        node.metadata.insert(
            ATTACHMENT_TEXT_INDEX_METADATA_KEY.into(),
            serde_json::json!({
                "att-legacy": "alpha planning memo beta mitigation checklist gamma rollout follow-up delta owner assignment epsilon final recap"
            }),
        );
        let stored = state
            .engine
            .store_node(node)
            .await
            .expect("node should store");

        let Json(response) = get_attachment_chunks(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), "att-legacy".to_string())),
            Query(AttachmentChunkQuery {
                limit: Some(4),
                offset: Some(0),
            }),
        )
        .await
        .expect("legacy chunk fallback should succeed");

        assert!(response.total_chunks >= 1);
        assert!(response.returned_chunks >= 1);
        assert_eq!(response.chunks[0].index, 0);
        assert!(response.chunks[0].text.contains("alpha planning memo"));
    }

    #[tokio::test]
    async fn reindex_attachment_refreshes_extraction_metadata_and_chunks() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let stored = state
            .engine
            .store_node(
                KnowledgeNode::new(NodeKind::Fact, "Attachment node".into()).with_namespace("ops"),
            )
            .await
            .expect("node should store");

        let attachment_id = "att-1".to_string();
        let file_name = "notes.txt".to_string();
        let scoped_dir = PathBuf::from(&state.engine.config.data_dir)
            .join("blobs")
            .join(stored.id.to_string());
        tokio::fs::create_dir_all(&scoped_dir)
            .await
            .expect("attachment dir should exist");
        let stored_path = scoped_dir.join(format!("{attachment_id}-{file_name}"));
        tokio::fs::write(
            &stored_path,
            b"Alpha planning notes.\nBeta follow up actions.\nGamma ownership map.",
        )
        .await
        .expect("attachment payload should write");

        let mut node = state
            .engine
            .get_node(stored.id)
            .await
            .expect("node fetch should work")
            .expect("node should exist");
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": attachment_id,
                    "file_name": file_name,
                    "content_type": "text/plain",
                    "size_bytes": 64,
                    "stored_path": stored_path.to_string_lossy().to_string(),
                    "uploaded_at": "2026-02-06T00:00:00Z",
                    "extraction_status": "unsupported",
                    "extracted_chars": 0
                }
            ]),
        );
        state
            .engine
            .update_node(node)
            .await
            .expect("node should update");

        let Json(response) = reindex_attachment(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), "att-1".to_string())),
        )
        .await
        .expect("reindex should succeed");
        assert_eq!(response.attachment_id, "att-1");
        assert_eq!(response.extraction_status, "indexed_text");
        assert!(response.extracted_chars > 0);
        assert!(response.search_chunk_count.is_some_and(|count| count >= 1));

        let refreshed = state
            .engine
            .get_node(stored.id)
            .await
            .expect("node fetch should succeed")
            .expect("node should exist");
        let parsed = parse_node_attachments(&refreshed);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].extraction_status.as_deref(), Some("indexed_text"));
        assert!(refreshed
            .metadata
            .contains_key(ATTACHMENT_TEXT_INDEX_METADATA_KEY));
        assert!(refreshed
            .metadata
            .contains_key(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY));
    }

    #[tokio::test]
    async fn reindex_attachment_rejects_transcribed_status() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut node =
            KnowledgeNode::new(NodeKind::Fact, "Attachment node".into()).with_namespace("ops");
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": "att-voice",
                    "file_name": "voice.wav",
                    "content_type": "audio/wav",
                    "size_bytes": 1024,
                    "stored_path": "/tmp/voice.wav",
                    "uploaded_at": "2026-02-06T00:00:00Z",
                    "extraction_status": "transcribed",
                    "extracted_chars": 42
                }
            ]),
        );
        let stored = state
            .engine
            .store_node(node)
            .await
            .expect("node should store");

        let err = reindex_attachment(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), "att-voice".to_string())),
        )
        .await
        .expect_err("transcribed attachment reindex should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err
            .1
            .contains("transcribed attachments are indexed from voice transcription"));
    }

    #[tokio::test]
    async fn delete_attachment_removes_metadata_entry() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let mut node =
            KnowledgeNode::new(NodeKind::Fact, "Attachment node".into()).with_namespace("ops");
        node.metadata.insert(
            "attachments".into(),
            serde_json::json!([
                {
                    "id": "att-1",
                    "file_name": "report.pdf",
                    "content_type": "application/pdf",
                    "size_bytes": 128,
                    "stored_path": "/tmp/does-not-exist.pdf",
                    "uploaded_at": "2026-02-06T00:00:00Z"
                }
            ]),
        );
        node.metadata.insert(
            ATTACHMENT_TEXT_INDEX_METADATA_KEY.into(),
            serde_json::json!({
                "att-1": "incident report searchable text"
            }),
        );
        node.metadata.insert(
            ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY.into(),
            serde_json::json!({
                "att-1": ["incident report searchable text"]
            }),
        );
        sync_attachment_search_blob_metadata(&mut node);
        let stored = state
            .engine
            .store_node(node)
            .await
            .expect("node should store");

        let Json(response) = delete_attachment(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), "att-1".to_string())),
        )
        .await
        .expect("delete attachment should succeed");

        assert_eq!(response.attachment_id, "att-1");
        assert_eq!(response.remaining_attachments, 0);

        let refreshed = state
            .engine
            .get_node(stored.id)
            .await
            .expect("node fetch should succeed")
            .expect("node should exist");
        assert!(parse_node_attachments(&refreshed).is_empty());
        assert!(!refreshed
            .metadata
            .contains_key(ATTACHMENT_TEXT_INDEX_METADATA_KEY));
        assert!(!refreshed
            .metadata
            .contains_key(ATTACHMENT_TEXT_CHUNK_INDEX_METADATA_KEY));
        assert!(!refreshed
            .metadata
            .contains_key(ATTACHMENT_SEARCH_BLOB_METADATA_KEY));
    }

    #[test]
    fn clip_url_normalization_strips_fragment_and_trailing_slash() {
        assert_eq!(
            normalize_clip_url("https://Example.com/path/#section"),
            Some("https://example.com/path".to_string())
        );
        assert_eq!(
            normalize_clip_url("example.com/path/?q=1#ignore"),
            Some("https://example.com/path?q=1".to_string())
        );
        assert_eq!(
            normalize_clip_url("localhost:9470/path"),
            Some("https://localhost:9470/path".to_string())
        );
        assert_eq!(normalize_clip_url(""), None);
        assert_eq!(normalize_clip_url("ftp://example.com/file"), None);
        assert_eq!(normalize_clip_url("example.com:port/path"), None);
    }

    #[tokio::test]
    async fn enrich_clip_uses_inline_html_metadata() {
        let html = r#"
        <html>
          <head>
            <title>Rust Patterns for Durable APIs</title>
            <meta name="description" content="Build resilient, observable API services with practical Rust patterns." />
            <meta name="keywords" content="rust,api,observability,architecture" />
            <meta property="og:site_name" content="MindVault Blog" />
          </head>
          <body><p>Use explicit contracts and predictable failure handling.</p></body>
        </html>
        "#;
        let Json(response) = enrich_clip(
            Extension(AuthContext::system_admin()),
            Json(ClipEnrichRequest {
                url: "example.com/blog/rust-patterns".to_string(),
                html: Some(html.to_string()),
            }),
        )
        .await
        .expect("clip enrichment should succeed");

        assert_eq!(
            response.normalized_url,
            "https://example.com/blog/rust-patterns"
        );
        assert_eq!(
            response.title.as_deref(),
            Some("Rust Patterns for Durable APIs")
        );
        assert!(response
            .description
            .as_deref()
            .is_some_and(|value| value.contains("resilient")));
        assert_eq!(response.site_name.as_deref(), Some("MindVault Blog"));
        assert!(response.suggested_tags.iter().any(|tag| tag == "rust"));
        assert!(response.suggested_tags.iter().any(|tag| tag == "api"));
        assert!(response
            .content_preview
            .as_deref()
            .is_some_and(|value| value.contains("explicit contracts")));
        assert_eq!(response.estimated_reading_minutes, Some(1));
        assert!(!response.fetched);
    }

    #[tokio::test]
    async fn enrich_clip_rejects_invalid_url() {
        let err = enrich_clip(
            Extension(AuthContext::system_admin()),
            Json(ClipEnrichRequest {
                url: "mailto:test@example.com".to_string(),
                html: Some("<html></html>".to_string()),
            }),
        )
        .await
        .expect_err("invalid URL should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err.1.contains("valid HTTP(S) URL"));
    }

    #[tokio::test]
    async fn enrich_clip_prefers_article_content_for_preview() {
        let article_words = std::iter::repeat("durable")
            .take(220)
            .collect::<Vec<_>>()
            .join(" ");
        let html = format!(
            "<html><body><nav><p>Navigation links and unrelated snippets.</p></nav><article><h1>System Design Notes</h1><p>{article_words}</p></article></body></html>"
        );

        let Json(response) = enrich_clip(
            Extension(AuthContext::system_admin()),
            Json(ClipEnrichRequest {
                url: "https://example.com/design".to_string(),
                html: Some(html),
            }),
        )
        .await
        .expect("clip enrichment should succeed");

        let preview = response
            .content_preview
            .expect("content preview should be populated");
        assert!(preview.contains("durable"));
        assert!(response
            .estimated_reading_minutes
            .is_some_and(|minutes| minutes >= 1));
    }

    #[tokio::test]
    async fn import_clip_dedupes_and_can_create_linked_note() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (created_status, Json(initial)) = import_clip(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(ClipImportRequest {
                url: "https://Example.com/path/#frag".to_string(),
                title: Some("Example Article".to_string()),
                excerpt: Some("Important details from the article".to_string()),
                tags: Some(vec!["Research".to_string(), "Rust!".to_string()]),
                namespace: Some("ops".to_string()),
                clip_source: Some("extension".to_string()),
                dedupe: Some(true),
                create_note: Some(false),
            }),
        )
        .await
        .expect("initial clip import should succeed");
        assert_eq!(created_status, StatusCode::CREATED);
        assert!(initial.created);
        assert_eq!(
            initial.bookmark.source.as_deref(),
            Some("https://example.com/path")
        );
        assert_eq!(
            initial
                .bookmark
                .metadata
                .get(CLIP_NORMALIZED_URL_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some("https://example.com/path")
        );
        assert!(initial.note.is_none());

        let original_id = initial.bookmark.id;
        let original_id_text = original_id.to_string();

        let (dedupe_status, Json(second)) = import_clip(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(ClipImportRequest {
                url: "example.com/path".to_string(),
                title: None,
                excerpt: Some("Captured again".to_string()),
                tags: Some(vec!["Research".to_string()]),
                namespace: Some("ops".to_string()),
                clip_source: Some("extension".to_string()),
                dedupe: Some(true),
                create_note: Some(true),
            }),
        )
        .await
        .expect("dedupe clip import should succeed");

        assert_eq!(dedupe_status, StatusCode::CREATED);
        assert!(!second.created);
        assert_eq!(second.bookmark.id, original_id);
        let note = second.note.expect("linked note should be created");
        assert_eq!(note.kind, NodeKind::Fact);
        assert_eq!(
            note.metadata
                .get(CLIP_LINKED_BOOKMARK_ID_METADATA_KEY)
                .and_then(serde_json::Value::as_str),
            Some(original_id_text.as_str())
        );

        let Json(relationships) = get_node_relationships(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(original_id.to_string()),
        )
        .await
        .expect("relationship overview should load");
        assert!(relationships
            .incoming
            .iter()
            .any(|edge| edge.related_node_id == note.id.to_string()
                && edge.relation_kind == "references"));
    }

    #[tokio::test]
    async fn node_version_history_tracks_updates_and_restore() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (status, Json(stored)) = store_node(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(StoreNodeRequest {
                kind: "fact".to_string(),
                content: "Original note content".to_string(),
                title: Some("Ops Journal".to_string()),
                source: Some("manual".to_string()),
                namespace: Some("ops".to_string()),
                tags: Some(vec!["journal".to_string()]),
                importance: Some(0.6),
                metadata: Some(std::collections::HashMap::from([(
                    "priority".to_string(),
                    serde_json::Value::String("normal".to_string()),
                )])),
            }),
        )
        .await
        .expect("node should be created");
        assert_eq!(status, StatusCode::CREATED);

        let mut edited = stored.clone();
        edited.content = "Original note content\nAdded follow-up actions".to_string();
        edited.title = Some("Ops Journal v2".to_string());
        edited.metadata.insert(
            "priority".to_string(),
            serde_json::Value::String("high".to_string()),
        );

        let Json(updated) = update_node(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(stored.id.to_string()),
            Json(edited),
        )
        .await
        .expect("node update should succeed");
        assert_eq!(updated.title.as_deref(), Some("Ops Journal v2"));

        let Json(versions) = list_node_versions(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(stored.id.to_string()),
        )
        .await
        .expect("version list should succeed");
        assert_eq!(versions.len(), 1);
        let version_id = versions[0].version_id.clone();

        let Json(detail) = get_node_version(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), version_id.clone())),
        )
        .await
        .expect("version detail should succeed");
        assert_eq!(detail.current.title.as_deref(), Some("Ops Journal v2"));
        assert!(detail.diff.added_line_count >= 1);
        assert!(detail
            .field_changes
            .iter()
            .any(|change| change.field == "title" && change.changed));

        let Json(restored) = restore_node_version(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path((stored.id.to_string(), version_id)),
        )
        .await
        .expect("version restore should succeed");
        assert_eq!(restored.content, "Original note content");
        assert_eq!(restored.title.as_deref(), Some("Ops Journal"));
        assert_eq!(
            restored
                .metadata
                .get("priority")
                .and_then(serde_json::Value::as_str),
            Some("normal")
        );

        let Json(versions_after_restore) = list_node_versions(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(stored.id.to_string()),
        )
        .await
        .expect("version list after restore should succeed");
        assert_eq!(versions_after_restore.len(), 2);
    }

    #[tokio::test]
    async fn node_versions_reject_template_nodes() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let (_status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: "Template content".into(),
                title: Some("Template Node".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["runbook".into()]),
                importance: Some(0.7),
                metadata: None,
                template_key: Some("ops.template".into()),
                template_variables: None,
            }),
        )
        .await
        .expect("template creation should succeed");

        let err = list_node_versions(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
        )
        .await
        .expect_err("template node history should fail");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err
            .1
            .contains("use template version endpoints for template nodes"));
    }

    #[tokio::test]
    async fn apply_template_creates_new_node() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let due_at = Utc::now().to_rfc3339();

        let (_status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: "Template content".into(),
                title: Some("Template title".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["planning".into()]),
                importance: Some(0.6),
                metadata: Some(std::collections::HashMap::from([(
                    TASK_DUE_AT_METADATA_KEY.to_string(),
                    serde_json::Value::String(due_at.clone()),
                )])),
                template_key: None,
                template_variables: None,
            }),
        )
        .await
        .expect("template creation should succeed");

        let (status, Json(response)) = apply_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
            Json(ApplyTemplateRequest {
                target_node_id: None,
                target_kind: None,
                overwrite: None,
            }),
        )
        .await
        .expect("apply should succeed");

        assert_eq!(status, StatusCode::CREATED);
        assert!(response.created);
        assert_eq!(response.node.kind, NodeKind::Task);
        assert_eq!(response.node.title.as_deref(), Some("Template title"));
        assert_eq!(response.node.content, "Template content");
        assert!(response
            .node
            .metadata
            .get(TASK_DUE_AT_METADATA_KEY)
            .is_some());
    }

    #[tokio::test]
    async fn apply_template_merge_fill_existing() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;
        let due_at = Utc::now().to_rfc3339();

        let (_status, Json(template)) = create_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateTemplateRequest {
                kind: "task".into(),
                content: "Template content".into(),
                title: Some("Template title".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["planning".into()]),
                importance: Some(0.6),
                metadata: Some(std::collections::HashMap::from([(
                    TASK_DUE_AT_METADATA_KEY.to_string(),
                    serde_json::Value::String(due_at.clone()),
                )])),
                template_key: None,
                template_variables: None,
            }),
        )
        .await
        .expect("template creation should succeed");

        let (status, Json(existing)) = store_node(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(StoreNodeRequest {
                kind: "task".into(),
                content: "Existing content".into(),
                title: Some("Existing title".into()),
                source: None,
                namespace: Some("ops".into()),
                tags: Some(vec!["existing".into()]),
                importance: Some(0.5),
                metadata: None,
            }),
        )
        .await
        .expect("node creation should succeed");
        assert_eq!(status, StatusCode::CREATED);

        let (status, Json(response)) = apply_template(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(template.id.to_string()),
            Json(ApplyTemplateRequest {
                target_node_id: Some(existing.id.to_string()),
                target_kind: None,
                overwrite: Some(false),
            }),
        )
        .await
        .expect("apply should succeed");

        assert_eq!(status, StatusCode::OK);
        assert!(!response.created);
        assert_eq!(response.node.content, "Existing content");
        assert_eq!(response.node.title.as_deref(), Some("Existing title"));
        assert!(response
            .node
            .metadata
            .get(TASK_DUE_AT_METADATA_KEY)
            .is_some());
        assert!(response
            .filled_fields
            .iter()
            .any(|field| field.contains(TASK_DUE_AT_METADATA_KEY)));
    }

    #[tokio::test]
    async fn saved_view_crud_round_trip() {
        let (state, _temp_dir) = create_state_with_embedding("unknown-provider", "any").await;

        let (status, Json(created)) = create_saved_view(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Json(CreateSavedViewRequest {
                name: "Today Focus".into(),
                view_type: "list".into(),
                filters: Some(serde_json::json!({"status": ["todo"], "tags": ["urgent"]})),
                sort: Some(serde_json::json!({"field": "due", "direction": "asc"})),
                group_by: Some("priority".into()),
                query: Some("project:ops".into()),
                namespace: Some("ops".into()),
            }),
        )
        .await
        .expect("saved view creation should succeed");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.name, "Today Focus");
        assert_eq!(created.view_type, "list");
        assert_eq!(created.group_by.as_deref(), Some("priority"));

        let Json(listed) = list_saved_views(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Query(SavedViewListQuery {
                namespace: Some("ops".into()),
                limit: Some(10),
                offset: Some(0),
            }),
        )
        .await
        .expect("saved view list should succeed");
        assert!(listed.iter().any(|item| item.id == created.id));

        let Json(updated) = update_saved_view(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(created.id.clone()),
            Json(UpdateSavedViewRequest {
                name: Some("Today Focus v2".into()),
                view_type: Some("kanban".into()),
                filters: Some(serde_json::json!({"status": ["doing"]})),
                sort: None,
                group_by: Some("status".into()),
                query: Some("tag:urgent".into()),
            }),
        )
        .await
        .expect("saved view update should succeed");
        assert_eq!(updated.name, "Today Focus v2");
        assert_eq!(updated.view_type, "kanban");
        assert_eq!(updated.group_by.as_deref(), Some("status"));

        let Json(delete_response) = delete_saved_view(
            Extension(AuthContext::system_admin()),
            State(Arc::clone(&state)),
            Path(created.id.clone()),
        )
        .await
        .expect("saved view delete should succeed");
        assert!(delete_response
            .get("deleted")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false));
    }
}
