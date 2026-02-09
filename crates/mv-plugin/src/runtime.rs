//! Plugin runtime: loads, manages, and dispatches WASM plugins.
//!
//! Provides a [`PluginRuntime`] that scans a plugins directory, compiles WASM
//! modules, and routes hook events to matching plugins.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::manager::PluginManager;
use crate::manifest::PluginManifest;

/// Information about a loaded plugin within the runtime.
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub loaded_at: DateTime<Utc>,
    pub hook_count: AtomicU64,
    pub wasm_size_bytes: u64,
}

/// Serializable summary of a loaded plugin.
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub hooks: Vec<String>,
    pub loaded_at: String,
    pub invocation_count: u64,
    pub wasm_size_bytes: u64,
    pub status: String,
}

/// Plugin runtime that manages WASM plugin lifecycle.
///
/// Handles discovery, loading, unloading, and hook dispatch for plugins stored
/// in a plugins directory.
pub struct PluginRuntime {
    manager: PluginManager,
    plugins: HashMap<String, Arc<LoadedPlugin>>,
    #[cfg(feature = "wasm-runtime")]
    engine: Option<wasmtime::Engine>,
    #[cfg(feature = "wasm-runtime")]
    wasm_plugins: HashMap<String, crate::wasm_plugin::WasmPlugin>,
}

impl PluginRuntime {
    /// Create a new plugin runtime backed by the given `PluginManager`.
    pub fn new(manager: PluginManager) -> Self {
        #[cfg(feature = "wasm-runtime")]
        let engine = crate::wasm_plugin::WasmPlugin::create_engine().ok();

        Self {
            manager,
            plugins: HashMap::new(),
            #[cfg(feature = "wasm-runtime")]
            engine,
            #[cfg(feature = "wasm-runtime")]
            wasm_plugins: HashMap::new(),
        }
    }

    /// Scan the plugins directory and load all discovered plugins.
    ///
    /// Returns the number of successfully loaded plugins.
    pub fn scan_and_load(&mut self) -> Result<usize, String> {
        let discovered = self.manager.discover()?;
        let mut loaded = 0;

        for (name, wasm_path, manifest) in discovered {
            match self.load_plugin(&name, &wasm_path, manifest) {
                Ok(()) => {
                    loaded += 1;
                    tracing::info!(plugin = %name, "plugin loaded");
                }
                Err(e) => {
                    tracing::warn!(plugin = %name, error = %e, "failed to load plugin");
                }
            }
        }

        Ok(loaded)
    }

    /// Load a single plugin by name, WASM path, and manifest.
    fn load_plugin(
        &mut self,
        name: &str,
        wasm_path: &std::path::Path,
        manifest: PluginManifest,
    ) -> Result<(), String> {
        let wasm_size = std::fs::metadata(wasm_path)
            .map(|m| m.len())
            .unwrap_or(0);

        #[cfg(feature = "wasm-runtime")]
        if let Some(ref engine) = self.engine {
            if wasm_path.exists() {
                let wasm_plugin =
                    crate::wasm_plugin::WasmPlugin::load(engine, wasm_path, manifest.clone())?;
                self.wasm_plugins.insert(name.to_string(), wasm_plugin);
            }
        }

        let loaded = Arc::new(LoadedPlugin {
            manifest,
            loaded_at: Utc::now(),
            hook_count: AtomicU64::new(0),
            wasm_size_bytes: wasm_size,
        });

        self.plugins.insert(name.to_string(), loaded);
        Ok(())
    }

    /// Unload a plugin by name.
    pub fn unload_plugin(&mut self, name: &str) -> bool {
        let removed = self.plugins.remove(name).is_some();

        #[cfg(feature = "wasm-runtime")]
        {
            self.wasm_plugins.remove(name);
        }

        if removed {
            tracing::info!(plugin = %name, "plugin unloaded");
        }
        removed
    }

    /// Reload a single plugin from disk.
    pub fn reload_plugin(&mut self, name: &str) -> Result<(), String> {
        // First discover to find the plugin
        let discovered = self.manager.discover()?;
        let found = discovered
            .into_iter()
            .find(|(n, _, _)| n == name)
            .ok_or_else(|| format!("plugin '{name}' not found on disk"))?;

        // Unload existing
        self.unload_plugin(name);

        // Load fresh
        self.load_plugin(&found.0, &found.1, found.2)
    }

    /// Execute a hook on all loaded WASM plugins that subscribe to it.
    #[cfg(feature = "wasm-runtime")]
    pub fn execute_hook(&self, ctx: &HookContext) -> Vec<(String, Result<HookResult, String>)> {
        let hook_name = ctx.hook_point.to_string();
        let mut results = Vec::new();

        let engine = match &self.engine {
            Some(e) => e,
            None => return results,
        };

        for (name, wasm_plugin) in &self.wasm_plugins {
            if !wasm_plugin.manifest().hooks.contains(&hook_name) {
                continue;
            }

            if let Some(loaded) = self.plugins.get(name) {
                loaded.hook_count.fetch_add(1, Ordering::Relaxed);
            }

            let result = wasm_plugin.execute_hook(engine, ctx);
            results.push((name.clone(), result));
        }

        results
    }

    /// Execute a hook with host state on all loaded WASM plugins that subscribe to it.
    #[cfg(feature = "wasm-runtime")]
    pub fn execute_hook_with_host(
        &self,
        ctx: &HookContext,
        host_state_factory: impl Fn(&PluginManifest) -> crate::host::HostState,
    ) -> Vec<(String, Result<HookResult, String>)> {
        let hook_name = ctx.hook_point.to_string();
        let mut results = Vec::new();

        let engine = match &self.engine {
            Some(e) => e,
            None => return results,
        };

        for (name, wasm_plugin) in &self.wasm_plugins {
            if !wasm_plugin.manifest().hooks.contains(&hook_name) {
                continue;
            }

            if let Some(loaded) = self.plugins.get(name) {
                loaded.hook_count.fetch_add(1, Ordering::Relaxed);
            }

            let host_state = host_state_factory(wasm_plugin.manifest());
            let result = wasm_plugin.execute_hook_with_host(engine, ctx, host_state);
            results.push((name.clone(), result));
        }

        results
    }

    /// List all loaded plugins with their info.
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|(name, loaded)| PluginInfo {
                id: loaded.manifest.id.clone(),
                name: loaded.manifest.name.clone(),
                version: loaded.manifest.version.clone(),
                description: loaded.manifest.description.clone(),
                author: loaded.manifest.author.clone(),
                hooks: loaded.manifest.hooks.clone(),
                loaded_at: loaded.loaded_at.to_rfc3339(),
                invocation_count: loaded.hook_count.load(Ordering::Relaxed),
                wasm_size_bytes: loaded.wasm_size_bytes,
                status: if self.is_wasm_loaded(name) {
                    "loaded".to_string()
                } else {
                    "registered".to_string()
                },
            })
            .collect()
    }

    /// Get info about a specific plugin.
    pub fn get_plugin(&self, name: &str) -> Option<PluginInfo> {
        self.plugins.get(name).map(|loaded| PluginInfo {
            id: loaded.manifest.id.clone(),
            name: loaded.manifest.name.clone(),
            version: loaded.manifest.version.clone(),
            description: loaded.manifest.description.clone(),
            author: loaded.manifest.author.clone(),
            hooks: loaded.manifest.hooks.clone(),
            loaded_at: loaded.loaded_at.to_rfc3339(),
            invocation_count: loaded.hook_count.load(Ordering::Relaxed),
            wasm_size_bytes: loaded.wasm_size_bytes,
            status: if self.is_wasm_loaded(name) {
                "loaded".to_string()
            } else {
                "registered".to_string()
            },
        })
    }

    /// Get the hooks a specific plugin subscribes to.
    pub fn get_plugin_hooks(&self, name: &str) -> Option<Vec<String>> {
        self.plugins.get(name).map(|p| p.manifest.hooks.clone())
    }

    /// Total number of loaded plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Check if a WASM module is loaded for a given plugin name.
    fn is_wasm_loaded(&self, _name: &str) -> bool {
        #[cfg(feature = "wasm-runtime")]
        {
            self.wasm_plugins.contains_key(_name)
        }
        #[cfg(not(feature = "wasm-runtime"))]
        {
            false
        }
    }

    /// Access the underlying plugin manager.
    pub fn manager(&self) -> &PluginManager {
        &self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_manager(dir: &std::path::Path) -> PluginManager {
        PluginManager::new(dir.to_path_buf())
    }

    #[test]
    fn new_runtime_is_empty() {
        let tmp = std::env::temp_dir().join(format!("mv_rt_test_{}", uuid::Uuid::now_v7()));
        let mgr = test_manager(&tmp);
        let runtime = PluginRuntime::new(mgr);
        assert_eq!(runtime.plugin_count(), 0);
        assert!(runtime.list_plugins().is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_empty_directory() {
        let tmp = std::env::temp_dir().join(format!("mv_rt_scan_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&tmp).unwrap();
        let mgr = test_manager(&tmp);
        let mut runtime = PluginRuntime::new(mgr);
        let loaded = runtime.scan_and_load().unwrap();
        assert_eq!(loaded, 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unload_nonexistent_returns_false() {
        let tmp = std::env::temp_dir().join(format!("mv_rt_unload_{}", uuid::Uuid::now_v7()));
        let mgr = test_manager(&tmp);
        let mut runtime = PluginRuntime::new(mgr);
        assert!(!runtime.unload_plugin("no-such-plugin"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn get_plugin_not_found() {
        let tmp = std::env::temp_dir().join(format!("mv_rt_get_{}", uuid::Uuid::now_v7()));
        let mgr = test_manager(&tmp);
        let runtime = PluginRuntime::new(mgr);
        assert!(runtime.get_plugin("missing").is_none());
        assert!(runtime.get_plugin_hooks("missing").is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
