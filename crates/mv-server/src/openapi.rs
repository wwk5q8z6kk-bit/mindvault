//! OpenAPI specification and Swagger UI for MindVault API.
#![allow(dead_code)]

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// MindVault API OpenAPI specification.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "MindVault API",
        version = env!("CARGO_PKG_VERSION"),
        description = "MindVault is a local-first knowledge management system with hybrid semantic retrieval.",
        license(name = "MIT"),
        contact(name = "MindVault Contributors")
    ),
    servers(
        (url = "http://localhost:9470", description = "Local development server")
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "nodes", description = "Knowledge node CRUD operations"),
        (name = "recall", description = "Semantic recall and search"),
        (name = "graph", description = "Relationship graph operations"),
        (name = "daily-notes", description = "Daily notes management"),
        (name = "calendar", description = "Calendar and iCal integration"),
        (name = "tasks", description = "Task management"),
        (name = "templates", description = "Template management"),
        (name = "files", description = "File attachments"),
        (name = "voice", description = "Voice notes and transcription"),
        (name = "assist", description = "AI writing assistance"),
        (name = "export-import", description = "Data export and import"),
        (name = "audit", description = "Audit logging"),
        (name = "metrics", description = "Prometheus metrics"),
        (name = "saved-searches", description = "Saved search presets"),
        (name = "saved-views", description = "Saved view presets"),
        (name = "permissions", description = "Permission templates and access keys")
    ),
    paths(
        // Health
        health,
        embedding_diagnostics,
        // Nodes
        store_node,
        list_nodes,
        get_node,
        update_node,
        delete_node,
        // Recall & Search
        recall,
        search,
        // Graph
        add_relationship,
        get_neighbors,
        // Daily notes
        list_daily_notes,
        ensure_daily_note,
        // Calendar
        list_calendar_items,
        export_calendar_ical,
        import_calendar_ical,
        // Tasks
        list_due_tasks,
        prioritize_tasks,
        complete_task,
        reopen_task,
        snooze_task_reminder,
        // Templates
        list_templates,
        create_template,
        update_template,
        delete_template,
        instantiate_template,
        apply_template,
        // Files
        upload_file,
        list_attachments_index,
        list_node_attachments,
        get_attachment_chunks,
        reindex_attachment,
        download_attachment,
        delete_attachment,
        // Voice
        upload_voice_note,
        // Assist
        assist_completion,
        assist_autocomplete,
        assist_links,
        assist_transform,
        // Export/Import
        export_bundle,
        import_bundle,
        // Audit
        list_audit_logs,
        // Saved searches
        list_saved_searches,
        create_saved_search,
        update_saved_search,
        delete_saved_search,
        run_saved_search,
        // Saved views
        list_saved_views,
        create_saved_view,
        update_saved_view,
        delete_saved_view,
        // Permissions
        list_permission_templates,
        create_permission_template,
        update_permission_template,
        delete_permission_template,
        list_access_keys,
        create_access_key,
        revoke_access_key,
    ),
    components(
        schemas(
            HealthResponse,
            KnowledgeNode,
            NodeKind,
            RecallRequest,
            RecallResult,
            StoreNodeRequest,
            UpdateNodeRequest,
            RelationshipRequest,
            AuditEntry,
            PermissionTemplateResponse,
            AccessKeyResponse,
            AccessKeyCreateResponse,
        )
    )
)]
pub struct ApiDoc;

// Placeholder path operations - these need to be defined with #[utoipa::path] in rest.rs
// For now, we'll create a minimal spec

#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse)
    )
)]
async fn health() {}

#[utoipa::path(
    get,
    path = "/api/v1/diagnostics/embedding",
    tag = "health",
    responses(
        (status = 200, description = "Embedding provider diagnostics")
    )
)]
async fn embedding_diagnostics() {}

#[utoipa::path(
    post,
    path = "/api/v1/nodes",
    tag = "nodes",
    request_body = StoreNodeRequest,
    responses(
        (status = 201, description = "Node created", body = KnowledgeNode)
    )
)]
async fn store_node() {}

#[utoipa::path(
    get,
    path = "/api/v1/nodes",
    tag = "nodes",
    params(
        ("limit" = Option<usize>, Query, description = "Max results"),
        ("offset" = Option<usize>, Query, description = "Pagination offset"),
        ("namespace" = Option<String>, Query, description = "Filter by namespace"),
        ("kind" = Option<String>, Query, description = "Filter by node kind")
    ),
    responses(
        (status = 200, description = "List of nodes", body = Vec<KnowledgeNode>)
    )
)]
async fn list_nodes() {}

#[utoipa::path(
    get,
    path = "/api/v1/nodes/{id}",
    tag = "nodes",
    params(
        ("id" = String, Path, description = "Node UUID")
    ),
    responses(
        (status = 200, description = "Node found", body = KnowledgeNode),
        (status = 404, description = "Node not found")
    )
)]
async fn get_node() {}

#[utoipa::path(
    put,
    path = "/api/v1/nodes/{id}",
    tag = "nodes",
    params(
        ("id" = String, Path, description = "Node UUID")
    ),
    request_body = UpdateNodeRequest,
    responses(
        (status = 200, description = "Node updated", body = KnowledgeNode),
        (status = 404, description = "Node not found")
    )
)]
async fn update_node() {}

#[utoipa::path(
    delete,
    path = "/api/v1/nodes/{id}",
    tag = "nodes",
    params(
        ("id" = String, Path, description = "Node UUID")
    ),
    responses(
        (status = 200, description = "Node deleted"),
        (status = 404, description = "Node not found")
    )
)]
async fn delete_node() {}

#[utoipa::path(
    post,
    path = "/api/v1/recall",
    tag = "recall",
    request_body = RecallRequest,
    responses(
        (status = 200, description = "Recall results", body = Vec<RecallResult>)
    )
)]
async fn recall() {}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "recall",
    params(
        ("q" = String, Query, description = "Search query"),
        ("limit" = Option<usize>, Query, description = "Max results"),
        ("type" = Option<String>, Query, description = "Search type: fulltext, vector, hybrid")
    ),
    responses(
        (status = 200, description = "Search results")
    )
)]
async fn search() {}

#[utoipa::path(
    post,
    path = "/api/v1/graph/relationships",
    tag = "graph",
    request_body = RelationshipRequest,
    responses(
        (status = 201, description = "Relationship created")
    )
)]
async fn add_relationship() {}

#[utoipa::path(
    get,
    path = "/api/v1/graph/neighbors/{id}",
    tag = "graph",
    params(
        ("id" = String, Path, description = "Node UUID"),
        ("depth" = Option<usize>, Query, description = "Traversal depth")
    ),
    responses(
        (status = 200, description = "Neighbor node IDs")
    )
)]
async fn get_neighbors() {}

#[utoipa::path(get, path = "/api/v1/daily-notes", tag = "daily-notes", responses((status = 200)))]
async fn list_daily_notes() {}

#[utoipa::path(post, path = "/api/v1/daily-notes/ensure", tag = "daily-notes", responses((status = 200)))]
async fn ensure_daily_note() {}

#[utoipa::path(get, path = "/api/v1/calendar/items", tag = "calendar", responses((status = 200)))]
async fn list_calendar_items() {}

#[utoipa::path(get, path = "/api/v1/calendar/ical", tag = "calendar", responses((status = 200)))]
async fn export_calendar_ical() {}

#[utoipa::path(post, path = "/api/v1/calendar/ical/import", tag = "calendar", responses((status = 200)))]
async fn import_calendar_ical() {}

#[utoipa::path(get, path = "/api/v1/tasks/due", tag = "tasks", responses((status = 200)))]
async fn list_due_tasks() {}

#[utoipa::path(post, path = "/api/v1/tasks/prioritize", tag = "tasks", responses((status = 200)))]
async fn prioritize_tasks() {}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/complete", tag = "tasks", responses((status = 200)))]
async fn complete_task() {}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/reopen", tag = "tasks", responses((status = 200)))]
async fn reopen_task() {}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/snooze", tag = "tasks", responses((status = 200)))]
async fn snooze_task_reminder() {}

#[utoipa::path(get, path = "/api/v1/templates", tag = "templates", responses((status = 200)))]
async fn list_templates() {}

#[utoipa::path(post, path = "/api/v1/templates", tag = "templates", responses((status = 201)))]
async fn create_template() {}

#[utoipa::path(patch, path = "/api/v1/templates/{id}", tag = "templates", responses((status = 200)))]
async fn update_template() {}

#[utoipa::path(delete, path = "/api/v1/templates/{id}", tag = "templates", responses((status = 200)))]
async fn delete_template() {}

#[utoipa::path(post, path = "/api/v1/templates/{id}/instantiate", tag = "templates", responses((status = 201)))]
async fn instantiate_template() {}

#[utoipa::path(post, path = "/api/v1/templates/{id}/apply", tag = "templates", responses((status = 200)))]
async fn apply_template() {}

#[utoipa::path(post, path = "/api/v1/files/upload", tag = "files", responses((status = 201)))]
async fn upload_file() {}

#[utoipa::path(
    get,
    path = "/api/v1/files",
    tag = "files",
    params(
        ("q" = Option<String>, Query, description = "Search text filter"),
        ("status" = Option<String>, Query, description = "Filter by extraction status"),
        ("failed_only" = Option<bool>, Query, description = "Only include failed extractions"),
        ("limit" = Option<usize>, Query, description = "Page size"),
        ("offset" = Option<usize>, Query, description = "Pagination offset"),
        ("sort" = Option<String>, Query, description = "Sort key (uploaded_desc/uploaded_asc/name_asc/name_desc/status_asc/status_desc)"),
        ("namespace" = Option<String>, Query, description = "Namespace filter"),
        ("kind" = Option<String>, Query, description = "Node kind filter")
    ),
    responses((status = 200))
)]
async fn list_attachments_index() {}

#[utoipa::path(get, path = "/api/v1/files/{node_id}", tag = "files", responses((status = 200)))]
async fn list_node_attachments() {}

#[utoipa::path(get, path = "/api/v1/files/{node_id}/{attachment_id}/chunks", tag = "files", responses((status = 200)))]
async fn get_attachment_chunks() {}

#[utoipa::path(post, path = "/api/v1/files/{node_id}/{attachment_id}/reindex", tag = "files", responses((status = 200)))]
async fn reindex_attachment() {}

#[utoipa::path(
    get,
    path = "/api/v1/files/{node_id}/{attachment_id}",
    tag = "files",
    params(
        ("node_id" = String, Path, description = "Node id"),
        ("attachment_id" = String, Path, description = "Attachment id"),
        ("inline" = Option<bool>, Query, description = "When true, return inline Content-Disposition")
    ),
    responses((status = 200))
)]
async fn download_attachment() {}

#[utoipa::path(delete, path = "/api/v1/files/{node_id}/{attachment_id}", tag = "files", responses((status = 200)))]
async fn delete_attachment() {}

#[utoipa::path(post, path = "/api/v1/voice/upload", tag = "voice", responses((status = 201)))]
async fn upload_voice_note() {}

#[utoipa::path(post, path = "/api/v1/assist/completion", tag = "assist", responses((status = 200)))]
async fn assist_completion() {}

#[utoipa::path(post, path = "/api/v1/assist/autocomplete", tag = "assist", responses((status = 200)))]
async fn assist_autocomplete() {}

#[utoipa::path(post, path = "/api/v1/assist/links", tag = "assist", responses((status = 200)))]
async fn assist_links() {}

#[utoipa::path(post, path = "/api/v1/assist/transform", tag = "assist", responses((status = 200)))]
async fn assist_transform() {}

#[utoipa::path(get, path = "/api/v1/export", tag = "export-import", responses((status = 200)))]
async fn export_bundle() {}

#[utoipa::path(post, path = "/api/v1/import", tag = "export-import", responses((status = 200)))]
async fn import_bundle() {}

#[utoipa::path(get, path = "/api/v1/audit", tag = "audit", responses((status = 200, body = Vec<AuditEntry>)))]
async fn list_audit_logs() {}

#[utoipa::path(get, path = "/api/v1/search/saved", tag = "saved-searches", responses((status = 200)))]
async fn list_saved_searches() {}

#[utoipa::path(post, path = "/api/v1/search/saved", tag = "saved-searches", responses((status = 201)))]
async fn create_saved_search() {}

#[utoipa::path(put, path = "/api/v1/search/saved/{id}", tag = "saved-searches", responses((status = 200)))]
async fn update_saved_search() {}

#[utoipa::path(delete, path = "/api/v1/search/saved/{id}", tag = "saved-searches", responses((status = 200)))]
async fn delete_saved_search() {}

#[utoipa::path(post, path = "/api/v1/search/saved/{id}/run", tag = "saved-searches", responses((status = 200)))]
async fn run_saved_search() {}

#[utoipa::path(get, path = "/api/v1/saved_views", tag = "saved-views", responses((status = 200)))]
async fn list_saved_views() {}

#[utoipa::path(post, path = "/api/v1/saved_views", tag = "saved-views", responses((status = 201)))]
async fn create_saved_view() {}

#[utoipa::path(patch, path = "/api/v1/saved_views/{id}", tag = "saved-views", responses((status = 200)))]
async fn update_saved_view() {}

#[utoipa::path(delete, path = "/api/v1/saved_views/{id}", tag = "saved-views", responses((status = 200)))]
async fn delete_saved_view() {}

#[utoipa::path(get, path = "/api/v1/permission-templates", tag = "permissions", responses((status = 200, body = Vec<PermissionTemplateResponse>)))]
async fn list_permission_templates() {}

#[utoipa::path(post, path = "/api/v1/permission-templates", tag = "permissions", responses((status = 201, body = PermissionTemplateResponse)))]
async fn create_permission_template() {}

#[utoipa::path(put, path = "/api/v1/permission-templates/{id}", tag = "permissions", responses((status = 200, body = PermissionTemplateResponse)))]
async fn update_permission_template() {}

#[utoipa::path(delete, path = "/api/v1/permission-templates/{id}", tag = "permissions", responses((status = 204)))]
async fn delete_permission_template() {}

#[utoipa::path(get, path = "/api/v1/access-keys", tag = "permissions", responses((status = 200, body = Vec<AccessKeyResponse>)))]
async fn list_access_keys() {}

#[utoipa::path(post, path = "/api/v1/access-keys", tag = "permissions", responses((status = 201, body = AccessKeyCreateResponse)))]
async fn create_access_key() {}

#[utoipa::path(delete, path = "/api/v1/access-keys/{id}", tag = "permissions", responses((status = 204)))]
async fn revoke_access_key() {}

// Schema types for OpenAPI
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub node_count: usize,
    pub version: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct KnowledgeNode {
    pub id: String,
    pub kind: NodeKind,
    pub title: Option<String>,
    pub content: String,
    pub namespace: String,
    pub tags: Vec<String>,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum NodeKind {
    Fact,
    Task,
    Event,
    Decision,
    Preference,
    Entity,
    CodeSnippet,
    Project,
    Conversation,
    Procedure,
    Observation,
    Bookmark,
    Template,
    SavedView,
}

#[derive(Deserialize, ToSchema)]
pub struct StoreNodeRequest {
    pub kind: String,
    pub content: String,
    pub title: Option<String>,
    pub namespace: Option<String>,
    pub tags: Option<Vec<String>>,
    pub importance: Option<f64>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateNodeRequest {
    pub kind: Option<String>,
    pub content: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub importance: Option<f64>,
}

#[derive(Deserialize, ToSchema)]
pub struct RecallRequest {
    pub text: String,
    pub strategy: Option<String>,
    pub limit: Option<usize>,
    pub min_score: Option<f64>,
    pub namespace: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RecallResult {
    pub node: KnowledgeNode,
    pub score: f64,
}

#[derive(Deserialize, ToSchema)]
pub struct RelationshipRequest {
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuditEntry {
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub subject: Option<String>,
    pub role: String,
    pub namespace: Option<String>,
    pub method: String,
    pub path: String,
    pub action: String,
    pub resource_id: Option<String>,
    pub status_code: u16,
    pub success: bool,
    pub latency_ms: u64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PermissionTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tier: String,
    pub scope_namespace: Option<String>,
    pub scope_tags: Vec<String>,
    pub allow_kinds: Vec<String>,
    pub allow_actions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessKeyResponse {
    pub id: String,
    pub name: Option<String>,
    pub template_id: String,
    pub template_name: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessKeyCreateResponse {
    pub token: String,
    pub access_key: AccessKeyResponse,
}

/// Create the Swagger UI router.
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/api/docs/{_:.*}").url("/api/openapi.json", ApiDoc::openapi())
}

/// Get the OpenAPI JSON spec.
pub fn openapi_json() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap_or_default()
}
