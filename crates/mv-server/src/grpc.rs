use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use serde_json::Value;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use mv_core::*;

use crate::auth::{auth_context_from_authorization_header, AuthContext};
use crate::limits::{enforce_namespace_quota, enforce_rate_limit, NamespaceQuotaError};
use crate::state::AppState;
use crate::validation::{
    validate_depth, validate_list_limit, validate_node_payload, validate_query_text,
    validate_recall_limit,
};

pub mod proto {
    tonic::include_proto!("mindvault.v1");
}

use proto::mind_vault_service_server::MindVaultService;
use proto::*;

pub struct MindVaultGrpc {
    state: Arc<AppState>,
}

impl MindVaultGrpc {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[allow(clippy::result_large_err)]
pub fn auth_interceptor(mut request: Request<()>) -> Result<Request<()>, Status> {
    let auth_header = request
        .metadata()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let auth = auth_context_from_authorization_header(auth_header)
        .map_err(|_| Status::unauthenticated("invalid auth token"))?;
    if let Err(rate) = enforce_rate_limit(&auth) {
        return Err(Status::resource_exhausted(format!(
            "rate limit exceeded ({}/{}s); retry in {}s",
            rate.max_requests, rate.window_secs, rate.retry_after_secs
        )));
    }
    request.extensions_mut().insert(auth);

    Ok(request)
}
fn node_to_proto(node: &KnowledgeNode) -> KnowledgeNodeProto {
    KnowledgeNodeProto {
        id: node.id.to_string(),
        kind: node.kind.as_str().to_string(),
        title: node.title.clone(),
        content: node.content.clone(),
        source: node.source.clone(),
        namespace: node.namespace.clone(),
        tags: node.tags.clone(),
        importance: node.importance,
        created_at: node.temporal.created_at.to_rfc3339(),
        updated_at: node.temporal.updated_at.to_rfc3339(),
        last_accessed_at: node.temporal.last_accessed_at.to_rfc3339(),
        access_count: node.temporal.access_count,
        version: node.temporal.version,
        expires_at: node.temporal.expires_at.map(|dt| dt.to_rfc3339()),
        metadata_json: serde_json::to_string(&node.metadata).unwrap_or_default(),
    }
}

#[allow(clippy::result_large_err)]
fn parse_metadata_json(
    raw: Option<&str>,
) -> Result<std::collections::HashMap<String, Value>, Status> {
    let Some(raw) = raw else {
        return Ok(Default::default());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Default::default());
    }

    serde_json::from_str(trimmed).map_err(|err| {
        Status::invalid_argument(format!("metadata_json must be valid JSON object: {err}"))
    })
}

#[allow(clippy::result_large_err)]
fn parse_optional_datetime(
    raw: Option<&str>,
    field_name: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, Status> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map(Some)
            .map_err(|err| {
                Status::invalid_argument(format!(
                    "{field_name} must be RFC3339 datetime when provided: {err}"
                ))
            }),
        None => Ok(None),
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

fn parse_kind_list(kinds: &[String]) -> Result<Option<Vec<NodeKind>>, String> {
    if kinds.is_empty() {
        return Ok(None);
    }

    let mut parsed = Vec::with_capacity(kinds.len());
    for kind in kinds {
        parsed.push(kind.parse().map_err(|e: String| e)?);
    }
    Ok(Some(parsed))
}

fn map_namespace_quota_status(err: NamespaceQuotaError) -> Status {
    match err {
        NamespaceQuotaError::Exceeded {
            namespace,
            quota,
            count,
        } => Status::resource_exhausted(format!(
            "namespace '{namespace}' quota exceeded ({count}/{quota} nodes)"
        )),
        NamespaceQuotaError::Backend(message) => Status::internal(message),
    }
}

#[allow(clippy::result_large_err)]
fn auth_context_from_request<T>(request: &Request<T>) -> Result<AuthContext, Status> {
    if let Some(auth) = request.extensions().get::<AuthContext>() {
        return Ok(auth.clone());
    }

    let auth_header = request
        .metadata()
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    auth_context_from_authorization_header(auth_header)
        .map_err(|_| Status::unauthenticated("invalid auth token"))
}

#[allow(clippy::result_large_err)]
fn ensure_read(auth: &AuthContext) -> Result<(), Status> {
    if auth.can_read() {
        Ok(())
    } else {
        Err(Status::permission_denied("read permission required"))
    }
}

#[allow(clippy::result_large_err)]
fn ensure_write(auth: &AuthContext) -> Result<(), Status> {
    if auth.can_write() {
        Ok(())
    } else {
        Err(Status::permission_denied("write permission required"))
    }
}

#[allow(clippy::result_large_err)]
fn ensure_namespace(auth: &AuthContext, namespace: &str) -> Result<(), Status> {
    if auth.allows_namespace(namespace) {
        Ok(())
    } else {
        Err(Status::permission_denied(format!(
            "namespace '{namespace}' is not permitted"
        )))
    }
}

#[allow(clippy::result_large_err)]
fn scoped_namespace_grpc(
    auth: &AuthContext,
    requested_namespace: Option<String>,
) -> Result<Option<String>, Status> {
    if auth.is_admin() {
        return Ok(requested_namespace);
    }

    match (&auth.namespace, requested_namespace) {
        (None, requested) => Ok(requested),
        (Some(allowed), Some(requested)) => {
            if requested == *allowed {
                Ok(Some(requested))
            } else {
                Err(Status::permission_denied(format!(
                    "namespace '{requested}' is not permitted"
                )))
            }
        }
        (Some(allowed), None) => Ok(Some(allowed.clone())),
    }
}

#[tonic::async_trait]
impl MindVaultService for MindVaultGrpc {
    async fn store_node(
        &self,
        request: Request<StoreNodeRequest>,
    ) -> Result<Response<StoreNodeResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_write(&auth)?;

        let req = request.into_inner();
        let kind: NodeKind = req
            .kind
            .parse()
            .map_err(|e: String| Status::invalid_argument(e))?;
        let metadata = parse_metadata_json(req.metadata_json.as_deref())?;
        let requested_namespace = if req.namespace.is_empty() {
            None
        } else {
            Some(req.namespace.as_str())
        };
        validate_node_payload(
            kind,
            req.title.as_deref(),
            &req.content,
            req.source.as_deref(),
            requested_namespace,
            &req.tags,
            req.importance,
            Some(&metadata),
        )
        .map_err(Status::invalid_argument)?;

        let mut node = KnowledgeNode::new(kind, req.content);
        if let Some(title) = req.title {
            node = node.with_title(title);
        }
        if let Some(source) = req.source {
            node = node.with_source(source);
        }
        let requested_namespace = if req.namespace.is_empty() {
            None
        } else {
            Some(req.namespace)
        };
        let namespace = requested_namespace
            .or_else(|| auth.namespace.clone())
            .unwrap_or_else(|| "default".to_string());
        ensure_namespace(&auth, &namespace)?;
        enforce_namespace_quota(&self.state.engine, &namespace)
            .await
            .map_err(map_namespace_quota_status)?;
        node = node.with_namespace(namespace);
        if !req.tags.is_empty() {
            node = node.with_tags(req.tags);
        }
        if let Some(importance) = req.importance {
            node = node.with_importance(importance);
        }
        if !metadata.is_empty() {
            node.metadata = metadata;
        }

        let stored = self
            .state
            .engine
            .store_node(node)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        self.state
            .notify_change(&stored.id.to_string(), "create", Some(&stored.namespace));

        Ok(Response::new(StoreNodeResponse {
            node: Some(node_to_proto(&stored)),
        }))
    }

    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_read(&auth)?;

        let req = request.into_inner();
        let uuid = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("invalid UUID: {e}")))?;

        let node = self
            .state
            .engine
            .get_node(uuid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let node = match node {
            Some(node) => {
                ensure_namespace(&auth, &node.namespace)?;
                Some(node_to_proto(&node))
            }
            None => None,
        };

        Ok(Response::new(GetNodeResponse { node }))
    }

    async fn update_node(
        &self,
        request: Request<UpdateNodeRequest>,
    ) -> Result<Response<UpdateNodeResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_write(&auth)?;

        let req = request.into_inner();
        let proto_node = req
            .node
            .ok_or_else(|| Status::invalid_argument("node required"))?;

        let uuid = Uuid::parse_str(&proto_node.id)
            .map_err(|e| Status::invalid_argument(format!("invalid UUID: {e}")))?;

        let existing = self
            .state
            .engine
            .get_node(uuid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("node not found"))?;
        let existing_namespace = existing.namespace.clone();
        ensure_namespace(&auth, &existing_namespace)?;

        let mut node = existing;

        if !proto_node.kind.trim().is_empty() {
            node.kind = proto_node
                .kind
                .parse()
                .map_err(|e: String| Status::invalid_argument(e))?;
        }
        node.content = proto_node.content;
        node.title = proto_node.title;
        node.source = proto_node.source;
        node.tags = proto_node.tags;
        node.importance = proto_node.importance.clamp(0.0, 1.0);
        node.temporal.expires_at =
            parse_optional_datetime(proto_node.expires_at.as_deref(), "expires_at")?;
        node.metadata = parse_metadata_json(Some(&proto_node.metadata_json))?;
        if !auth.is_admin() {
            node.namespace = auth
                .namespace
                .clone()
                .unwrap_or_else(|| node.namespace.clone());
        } else if !proto_node.namespace.is_empty() {
            node.namespace = proto_node.namespace;
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
        .map_err(Status::invalid_argument)?;
        if node.namespace != existing_namespace {
            enforce_namespace_quota(&self.state.engine, &node.namespace)
                .await
                .map_err(map_namespace_quota_status)?;
        }
        ensure_namespace(&auth, &node.namespace)?;
        node.temporal.updated_at = chrono::Utc::now();
        node.temporal.version += 1;

        let updated = self
            .state
            .engine
            .update_node(node)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        self.state
            .notify_change(&updated.id.to_string(), "update", Some(&updated.namespace));

        Ok(Response::new(UpdateNodeResponse {
            node: Some(node_to_proto(&updated)),
        }))
    }

    async fn delete_node(
        &self,
        request: Request<DeleteNodeRequest>,
    ) -> Result<Response<DeleteNodeResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_write(&auth)?;

        let req = request.into_inner();
        let uuid = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("invalid UUID: {e}")))?;

        if let Some(node) = self
            .state
            .engine
            .get_node(uuid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            ensure_namespace(&auth, &node.namespace)?;
        }

        let deleted = self
            .state
            .engine
            .delete_node(uuid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if deleted {
            self.state.notify_change(&req.id, "delete", None);
        }

        Ok(Response::new(DeleteNodeResponse { deleted }))
    }

    async fn recall(
        &self,
        request: Request<RecallRequest>,
    ) -> Result<Response<RecallResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_read(&auth)?;

        let req = request.into_inner();
        validate_query_text("text", &req.text).map_err(Status::invalid_argument)?;

        let strategy: SearchStrategy = if req.strategy.is_empty() {
            SearchStrategy::Hybrid
        } else {
            req.strategy
                .parse()
                .map_err(|e: String| Status::invalid_argument(e))?
        };

        let kinds = parse_kind_list(&req.kinds).map_err(Status::invalid_argument)?;

        let requested_namespace = req.namespace.filter(|ns| !ns.is_empty());
        let namespace = scoped_namespace_grpc(&auth, requested_namespace)?;
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            10
        };
        validate_recall_limit(limit).map_err(Status::invalid_argument)?;

        let query = MemoryQuery {
            text: req.text,
            strategy,
            limit,
            min_score: req.min_score,
            filters: QueryFilters {
                namespace,
                kinds,
                tags: if req.tags.is_empty() {
                    None
                } else {
                    Some(req.tags)
                },
                ..Default::default()
            },
        };

        let results = self
            .state
            .engine
            .recall(&query)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let protos: Vec<SearchResultProto> = results
            .into_iter()
            .map(|r| SearchResultProto {
                node: Some(node_to_proto(&r.node)),
                score: r.score,
                match_source: match_source_to_str(r.match_source).to_string(),
            })
            .collect();

        Ok(Response::new(RecallResponse { results: protos }))
    }

    async fn list_nodes(
        &self,
        request: Request<ListNodesRequest>,
    ) -> Result<Response<ListNodesResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_read(&auth)?;

        let req = request.into_inner();

        let kinds = parse_kind_list(&req.kinds).map_err(Status::invalid_argument)?;

        let requested_namespace = req.namespace.filter(|ns| !ns.is_empty());
        let namespace = scoped_namespace_grpc(&auth, requested_namespace)?;
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            50
        };
        validate_list_limit(limit).map_err(Status::invalid_argument)?;

        let filters = QueryFilters {
            namespace,
            kinds,
            ..Default::default()
        };

        let offset = req.offset as usize;

        let nodes = self
            .state
            .engine
            .list_nodes(&filters, limit, offset)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let total = self
            .state
            .engine
            .store
            .nodes
            .count(&filters)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ListNodesResponse {
            nodes: nodes.iter().map(node_to_proto).collect(),
            total: total as u64,
        }))
    }

    async fn add_relationship(
        &self,
        request: Request<AddRelationshipRequest>,
    ) -> Result<Response<AddRelationshipResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_write(&auth)?;

        let req = request.into_inner();

        let from = Uuid::parse_str(&req.from_node)
            .map_err(|e| Status::invalid_argument(format!("invalid from_node: {e}")))?;
        let to = Uuid::parse_str(&req.to_node)
            .map_err(|e| Status::invalid_argument(format!("invalid to_node: {e}")))?;
        let kind: RelationKind = req
            .kind
            .parse()
            .map_err(|e: String| Status::invalid_argument(e))?;

        let rel = Relationship::new(from, to, kind).with_weight(req.weight);
        let rel_id = rel.id;

        let from_node = self
            .state
            .engine
            .get_node(from)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("from_node not found"))?;
        let to_node = self
            .state
            .engine
            .get_node(to)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("to_node not found"))?;

        ensure_namespace(&auth, &from_node.namespace)?;
        ensure_namespace(&auth, &to_node.namespace)?;

        self.state
            .engine
            .add_relationship(rel)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(AddRelationshipResponse {
            id: rel_id.to_string(),
        }))
    }

    async fn get_neighbors(
        &self,
        request: Request<GetNeighborsRequest>,
    ) -> Result<Response<GetNeighborsResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_read(&auth)?;

        let req = request.into_inner();
        let uuid = Uuid::parse_str(&req.node_id)
            .map_err(|e| Status::invalid_argument(format!("invalid UUID: {e}")))?;
        let depth = if req.depth > 0 { req.depth as usize } else { 2 };
        validate_depth(depth).map_err(Status::invalid_argument)?;

        let source_node = self
            .state
            .engine
            .get_node(uuid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("node not found"))?;
        ensure_namespace(&auth, &source_node.namespace)?;

        let neighbors = self
            .state
            .engine
            .get_neighbors(uuid, depth)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut visible_neighbors = Vec::new();
        for neighbor_id in neighbors {
            if let Some(node) = self
                .state
                .engine
                .get_node(neighbor_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            {
                if auth.allows_namespace(&node.namespace) {
                    visible_neighbors.push(neighbor_id.to_string());
                }
            }
        }

        Ok(Response::new(GetNeighborsResponse {
            neighbor_ids: visible_neighbors,
        }))
    }

    async fn health(
        &self,
        request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_read(&auth)?;

        let count = self
            .state
            .engine
            .node_count()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(HealthResponse {
            status: "ok".into(),
            node_count: count as u64,
            version: env!("CARGO_PKG_VERSION").into(),
        }))
    }

    type WatchChangesStream = Pin<Box<dyn Stream<Item = Result<ChangeEvent, Status>> + Send>>;

    async fn watch_changes(
        &self,
        request: Request<WatchChangesRequest>,
    ) -> Result<Response<Self::WatchChangesStream>, Status> {
        let auth = auth_context_from_request(&request)?;
        ensure_read(&auth)?;

        let requested_namespace = request.into_inner().namespace;
        let namespace_filter = scoped_namespace_grpc(&auth, requested_namespace)?;
        let mut rx = self.state.change_tx.subscribe();

        let stream = async_stream::try_stream! {
            loop {
                match rx.recv().await {
                    Ok(notification) => {
                        if let Some(ref namespace) = namespace_filter {
                            if notification.namespace.as_deref() != Some(namespace.as_str()) {
                                continue;
                            }
                        }
                        yield ChangeEvent {
                            node_id: notification.node_id,
                            operation: notification.operation,
                            timestamp: notification.timestamp,
                            node: None,
                        };
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("change stream lagged by {n} events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}
