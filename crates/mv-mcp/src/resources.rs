use std::collections::HashMap;
use std::sync::Arc;

use mv_core::*;
use mv_engine::engine::MindVaultEngine;
use serde_json::json;

use crate::protocol::{ResourceContent, ResourceDefinition};

/// Return all resource definitions exposed by this MCP server.
pub fn list_resources() -> Vec<ResourceDefinition> {
	vec![
		ResourceDefinition {
			uri: "mindvault://recent".into(),
			name: "Recent Nodes".into(),
			description: "The most recently created or updated knowledge nodes.".into(),
			mime_type: "application/json".into(),
		},
		ResourceDefinition {
			uri: "mindvault://tags".into(),
			name: "Tag Cloud".into(),
			description: "All tags with occurrence counts.".into(),
			mime_type: "application/json".into(),
		},
		ResourceDefinition {
			uri: "mindvault://stats".into(),
			name: "Vault Statistics".into(),
			description: "Overview statistics for the vault (node count, kinds breakdown).".into(),
			mime_type: "application/json".into(),
		},
	]
}

/// Read a resource by URI.
pub async fn read_resource(
	engine: &Arc<MindVaultEngine>,
	uri: &str,
) -> Result<ResourceContent, String> {
	match uri {
		"mindvault://recent" => read_recent(engine).await,
		"mindvault://tags" => read_tags(engine).await,
		"mindvault://stats" => read_stats(engine).await,
		_ => Err(format!("unknown resource URI: {uri}")),
	}
}

async fn read_recent(engine: &Arc<MindVaultEngine>) -> Result<ResourceContent, String> {
	let filters = QueryFilters::default();
	let nodes = engine
		.list_nodes(&filters, 20, 0)
		.await
		.map_err(|e| format!("list_nodes failed: {e}"))?;

	let items: Vec<serde_json::Value> = nodes
		.iter()
		.map(|n| {
			json!({
				"id": n.id.to_string(),
				"kind": n.kind.as_str(),
				"title": n.title,
				"namespace": n.namespace,
				"tags": n.tags,
				"created_at": n.temporal.created_at.to_rfc3339(),
				"updated_at": n.temporal.updated_at.to_rfc3339(),
			})
		})
		.collect();

	Ok(ResourceContent {
		uri: "mindvault://recent".into(),
		mime_type: "application/json".into(),
		text: serde_json::to_string_pretty(&json!({ "nodes": items, "count": items.len() }))
			.unwrap_or_default(),
	})
}

async fn read_tags(engine: &Arc<MindVaultEngine>) -> Result<ResourceContent, String> {
	// Fetch a broad set of nodes and aggregate tags
	let filters = QueryFilters::default();
	let nodes = engine
		.list_nodes(&filters, 1000, 0)
		.await
		.map_err(|e| format!("list_nodes failed: {e}"))?;

	let mut tag_counts: HashMap<String, usize> = HashMap::new();
	for node in &nodes {
		for tag in &node.tags {
			*tag_counts.entry(tag.clone()).or_default() += 1;
		}
	}

	let mut tag_list: Vec<serde_json::Value> = tag_counts
		.iter()
		.map(|(tag, count)| json!({"tag": tag, "count": count}))
		.collect();
	tag_list.sort_by(|a, b| {
		let ca = a["count"].as_u64().unwrap_or(0);
		let cb = b["count"].as_u64().unwrap_or(0);
		cb.cmp(&ca)
	});

	Ok(ResourceContent {
		uri: "mindvault://tags".into(),
		mime_type: "application/json".into(),
		text: serde_json::to_string_pretty(&json!({
			"tags": tag_list,
			"total_unique": tag_counts.len(),
		}))
		.unwrap_or_default(),
	})
}

async fn read_stats(engine: &Arc<MindVaultEngine>) -> Result<ResourceContent, String> {
	let total = engine
		.node_count()
		.await
		.map_err(|e| format!("node_count failed: {e}"))?;

	// Count per kind
	let mut kind_counts: HashMap<&str, usize> = HashMap::new();
	let kind_list = [
		NodeKind::Fact,
		NodeKind::Task,
		NodeKind::Event,
		NodeKind::Decision,
		NodeKind::Preference,
		NodeKind::Entity,
		NodeKind::CodeSnippet,
		NodeKind::Project,
		NodeKind::Conversation,
		NodeKind::Procedure,
		NodeKind::Observation,
		NodeKind::Bookmark,
		NodeKind::Template,
		NodeKind::SavedView,
	];
	for kind in &kind_list {
		let filters = QueryFilters {
			kinds: Some(vec![*kind]),
			..Default::default()
		};
		if let Ok(nodes) = engine.list_nodes(&filters, 0, 0).await {
			kind_counts.insert(kind.as_str(), nodes.len());
		}
	}

	let embedding_status = engine.embedding_runtime_status();

	Ok(ResourceContent {
		uri: "mindvault://stats".into(),
		mime_type: "application/json".into(),
		text: serde_json::to_string_pretty(&json!({
			"total_nodes": total,
			"kinds": kind_counts,
			"embedding": {
				"provider": embedding_status.effective_provider,
				"model": embedding_status.effective_model,
				"dimensions": embedding_status.effective_dimensions,
			},
		}))
		.unwrap_or_default(),
	})
}
