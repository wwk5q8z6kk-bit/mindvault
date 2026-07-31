use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

const ROOT_KEY_LEN: usize = 32;
const DEFAULT_SCOPE: &str = "__global__";

/// Per-scope sealed-runtime state.
///
/// `degraded_security` lives HERE, beside the key it describes, rather than in a
/// process-global flag. Previously the keys were per-scope but the flag was a
/// single `AtomicBool`, so the last caller of `set_runtime_root_key_for_scope`
/// decided the security posture for every open vault, and cleanup only restored
/// it once the map emptied. That is user-visible: `runtime_is_degraded_security`
/// is surfaced through `mv-server/src/rest/keychain.rs` and
/// `mv-cli/src/commands/keychain.rs`.
#[derive(Clone, Copy)]
struct ScopeState {
    key: [u8; ROOT_KEY_LEN],
    degraded: bool,
}

fn root_key_slot() -> &'static RwLock<HashMap<String, ScopeState>> {
    static ROOT_KEY: OnceLock<RwLock<HashMap<String, ScopeState>>> = OnceLock::new();
    ROOT_KEY.get_or_init(|| RwLock::new(HashMap::new()))
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
        guard.insert(
            normalize_scope(scope).to_string(),
            ScopeState { key, degraded: degraded_security },
        );
    }
}

pub fn clear_runtime_root_key_for_scope(scope: &str) {
    if let Ok(mut guard) = root_key_slot().write() {
        guard.remove(normalize_scope(scope));
    }
}

pub fn runtime_root_key_for_scope(scope: &str) -> Option<[u8; ROOT_KEY_LEN]> {
    root_key_slot()
        .read()
        .ok()
        .and_then(|guard| guard.get(normalize_scope(scope)).map(|s| s.key))
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

/// Degraded-security posture for one scope. Prefer this over the unscoped form.
pub fn runtime_is_degraded_security_for_scope(scope: &str) -> bool {
    root_key_slot()
        .read()
        .ok()
        .and_then(|guard| guard.get(normalize_scope(scope)).map(|s| s.degraded))
        .unwrap_or(false)
}

/// Unscoped view, kept for callers that have no scope in hand.
///
/// Defined as "ANY open scope is degraded". This direction is deliberate: for a
/// security posture, over-reporting degraded is safe and under-reporting is not.
/// A caller that can name its scope should use
/// [`runtime_is_degraded_security_for_scope`] and get an exact answer.
pub fn runtime_is_degraded_security() -> bool {
    root_key_slot()
        .read()
        .map(|guard| guard.values().any(|s| s.degraded))
        .unwrap_or(false)
}

pub fn set_sealed_mode_enabled(enabled: bool) {
    sealed_mode_slot().store(enabled, Ordering::SeqCst);
}

pub fn sealed_mode_enabled() -> bool {
    sealed_mode_slot().load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard: the degraded-security flag must be per-scope.
    ///
    /// It used to be a single process-global `AtomicBool` while root keys were
    /// per-scope, so the last caller of `set_runtime_root_key_for_scope` decided
    /// the posture for every open vault. That reached users:
    /// `runtime_is_degraded_security` is surfaced via
    /// `mv-server/src/rest/keychain.rs` and `mv-cli/src/commands/keychain.rs`.
    ///
    /// It also caused the intermittent failure of
    /// `mv-index::tantivy_index::tests::sealed_tantivy_files_do_not_store_plaintext_payload`
    /// (measured flake rate 1/12 serial runs, 2026-07-27) and all 15 parallel
    /// failures in `mv-server`'s `api_integration` suite.
    #[test]
    fn degraded_flag_is_isolated_per_scope() {
        let secure = "scope-secure";
        let degraded = "scope-degraded";

        set_runtime_root_key_for_scope(secure, [1u8; ROOT_KEY_LEN], false);
        set_runtime_root_key_for_scope(degraded, [2u8; ROOT_KEY_LEN], true);

        assert!(
            !runtime_is_degraded_security_for_scope(secure),
            "a scope opened with degraded_security=false must never report degraded \
             because an unrelated scope opened in degraded mode"
        );
        assert!(
            runtime_is_degraded_security_for_scope(degraded),
            "the degraded scope must report its own posture"
        );

        clear_runtime_root_key_for_scope(secure);
        clear_runtime_root_key_for_scope(degraded);
    }

    /// Clearing one scope must not disturb a surviving scope's posture.
    #[test]
    fn clearing_one_scope_leaves_survivors_intact() {
        let survivor = "scope-survivor";
        let transient = "scope-transient";

        set_runtime_root_key_for_scope(survivor, [3u8; ROOT_KEY_LEN], false);
        set_runtime_root_key_for_scope(transient, [4u8; ROOT_KEY_LEN], true);
        clear_runtime_root_key_for_scope(transient);

        assert!(
            !runtime_is_degraded_security_for_scope(survivor),
            "the survivor never opted into degraded security"
        );
        assert!(
            !runtime_is_degraded_security_for_scope(transient),
            "a cleared scope has no posture to report"
        );

        clear_runtime_root_key_for_scope(survivor);
    }

    /// The unscoped view is deliberately conservative: ANY degraded scope makes
    /// it true. Over-reporting degraded is safe; under-reporting is not.
    #[test]
    fn unscoped_view_is_fail_safe() {
        let a = "scope-fs-a";
        let b = "scope-fs-b";

        set_runtime_root_key_for_scope(a, [5u8; ROOT_KEY_LEN], false);
        set_runtime_root_key_for_scope(b, [6u8; ROOT_KEY_LEN], true);
        assert!(
            runtime_is_degraded_security(),
            "unscoped view must report degraded while any scope is degraded"
        );

        clear_runtime_root_key_for_scope(b);
        assert!(
            !runtime_is_degraded_security_for_scope(a),
            "scope a was never degraded"
        );

        clear_runtime_root_key_for_scope(a);
    }
}
