//! Plugin manifest definition.

use serde::{Deserialize, Serialize};

/// Plugin manifest (loaded from plugin.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub permissions: Vec<PluginPermission>,
    pub hooks: Vec<String>,
    pub entry_point: Option<String>,
}

impl PluginManifest {
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: None,
            author: None,
            permissions: Vec::new(),
            hooks: Vec::new(),
            entry_point: None,
        }
    }
}

/// Permissions a plugin may request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    ReadNodes,
    WriteNodes,
    Search,
    GraphAccess,
    NetworkAccess,
    FileSystemRead,
    FileSystemWrite,
}
