use std::collections::HashMap;
use std::fmt;

use mv_core::{ExecProxyRequest, HttpProxyRequest, NodeKind};
use mv_engine::recurrence::validate_recurrence_metadata_for_kind;
use serde_json::Value;

/// Structured validation error with meaningful variants.
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// A required field is empty or blank.
    Empty { field: String },
    /// A field exceeds its maximum allowed length/size.
    TooLong { field: String, max: usize },
    /// A numeric value is out of its allowed range.
    OutOfRange { field: String, detail: String },
    /// A field contains characters that are not permitted.
    InvalidChars { field: String },
    /// A collection (tags, headers, args) has too many items.
    TooMany { field: String, max: usize },
    /// A field value is not one of the allowed options.
    NotAllowed { field: String, detail: String },
    /// Metadata serialization or structural issue.
    BadMetadata { detail: String },
    /// Recurrence metadata issue (delegated from mv-engine).
    Recurrence { detail: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(f, "{field} cannot be empty"),
            Self::TooLong { field, max } => {
                write!(f, "{field} exceeds max length of {max}")
            }
            Self::OutOfRange { field, detail } => write!(f, "{field} {detail}"),
            Self::InvalidChars { field } => {
                write!(f, "{field} contains invalid characters")
            }
            Self::TooMany { field, max } => {
                write!(f, "{field} cannot exceed {max} items")
            }
            Self::NotAllowed { field, detail } => {
                write!(f, "{field} {detail}")
            }
            Self::BadMetadata { detail } => write!(f, "{detail}"),
            Self::Recurrence { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for ValidationError {}

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

// --- Proxy validation constants ---
const MAX_PROXY_URL_LEN: usize = 8192;
const MAX_PROXY_BODY_LEN: usize = 1024 * 1024; // 1 MB
const MAX_PROXY_HEADERS: usize = 32;
const MAX_PROXY_HEADER_NAME_LEN: usize = 256;
const MAX_PROXY_HEADER_VALUE_LEN: usize = 8192;
const MAX_PROXY_ARGS: usize = 64;
const MAX_PROXY_ARG_LEN: usize = 4096;
const MAX_PROXY_TIMEOUT_SECS: u64 = 300;
const MIN_PROXY_INTENT_LEN: usize = 10;
const MAX_PROXY_INTENT_LEN: usize = 500;
const MAX_PROXY_SECRET_REF_LEN: usize = 256;
const MAX_PROXY_COMMAND_LEN: usize = 256;

const ALLOWED_HTTP_METHODS: &[&str] = &["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];

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
) -> Result<(), ValidationError> {
    validate_required_text("content", content, MAX_NODE_CONTENT_LEN)?;
    validate_optional_text("title", title, MAX_NODE_TITLE_LEN)?;
    validate_optional_text("source", source, MAX_NODE_SOURCE_LEN)?;

    if let Some(namespace) = namespace {
        validate_namespace(namespace)?;
    }

    validate_tags(tags)?;

    if let Some(importance) = importance {
        if !importance.is_finite() || !(0.0..=1.0).contains(&importance) {
            return Err(ValidationError::OutOfRange {
                field: "importance".into(),
                detail: "must be a finite value between 0.0 and 1.0".into(),
            });
        }
    }

    validate_metadata(metadata)?;
    validate_recurrence_metadata_for_kind(kind, metadata)
        .map_err(|detail| ValidationError::Recurrence { detail })?;

    Ok(())
}

pub fn validate_query_text(field_name: &str, text: &str) -> Result<(), ValidationError> {
    validate_required_text(field_name, text, MAX_QUERY_TEXT_LEN)
}

pub fn validate_recall_limit(limit: usize) -> Result<(), ValidationError> {
    if limit == 0 {
        return Err(ValidationError::OutOfRange {
            field: "limit".into(),
            detail: "must be greater than 0".into(),
        });
    }
    if limit > MAX_RECALL_LIMIT {
        return Err(ValidationError::OutOfRange {
            field: "limit".into(),
            detail: format!("must be <= {MAX_RECALL_LIMIT}"),
        });
    }
    Ok(())
}

pub fn validate_list_limit(limit: usize) -> Result<(), ValidationError> {
    if limit == 0 {
        return Err(ValidationError::OutOfRange {
            field: "limit".into(),
            detail: "must be greater than 0".into(),
        });
    }
    if limit > MAX_LIST_LIMIT {
        return Err(ValidationError::OutOfRange {
            field: "limit".into(),
            detail: format!("must be <= {MAX_LIST_LIMIT}"),
        });
    }
    Ok(())
}

pub fn validate_text_input(name: &str, value: &str) -> Result<(), ValidationError> {
    validate_required_text(name, value, MAX_NODE_TITLE_LEN)
}

pub fn validate_namespace_input(namespace: Option<&str>) -> Result<(), ValidationError> {
    if let Some(namespace) = namespace {
        validate_namespace(namespace)?;
    }
    Ok(())
}

pub fn validate_tags_input(tags: &[String]) -> Result<(), ValidationError> {
    validate_tags(tags)
}

pub fn validate_depth(depth: usize) -> Result<(), ValidationError> {
    if depth == 0 {
        return Err(ValidationError::OutOfRange {
            field: "depth".into(),
            detail: "must be greater than 0".into(),
        });
    }
    if depth > MAX_NEIGHBOR_DEPTH {
        return Err(ValidationError::OutOfRange {
            field: "depth".into(),
            detail: format!("must be <= {MAX_NEIGHBOR_DEPTH}"),
        });
    }
    Ok(())
}

fn validate_required_text(name: &str, value: &str, max_len: usize) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::Empty { field: name.into() });
    }

    if value.len() > max_len {
        return Err(ValidationError::TooLong {
            field: name.into(),
            max: max_len,
        });
    }

    Ok(())
}

fn validate_optional_text(
    name: &str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), ValidationError> {
    if let Some(value) = value {
        validate_required_text(name, value, max_len)?;
    }
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<(), ValidationError> {
    validate_required_text("namespace", namespace, MAX_NAMESPACE_LEN)?;
    if namespace
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/')))
    {
        return Err(ValidationError::InvalidChars {
            field: "namespace".into(),
        });
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<(), ValidationError> {
    if tags.len() > MAX_TAGS {
        return Err(ValidationError::TooMany {
            field: "tags".into(),
            max: MAX_TAGS,
        });
    }

    for tag in tags {
        validate_required_text("tag", tag, MAX_TAG_LEN)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Proxy validation
// ---------------------------------------------------------------------------

/// Validate an HTTP proxy request before passing to ProxyEngine.
pub fn validate_http_proxy_request(req: &HttpProxyRequest) -> Result<(), ValidationError> {
    // URL
    if req.url.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "url".into(),
        });
    }
    if req.url.len() > MAX_PROXY_URL_LEN {
        return Err(ValidationError::TooLong {
            field: "url".into(),
            max: MAX_PROXY_URL_LEN,
        });
    }

    // Method
    let method_upper = req.method.to_ascii_uppercase();
    if !ALLOWED_HTTP_METHODS.contains(&method_upper.as_str()) {
        return Err(ValidationError::NotAllowed {
            field: "method".into(),
            detail: format!(
                "'{}' is not allowed; permitted: {}",
                req.method,
                ALLOWED_HTTP_METHODS.join(", ")
            ),
        });
    }

    // Body
    if let Some(ref body) = req.body {
        if body.len() > MAX_PROXY_BODY_LEN {
            return Err(ValidationError::TooLong {
                field: "body".into(),
                max: MAX_PROXY_BODY_LEN,
            });
        }
    }

    // Headers
    if req.headers.len() > MAX_PROXY_HEADERS {
        return Err(ValidationError::TooMany {
            field: "headers".into(),
            max: MAX_PROXY_HEADERS,
        });
    }
    for (name, value) in &req.headers {
        if name.len() > MAX_PROXY_HEADER_NAME_LEN {
            return Err(ValidationError::TooLong {
                field: format!("header name '{}'", &name[..64.min(name.len())]),
                max: MAX_PROXY_HEADER_NAME_LEN,
            });
        }
        if value.len() > MAX_PROXY_HEADER_VALUE_LEN {
            return Err(ValidationError::TooLong {
                field: format!("header value for '{name}'"),
                max: MAX_PROXY_HEADER_VALUE_LEN,
            });
        }
    }

    // Secret ref
    validate_proxy_secret_ref(&req.secret_ref)?;

    // Intent
    validate_proxy_intent(&req.intent)?;

    Ok(())
}

/// Validate an exec proxy request before passing to ProxyEngine.
pub fn validate_exec_proxy_request(req: &ExecProxyRequest) -> Result<(), ValidationError> {
    // Command
    if req.command.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "command".into(),
        });
    }
    if req.command.len() > MAX_PROXY_COMMAND_LEN {
        return Err(ValidationError::TooLong {
            field: "command".into(),
            max: MAX_PROXY_COMMAND_LEN,
        });
    }

    // Args
    if req.args.len() > MAX_PROXY_ARGS {
        return Err(ValidationError::TooMany {
            field: "args".into(),
            max: MAX_PROXY_ARGS,
        });
    }
    for (i, arg) in req.args.iter().enumerate() {
        if arg.len() > MAX_PROXY_ARG_LEN {
            return Err(ValidationError::TooLong {
                field: format!("arg[{i}]"),
                max: MAX_PROXY_ARG_LEN,
            });
        }
    }

    // Env inject: validate secret refs
    for (env_var, secret_key) in &req.env_inject {
        if env_var.trim().is_empty() {
            return Err(ValidationError::Empty {
                field: "env_inject key".into(),
            });
        }
        validate_proxy_secret_ref(secret_key)?;
    }

    // Timeout
    if req.timeout_seconds > MAX_PROXY_TIMEOUT_SECS {
        return Err(ValidationError::OutOfRange {
            field: "timeout_seconds".into(),
            detail: format!("cannot exceed {MAX_PROXY_TIMEOUT_SECS}"),
        });
    }

    // Intent
    validate_proxy_intent(&req.intent)?;

    Ok(())
}

fn validate_proxy_secret_ref(secret_ref: &str) -> Result<(), ValidationError> {
    if secret_ref.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "secret_ref".into(),
        });
    }
    if secret_ref.len() > MAX_PROXY_SECRET_REF_LEN {
        return Err(ValidationError::TooLong {
            field: "secret_ref".into(),
            max: MAX_PROXY_SECRET_REF_LEN,
        });
    }
    Ok(())
}

fn validate_proxy_intent(intent: &str) -> Result<(), ValidationError> {
    let trimmed = intent.trim();
    if trimmed.len() < MIN_PROXY_INTENT_LEN {
        return Err(ValidationError::OutOfRange {
            field: "intent".into(),
            detail: format!("must be at least {MIN_PROXY_INTENT_LEN} characters"),
        });
    }
    if intent.len() > MAX_PROXY_INTENT_LEN {
        return Err(ValidationError::TooLong {
            field: "intent".into(),
            max: MAX_PROXY_INTENT_LEN,
        });
    }
    Ok(())
}

fn validate_metadata(metadata: Option<&HashMap<String, Value>>) -> Result<(), ValidationError> {
    let Some(metadata) = metadata else {
        return Ok(());
    };

    let encoded = serde_json::to_string(metadata).map_err(|err| ValidationError::BadMetadata {
        detail: format!("metadata must be valid JSON serializable object: {err}"),
    })?;
    if encoded.len() > MAX_METADATA_JSON_LEN {
        return Err(ValidationError::TooLong {
            field: "metadata".into(),
            max: MAX_METADATA_JSON_LEN,
        });
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
        assert!(matches!(err, ValidationError::Empty { ref field } if field == "content"));
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
        assert!(matches!(err, ValidationError::InvalidChars { .. }));
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
        assert!(
            matches!(err, ValidationError::OutOfRange { ref field, .. } if field == "importance")
        );
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
        assert!(matches!(err, ValidationError::Recurrence { .. }));
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

    // --- Proxy validation tests ---

    fn make_http_proxy_req() -> HttpProxyRequest {
        HttpProxyRequest {
            method: "GET".into(),
            url: "https://api.example.com/v1/data".into(),
            headers: HashMap::new(),
            body: None,
            secret_ref: "MY_API_KEY".into(),
            inject_as: mv_core::SecretInjection::BearerHeader,
            intent: "fetch user profile data for dashboard display".into(),
        }
    }

    fn make_exec_proxy_req() -> ExecProxyRequest {
        ExecProxyRequest {
            command: "gh".into(),
            args: vec!["pr".into(), "list".into()],
            env_inject: {
                let mut m = HashMap::new();
                m.insert("GH_TOKEN".into(), "GITHUB_TOKEN".into());
                m
            },
            working_dir: None,
            timeout_seconds: 30,
            intent: "list open pull requests for review".into(),
        }
    }

    #[test]
    fn proxy_http_valid_request_passes() {
        assert!(validate_http_proxy_request(&make_http_proxy_req()).is_ok());
    }

    #[test]
    fn proxy_http_rejects_bad_method() {
        let mut req = make_http_proxy_req();
        req.method = "CONNECT".into();
        assert!(matches!(
            validate_http_proxy_request(&req).unwrap_err(),
            ValidationError::NotAllowed { .. }
        ));
    }

    #[test]
    fn proxy_http_rejects_oversized_body() {
        let mut req = make_http_proxy_req();
        req.body = Some("x".repeat(MAX_PROXY_BODY_LEN + 1));
        assert!(matches!(
            validate_http_proxy_request(&req).unwrap_err(),
            ValidationError::TooLong { ref field, .. } if field == "body"
        ));
    }

    #[test]
    fn proxy_http_rejects_too_many_headers() {
        let mut req = make_http_proxy_req();
        for i in 0..MAX_PROXY_HEADERS + 1 {
            req.headers.insert(format!("X-Header-{i}"), "v".into());
        }
        assert!(matches!(
            validate_http_proxy_request(&req).unwrap_err(),
            ValidationError::TooMany { ref field, .. } if field == "headers"
        ));
    }

    #[test]
    fn proxy_http_rejects_short_intent() {
        let mut req = make_http_proxy_req();
        req.intent = "short".into();
        assert!(matches!(
            validate_http_proxy_request(&req).unwrap_err(),
            ValidationError::OutOfRange { ref field, .. } if field == "intent"
        ));
    }

    #[test]
    fn proxy_http_rejects_long_intent() {
        let mut req = make_http_proxy_req();
        req.intent = "x".repeat(MAX_PROXY_INTENT_LEN + 1);
        assert!(matches!(
            validate_http_proxy_request(&req).unwrap_err(),
            ValidationError::TooLong { ref field, .. } if field == "intent"
        ));
    }

    #[test]
    fn proxy_exec_valid_request_passes() {
        assert!(validate_exec_proxy_request(&make_exec_proxy_req()).is_ok());
    }

    #[test]
    fn proxy_exec_rejects_too_many_args() {
        let mut req = make_exec_proxy_req();
        req.args = (0..MAX_PROXY_ARGS + 1).map(|i| format!("arg{i}")).collect();
        assert!(matches!(
            validate_exec_proxy_request(&req).unwrap_err(),
            ValidationError::TooMany { ref field, .. } if field == "args"
        ));
    }

    #[test]
    fn proxy_exec_rejects_oversized_timeout() {
        let mut req = make_exec_proxy_req();
        req.timeout_seconds = MAX_PROXY_TIMEOUT_SECS + 1;
        assert!(matches!(
            validate_exec_proxy_request(&req).unwrap_err(),
            ValidationError::OutOfRange { ref field, .. } if field == "timeout_seconds"
        ));
    }
}
