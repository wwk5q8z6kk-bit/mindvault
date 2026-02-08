//! Plugin lifecycle management: install, uninstall, and discovery.

use crate::manifest::PluginManifest;
use std::path::PathBuf;
use uuid::Uuid;

/// Manages plugin files on disk (install, uninstall, discovery).
#[derive(Clone)]
pub struct PluginManager {
	plugins_dir: PathBuf,
}

impl PluginManager {
	pub fn is_valid_plugin_name(name: &str) -> bool {
		let trimmed = name.trim();
		if trimmed.is_empty() || trimmed.len() > 64 {
			return false;
		}
		if trimmed.contains("..") {
			return false;
		}
		if trimmed.contains('/') || trimmed.contains('\\') {
			return false;
		}
		trimmed
			.chars()
			.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
	}

	pub fn new(plugins_dir: PathBuf) -> Self {
		Self { plugins_dir }
	}

	pub fn plugins_dir(&self) -> &PathBuf {
		&self.plugins_dir
	}

	/// Install a plugin by writing its WASM bytes and manifest to disk.
	/// Returns a freshly generated plugin UUID.
	pub fn install(
		&self,
		name: &str,
		wasm_bytes: &[u8],
		manifest: &PluginManifest,
	) -> Result<Uuid, String> {
		if !Self::is_valid_plugin_name(name) {
			return Err("invalid plugin name".into());
		}
		let id = Uuid::now_v7();
		let plugin_dir = self.plugins_dir.join(name);
		std::fs::create_dir_all(&plugin_dir)
			.map_err(|e| format!("failed to create plugin directory: {e}"))?;

		let wasm_path = plugin_dir.join("plugin.wasm");
		std::fs::write(&wasm_path, wasm_bytes)
			.map_err(|e| format!("failed to write WASM module: {e}"))?;

		let manifest_path = plugin_dir.join("manifest.json");
		let manifest_json =
			serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
		std::fs::write(&manifest_path, manifest_json)
			.map_err(|e| format!("failed to write manifest: {e}"))?;

		tracing::info!(plugin = name, uuid = %id, "Plugin installed");
		Ok(id)
	}

	/// Remove a plugin directory from disk.
	pub fn uninstall(&self, name: &str) -> Result<(), String> {
		if !Self::is_valid_plugin_name(name) {
			return Err("invalid plugin name".into());
		}
		let plugin_dir = self.plugins_dir.join(name);
		if plugin_dir.exists() {
			std::fs::remove_dir_all(&plugin_dir)
				.map_err(|e| format!("failed to remove plugin: {e}"))?;
			tracing::info!(plugin = name, "Plugin uninstalled");
		}
		Ok(())
	}

	/// Scan the plugins directory and return discovered manifest paths.
	pub fn discover(&self) -> Result<Vec<(String, PathBuf, PluginManifest)>, String> {
		let mut found = Vec::new();
		if !self.plugins_dir.exists() {
			return Ok(found);
		}
		for entry in std::fs::read_dir(&self.plugins_dir).map_err(|e| e.to_string())? {
			let entry = entry.map_err(|e| e.to_string())?;
			if entry.path().is_dir() {
				let manifest_path = entry.path().join("manifest.json");
				if manifest_path.exists() {
					let content =
						std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
					let manifest: PluginManifest =
						serde_json::from_str(&content).map_err(|e| e.to_string())?;
					let name = entry
						.file_name()
						.to_string_lossy()
						.to_string();
					if !Self::is_valid_plugin_name(&name) {
						continue;
					}
					let wasm_path = entry.path().join("plugin.wasm");
					found.push((name, wasm_path, manifest));
				}
			}
		}
		Ok(found)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::manifest::PluginManifest;

	#[test]
	fn install_and_discover() {
		let tmp = std::env::temp_dir().join(format!("mv_plugin_test_{}", Uuid::now_v7()));
		let mgr = PluginManager::new(tmp.clone());

		let manifest = PluginManifest::new("test-plugin", "Test Plugin", "0.1.0");
		let wasm_bytes = b"\0asm fake module";

		let id = mgr.install("test-plugin", wasm_bytes, &manifest).unwrap();
		assert!(!id.is_nil());

		let discovered = mgr.discover().unwrap();
		assert_eq!(discovered.len(), 1);
		assert_eq!(discovered[0].0, "test-plugin");
		assert_eq!(discovered[0].2.name, "Test Plugin");

		mgr.uninstall("test-plugin").unwrap();
		let discovered = mgr.discover().unwrap();
		assert_eq!(discovered.len(), 0);

		// Clean up
		let _ = std::fs::remove_dir_all(&tmp);
	}

	#[test]
	fn uninstall_nonexistent_is_ok() {
		let tmp = std::env::temp_dir().join(format!("mv_plugin_test_ne_{}", Uuid::now_v7()));
		let mgr = PluginManager::new(tmp.clone());
		assert!(mgr.uninstall("no-such-plugin").is_ok());
		let _ = std::fs::remove_dir_all(&tmp);
	}
}
