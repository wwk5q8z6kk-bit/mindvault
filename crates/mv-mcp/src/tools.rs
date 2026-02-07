use std::collections::HashMap;
use std::sync::Arc;

use mv_core::*;
use mv_engine::engine::MindVaultEngine;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::protocol::{ToolDefinition, ToolResult};

/// Return all tool definitions exposed by this MCP server.
pub fn list_tools() -> Vec<ToolDefinition> {
	vec![
		ToolDefinition {
			name: "mindvault_store".into(),
			description: "Store a new knowledge node in MindVault.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"content": {
						"type": "string",
						"description": "The main content/body of the knowledge node"
					},
					"kind": {
						"type": "string",
						"description": "Node kind: fact, task, event, decision, preference, entity, code_snippet, project, conversation, procedure, observation, bookmark, template, saved_view",
						"default": "fact"
					},
					"title": {
						"type": "string",
						"description": "Optional title for the node"
					},
					"tags": {
						"type": "array",
						"items": { "type": "string" },
						"description": "Tags for categorization"
					},
					"namespace": {
						"type": "string",
						"description": "Namespace (default: 'default')",
						"default": "default"
					},
					"importance": {
						"type": "number",
						"description": "Importance score from 0.0 to 1.0",
						"default": 0.5
					},
					"metadata": {
						"type": "object",
						"description": "Additional key-value metadata"
					}
				},
				"required": ["content"]
			}),
		},
		ToolDefinition {
			name: "mindvault_recall".into(),
			description: "Semantic recall — find knowledge nodes matching a natural-language query using hybrid vector + full-text search.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"query": {
						"type": "string",
						"description": "Natural-language query"
					},
					"limit": {
						"type": "integer",
						"description": "Max results to return",
						"default": 10
					},
					"strategy": {
						"type": "string",
						"description": "Search strategy: hybrid, vector, fulltext, graph",
						"default": "hybrid"
					},
					"min_score": {
						"type": "number",
						"description": "Minimum relevance score threshold",
						"default": 0.0
					},
					"namespace": {
						"type": "string",
						"description": "Filter by namespace"
					},
					"kinds": {
						"type": "array",
						"items": { "type": "string" },
						"description": "Filter by node kinds"
					},
					"tags": {
						"type": "array",
						"items": { "type": "string" },
						"description": "Filter by tags"
					}
				},
				"required": ["query"]
			}),
		},
		ToolDefinition {
			name: "mindvault_search".into(),
			description: "Full-text search across all knowledge nodes.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"query": {
						"type": "string",
						"description": "Search query text"
					},
					"limit": {
						"type": "integer",
						"description": "Max results",
						"default": 10
					},
					"namespace": {
						"type": "string",
						"description": "Filter by namespace"
					},
					"kinds": {
						"type": "array",
						"items": { "type": "string" },
						"description": "Filter by node kinds"
					}
				},
				"required": ["query"]
			}),
		},
		ToolDefinition {
			name: "mindvault_get_node".into(),
			description: "Get a single knowledge node by its UUID.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"id": {
						"type": "string",
						"description": "UUID of the node"
					}
				},
				"required": ["id"]
			}),
		},
		ToolDefinition {
			name: "mindvault_update_node".into(),
			description: "Update an existing knowledge node. Provide the node ID and the fields to change.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"id": {
						"type": "string",
						"description": "UUID of the node to update"
					},
					"content": {
						"type": "string",
						"description": "New content (replaces existing)"
					},
					"title": {
						"type": "string",
						"description": "New title"
					},
					"tags": {
						"type": "array",
						"items": { "type": "string" },
						"description": "New tags (replaces existing)"
					},
					"importance": {
						"type": "number",
						"description": "New importance (0.0 to 1.0)"
					},
					"metadata": {
						"type": "object",
						"description": "Metadata fields to merge"
					}
				},
				"required": ["id"]
			}),
		},
		ToolDefinition {
			name: "mindvault_delete_node".into(),
			description: "Delete a knowledge node by its UUID.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"id": {
						"type": "string",
						"description": "UUID of the node to delete"
					}
				},
				"required": ["id"]
			}),
		},
		ToolDefinition {
			name: "mindvault_list_nodes".into(),
			description: "List knowledge nodes with optional filters.".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"namespace": {
						"type": "string",
						"description": "Filter by namespace"
					},
					"kinds": {
						"type": "array",
						"items": { "type": "string" },
						"description": "Filter by node kinds"
					},
					"tags": {
						"type": "array",
						"items": { "type": "string" },
						"description": "Filter by tags"
					},
					"limit": {
						"type": "integer",
						"description": "Max results",
						"default": 20
					},
					"offset": {
						"type": "integer",
						"description": "Pagination offset",
						"default": 0
					}
				}
			}),
		},
		ToolDefinition {
			name: "mindvault_graph_neighbors".into(),
			description: "Get graph neighbors of a node (related nodes within a traversal depth).".into(),
			input_schema: json!({
				"type": "object",
				"properties": {
					"id": {
						"type": "string",
						"description": "UUID of the node"
					},
					"depth": {
						"type": "integer",
						"description": "Traversal depth",
						"default": 2
					}
				},
				"required": ["id"]
			}),
		},
	]
}

/// Dispatch a tool call to the appropriate handler.
pub async fn call_tool(
	engine: &Arc<MindVaultEngine>,
	name: &str,
	params: Value,
) -> ToolResult {
	match name {
		"mindvault_store" => tool_store(engine, params).await,
		"mindvault_recall" => tool_recall(engine, params).await,
		"mindvault_search" => tool_search(engine, params).await,
		"mindvault_get_node" => tool_get_node(engine, params).await,
		"mindvault_update_node" => tool_update_node(engine, params).await,
		"mindvault_delete_node" => tool_delete_node(engine, params).await,
		"mindvault_list_nodes" => tool_list_nodes(engine, params).await,
		"mindvault_graph_neighbors" => tool_graph_neighbors(engine, params).await,
		_ => ToolResult::error(format!("unknown tool: {name}")),
	}
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

async fn tool_store(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let content = match params.get("content").and_then(|v| v.as_str()) {
		Some(c) => c.to_string(),
		None => return ToolResult::error("missing required parameter: content"),
	};

	let kind_str = params
		.get("kind")
		.and_then(|v| v.as_str())
		.unwrap_or("fact");
	let kind: NodeKind = match kind_str.parse() {
		Ok(k) => k,
		Err(e) => return ToolResult::error(format!("invalid kind: {e}")),
	};

	let mut node = KnowledgeNode::new(kind, content);

	if let Some(title) = params.get("title").and_then(|v| v.as_str()) {
		node = node.with_title(title);
	}
	if let Some(tags) = params.get("tags").and_then(|v| v.as_array()) {
		let tag_vec: Vec<String> = tags
			.iter()
			.filter_map(|t| t.as_str().map(String::from))
			.collect();
		node = node.with_tags(tag_vec);
	}
	if let Some(ns) = params.get("namespace").and_then(|v| v.as_str()) {
		node = node.with_namespace(ns);
	}
	if let Some(importance) = params.get("importance").and_then(|v| v.as_f64()) {
		node = node.with_importance(importance);
	}
	if let Some(metadata) = params.get("metadata").and_then(|v| v.as_object()) {
		for (k, v) in metadata {
			node.metadata.insert(k.clone(), v.clone());
		}
	}

	match engine.store_node(node).await {
		Ok(stored) => {
			let result = json!({
				"id": stored.id.to_string(),
				"kind": stored.kind.as_str(),
				"title": stored.title,
				"namespace": stored.namespace,
				"tags": stored.tags,
				"importance": stored.importance,
				"created_at": stored.temporal.created_at.to_rfc3339(),
			});
			ToolResult::text(serde_json::to_string_pretty(&result).unwrap_or_default())
		}
		Err(e) => ToolResult::error(format!("store failed: {e}")),
	}
}

async fn tool_recall(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let text = match params.get("query").and_then(|v| v.as_str()) {
		Some(q) => q.to_string(),
		None => return ToolResult::error("missing required parameter: query"),
	};

	let limit = params
		.get("limit")
		.and_then(|v| v.as_u64())
		.unwrap_or(10) as usize;
	let min_score = params
		.get("min_score")
		.and_then(|v| v.as_f64())
		.unwrap_or(0.0);
	let strategy_str = params
		.get("strategy")
		.and_then(|v| v.as_str())
		.unwrap_or("hybrid");
	let strategy: SearchStrategy = match strategy_str.parse() {
		Ok(s) => s,
		Err(e) => return ToolResult::error(format!("invalid strategy: {e}")),
	};

	let mut query = MemoryQuery::new(text)
		.with_strategy(strategy)
		.with_limit(limit)
		.with_min_score(min_score);

	if let Some(ns) = params.get("namespace").and_then(|v| v.as_str()) {
		query = query.with_namespace(ns);
	}
	if let Some(kinds) = params.get("kinds").and_then(|v| v.as_array()) {
		let parsed: Result<Vec<NodeKind>, _> = kinds
			.iter()
			.filter_map(|k| k.as_str())
			.map(|s| s.parse::<NodeKind>().map_err(|e| e.to_string()))
			.collect();
		match parsed {
			Ok(k) => query = query.with_kinds(k),
			Err(e) => return ToolResult::error(format!("invalid kind filter: {e}")),
		}
	}
	if let Some(tags) = params.get("tags").and_then(|v| v.as_array()) {
		let tag_vec: Vec<String> = tags
			.iter()
			.filter_map(|t| t.as_str().map(String::from))
			.collect();
		query = query.with_tags(tag_vec);
	}

	match engine.recall(&query).await {
		Ok(results) => {
			let items: Vec<Value> = results
				.iter()
				.map(|r| {
					json!({
						"id": r.node.id.to_string(),
						"kind": r.node.kind.as_str(),
						"title": r.node.title,
						"content": r.node.content,
						"namespace": r.node.namespace,
						"tags": r.node.tags,
						"score": r.score,
						"match_source": format!("{:?}", r.match_source).to_lowercase(),
					})
				})
				.collect();
			ToolResult::text(
				serde_json::to_string_pretty(&json!({ "results": items, "count": items.len() }))
					.unwrap_or_default(),
			)
		}
		Err(e) => ToolResult::error(format!("recall failed: {e}")),
	}
}

async fn tool_search(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let text = match params.get("query").and_then(|v| v.as_str()) {
		Some(q) => q.to_string(),
		None => return ToolResult::error("missing required parameter: query"),
	};

	let limit = params
		.get("limit")
		.and_then(|v| v.as_u64())
		.unwrap_or(10) as usize;

	let mut query = MemoryQuery::new(text)
		.with_strategy(SearchStrategy::FullText)
		.with_limit(limit);

	if let Some(ns) = params.get("namespace").and_then(|v| v.as_str()) {
		query = query.with_namespace(ns);
	}
	if let Some(kinds) = params.get("kinds").and_then(|v| v.as_array()) {
		let parsed: Result<Vec<NodeKind>, _> = kinds
			.iter()
			.filter_map(|k| k.as_str())
			.map(|s| s.parse::<NodeKind>().map_err(|e| e.to_string()))
			.collect();
		match parsed {
			Ok(k) => query = query.with_kinds(k),
			Err(e) => return ToolResult::error(format!("invalid kind filter: {e}")),
		}
	}

	match engine.recall(&query).await {
		Ok(results) => {
			let items: Vec<Value> = results
				.iter()
				.map(|r| {
					json!({
						"id": r.node.id.to_string(),
						"kind": r.node.kind.as_str(),
						"title": r.node.title,
						"content": r.node.content,
						"namespace": r.node.namespace,
						"tags": r.node.tags,
						"score": r.score,
					})
				})
				.collect();
			ToolResult::text(
				serde_json::to_string_pretty(&json!({ "results": items, "count": items.len() }))
					.unwrap_or_default(),
			)
		}
		Err(e) => ToolResult::error(format!("search failed: {e}")),
	}
}

async fn tool_get_node(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let id = match parse_uuid(&params, "id") {
		Ok(id) => id,
		Err(e) => return e,
	};

	match engine.get_node(id).await {
		Ok(Some(node)) => {
			ToolResult::text(serde_json::to_string_pretty(&node).unwrap_or_default())
		}
		Ok(None) => ToolResult::error(format!("node not found: {id}")),
		Err(e) => ToolResult::error(format!("get_node failed: {e}")),
	}
}

async fn tool_update_node(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let id = match parse_uuid(&params, "id") {
		Ok(id) => id,
		Err(e) => return e,
	};

	let existing = match engine.get_node(id).await {
		Ok(Some(node)) => node,
		Ok(None) => return ToolResult::error(format!("node not found: {id}")),
		Err(e) => return ToolResult::error(format!("get_node failed: {e}")),
	};

	let mut node = existing;

	if let Some(content) = params.get("content").and_then(|v| v.as_str()) {
		node.content = content.to_string();
	}
	if let Some(title) = params.get("title").and_then(|v| v.as_str()) {
		node.title = Some(title.to_string());
	}
	if let Some(tags) = params.get("tags").and_then(|v| v.as_array()) {
		node.tags = tags
			.iter()
			.filter_map(|t| t.as_str().map(String::from))
			.collect();
	}
	if let Some(importance) = params.get("importance").and_then(|v| v.as_f64()) {
		node.importance = importance.clamp(0.0, 1.0);
	}
	if let Some(metadata) = params.get("metadata").and_then(|v| v.as_object()) {
		for (k, v) in metadata {
			node.metadata.insert(k.clone(), v.clone());
		}
	}

	match engine.update_node(node).await {
		Ok(updated) => {
			let result = json!({
				"id": updated.id.to_string(),
				"kind": updated.kind.as_str(),
				"title": updated.title,
				"namespace": updated.namespace,
				"tags": updated.tags,
				"importance": updated.importance,
				"updated_at": updated.temporal.updated_at.to_rfc3339(),
			});
			ToolResult::text(serde_json::to_string_pretty(&result).unwrap_or_default())
		}
		Err(e) => ToolResult::error(format!("update failed: {e}")),
	}
}

async fn tool_delete_node(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let id = match parse_uuid(&params, "id") {
		Ok(id) => id,
		Err(e) => return e,
	};

	match engine.delete_node(id).await {
		Ok(true) => ToolResult::text(json!({"deleted": true, "id": id.to_string()}).to_string()),
		Ok(false) => ToolResult::error(format!("node not found: {id}")),
		Err(e) => ToolResult::error(format!("delete failed: {e}")),
	}
}

async fn tool_list_nodes(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let limit = params
		.get("limit")
		.and_then(|v| v.as_u64())
		.unwrap_or(20) as usize;
	let offset = params
		.get("offset")
		.and_then(|v| v.as_u64())
		.unwrap_or(0) as usize;

	let mut filters = QueryFilters::default();

	if let Some(ns) = params.get("namespace").and_then(|v| v.as_str()) {
		filters.namespace = Some(ns.to_string());
	}
	if let Some(kinds) = params.get("kinds").and_then(|v| v.as_array()) {
		let parsed: Result<Vec<NodeKind>, _> = kinds
			.iter()
			.filter_map(|k| k.as_str())
			.map(|s| s.parse::<NodeKind>().map_err(|e| e.to_string()))
			.collect();
		match parsed {
			Ok(k) => filters.kinds = Some(k),
			Err(e) => return ToolResult::error(format!("invalid kind filter: {e}")),
		}
	}
	if let Some(tags) = params.get("tags").and_then(|v| v.as_array()) {
		let tag_vec: Vec<String> = tags
			.iter()
			.filter_map(|t| t.as_str().map(String::from))
			.collect();
		filters.tags = Some(tag_vec);
	}

	match engine.list_nodes(&filters, limit, offset).await {
		Ok(nodes) => {
			let items: Vec<Value> = nodes
				.iter()
				.map(|n| {
					json!({
						"id": n.id.to_string(),
						"kind": n.kind.as_str(),
						"title": n.title,
						"content_preview": truncate(&n.content, 200),
						"namespace": n.namespace,
						"tags": n.tags,
						"importance": n.importance,
						"created_at": n.temporal.created_at.to_rfc3339(),
					})
				})
				.collect();
			ToolResult::text(
				serde_json::to_string_pretty(&json!({
					"nodes": items,
					"count": items.len(),
					"offset": offset,
				}))
				.unwrap_or_default(),
			)
		}
		Err(e) => ToolResult::error(format!("list_nodes failed: {e}")),
	}
}

async fn tool_graph_neighbors(engine: &Arc<MindVaultEngine>, params: Value) -> ToolResult {
	let id = match parse_uuid(&params, "id") {
		Ok(id) => id,
		Err(e) => return e,
	};

	let depth = params
		.get("depth")
		.and_then(|v| v.as_u64())
		.unwrap_or(2) as usize;

	match engine.get_neighbors(id, depth).await {
		Ok(neighbor_ids) => {
			// Fetch full nodes for each neighbor
			let mut neighbors: Vec<Value> = Vec::new();
			for nid in &neighbor_ids {
				if let Ok(Some(node)) = engine.get_node(*nid).await {
					neighbors.push(json!({
						"id": node.id.to_string(),
						"kind": node.kind.as_str(),
						"title": node.title,
						"content_preview": truncate(&node.content, 200),
						"namespace": node.namespace,
						"tags": node.tags,
					}));
				} else {
					neighbors.push(json!({
						"id": nid.to_string(),
						"error": "node not found",
					}));
				}
			}
			ToolResult::text(
				serde_json::to_string_pretty(&json!({
					"node_id": id.to_string(),
					"depth": depth,
					"neighbors": neighbors,
					"count": neighbors.len(),
				}))
				.unwrap_or_default(),
			)
		}
		Err(e) => ToolResult::error(format!("get_neighbors failed: {e}")),
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_uuid(params: &Value, field: &str) -> Result<Uuid, ToolResult> {
	let s = params
		.get(field)
		.and_then(|v| v.as_str())
		.ok_or_else(|| ToolResult::error(format!("missing required parameter: {field}")))?;
	Uuid::parse_str(s).map_err(|e| ToolResult::error(format!("invalid UUID for {field}: {e}")))
}

fn truncate(s: &str, max_len: usize) -> String {
	if s.len() <= max_len {
		s.to_string()
	} else {
		let truncated: String = s.chars().take(max_len).collect();
		format!("{truncated}...")
	}
}
