//! MindVault Plugin System
//!
//! Provides a trait-based plugin framework with hook points for extending
//! MindVault's functionality. WASM runtime support is available behind the
//! `wasm-runtime` feature flag.

pub mod abi;
#[cfg(feature = "wasm-runtime")]
pub mod host;
pub mod hooks;
pub mod manager;
pub mod manifest;
pub mod registry;
pub mod sandbox;
#[cfg(feature = "wasm-runtime")]
pub mod wasm_plugin;

pub use hooks::{HookContext, HookPoint, HookResult};
pub use manager::PluginManager;
pub use manifest::{PluginManifest, PluginPermission};
pub use registry::PluginRegistry;
#[cfg(feature = "wasm-runtime")]
pub use host::{HostState, register_host_functions, create_dispatch};
#[cfg(feature = "wasm-runtime")]
pub use wasm_plugin::WasmPlugin;
