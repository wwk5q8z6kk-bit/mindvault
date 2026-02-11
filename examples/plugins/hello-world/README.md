# Hello World Plugin

A minimal MindVault WASM plugin that logs a greeting when a node is ingested.

## Build

```bash
# Install the wasm32 target (one-time)
rustup target add wasm32-unknown-unknown

# Build the plugin
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM binary will be at:
`target/wasm32-unknown-unknown/release/hello_world_plugin.wasm`

## Install

Copy the manifest and WASM binary into the MindVault plugins directory:

```bash
PLUGIN_DIR=~/.mindvault/plugins/hello-world
mkdir -p "$PLUGIN_DIR"
cp manifest.json "$PLUGIN_DIR/"
cp target/wasm32-unknown-unknown/release/hello_world_plugin.wasm "$PLUGIN_DIR/plugin.wasm"
```

Or install via the REST API:

```bash
curl -X POST http://127.0.0.1:9470/api/v1/plugins/install \
  -F manifest=@manifest.json \
  -F wasm=@target/wasm32-unknown-unknown/release/hello_world_plugin.wasm
```

## Test

1. Reload plugins: `curl -X POST http://127.0.0.1:9470/api/v1/plugins/reload`
2. Create a node: `mv store "Hello world test"`
3. Check the chronicle for plugin output:
   `curl http://127.0.0.1:9470/api/v1/agent/chronicle`

## Plugin ABI

MindVault WASM plugins must export:

| Export | Signature | Purpose |
|--------|-----------|---------|
| `memory` | WebAssembly.Memory | Linear memory for host-guest communication |
| `mv_alloc` | `(i32) -> i32` | Allocate bytes, return pointer |
| `mv_plugin_hook` | `(i32, i32) -> i64` | Process hook: (ctx_ptr, ctx_len) -> packed(result_ptr, result_len) |

The host passes a JSON `HookContext` and expects a JSON `HookResult` back.
Return `0i64` for "success, no modifications".
