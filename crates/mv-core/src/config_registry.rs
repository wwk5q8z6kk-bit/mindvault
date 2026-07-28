use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

/// Trait implemented by each config section (server, storage, ai, etc.)
pub trait ConfigSection: Any + Send + Sync + 'static {
    /// The TOML section name (e.g., "server", "storage", "ai")
    fn section_name(&self) -> &'static str;

    /// Validate this section's values. Returns `Ok(())` or an error message.
    fn validate(&self) -> Result<(), String>;

    /// List the known keys for this section (for unhandled key detection).
    fn known_keys(&self) -> &'static [&'static str];
}

/// Type-erased wrapper that preserves both `Any` downcast and `ConfigSection` methods.
struct SectionEntry {
    value: Box<dyn Any + Send + Sync>,
    name: &'static str,
    keys: &'static [&'static str],
    validate_fn: fn(&dyn Any) -> Result<(), String>,
}

/// Registry of typed config sections. Additive wrapper around existing config flow.
pub struct ConfigRegistry {
    entries: HashMap<TypeId, SectionEntry>,
}

impl Default for ConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a config section.
    pub fn register<T: ConfigSection>(&mut self, section: T) {
        let type_id = TypeId::of::<T>();
        let name = section.section_name();
        let keys = section.known_keys();

        fn do_validate<T: ConfigSection>(any: &dyn Any) -> Result<(), String> {
            any.downcast_ref::<T>()
                .expect("type mismatch in config registry")
                .validate()
        }

        self.entries.insert(
            type_id,
            SectionEntry {
                value: Box::new(section),
                name,
                keys,
                validate_fn: do_validate::<T>,
            },
        );
    }

    /// Get a config section by type.
    pub fn get<T: ConfigSection>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.entries
            .get(&type_id)
            .and_then(|entry| entry.value.downcast_ref::<T>())
    }

    /// List all registered section names (sorted, for agent discoverability).
    pub fn list_sections(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.values().map(|e| e.name).collect();
        names.sort();
        names
    }

    /// List all registered section names with their known keys.
    pub fn list_sections_with_keys(&self) -> Vec<SectionInfo> {
        let mut result: Vec<SectionInfo> = self
            .entries
            .values()
            .map(|entry| SectionInfo {
                name: entry.name.to_string(),
                keys: entry.keys.iter().map(|k| k.to_string()).collect(),
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Validate all registered sections.
    /// Returns `Ok(())` or a list of `(section_name, error_message)` pairs.
    pub fn validate_all(&self) -> Result<(), Vec<(String, String)>> {
        let mut errors = Vec::new();
        for entry in self.entries.values() {
            if let Err(msg) = (entry.validate_fn)(entry.value.as_ref()) {
                errors.push((entry.name.to_string(), msg));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort_by(|a, b| a.0.cmp(&b.0));
            Err(errors)
        }
    }

    /// Find TOML keys that no section claimed.
    /// Keys should be in "section.field" format.
    pub fn find_unhandled_keys(&self, toml_keys: &[String]) -> Vec<String> {
        let section_keys: HashMap<&str, &[&str]> =
            self.entries.values().map(|e| (e.name, e.keys)).collect();

        toml_keys
            .iter()
            .filter(|key| {
                if let Some((section, field)) = key.split_once('.') {
                    match section_keys.get(section) {
                        Some(keys) => !keys.contains(&field),
                        None => true, // unknown section
                    }
                } else {
                    // top-level key — not claimed by any section
                    !section_keys.contains_key(key.as_str())
                }
            })
            .cloned()
            .collect()
    }

    /// Return all known MindVault config sections with their keys.
    /// This is a static catalog — it does not require a parsed config file.
    /// Useful for introspection endpoints (agent discoverability).
    pub fn builtin_section_catalog() -> Vec<SectionInfo> {
        let mut sections: Vec<SectionInfo> = BUILTIN_SECTIONS
            .iter()
            .map(|(name, keys)| SectionInfo {
                name: name.to_string(),
                keys: keys.iter().map(|k| k.to_string()).collect(),
            })
            .collect();
        sections.sort_by(|a, b| a.name.cmp(&b.name));
        sections
    }

    /// Return a static catalog filtered to the given section names.
    pub fn builtin_section_catalog_scoped(sections: &[&str]) -> Vec<SectionInfo> {
        let allowed: HashSet<&str> = sections.iter().copied().collect();
        let mut result: Vec<SectionInfo> = BUILTIN_SECTIONS
            .iter()
            .filter(|(name, _)| allowed.contains(name))
            .map(|(name, keys)| SectionInfo {
                name: name.to_string(),
                keys: keys.iter().map(|k| k.to_string()).collect(),
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Create a scoped view limited to the specified section names.
    pub fn scope(&self, sections: &[&str]) -> ConfigScope<'_> {
        ConfigScope {
            registry: self,
            allowed_sections: sections.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Scope for AI/search modules.
    pub fn ai_scope(&self) -> ConfigScope<'_> {
        self.scope(&["ai", "search", "embedding", "llm"])
    }

    /// Scope for email adapter.
    pub fn email_scope(&self) -> ConfigScope<'_> {
        self.scope(&["email"])
    }

    /// Scope for storage operations.
    pub fn storage_scope(&self) -> ConfigScope<'_> {
        self.scope(&["storage", "encryption"])
    }
}

/// A scoped view into the config registry, limiting visibility to specific sections.
pub struct ConfigScope<'a> {
    registry: &'a ConfigRegistry,
    allowed_sections: HashSet<String>,
}

impl<'a> ConfigScope<'a> {
    /// Get a config section by type (only if it is in this scope's allow list).
    pub fn get<T: ConfigSection>(&self) -> Option<&T> {
        let section = self.registry.get::<T>()?;
        if self.allowed_sections.contains(section.section_name()) {
            Some(section)
        } else {
            None
        }
    }

    /// List only the section names visible to this scope (sorted).
    pub fn list_sections(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .registry
            .entries
            .values()
            .filter(|e| self.allowed_sections.contains(e.name))
            .map(|e| e.name)
            .collect();
        names.sort();
        names
    }

    /// List sections with keys, filtered to this scope (sorted).
    pub fn list_sections_with_keys(&self) -> Vec<SectionInfo> {
        let mut result: Vec<SectionInfo> = self
            .registry
            .entries
            .values()
            .filter(|entry| self.allowed_sections.contains(entry.name))
            .map(|entry| SectionInfo {
                name: entry.name.to_string(),
                keys: entry.keys.iter().map(|k| k.to_string()).collect(),
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }
}

/// Info about a registered config section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SectionInfo {
    pub name: String,
    pub keys: Vec<String>,
}

/// Canonical list of MindVault config sections and their known keys.
const BUILTIN_SECTIONS: &[(&str, &[&str])] = &[
    (
        "server",
        &[
            "bind_host",
            "rest_port",
            "grpc_port",
            "socket_path",
            "cors_allowed_origins",
        ],
    ),
    ("storage", &["data_dir"]),
    (
        "profile",
        &["display_name", "primary_email", "timezone", "signature"],
    ),
    (
        "embedding",
        &["provider", "model", "dimensions", "base_url"],
    ),
    (
        "search",
        &[
            "default_limit",
            "default_strategy",
            "min_score",
            "vector_weight",
            "fulltext_weight",
            "rrf_k",
        ],
    ),
    ("graph", &["default_traversal_depth", "graph_boost_factor"]),
    (
        "ai",
        &[
            "auto_tagging_enabled",
            "auto_tagging_max_generated_tags",
            "auto_tagging_max_total_tags",
            "auto_tagging_similarity_seed_limit",
            "auto_tagging_min_token_length",
        ],
    ),
    (
        "watcher",
        &[
            "enabled",
            "interval_secs",
            "lookback_hours",
            "max_nodes_per_cycle",
            "expiry_days",
        ],
    ),
    ("ai_sidecar", &["enabled", "base_url", "timeout_secs"]),
    (
        "linking",
        &[
            "auto_backlinks_enabled",
            "auto_backlinks_scan_limit",
            "auto_backlinks_max_targets",
        ],
    ),
    (
        "daily_notes",
        &[
            "enabled",
            "midnight_scheduler_enabled",
            "namespace",
            "title_template",
            "content_template",
            "default_importance",
        ],
    ),
    (
        "recurrence",
        &[
            "enabled",
            "scheduler_interval_secs",
            "max_instances_per_template",
        ],
    ),
    (
        "encryption",
        &[
            "sealed_mode",
            "enabled",
            "argon2_memory_kib",
            "argon2_iterations",
            "argon2_parallelism",
        ],
    ),
    (
        "llm",
        &[
            "enabled",
            "base_url",
            "model",
            "max_tokens",
            "temperature",
            "timeout_secs",
        ],
    ),
    (
        "email",
        &[
            "enabled",
            "namespace",
            "poll_interval_secs",
            "max_fetch",
            "max_attachment_bytes",
            "mark_seen",
            "imap_host",
            "imap_port",
            "imap_username",
            "imap_folder",
            "imap_starttls",
            "smtp_host",
            "smtp_port",
            "smtp_username",
            "smtp_from",
            "smtp_starttls",
        ],
    ),
    (
        "google_calendar",
        &[
            "enabled",
            "namespace",
            "calendar_id",
            "sync_interval_secs",
            "lookback_days",
            "lookahead_days",
            "max_results",
            "import_events",
            "export_events",
            "client_id",
            "client_secret",
            "refresh_token",
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSection;
    impl ConfigSection for TestSection {
        fn section_name(&self) -> &'static str {
            "test"
        }
        fn validate(&self) -> Result<(), String> {
            Ok(())
        }
        fn known_keys(&self) -> &'static [&'static str] {
            &["key_a", "key_b"]
        }
    }

    struct BadSection;
    impl ConfigSection for BadSection {
        fn section_name(&self) -> &'static str {
            "bad"
        }
        fn validate(&self) -> Result<(), String> {
            Err("invalid value".to_string())
        }
        fn known_keys(&self) -> &'static [&'static str] {
            &["broken"]
        }
    }

    #[test]
    fn register_and_get_section() {
        let mut registry = ConfigRegistry::new();
        registry.register(TestSection);
        assert!(registry.get::<TestSection>().is_some());
        assert!(registry.get::<BadSection>().is_none());
    }

    #[test]
    fn list_sections_returns_sorted_names() {
        let mut registry = ConfigRegistry::new();
        registry.register(BadSection);
        registry.register(TestSection);
        assert_eq!(registry.list_sections(), vec!["bad", "test"]);
    }

    #[test]
    fn list_sections_with_keys_returns_sorted_info() {
        let mut registry = ConfigRegistry::new();
        registry.register(TestSection);
        let sections = registry.list_sections_with_keys();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "test");
        assert_eq!(sections[0].keys, vec!["key_a", "key_b"]);
    }

    #[test]
    fn validate_all_passes_for_valid_sections() {
        let mut registry = ConfigRegistry::new();
        registry.register(TestSection);
        assert!(registry.validate_all().is_ok());
    }

    #[test]
    fn validate_all_collects_errors() {
        let mut registry = ConfigRegistry::new();
        registry.register(TestSection);
        registry.register(BadSection);
        let err = registry.validate_all().unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].0, "bad");
        assert_eq!(err[0].1, "invalid value");
    }

    #[test]
    fn find_unhandled_keys_detects_unknown() {
        let mut registry = ConfigRegistry::new();
        registry.register(TestSection);
        let unhandled = registry.find_unhandled_keys(&[
            "test.key_a".to_string(),
            "test.key_c".to_string(),
            "unknown.field".to_string(),
        ]);
        assert!(unhandled.contains(&"test.key_c".to_string()));
        assert!(unhandled.contains(&"unknown.field".to_string()));
        assert!(!unhandled.contains(&"test.key_a".to_string()));
    }

    #[test]
    fn find_unhandled_keys_empty_for_valid() {
        let mut registry = ConfigRegistry::new();
        registry.register(TestSection);
        let unhandled =
            registry.find_unhandled_keys(&["test.key_a".to_string(), "test.key_b".to_string()]);
        assert!(unhandled.is_empty());
    }

    #[test]
    fn default_creates_empty_registry() {
        let registry = ConfigRegistry::default();
        assert!(registry.list_sections().is_empty());
        assert!(registry.validate_all().is_ok());
    }

    #[test]
    fn builtin_catalog_returns_all_16_sections() {
        let catalog = ConfigRegistry::builtin_section_catalog();
        assert_eq!(catalog.len(), 16);
        let names: Vec<&str> = catalog.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"server"));
        assert!(names.contains(&"storage"));
        assert!(names.contains(&"ai"));
        assert!(names.contains(&"email"));
        assert!(names.contains(&"google_calendar"));
    }

    #[test]
    fn builtin_catalog_is_sorted() {
        let catalog = ConfigRegistry::builtin_section_catalog();
        let names: Vec<&str> = catalog.iter().map(|s| s.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn builtin_catalog_server_has_expected_keys() {
        let catalog = ConfigRegistry::builtin_section_catalog();
        let server = catalog.iter().find(|s| s.name == "server").unwrap();
        assert!(server.keys.contains(&"bind_host".to_string()));
        assert!(server.keys.contains(&"rest_port".to_string()));
        assert!(server.keys.contains(&"cors_allowed_origins".to_string()));
    }

    // --- ConfigScope tests ---

    struct AiSection;
    impl ConfigSection for AiSection {
        fn section_name(&self) -> &'static str {
            "ai"
        }
        fn validate(&self) -> Result<(), String> {
            Ok(())
        }
        fn known_keys(&self) -> &'static [&'static str] {
            &["model"]
        }
    }

    struct StorageSection;
    impl ConfigSection for StorageSection {
        fn section_name(&self) -> &'static str {
            "storage"
        }
        fn validate(&self) -> Result<(), String> {
            Ok(())
        }
        fn known_keys(&self) -> &'static [&'static str] {
            &["data_dir"]
        }
    }

    struct EmailSection;
    impl ConfigSection for EmailSection {
        fn section_name(&self) -> &'static str {
            "email"
        }
        fn validate(&self) -> Result<(), String> {
            Ok(())
        }
        fn known_keys(&self) -> &'static [&'static str] {
            &["enabled"]
        }
    }

    #[test]
    fn scope_get_returns_allowed_section() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        registry.register(StorageSection);
        let scope = registry.scope(&["ai"]);
        assert!(scope.get::<AiSection>().is_some());
    }

    #[test]
    fn scope_get_returns_none_for_disallowed_section() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        registry.register(StorageSection);
        let scope = registry.scope(&["ai"]);
        assert!(scope.get::<StorageSection>().is_none());
    }

    #[test]
    fn scope_list_sections_only_returns_allowed() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        registry.register(StorageSection);
        registry.register(EmailSection);
        let scope = registry.scope(&["ai", "email"]);
        let sections = scope.list_sections();
        assert_eq!(sections, vec!["ai", "email"]);
    }

    #[test]
    fn scope_list_sections_with_keys_only_returns_allowed() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        registry.register(StorageSection);
        let scope = registry.scope(&["storage"]);
        let sections = scope.list_sections_with_keys();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "storage");
        assert_eq!(sections[0].keys, vec!["data_dir"]);
    }

    #[test]
    fn scope_with_empty_allow_list_returns_nothing() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        let scope = registry.scope(&[]);
        assert!(scope.list_sections().is_empty());
        assert!(scope.get::<AiSection>().is_none());
    }

    #[test]
    fn predefined_ai_scope_has_expected_sections() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        registry.register(StorageSection);
        registry.register(EmailSection);
        let scope = registry.ai_scope();
        // ai is registered and in the allow list
        assert!(scope.get::<AiSection>().is_some());
        // storage and email are not in ai scope
        assert!(scope.get::<StorageSection>().is_none());
        assert!(scope.get::<EmailSection>().is_none());
    }

    #[test]
    fn predefined_email_scope_has_expected_sections() {
        let mut registry = ConfigRegistry::new();
        registry.register(AiSection);
        registry.register(EmailSection);
        let scope = registry.email_scope();
        assert!(scope.get::<EmailSection>().is_some());
        assert!(scope.get::<AiSection>().is_none());
    }

    #[test]
    fn predefined_storage_scope_has_expected_sections() {
        let mut registry = ConfigRegistry::new();
        registry.register(StorageSection);
        registry.register(AiSection);
        let scope = registry.storage_scope();
        assert!(scope.get::<StorageSection>().is_some());
        assert!(scope.get::<AiSection>().is_none());
    }

    #[test]
    fn builtin_catalog_scoped_filters_correctly() {
        let scoped = ConfigRegistry::builtin_section_catalog_scoped(&["ai", "email"]);
        let names: Vec<&str> = scoped.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["ai", "email"]);
    }

    #[test]
    fn builtin_catalog_scoped_empty_returns_empty() {
        let scoped = ConfigRegistry::builtin_section_catalog_scoped(&[]);
        assert!(scoped.is_empty());
    }

    #[test]
    fn builtin_catalog_scoped_unknown_section_ignored() {
        let scoped = ConfigRegistry::builtin_section_catalog_scoped(&["nonexistent"]);
        assert!(scoped.is_empty());
    }
}
