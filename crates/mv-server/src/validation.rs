use std::collections::HashMap;

use mv_core::NodeKind;
use mv_engine::recurrence::validate_recurrence_metadata_for_kind;
use serde_json::Value;

const MAX_NODE_CONTENT_LEN: usize = 64 * 1024;
const MAX_NODE_TITLE_LEN: usize = 512;
const MAX_NODE_SOURCE_LEN: usize = 2048;
const MAX_NAMESPACE_LEN: usize = 128;
const MAX_TAGS: usize = 32;
const MAX_TAG_LEN: usize = 64;
const MAX_METADATA_JSON_LEN: usize = 64 * 1024;
const MAX_QUERY_TEXT_LEN: usize = 4096;
const MAX_RECALL_LIMIT: usize = 200;
const MAX_LIST_LIMIT: usize = 500;
const MAX_NEIGHBOR_DEPTH: usize = 8;

#[allow(clippy::too_many_arguments)]
pub fn validate_node_payload(
    kind: NodeKind,
    title: Option<&str>,
    content: &str,
    source: Option<&str>,
    namespace: Option<&str>,
    tags: &[String],
    importance: Option<f64>,
    metadata: Option<&HashMap<String, Value>>,
) -> Result<(), String> {
    validate_required_text("content", content, MAX_NODE_CONTENT_LEN)?;
    validate_optional_text("title", title, MAX_NODE_TITLE_LEN)?;
    validate_optional_text("source", source, MAX_NODE_SOURCE_LEN)?;

    if let Some(namespace) = namespace {
        validate_namespace(namespace)?;
    }

    validate_tags(tags)?;

    if let Some(importance) = importance {
        if !importance.is_finite() || !(0.0..=1.0).contains(&importance) {
            return Err("importance must be a finite value between 0.0 and 1.0".into());
        }
    }

    validate_metadata(metadata)?;
    validate_recurrence_metadata_for_kind(kind, metadata)?;

    Ok(())
}

pub fn validate_query_text(field_name: &str, text: &str) -> Result<(), String> {
    validate_required_text(field_name, text, MAX_QUERY_TEXT_LEN)
}

pub fn validate_recall_limit(limit: usize) -> Result<(), String> {
    if limit == 0 {
        return Err("limit must be greater than 0".into());
    }
    if limit > MAX_RECALL_LIMIT {
        return Err(format!("limit must be <= {MAX_RECALL_LIMIT}"));
    }
    Ok(())
}

pub fn validate_list_limit(limit: usize) -> Result<(), String> {
    if limit == 0 {
        return Err("limit must be greater than 0".into());
    }
    if limit > MAX_LIST_LIMIT {
        return Err(format!("limit must be <= {MAX_LIST_LIMIT}"));
    }
    Ok(())
}

pub fn validate_text_input(name: &str, value: &str) -> Result<(), String> {
    validate_required_text(name, value, MAX_NODE_TITLE_LEN)
}

pub fn validate_namespace_input(namespace: Option<&str>) -> Result<(), String> {
    if let Some(namespace) = namespace {
        validate_namespace(namespace)?;
    }
    Ok(())
}

pub fn validate_tags_input(tags: &[String]) -> Result<(), String> {
    validate_tags(tags)
}

pub fn validate_depth(depth: usize) -> Result<(), String> {
    if depth == 0 {
        return Err("depth must be greater than 0".into());
    }
    if depth > MAX_NEIGHBOR_DEPTH {
        return Err(format!("depth must be <= {MAX_NEIGHBOR_DEPTH}"));
    }
    Ok(())
}

fn validate_required_text(name: &str, value: &str, max_len: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} cannot be empty"));
    }

    if value.len() > max_len {
        return Err(format!("{name} exceeds max length of {max_len}"));
    }

    Ok(())
}

fn validate_optional_text(name: &str, value: Option<&str>, max_len: usize) -> Result<(), String> {
    if let Some(value) = value {
        validate_required_text(name, value, max_len)?;
    }
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<(), String> {
    validate_required_text("namespace", namespace, MAX_NAMESPACE_LEN)?;
    if namespace
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/')))
    {
        return Err("namespace contains invalid characters".into());
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<(), String> {
    if tags.len() > MAX_TAGS {
        return Err(format!("tags cannot exceed {MAX_TAGS} items"));
    }

    for tag in tags {
        validate_required_text("tag", tag, MAX_TAG_LEN)?;
    }

    Ok(())
}

fn validate_metadata(metadata: Option<&HashMap<String, Value>>) -> Result<(), String> {
    let Some(metadata) = metadata else {
        return Ok(());
    };

    let encoded = serde_json::to_string(metadata)
        .map_err(|err| format!("metadata must be valid JSON serializable object: {err}"))?;
    if encoded.len() > MAX_METADATA_JSON_LEN {
        return Err(format!(
            "metadata exceeds max serialized size of {MAX_METADATA_JSON_LEN} bytes"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_node_payload_rejects_empty_content() {
        let err = validate_node_payload(
            NodeKind::Fact,
            None,
            "   ",
            None,
            Some("default"),
            &[],
            Some(0.5),
            None,
        )
        .expect_err("empty content should fail");
        assert!(err.contains("content cannot be empty"));
    }

    #[test]
    fn validate_node_payload_rejects_bad_namespace() {
        let err = validate_node_payload(
            NodeKind::Fact,
            None,
            "ok",
            None,
            Some("bad namespace"),
            &[],
            Some(0.5),
            None,
        )
        .expect_err("namespace should fail");
        assert!(err.contains("invalid characters"));
    }

    #[test]
    fn validate_node_payload_rejects_invalid_importance() {
        let err = validate_node_payload(
            NodeKind::Fact,
            None,
            "ok",
            None,
            Some("ns"),
            &[],
            Some(1.5),
            None,
        )
        .expect_err("importance should fail");
        assert!(err.contains("importance"));
    }

    #[test]
    fn validate_node_payload_rejects_task_recurrence_for_non_task_kind() {
        let metadata = serde_json::json!({
            "task_recurrence": {
                "frequency": "daily",
                "interval": 1
            }
        });
        let metadata: HashMap<String, Value> =
            serde_json::from_value(metadata).expect("metadata should deserialize");
        let err = validate_node_payload(
            NodeKind::Fact,
            None,
            "ok",
            None,
            Some("ns"),
            &[],
            Some(0.5),
            Some(&metadata),
        )
        .expect_err("task_recurrence on fact should fail");
        assert!(err.contains("task_recurrence"));
    }

    #[test]
    fn validate_limit_checks_boundaries() {
        assert!(validate_recall_limit(1).is_ok());
        assert!(validate_recall_limit(201).is_err());
        assert!(validate_list_limit(500).is_ok());
        assert!(validate_list_limit(501).is_err());
    }

    #[test]
    fn validate_depth_checks_boundaries() {
        assert!(validate_depth(1).is_ok());
        assert!(validate_depth(9).is_err());
    }
}
