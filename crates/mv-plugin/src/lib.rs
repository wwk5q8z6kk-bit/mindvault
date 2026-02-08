//! MindVault Plugin System
//!
//! Provides a trait-based plugin framework with hook points for extending
//! MindVault's functionality. Concrete WASM runtime support can be added
//! behind a feature flag.

pub mod hooks;
pub mod manifest;
pub mod registry;

pub use hooks::{HookContext, HookPoint, HookResult};
pub use manifest::{PluginManifest, PluginPermission};
pub use registry::PluginRegistry;
