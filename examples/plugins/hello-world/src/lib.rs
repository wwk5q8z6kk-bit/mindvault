//! Hello World — a minimal MindVault WASM plugin.
//!
//! This plugin subscribes to the `post_ingest` hook and logs a greeting
//! whenever a new node is ingested into the vault.
//!
//! ## ABI Contract
//!
//! MindVault plugins must export three items:
//! - `memory`           — WebAssembly linear memory
//! - `mv_alloc(size)`   — allocate `size` bytes, return pointer
//! - `mv_plugin_hook(ptr, len)` — process a hook invocation
//!
//! The host writes a JSON-serialised `HookContext` into guest memory via
//! `mv_alloc`, then calls `mv_plugin_hook`. The plugin returns a packed
//! `i64`: upper 32 bits = result pointer, lower 32 bits = result length.
//! Returning 0 means "no result" (success, no modifications).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ABI: memory allocator
// ---------------------------------------------------------------------------

/// Allocate `size` bytes of memory for the host to write into.
#[no_mangle]
pub extern "C" fn mv_alloc(size: i32) -> i32 {
    if size <= 0 {
        return -1;
    }
    let mut buf = Vec::<u8>::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as i32
}

// ---------------------------------------------------------------------------
// Types matching MindVault's plugin protocol (subset)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct HookContext {
    hook_point: String,
    node: Option<Node>,
    #[allow(dead_code)]
    query: Option<String>,
}

#[derive(Deserialize)]
struct Node {
    #[allow(dead_code)]
    id: String,
    title: Option<String>,
}

#[derive(Serialize)]
struct HookResult {
    success: bool,
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl HookResult {
    fn ok(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            output: Some(msg.into()),
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ABI: hook entry point
// ---------------------------------------------------------------------------

/// Main hook handler called by the MindVault runtime.
///
/// # Safety
/// `ctx_ptr` must point to a valid UTF-8 JSON blob of `ctx_len` bytes
/// that was previously allocated via `mv_alloc`.
#[no_mangle]
pub extern "C" fn mv_plugin_hook(ctx_ptr: i32, ctx_len: i32) -> i64 {
    let slice = unsafe {
        std::slice::from_raw_parts(ctx_ptr as *const u8, ctx_len as usize)
    };

    let ctx: HookContext = match serde_json::from_slice(slice) {
        Ok(c) => c,
        Err(_) => return pack_result(&HookResult {
            success: false,
            output: None,
            error: Some("failed to parse HookContext".into()),
        }),
    };

    let result = handle_hook(&ctx);
    pack_result(&result)
}

fn handle_hook(ctx: &HookContext) -> HookResult {
    match ctx.hook_point.as_str() {
        "post_ingest" => {
            let title = ctx
                .node
                .as_ref()
                .and_then(|n| n.title.as_deref())
                .unwrap_or("<untitled>");
            HookResult::ok(format!("Hello from plugin! Ingested node: {title}"))
        }
        other => HookResult::ok(format!("Hello from plugin! Unhandled hook: {other}")),
    }
}

/// Serialise a `HookResult` into guest memory and return the packed
/// (pointer, length) as an i64.
fn pack_result(result: &HookResult) -> i64 {
    let json = match serde_json::to_vec(result) {
        Ok(v) => v,
        Err(_) => return 0, // fallback: no result
    };
    let len = json.len() as u32;
    let ptr = mv_alloc(len as i32);
    if ptr < 0 {
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(json.as_ptr(), ptr as *mut u8, len as usize);
    }
    ((ptr as u32 as i64) << 32) | (len as i64)
}
