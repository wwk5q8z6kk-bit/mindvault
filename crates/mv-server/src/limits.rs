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

    fn check(&self, key: &str) -> Result<(), RateLimitExceeded> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: &str, now: Instant) -> Result<(), RateLimitExceeded> {
        if !self.config.enabled {
            return Ok(());
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

        queue.push_back(now);
        Ok(())
    }
}

static RATE_LIMITER: OnceLock<RequestRateLimiter> = OnceLock::new();
static NAMESPACE_NODE_QUOTA: OnceLock<Option<usize>> = OnceLock::new();

pub fn enforce_rate_limit(auth: &AuthContext) -> Result<(), RateLimitExceeded> {
    let key = rate_limit_key(auth);
    RATE_LIMITER
        .get_or_init(|| RequestRateLimiter::new(RateLimitConfig::from_env()))
        .check(&key)
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
    *NAMESPACE_NODE_QUOTA.get_or_init(|| read_env_usize(ENV_NAMESPACE_NODE_QUOTA))
}

fn rate_limit_key(auth: &AuthContext) -> String {
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
}
