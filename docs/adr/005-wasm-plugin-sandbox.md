# ADR-005: WASM Plugin Sandbox (Wasmtime)

## Status
Accepted

## Context
MindVault needs a plugin system that allows third-party extensions while maintaining security. Plugins must not be able to access arbitrary filesystem, network, or vault data without permission.

## Decision
Use **Wasmtime** (Bytecode Alliance) as the WASM runtime with **WASI** for sandboxed I/O.

Architecture:
- Plugins are `.wasm` files with a JSON manifest declaring permissions and hook points.
- Host functions (`mv_read_node`, `mv_write_node`, `mv_search`, `mv_log`) are gated by a `PermissionGate`.
- Plugins declare required permissions: `ReadNodes`, `WriteNodes`, `Network`, `Filesystem`.
- The `PluginManager` handles lifecycle: install, uninstall, reload, load_all.
- Fuel budgeting limits CPU consumption per plugin execution.
- The WASM runtime is behind a `wasm-runtime` feature flag.

## Consequences
- **Positive:** Strong sandboxing — plugins cannot escape the WASM boundary.
- **Positive:** Wasmtime is security-audited and production-grade.
- **Positive:** Language-agnostic — plugins can be written in Rust, C, Go, AssemblyScript, etc.
- **Negative:** WASM adds serialization overhead for host function calls.
- **Negative:** Debugging WASM plugins is harder than native code.
- **Negative:** The component model is still evolving.
