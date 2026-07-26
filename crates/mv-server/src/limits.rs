use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use mv_core::{NodeStore, QueryFilters};
use mv_engine::engine::MindVaultEngine;

use crate::auth::{AuthContext, AuthRole};

const ENV_RATE_LIMIT_REQUESTS: &str = "MINDVAULT_RATE_LIMIT_REQUESTS";
const ENV_RATE_LIMIT_WINDOW_SECS: &str = "MINDVAULT_RATE_LIMIT_WINDOW_SECS";
const ENV_NAMESPACE_NODE_QUOTA: &str = "MINDVAULT_NAMESPACE_NODE_QUOTA";

const DEFAULT_RATE_LIMIT_REQUESTS: usize = 120;
const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;

#[derive(Debug, Clone, Copy)]
struct RateLimitConfig {
    enabled: bool,
    max_requests: usize,
    window: Duration,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        let max_requests =
            read_env_usize(ENV_RATE_LIMIT_REQUESTS).unwrap_or(DEFAULT_RATE_LIMIT_REQUESTS);
        let window_secs = read_env_u64(ENV_RATE_LIMIT_WINDOW_SECS)
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RATE_LIMIT_WINDOW_SECS);
        let enabled = max_requests > 0;

        Self {
            enabled,
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitExceeded {
    pub retry_after_secs: u64,
    pub max_requests: usize,
    pub window_secs: u64,
}

/// Returned on successful rate-limit check with remaining capacity info.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitStatus {
    pub limit: usize,
    pub remaining: usize,
    pub reset_secs: u64,
}

#[derive(Debug)]
pub enum NamespaceQuotaError {
    Exceeded {
        namespace: String,
        quota: usize,
        count: usize,
    },
    Backend(String),
}

struct RequestRateLimiter {
    config: RateLimitConfig,
    buckets: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RequestRateLimiter {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn check(&self, key: &str) -> Result<RateLimitStatus, RateLimitExceeded> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: &str, now: Instant) -> Result<RateLimitStatus, RateLimitExceeded> {
        self.check_internal(key, now, true)
    }

    fn check_without_record(&self, key: &str) -> Result<RateLimitStatus, RateLimitExceeded> {
        self.check_at_without_record(key, Instant::now())
    }

    fn check_at_without_record(
        &self,
        key: &str,
        now: Instant,
    ) -> Result<RateLimitStatus, RateLimitExceeded> {
        self.check_internal(key, now, false)
    }

    fn check_internal(
        &self,
        key: &str,
        now: Instant,
        record: bool,
    ) -> Result<RateLimitStatus, RateLimitExceeded> {
        if !self.config.enabled {
            return Ok(RateLimitStatus {
                limit: 0,
                remaining: 0,
                reset_secs: 0,
            });
        }

        let mut buckets = match self.buckets.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let queue = buckets.entry(key.to_string()).or_default();
        while let Some(front) = queue.front() {
            if now.duration_since(*front) >= self.config.window {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() >= self.config.max_requests {
            let retry_after_secs = queue
                .front()
                .map(|front| {
                    self.config
                        .window
                        .saturating_sub(now.duration_since(*front))
                        .as_secs()
                        .max(1)
                })
                .unwrap_or(1);

            return Err(RateLimitExceeded {
                retry_after_secs,
                max_requests: self.config.max_requests,
                window_secs: self.config.window.as_secs(),
            });
        }

        let reset_secs = queue
            .front()
            .map(|front| {
                self.config
                    .window
                    .saturating_sub(now.duration_since(*front))
                    .as_secs()
            })
            .unwrap_or(self.config.window.as_secs());

        if record {
            queue.push_back(now);
        }
        Ok(RateLimitStatus {
            limit: self.config.max_requests,
            remaining: self.config.max_requests.saturating_sub(queue.len()),
            reset_secs,
        })
    }
}

static RATE_LIMITER: OnceLock<RequestRateLimiter> = OnceLock::new();
static PROXY_RATE_LIMITER: OnceLock<RequestRateLimiter> = OnceLock::new();

pub fn enforce_rate_limit(auth: &AuthContext) -> Result<RateLimitStatus, RateLimitExceeded> {
    let key = rate_limit_key(auth);
    RATE_LIMITER
        .get_or_init(|| RequestRateLimiter::new(RateLimitConfig::from_env()))
        .check(&key)
}

const ENV_AI_RATE_LIMIT_REQUESTS: &str = "MINDVAULT_AI_RATE_LIMIT_REQUESTS";
const ENV_AI_RATE_LIMIT_WINDOW_SECS: &str = "MINDVAULT_AI_RATE_LIMIT_WINDOW_SECS";
const DEFAULT_AI_RATE_LIMIT_REQUESTS: usize = 60;
const DEFAULT_AI_RATE_LIMIT_WINDOW_SECS: u64 = 60;

static AI_RATE_LIMITER: OnceLock<RequestRateLimiter> = OnceLock::new();

/// Per-subject throttle for expensive AI endpoints (chat + assist).
///
/// Local system admin (auth disabled / no subject) is exempt; the global
/// request rate limiter still applies via middleware.
pub fn enforce_ai_rate_limit(auth: &AuthContext) -> Result<RateLimitStatus, RateLimitExceeded> {
    if auth.subject.is_none() && auth.is_admin() {
        return Ok(RateLimitStatus {
            limit: 0,
            remaining: 0,
            reset_secs: 0,
        });
    }

    let key = format!("ai|{}", rate_limit_key(auth));
    AI_RATE_LIMITER
        .get_or_init(|| {
            let max_requests = read_env_usize(ENV_AI_RATE_LIMIT_REQUESTS)
                .unwrap_or(DEFAULT_AI_RATE_LIMIT_REQUESTS);
            let window_secs = read_env_u64(ENV_AI_RATE_LIMIT_WINDOW_SECS)
                .filter(|value| *value > 0)
                .unwrap_or(DEFAULT_AI_RATE_LIMIT_WINDOW_SECS);
            RequestRateLimiter::new(RateLimitConfig {
                enabled: max_requests > 0,
                max_requests,
                window: Duration::from_secs(window_secs),
            })
        })
        .check(&key)
}

const ENV_PROXY_RATE_LIMIT_REQUESTS: &str = "MINDVAULT_PROXY_RATE_LIMIT_REQUESTS";
const ENV_PROXY_RATE_LIMIT_WINDOW_SECS: &str = "MINDVAULT_PROXY_RATE_LIMIT_WINDOW_SECS";

const DEFAULT_PROXY_RATE_LIMIT_REQUESTS: usize = 60;
const DEFAULT_PROXY_RATE_LIMIT_WINDOW_SECS: u64 = 3600;

/// Enforce per-consumer per-secret rate limiting for proxy requests.
///
/// Key format: `proxy:{consumer}:{secret_ref}` — so each consumer gets its own
/// rate limit bucket for each secret they access via the proxy.
pub fn enforce_proxy_rate_limit(consumer: &str, secret_ref: &str) -> Result<(), RateLimitExceeded> {
    let key = format!("proxy:{consumer}:{secret_ref}");
    PROXY_RATE_LIMITER
        .get_or_init(|| {
            let max_requests = read_env_usize(ENV_PROXY_RATE_LIMIT_REQUESTS)
                .unwrap_or(DEFAULT_PROXY_RATE_LIMIT_REQUESTS);
            let window_secs = read_env_u64(ENV_PROXY_RATE_LIMIT_WINDOW_SECS)
                .filter(|v| *v > 0)
                .unwrap_or(DEFAULT_PROXY_RATE_LIMIT_WINDOW_SECS);
            RequestRateLimiter::new(RateLimitConfig {
                enabled: max_requests > 0,
                max_requests,
                window: Duration::from_secs(window_secs),
            })
        })
        .check(&key)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Keychain credential read rate limiting
// ---------------------------------------------------------------------------

const ENV_KEYCHAIN_READ_RATE_LIMIT: &str = "MINDVAULT_KEYCHAIN_READ_RATE_LIMIT";
const ENV_KEYCHAIN_READ_RATE_LIMIT_WINDOW: &str = "MINDVAULT_KEYCHAIN_READ_RATE_LIMIT_WINDOW";

const DEFAULT_KEYCHAIN_READ_RATE_LIMIT: usize = 30;
const DEFAULT_KEYCHAIN_READ_RATE_LIMIT_WINDOW: u64 = 60;

static KEYCHAIN_READ_RATE_LIMITER: OnceLock<RequestRateLimiter> = OnceLock::new();

/// Enforce per-subject per-credential rate limiting for keychain credential reads.
///
/// Key format: `kc-read:{subject}:{credential_id}` — each subject gets a separate
/// rate limit bucket for each credential they decrypt.
pub fn enforce_keychain_read_rate_limit(
    subject: &str,
    credential_id: &str,
) -> Result<(), RateLimitExceeded> {
    let key = format!("kc-read:{subject}:{credential_id}");
    KEYCHAIN_READ_RATE_LIMITER
        .get_or_init(|| {
            let max_requests = read_env_usize(ENV_KEYCHAIN_READ_RATE_LIMIT)
                .unwrap_or(DEFAULT_KEYCHAIN_READ_RATE_LIMIT);
            let window_secs = read_env_u64(ENV_KEYCHAIN_READ_RATE_LIMIT_WINDOW)
                .filter(|v| *v > 0)
                .unwrap_or(DEFAULT_KEYCHAIN_READ_RATE_LIMIT_WINDOW);
            RequestRateLimiter::new(RateLimitConfig {
                enabled: max_requests > 0,
                max_requests,
                window: Duration::from_secs(window_secs),
            })
        })
        .check(&key)
        .map(|_| ())
}

const ENV_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT: &str = "MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT";
const ENV_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW: &str =
    "MINDVAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW";

const DEFAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT: usize = 5;
const DEFAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW: u64 = 300;

static KEYCHAIN_UNSEAL_FAILURE_RATE_LIMITER: OnceLock<RequestRateLimiter> = OnceLock::new();

/// Check if a subject is currently blocked from unseal attempts due to too many recent failures.
pub fn enforce_keychain_unseal_failure_backoff(subject: &str) -> Result<(), RateLimitExceeded> {
    let key = format!("kc-unseal:{subject}");
    KEYCHAIN_UNSEAL_FAILURE_RATE_LIMITER
        .get_or_init(|| {
            let max_requests = read_env_usize(ENV_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT)
                .unwrap_or(DEFAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT);
            let window_secs = read_env_u64(ENV_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW)
                .filter(|v| *v > 0)
                .unwrap_or(DEFAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW);
            RequestRateLimiter::new(RateLimitConfig {
                enabled: max_requests > 0,
                max_requests,
                window: Duration::from_secs(window_secs),
            })
        })
        .check_without_record(&key)
        .map(|_| ())
}

/// Record a failed unseal attempt for a subject.
pub fn record_keychain_unseal_failure(subject: &str) -> Result<(), RateLimitExceeded> {
    let key = format!("kc-unseal:{subject}");
    KEYCHAIN_UNSEAL_FAILURE_RATE_LIMITER
        .get_or_init(|| {
            let max_requests = read_env_usize(ENV_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT)
                .unwrap_or(DEFAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT);
            let window_secs = read_env_u64(ENV_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW)
                .filter(|v| *v > 0)
                .unwrap_or(DEFAULT_KEYCHAIN_UNSEAL_FAILURE_RATE_LIMIT_WINDOW);
            RequestRateLimiter::new(RateLimitConfig {
                enabled: max_requests > 0,
                max_requests,
                window: Duration::from_secs(window_secs),
            })
        })
        .check(&key)
        .map(|_| ())
}

pub async fn enforce_namespace_quota(
    engine: &MindVaultEngine,
    namespace: &str,
) -> Result<(), NamespaceQuotaError> {
    let quota = namespace_node_quota();
    let Some(quota) = quota else {
        return Ok(());
    };

    let filters = QueryFilters {
        namespace: Some(namespace.to_string()),
        ..Default::default()
    };
    let count = engine
        .store
        .nodes
        .count(&filters)
        .await
        .map_err(|err| NamespaceQuotaError::Backend(err.to_string()))?;

    if count >= quota {
        return Err(NamespaceQuotaError::Exceeded {
            namespace: namespace.to_string(),
            quota,
            count,
        });
    }

    Ok(())
}

fn namespace_node_quota() -> Option<usize> {
    // Read on each call so tests/ops can change MINDVAULT_NAMESPACE_NODE_QUOTA
    // without process restart (quota checks are infrequent write-path only).
    read_env_usize(ENV_NAMESPACE_NODE_QUOTA)
}

pub(crate) fn rate_limit_key(auth: &AuthContext) -> String {
    let subject = auth.subject.as_deref().unwrap_or("system");
    let namespace = auth.namespace.as_deref().unwrap_or("*");
    format!("{subject}|{}|{namespace}", role_as_str(auth.role))
}

fn role_as_str(role: AuthRole) -> &'static str {
    match role {
        AuthRole::Admin => "admin",
        AuthRole::Write => "write",
        AuthRole::Read => "read",
    }
}

fn read_env_usize(key: &str) -> Option<usize> {
    let raw = std::env::var(key).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    raw.parse::<usize>().ok()
}

fn read_env_u64(key: &str) -> Option<u64> {
    let raw = std::env::var(key).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    raw.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("env lock")
    }

    #[test]
    fn rate_limiter_blocks_after_capacity() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            enabled: true,
            max_requests: 2,
            window: Duration::from_secs(60),
        });
        let now = Instant::now();

        assert!(limiter.check_at("k", now).is_ok());
        assert!(limiter.check_at("k", now + Duration::from_secs(1)).is_ok());
        assert!(limiter.check_at("k", now + Duration::from_secs(2)).is_err());
    }

    #[test]
    fn rate_limiter_resets_after_window() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            enabled: true,
            max_requests: 1,
            window: Duration::from_secs(10),
        });
        let now = Instant::now();

        assert!(limiter.check_at("k", now).is_ok());
        assert!(limiter.check_at("k", now + Duration::from_secs(1)).is_err());
        assert!(limiter.check_at("k", now + Duration::from_secs(11)).is_ok());
    }

    #[test]
    fn proxy_rate_limiter_separate_keys() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            enabled: true,
            max_requests: 1,
            window: Duration::from_secs(3600),
        });
        let now = Instant::now();

        // Different consumers get separate buckets
        assert!(limiter.check_at("proxy:alice:KEY", now).is_ok());
        assert!(limiter.check_at("proxy:bob:KEY", now).is_ok());

        // Same consumer, same key is blocked
        assert!(limiter
            .check_at("proxy:alice:KEY", now + Duration::from_secs(1))
            .is_err());
    }

    #[test]
    fn rate_limiter_returns_remaining_count() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            enabled: true,
            max_requests: 3,
            window: Duration::from_secs(60),
        });
        let now = Instant::now();

        let status = limiter.check_at("k", now).unwrap();
        assert_eq!(status.limit, 3);
        assert_eq!(status.remaining, 2);

        let status = limiter.check_at("k", now + Duration::from_secs(1)).unwrap();
        assert_eq!(status.remaining, 1);

        let status = limiter.check_at("k", now + Duration::from_secs(2)).unwrap();
        assert_eq!(status.remaining, 0);

        // Next one should be rejected
        assert!(limiter.check_at("k", now + Duration::from_secs(3)).is_err());
    }

    #[test]
    fn rate_limiter_disabled_allows_requests() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            enabled: false,
            max_requests: 0,
            window: Duration::from_secs(1),
        });
        let now = Instant::now();

        for i in 0..100 {
            assert!(limiter
                .check_at("k", now + Duration::from_millis(i))
                .is_ok());
        }
    }

    #[test]
    fn rate_limit_key_isolates_subjects_and_namespaces() {
        let alice_a = AuthContext {
            subject: Some("alice".into()),
            role: AuthRole::Read,
            namespace: Some("team-a".into()),
            consumer_name: None,
        };
        let alice_b = AuthContext {
            subject: Some("alice".into()),
            role: AuthRole::Read,
            namespace: Some("team-b".into()),
            consumer_name: None,
        };
        let bob_a = AuthContext {
            subject: Some("bob".into()),
            role: AuthRole::Read,
            namespace: Some("team-a".into()),
            consumer_name: None,
        };

        assert_ne!(rate_limit_key(&alice_a), rate_limit_key(&alice_b));
        assert_ne!(rate_limit_key(&alice_a), rate_limit_key(&bob_a));
        assert_eq!(rate_limit_key(&alice_a), "alice|read|team-a");
    }

    #[test]
    fn ai_rate_limit_exempts_local_system_admin() {
        let status = enforce_ai_rate_limit(&AuthContext::system_admin())
            .expect("system admin should be exempt");
        assert_eq!(status.limit, 0);
    }

    #[test]
    fn ai_rate_limit_bucket_keys_isolate_subjects() {
        let limiter = RequestRateLimiter::new(RateLimitConfig {
            enabled: true,
            max_requests: 2,
            window: Duration::from_secs(60),
        });
        let now = Instant::now();
        let key_a = "ai|alice|read|team-a";
        let key_b = "ai|bob|read|team-a";

        assert!(limiter.check_at(key_a, now).is_ok());
        assert!(limiter.check_at(key_a, now + Duration::from_millis(1)).is_ok());
        assert!(limiter
            .check_at(key_a, now + Duration::from_millis(2))
            .is_err());
        assert!(limiter.check_at(key_b, now).is_ok());
    }

    #[test]
    fn namespace_node_quota_reads_env_each_call() {
        let _guard = env_test_lock();
        let key = ENV_NAMESPACE_NODE_QUOTA;
        let original = std::env::var(key).ok();
        std::env::remove_var(key);
        assert_eq!(namespace_node_quota(), None);

        std::env::set_var(key, "3");
        assert_eq!(namespace_node_quota(), Some(3));

        std::env::set_var(key, "1");
        assert_eq!(namespace_node_quota(), Some(1));

        std::env::set_var(key, "");
        assert_eq!(namespace_node_quota(), None);

        std::env::set_var(key, "not-a-number");
        assert_eq!(namespace_node_quota(), None);

        match original {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
}
