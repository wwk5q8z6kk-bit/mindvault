use std::sync::Arc;

use mv_core::{KnowledgeNode, NodeKind, PermissionTemplate, PermissionTier, QueryFilters};
use mv_engine::engine::MindVaultEngine;

const DEFAULT_RESOURCE_LIMIT: usize = 1000;

#[derive(Debug, Clone)]
pub struct McpContext {
    scope: McpScope,
    template_name: Option<String>,
    key_id: Option<String>,
}

impl McpContext {
    pub async fn from_access_key(
        engine: &Arc<MindVaultEngine>,
        token: &str,
    ) -> Result<Self, String> {
        let (key, template) = engine
            .resolve_access_key(token)
            .await
            .map_err(|e| format!("access key lookup failed: {e}"))?
            .ok_or("access key not found or expired".to_string())?;

        let scope = McpScope::from_template(&template);
        let key_id = Some(key.id.to_string());
        let template_name = Some(format!("{}:{}", template.name, key.id));

        Ok(Self {
            scope,
            template_name,
            key_id,
        })
    }

    pub fn unscoped_read_only() -> Self {
        Self {
            scope: McpScope::read_only(),
            template_name: None,
            key_id: None,
        }
    }

    pub fn scope(&self) -> &McpScope {
        &self.scope
    }

    pub fn key_id(&self) -> Option<&str> {
        self.key_id.as_deref()
    }

    pub fn can_read(&self) -> bool {
        true
    }

    pub fn can_write(&self) -> bool {
        self.scope.allow_write
    }

    pub fn summary(&self) -> String {
        match &self.template_name {
            Some(name) => format!("scoped({name})"),
            None => "unscoped(read-only)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpScope {
    pub namespace: Option<String>,
    pub tags: Vec<String>,
    pub kinds: Vec<NodeKind>,
    pub allow_write: bool,
    pub allow_actions: Vec<String>,
    pub resource_limit: usize,
    pub tier: PermissionTier,
}

impl McpScope {
    pub fn from_template(template: &PermissionTemplate) -> Self {
        let allow_write = matches!(
            template.tier,
            PermissionTier::Edit | PermissionTier::Action | PermissionTier::Admin
        );

        Self {
            namespace: template.scope_namespace.clone(),
            tags: template.scope_tags.clone(),
            kinds: template.allow_kinds.clone(),
            allow_write,
            allow_actions: template.allow_actions.clone(),
            resource_limit: DEFAULT_RESOURCE_LIMIT,
            tier: template.tier,
        }
    }

    pub fn read_only() -> Self {
        Self {
            namespace: None,
            tags: Vec::new(),
            kinds: Vec::new(),
            allow_write: false,
            allow_actions: Vec::new(),
            resource_limit: DEFAULT_RESOURCE_LIMIT,
            tier: PermissionTier::View,
        }
    }

    pub fn is_unscoped(&self) -> bool {
        self.namespace.is_none() && self.tags.is_empty() && self.kinds.is_empty()
    }

    pub fn is_admin(&self) -> bool {
        self.tier == PermissionTier::Admin
    }

    pub fn ensure_action(&self, action: &str) -> Result<(), String> {
        if self.allows_action(action) {
            Ok(())
        } else {
            Err(format!("action '{action}' not permitted by access scope"))
        }
    }

    pub fn apply_filters(&self, filters: &mut QueryFilters) -> Result<(), String> {
        if let Some(ns) = &self.namespace {
            if let Some(req) = &filters.namespace {
                if req != ns {
                    return Err(format!("namespace '{req}' not permitted"));
                }
            }
            filters.namespace = Some(ns.clone());
        }

        if !self.kinds.is_empty() {
            if let Some(req) = &filters.kinds {
                if !req.iter().all(|k| self.kinds.contains(k)) {
                    return Err("one or more kinds not permitted".into());
                }
            } else {
                filters.kinds = Some(self.kinds.clone());
            }
        }

        if !self.tags.is_empty() {
            if let Some(req) = &filters.tags {
                if !req.iter().all(|tag| self.contains_tag(tag)) {
                    return Err("one or more tags not permitted".into());
                }
            } else {
                filters.tags = Some(self.tags.clone());
            }
        }

        Ok(())
    }

    pub fn check_node(&self, node: &KnowledgeNode) -> Result<(), String> {
        if let Some(ns) = &self.namespace {
            if &node.namespace != ns {
                return Err("node not permitted by namespace scope".into());
            }
        }

        if !self.kinds.is_empty() && !self.kinds.contains(&node.kind) {
            return Err("node kind not permitted".into());
        }

        if !self.tags.is_empty()
            && !node.tags.iter().any(|tag| self.contains_tag(tag))
        {
            return Err("node tags not permitted".into());
        }

        Ok(())
    }

    pub fn normalize_namespace(&self, requested: Option<&str>) -> Result<Option<String>, String> {
        if let Some(ns) = &self.namespace {
            if let Some(req) = requested {
                if req != ns {
                    return Err(format!("namespace '{req}' not permitted"));
                }
            }
            return Ok(Some(ns.clone()));
        }

        Ok(requested.map(|s| s.to_string()))
    }

    pub fn ensure_kind(&self, kind: NodeKind) -> Result<(), String> {
        if self.kinds.is_empty() || self.kinds.contains(&kind) {
            Ok(())
        } else {
            Err(format!("kind '{}' not permitted", kind.as_str()))
        }
    }

    pub fn ensure_write_allowed(&self) -> Result<(), String> {
        if self.allow_write {
            Ok(())
        } else {
            Err("write/propose operations are not permitted".into())
        }
    }

    pub fn ensure_tags_for_proposal(&self, mut tags: Vec<String>) -> Vec<String> {
        if self.tags.is_empty() {
            return tags;
        }

        for scope_tag in &self.tags {
            if !tags.iter().any(|t| t.eq_ignore_ascii_case(scope_tag)) {
                tags.push(scope_tag.clone());
            }
        }

        tags
    }

    fn allows_action(&self, action: &str) -> bool {
        if self.allow_actions.is_empty() {
            return true;
        }

        self.allow_actions
            .iter()
            .any(|allowed| action_matches(allowed, action))
    }

    fn contains_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }
}

fn action_matches(allowed: &str, action: &str) -> bool {
    if allowed == "*" || allowed == "mcp:*" {
        return true;
    }
    if allowed == action {
        return true;
    }
    if let Some(prefix) = allowed.strip_suffix(".*") {
        return action.starts_with(prefix);
    }
    false
}
