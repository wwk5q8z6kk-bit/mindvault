use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

const ROOT_KEY_LEN: usize = 32;
const DEFAULT_SCOPE: &str = "__global__";

fn root_key_slot() -> &'static RwLock<HashMap<String, [u8; ROOT_KEY_LEN]>> {
    static ROOT_KEY: OnceLock<RwLock<HashMap<String, [u8; ROOT_KEY_LEN]>>> = OnceLock::new();
    ROOT_KEY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn degraded_slot() -> &'static AtomicBool {
    static DEGRADED: OnceLock<AtomicBool> = OnceLock::new();
    DEGRADED.get_or_init(|| AtomicBool::new(false))
}

fn sealed_mode_slot() -> &'static AtomicBool {
    static SEALED_MODE: OnceLock<AtomicBool> = OnceLock::new();
    SEALED_MODE.get_or_init(|| AtomicBool::new(false))
}

fn normalize_scope(scope: &str) -> &str {
    if scope.is_empty() {
        DEFAULT_SCOPE
    } else {
        scope
    }
}

pub fn runtime_scope_from_parent(path: &Path) -> String {
    path.parent().unwrap_or(path).to_string_lossy().into_owned()
}

pub fn set_runtime_root_key_for_scope(
    scope: &str,
    key: [u8; ROOT_KEY_LEN],
    degraded_security: bool,
) {
    if let Ok(mut guard) = root_key_slot().write() {
        guard.insert(normalize_scope(scope).to_string(), key);
    }
    degraded_slot().store(degraded_security, Ordering::SeqCst);
}

pub fn clear_runtime_root_key_for_scope(scope: &str) {
    if let Ok(mut guard) = root_key_slot().write() {
        guard.remove(normalize_scope(scope));
        if guard.is_empty() {
            degraded_slot().store(false, Ordering::SeqCst);
        }
    }
}

pub fn runtime_root_key_for_scope(scope: &str) -> Option<[u8; ROOT_KEY_LEN]> {
    root_key_slot()
        .read()
        .ok()
        .and_then(|guard| guard.get(normalize_scope(scope)).copied())
}

pub fn runtime_has_key_for_scope(scope: &str) -> bool {
    root_key_slot()
        .read()
        .map(|guard| guard.contains_key(normalize_scope(scope)))
        .unwrap_or(false)
}

pub fn set_runtime_root_key(key: [u8; ROOT_KEY_LEN], degraded_security: bool) {
    set_runtime_root_key_for_scope(DEFAULT_SCOPE, key, degraded_security);
}

pub fn clear_runtime_root_key() {
    clear_runtime_root_key_for_scope(DEFAULT_SCOPE);
}

pub fn runtime_root_key() -> Option<[u8; ROOT_KEY_LEN]> {
    runtime_root_key_for_scope(DEFAULT_SCOPE)
}

pub fn runtime_has_key() -> bool {
    runtime_has_key_for_scope(DEFAULT_SCOPE)
}

pub fn runtime_is_degraded_security() -> bool {
    degraded_slot().load(Ordering::SeqCst)
}

pub fn set_sealed_mode_enabled(enabled: bool) {
    sealed_mode_slot().store(enabled, Ordering::SeqCst);
}

pub fn sealed_mode_enabled() -> bool {
    sealed_mode_slot().load(Ordering::SeqCst)
}
