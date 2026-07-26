//! MindVault Plugin System
//!
//! Provides a trait-based plugin framework with hook points for extending
//! MindVault's functionality. WASM runtime support is available behind the
//! `wasm-runtime` feature flag.

#[cfg(feature = "wasm-runtime")]
pub(crate) mod abi;
pub(crate) mod hooks;
#[cfg(feature = "wasm-runtime")]
pub(crate) mod host;
pub(crate) mod manager;
pub(crate) mod manifest;
pub(crate) mod registry;
pub(crate) mod runtime;
#[cfg(feature = "wasm-runtime")]
pub(crate) mod sandbox;
#[cfg(feature = "wasm-runtime")]
pub(crate) mod wasm_plugin;

pub use hooks::{HookContext, HookPoint, HookResult};
#[cfg(feature = "wasm-runtime")]
pub use host::{create_dispatch, register_host_functions, HostState};
pub use manager::PluginManager;
pub use manifest::{PluginManifest, PluginPermission};
pub use registry::PluginRegistry;
pub use runtime::{PluginInfo, PluginRuntime};
#[cfg(feature = "wasm-runtime")]
pub use wasm_plugin::WasmPlugin;
