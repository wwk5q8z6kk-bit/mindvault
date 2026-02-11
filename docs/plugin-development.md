# Plugin Development Guide

## Overview

MindVault plugins are WASM modules that extend the system's functionality through hook points. Plugins run in a sandboxed Wasmtime runtime with explicit permission grants.

## Prerequisites

- Rust toolchain with `wasm32-wasip1` target: `rustup target add wasm32-wasip1`
- MindVault with the `wasm-runtime` feature enabled

## Plugin Structure

A plugin consists of:
1. A `.wasm` binary compiled from any language targeting WASI
2. A `manifest.json` describing the plugin

### Manifest Format

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "A sample MindVault plugin",
  "author": "Your Name",
  "permissions": ["ReadNodes", "WriteNodes"],
  "hooks": ["PostIngest", "OnChange"]
}
```

### Permissions

| Permission | Description |
|------------|-------------|
| `ReadNodes` | Read nodes via `mv_read_node` and `mv_search` |
| `WriteNodes` | Create/update nodes via `mv_write_node` |
| `Network` | Make outbound HTTP requests (WASI) |
| `Filesystem` | Access sandboxed filesystem (WASI) |

### Hook Points

| Hook | Trigger | Use Case |
|------|---------|----------|
| `PreIngest` | Before a node is stored | Validate, transform, enrich |
| `PostIngest` | After a node is stored | Index, notify, log |
| `PreSearch` | Before a search executes | Modify query, add filters |
| `PostSearch` | After search results | Re-rank, filter, augment |
| `OnChange` | When a node is updated | Sync, react, propagate |
| `OnIntent` | When an intent is detected | Custom intent handling |

## WASM ABI Contract

Your WASM module must export three symbols:

| Export | Signature | Description |
|--------|-----------|-------------|
| `memory` | (linear memory) | Shared memory for data exchange |
| `mv_alloc` | `(size: i32) -> i32` | Allocate `size` bytes, return pointer |
| `mv_plugin_hook` | `(ptr: i32, len: i32) -> i64` | Receive JSON `HookContext` at `(ptr, len)`, return packed `(result_ptr << 32 \| result_len)` |

The host serializes `HookContext` as JSON, calls `mv_alloc` to get guest memory, writes the bytes, then calls `mv_plugin_hook`. The return value packs the result pointer in the upper 32 bits and the result length in the lower 32 bits. Return `0` for a default success response.

## Host API

From within `mv_plugin_hook`, plugins can construct `HostRequest` JSON and call back into the host:

### mv_read_node
Read a node by UUID.
```json
{"method": "mv_read_node", "params": {"id": "uuid-here"}}
```

### mv_write_node
Create or update a node.
```json
{"method": "mv_write_node", "params": {"kind": "fact", "content": "...", "tags": ["tag1"]}}
```

### mv_search
Search the vault.
```json
{"method": "mv_search", "params": {"query": "search text", "limit": 10}}
```

### mv_log
Log a message (always permitted).
```json
{"method": "mv_log", "params": {"level": "info", "message": "Plugin executed"}}
```

## Building a Plugin (Rust)

```rust
// lib.rs
use std::alloc::{alloc, Layout};

/// Allocate memory for the host to write into.
#[no_mangle]
pub extern "C" fn mv_alloc(size: i32) -> i32 {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { alloc(layout) as i32 }
}

/// Main hook entry point. Receives JSON HookContext at (ptr, len).
/// Returns packed i64: (result_ptr << 32) | result_len.
#[no_mangle]
pub extern "C" fn mv_plugin_hook(ctx_ptr: i32, ctx_len: i32) -> i64 {
    // Read HookContext JSON from guest memory
    let ctx_bytes = unsafe {
        std::slice::from_raw_parts(ctx_ptr as *const u8, ctx_len as usize)
    };
    let _ctx: serde_json::Value = serde_json::from_slice(ctx_bytes).unwrap();

    // Build a HookResult
    let result = r#"{"success":true}"#;
    let result_bytes = result.as_bytes();
    let result_ptr = mv_alloc(result_bytes.len() as i32);
    unsafe {
        std::ptr::copy_nonoverlapping(
            result_bytes.as_ptr(),
            result_ptr as *mut u8,
            result_bytes.len(),
        );
    }

    // Pack (result_ptr, result_len) into i64
    ((result_ptr as i64) << 32) | (result_bytes.len() as i64)
}
```

Build:
```bash
cargo build --target wasm32-wasip1 --release
```

## Installing

### Via REST API
```bash
curl -X POST http://localhost:9470/api/v1/plugins/install \
  -F "file=@target/wasm32-wasip1/release/my_plugin.wasm" \
  -F "manifest=@manifest.json"
```

### Via Plugins Directory
Place `plugin.wasm` and `manifest.json` in `~/.mindvault/plugins/my-plugin/`.

## Testing

Test your plugin by:
1. Building with `cargo build --target wasm32-wasip1`
2. Installing via the REST API or plugins directory
3. Checking the plugins list: `GET /api/v1/plugins`
4. Triggering the relevant hook (e.g., create a node for PostIngest)
5. Checking chronicle logs for plugin execution entries

## Community Modules

### Manifest Community Fields

When publishing a plugin for others to use, include the optional community fields in your `manifest.json`:

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "Enriches notes with external data",
  "author": "Your Name",
  "permissions": ["ReadNodes", "WriteNodes"],
  "hooks": ["PostIngest"],
  "repository": "https://github.com/user/mv-plugin-enricher",
  "license": "MIT",
  "homepage": "https://example.com/mv-plugin-enricher",
  "checksum": "a1b2c3d4...sha256hex...of-the-wasm-binary",
  "min_mindvault_version": "0.9.0",
  "keywords": ["enrichment", "notes", "external-data"]
}
```

| Field | Purpose |
|-------|---------|
| `repository` | Source code URL for inspection and contribution |
| `license` | SPDX license identifier (e.g. `MIT`, `Apache-2.0`) |
| `homepage` | Project website or documentation |
| `checksum` | SHA-256 hex digest of `plugin.wasm` for integrity verification |
| `min_mindvault_version` | Minimum MindVault version required (semver) |
| `keywords` | Tags for discovery and search |

### Generating a Checksum

```bash
shasum -a 256 target/wasm32-wasip1/release/my_plugin.wasm
# => a1b2c3d4e5f6...  my_plugin.wasm
```

Copy the hex digest into the manifest's `checksum` field. MindVault verifies this on install — a mismatch rejects installation.

### Publishing a Module

There is no central registry yet. To share a module:

1. Distribute the `.wasm` file and `manifest.json` together (e.g. as a GitHub release).
2. Include the SHA-256 checksum in the manifest so consumers can verify integrity automatically.
3. Document required permissions and expected hook usage in your README.
4. Specify `min_mindvault_version` so users know compatibility requirements.
5. Recommend installing in a sandboxed environment first.
6. Tag your repository with `mindvault-plugin` for discoverability.
