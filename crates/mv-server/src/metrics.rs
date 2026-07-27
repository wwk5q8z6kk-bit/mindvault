//! Lightweight Prometheus-style metrics for MindVault server.
//!
//! Exposes counters, gauges, and request-latency histograms (overall + per API
//! group) via `/metrics`, plus alert-oriented health hints for
//! `/api/v1/metrics/summary`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use axum::{
    extract::Request,
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Fixed histogram buckets in milliseconds.
const LATENCY_BUCKETS_MS: &[u64] = &[5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];

/// Core REST API groups used for labeled latency histograms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiGroup {
    Health,
    Nodes,
    Recall,
    Search,
    Keychain,
    Metrics,
    Other,
}

impl ApiGroup {
    pub const ALL: &[ApiGroup] = &[
        ApiGroup::Health,
        ApiGroup::Nodes,
        ApiGroup::Recall,
        ApiGroup::Search,
        ApiGroup::Keychain,
        ApiGroup::Metrics,
        ApiGroup::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ApiGroup::Health => "health",
            ApiGroup::Nodes => "nodes",
            ApiGroup::Recall => "recall",
            ApiGroup::Search => "search",
            ApiGroup::Keychain => "keychain",
            ApiGroup::Metrics => "metrics",
            ApiGroup::Other => "other",
        }
    }

    /// Map a request path onto a core API group.
    pub fn from_path(path: &str) -> Self {
        let path = path.split('?').next().unwrap_or(path);
        if path == "/api/v1/health" || path == "/health" || path.starts_with("/api/v1/diagnostics/")
        {
            ApiGroup::Health
        } else if path.starts_with("/api/v1/nodes") {
            ApiGroup::Nodes
        } else if path.starts_with("/api/v1/recall") {
            ApiGroup::Recall
        } else if path.starts_with("/api/v1/search") {
            ApiGroup::Search
        } else if path.starts_with("/api/v1/keychain") {
            ApiGroup::Keychain
        } else if path == "/metrics"
            || path.starts_with("/api/v1/metrics")
            || path.starts_with("/api/v1/provenance")
        {
            ApiGroup::Metrics
        } else {
            ApiGroup::Other
        }
    }
}

#[derive(Debug)]
struct LatencyHistogram {
    buckets: Vec<AtomicU64>,
    sum_us: AtomicU64,
    count: AtomicU64,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        let buckets: Vec<AtomicU64> = (0..LATENCY_BUCKETS_MS.len() + 1)
            .map(|_| AtomicU64::new(0))
            .collect();
        Self {
            buckets,
            sum_us: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }
}

impl LatencyHistogram {
    fn observe_us(&self, latency_us: u64) {
        let latency_ms = latency_us / 1000;
        for (i, &bound) in LATENCY_BUCKETS_MS.iter().enumerate() {
            if latency_ms <= bound {
                self.buckets[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        self.buckets.last().unwrap().fetch_add(1, Ordering::Relaxed);
        self.sum_us.fetch_add(latency_us, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> LatencySnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum_us = self.sum_us.load(Ordering::Relaxed);
        let bucket_counts: Vec<u64> = self
            .buckets
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect();
        let avg_ms = if count == 0 {
            0.0
        } else {
            (sum_us as f64 / count as f64) / 1000.0
        };
        LatencySnapshot {
            count,
            sum_ms: sum_us as f64 / 1000.0,
            avg_ms,
            p50_ms: percentile_from_buckets(&bucket_counts, 0.50),
            p95_ms: percentile_from_buckets(&bucket_counts, 0.95),
            p99_ms: percentile_from_buckets(&bucket_counts, 0.99),
        }
    }
}

/// Approximate a percentile from a cumulative histogram.
fn percentile_from_buckets(bucket_counts: &[u64], quantile: f64) -> Option<f64> {
    let total = *bucket_counts.last()?;
    if total == 0 {
        return None;
    }
    let target = ((total as f64) * quantile).ceil().max(1.0) as u64;
    for (i, &bound_ms) in LATENCY_BUCKETS_MS.iter().enumerate() {
        if bucket_counts[i] >= target {
            return Some(bound_ms as f64);
        }
    }
    Some(*LATENCY_BUCKETS_MS.last().unwrap_or(&10000) as f64)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LatencySnapshot {
    pub count: u64,
    pub sum_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
}

#[derive(Debug)]
pub struct MetricsCounters {
    rest_requests_total: AtomicU64,
    rest_errors_total: AtomicU64,
    grpc_requests_total: AtomicU64,
    grpc_errors_total: AtomicU64,
    vault_sealed_http_requests_blocked_total: AtomicU64,
    vault_sealed_grpc_requests_blocked_total: AtomicU64,
    vault_unseal_failures_total: AtomicU64,
    vault_unseal_rate_limited_total: AtomicU64,
    vault_sealed_migration_success_total: AtomicU64,
    vault_sealed_migration_failures_total: AtomicU64,
    vault_runtime_rebuild_success_total: AtomicU64,
    vault_runtime_rebuild_failures_total: AtomicU64,
    /// Aggregate REST latency histogram (all groups).
    rest_latency: LatencyHistogram,
    /// Per-API-group REST latency histograms (indexed by [`ApiGroup`] order).
    group_latency: [LatencyHistogram; 7],
}

impl Default for MetricsCounters {
    fn default() -> Self {
        Self {
            rest_requests_total: AtomicU64::new(0),
            rest_errors_total: AtomicU64::new(0),
            grpc_requests_total: AtomicU64::new(0),
            grpc_errors_total: AtomicU64::new(0),
            vault_sealed_http_requests_blocked_total: AtomicU64::new(0),
            vault_sealed_grpc_requests_blocked_total: AtomicU64::new(0),
            vault_unseal_failures_total: AtomicU64::new(0),
            vault_unseal_rate_limited_total: AtomicU64::new(0),
            vault_sealed_migration_success_total: AtomicU64::new(0),
            vault_sealed_migration_failures_total: AtomicU64::new(0),
            vault_runtime_rebuild_success_total: AtomicU64::new(0),
            vault_runtime_rebuild_failures_total: AtomicU64::new(0),
            rest_latency: LatencyHistogram::default(),
            group_latency: [
                LatencyHistogram::default(),
                LatencyHistogram::default(),
                LatencyHistogram::default(),
                LatencyHistogram::default(),
                LatencyHistogram::default(),
                LatencyHistogram::default(),
                LatencyHistogram::default(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct MetricsSnapshot {
    pub rest_requests_total: u64,
    pub rest_errors_total: u64,
    pub grpc_requests_total: u64,
    pub grpc_errors_total: u64,
    pub vault_sealed_http_requests_blocked_total: u64,
    pub vault_sealed_grpc_requests_blocked_total: u64,
    pub vault_unseal_failures_total: u64,
    pub vault_unseal_rate_limited_total: u64,
    pub vault_sealed_migration_success_total: u64,
    pub vault_sealed_migration_failures_total: u64,
    pub vault_runtime_rebuild_success_total: u64,
    pub vault_runtime_rebuild_failures_total: u64,
}

/// Default alert thresholds for operational dashboards.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AlertThresholds {
    pub error_rate_warning: f64,
    pub error_rate_critical: f64,
    pub p95_latency_ms_warning: f64,
    pub p95_latency_ms_critical: f64,
    pub vault_unseal_failures_warning: u64,
    pub vault_sealed_blocked_warning: u64,
    pub vault_migration_failures_warning: u64,
    pub vault_rebuild_failures_warning: u64,
    /// Minimum REST samples before latency/error-rate hints fire.
    pub min_samples_for_latency_alerts: u64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            error_rate_warning: 0.05,
            error_rate_critical: 0.20,
            p95_latency_ms_warning: 1000.0,
            p95_latency_ms_critical: 5000.0,
            vault_unseal_failures_warning: 1,
            vault_sealed_blocked_warning: 1,
            vault_migration_failures_warning: 1,
            vault_rebuild_failures_warning: 1,
            min_samples_for_latency_alerts: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HintSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HealthHint {
    pub code: String,
    pub severity: HintSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricsHealthReport {
    pub status: HealthStatus,
    pub hints: Vec<HealthHint>,
    pub thresholds: AlertThresholds,
    pub rest_error_rate: f64,
    pub latency: LatencySnapshot,
    pub latency_by_api_group: Vec<(String, LatencySnapshot)>,
}

static METRICS: OnceLock<MetricsCounters> = OnceLock::new();

pub fn init_metrics() {
    let _ = METRICS.get_or_init(MetricsCounters::default);
}

pub fn get_metrics() -> &'static MetricsCounters {
    METRICS.get_or_init(MetricsCounters::default)
}

impl MetricsCounters {
    fn group_index(group: ApiGroup) -> usize {
        match group {
            ApiGroup::Health => 0,
            ApiGroup::Nodes => 1,
            ApiGroup::Recall => 2,
            ApiGroup::Search => 3,
            ApiGroup::Keychain => 4,
            ApiGroup::Metrics => 5,
            ApiGroup::Other => 6,
        }
    }

    pub fn incr_rest_request(&self) {
        self.rest_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_rest_error(&self) {
        self.rest_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_grpc_request(&self) {
        self.grpc_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_grpc_error(&self) {
        self.grpc_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_sealed_http_blocked(&self) {
        self.vault_sealed_http_requests_blocked_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_sealed_grpc_blocked(&self) {
        self.vault_sealed_grpc_requests_blocked_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_unseal_failure(&self) {
        self.vault_unseal_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_unseal_rate_limited(&self) {
        self.vault_unseal_rate_limited_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_migration_success(&self) {
        self.vault_sealed_migration_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_migration_failure(&self) {
        self.vault_sealed_migration_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_rebuild_success(&self) {
        self.vault_runtime_rebuild_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_vault_rebuild_failure(&self) {
        self.vault_runtime_rebuild_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn observe_rest_latency_us(&self, latency_us: u64) {
        self.observe_rest_latency_for_group_us(ApiGroup::Other, latency_us);
    }

    pub fn observe_rest_latency_for_group_us(&self, group: ApiGroup, latency_us: u64) {
        self.rest_latency.observe_us(latency_us);
        self.group_latency[Self::group_index(group)].observe_us(latency_us);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            rest_requests_total: self.rest_requests_total.load(Ordering::Relaxed),
            rest_errors_total: self.rest_errors_total.load(Ordering::Relaxed),
            grpc_requests_total: self.grpc_requests_total.load(Ordering::Relaxed),
            grpc_errors_total: self.grpc_errors_total.load(Ordering::Relaxed),
            vault_sealed_http_requests_blocked_total: self
                .vault_sealed_http_requests_blocked_total
                .load(Ordering::Relaxed),
            vault_sealed_grpc_requests_blocked_total: self
                .vault_sealed_grpc_requests_blocked_total
                .load(Ordering::Relaxed),
            vault_unseal_failures_total: self.vault_unseal_failures_total.load(Ordering::Relaxed),
            vault_unseal_rate_limited_total: self
                .vault_unseal_rate_limited_total
                .load(Ordering::Relaxed),
            vault_sealed_migration_success_total: self
                .vault_sealed_migration_success_total
                .load(Ordering::Relaxed),
            vault_sealed_migration_failures_total: self
                .vault_sealed_migration_failures_total
                .load(Ordering::Relaxed),
            vault_runtime_rebuild_success_total: self
                .vault_runtime_rebuild_success_total
                .load(Ordering::Relaxed),
            vault_runtime_rebuild_failures_total: self
                .vault_runtime_rebuild_failures_total
                .load(Ordering::Relaxed),
        }
    }

    pub fn latency_snapshot(&self) -> LatencySnapshot {
        self.rest_latency.snapshot()
    }

    pub fn latency_by_api_group(&self) -> Vec<(String, LatencySnapshot)> {
        ApiGroup::ALL
            .iter()
            .map(|group| {
                (
                    group.as_str().to_string(),
                    self.group_latency[Self::group_index(*group)].snapshot(),
                )
            })
            .collect()
    }

    /// Build an alert-oriented health report for operational dashboards.
    pub fn health_report(&self, thresholds: AlertThresholds) -> MetricsHealthReport {
        let snapshot = self.snapshot();
        let latency = self.latency_snapshot();
        let latency_by_api_group = self.latency_by_api_group();
        let rest_error_rate = if snapshot.rest_requests_total == 0 {
            0.0
        } else {
            snapshot.rest_errors_total as f64 / snapshot.rest_requests_total as f64
        };

        let mut hints = Vec::new();

        if snapshot.rest_requests_total >= thresholds.min_samples_for_latency_alerts {
            if rest_error_rate >= thresholds.error_rate_critical {
                hints.push(HealthHint {
                    code: "rest_error_rate_critical".into(),
                    severity: HintSeverity::Critical,
                    message: format!(
                        "REST error rate {:.1}% exceeds critical threshold {:.0}%",
                        rest_error_rate * 100.0,
                        thresholds.error_rate_critical * 100.0
                    ),
                });
            } else if rest_error_rate >= thresholds.error_rate_warning {
                hints.push(HealthHint {
                    code: "rest_error_rate_warning".into(),
                    severity: HintSeverity::Warning,
                    message: format!(
                        "REST error rate {:.1}% exceeds warning threshold {:.0}%",
                        rest_error_rate * 100.0,
                        thresholds.error_rate_warning * 100.0
                    ),
                });
            }

            if let Some(p95) = latency.p95_ms {
                if p95 >= thresholds.p95_latency_ms_critical {
                    hints.push(HealthHint {
                        code: "rest_p95_latency_critical".into(),
                        severity: HintSeverity::Critical,
                        message: format!(
                            "REST p95 latency {p95:.0}ms exceeds critical threshold {:.0}ms",
                            thresholds.p95_latency_ms_critical
                        ),
                    });
                } else if p95 >= thresholds.p95_latency_ms_warning {
                    hints.push(HealthHint {
                        code: "rest_p95_latency_warning".into(),
                        severity: HintSeverity::Warning,
                        message: format!(
                            "REST p95 latency {p95:.0}ms exceeds warning threshold {:.0}ms",
                            thresholds.p95_latency_ms_warning
                        ),
                    });
                }
            }

            for (group, group_latency) in &latency_by_api_group {
                if group_latency.count < thresholds.min_samples_for_latency_alerts {
                    continue;
                }
                if let Some(p95) = group_latency.p95_ms {
                    if p95 >= thresholds.p95_latency_ms_critical {
                        hints.push(HealthHint {
                            code: format!("api_group_{group}_p95_latency_critical"),
                            severity: HintSeverity::Critical,
                            message: format!(
                                "API group '{group}' p95 latency {p95:.0}ms exceeds critical threshold {:.0}ms",
                                thresholds.p95_latency_ms_critical
                            ),
                        });
                    } else if p95 >= thresholds.p95_latency_ms_warning {
                        hints.push(HealthHint {
                            code: format!("api_group_{group}_p95_latency_warning"),
                            severity: HintSeverity::Warning,
                            message: format!(
                                "API group '{group}' p95 latency {p95:.0}ms exceeds warning threshold {:.0}ms",
                                thresholds.p95_latency_ms_warning
                            ),
                        });
                    }
                }
            }
        }

        if snapshot.vault_unseal_failures_total >= thresholds.vault_unseal_failures_warning {
            hints.push(HealthHint {
                code: "vault_unseal_failures".into(),
                severity: HintSeverity::Warning,
                message: format!(
                    "{} vault unseal failure(s) recorded",
                    snapshot.vault_unseal_failures_total
                ),
            });
        }
        if snapshot.vault_unseal_rate_limited_total > 0 {
            hints.push(HealthHint {
                code: "vault_unseal_rate_limited".into(),
                severity: HintSeverity::Warning,
                message: format!(
                    "{} vault unseal attempt(s) rate-limited",
                    snapshot.vault_unseal_rate_limited_total
                ),
            });
        }
        let sealed_blocked = snapshot.vault_sealed_http_requests_blocked_total
            + snapshot.vault_sealed_grpc_requests_blocked_total;
        if sealed_blocked >= thresholds.vault_sealed_blocked_warning {
            hints.push(HealthHint {
                code: "vault_sealed_requests_blocked".into(),
                severity: HintSeverity::Warning,
                message: format!("{sealed_blocked} request(s) blocked while vault was sealed"),
            });
        }
        if snapshot.vault_sealed_migration_failures_total
            >= thresholds.vault_migration_failures_warning
        {
            hints.push(HealthHint {
                code: "vault_migration_failures".into(),
                severity: HintSeverity::Critical,
                message: format!(
                    "{} sealed migration failure(s) recorded",
                    snapshot.vault_sealed_migration_failures_total
                ),
            });
        }
        if snapshot.vault_runtime_rebuild_failures_total
            >= thresholds.vault_rebuild_failures_warning
        {
            hints.push(HealthHint {
                code: "vault_rebuild_failures".into(),
                severity: HintSeverity::Critical,
                message: format!(
                    "{} runtime rebuild failure(s) recorded",
                    snapshot.vault_runtime_rebuild_failures_total
                ),
            });
        }

        if hints.is_empty() {
            hints.push(HealthHint {
                code: "ok".into(),
                severity: HintSeverity::Info,
                message: "All monitored metrics within thresholds".into(),
            });
        }

        let status = if hints
            .iter()
            .any(|h| matches!(h.severity, HintSeverity::Critical))
        {
            HealthStatus::Unhealthy
        } else if hints
            .iter()
            .any(|h| matches!(h.severity, HintSeverity::Warning))
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        MetricsHealthReport {
            status,
            hints,
            thresholds,
            rest_error_rate,
            latency,
            latency_by_api_group,
        }
    }
}

pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let metrics = get_metrics();
    let api_group = ApiGroup::from_path(request.uri().path());
    metrics.incr_rest_request();
    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed_us = start.elapsed().as_micros() as u64;
    metrics.observe_rest_latency_for_group_us(api_group, elapsed_us);
    if !response.status().is_success() {
        metrics.incr_rest_error();
    }
    response
}

fn append_histogram(
    body: &mut String,
    name: &str,
    help: &str,
    histogram: &LatencyHistogram,
    label: Option<(&str, &str)>,
) {
    body.push_str(&format!("# HELP {name} {help}\n# TYPE {name} histogram\n"));
    let label_prefix = match label {
        Some((k, v)) => format!("{k}=\"{v}\","),
        None => String::new(),
    };
    for (i, &bound_ms) in LATENCY_BUCKETS_MS.iter().enumerate() {
        let count = histogram.buckets[i].load(Ordering::Relaxed);
        let bound_s = bound_ms as f64 / 1000.0;
        body.push_str(&format!(
            "{name}_bucket{{{label_prefix}le=\"{bound_s}\"}} {count}\n"
        ));
    }
    let inf_count = histogram.buckets.last().unwrap().load(Ordering::Relaxed);
    body.push_str(&format!(
        "{name}_bucket{{{label_prefix}le=\"+Inf\"}} {inf_count}\n"
    ));
    let sum_s = histogram.sum_us.load(Ordering::Relaxed) as f64 / 1_000_000.0;
    let count = histogram.count.load(Ordering::Relaxed);
    match label {
        Some((k, v)) => {
            body.push_str(&format!(
                "{name}_sum{{{k}=\"{v}\"}} {sum_s}\n{name}_count{{{k}=\"{v}\"}} {count}\n"
            ));
        }
        None => {
            body.push_str(&format!("{name}_sum {sum_s}\n{name}_count {count}\n"));
        }
    }
}

pub async fn metrics_handler() -> impl IntoResponse {
    let m = get_metrics();
    let snapshot = m.snapshot();

    let mut body = format!(
        "# HELP mindvault_rest_requests_total Total REST requests handled\n\
# TYPE mindvault_rest_requests_total counter\n\
mindvault_rest_requests_total {}\n\
# HELP mindvault_rest_errors_total Total REST requests returning non-2xx\n\
# TYPE mindvault_rest_errors_total counter\n\
mindvault_rest_errors_total {}\n\
# HELP mindvault_grpc_requests_total Total gRPC requests handled\n\
# TYPE mindvault_grpc_requests_total counter\n\
mindvault_grpc_requests_total {}\n\
# HELP mindvault_grpc_errors_total Total gRPC requests returning errors\n\
# TYPE mindvault_grpc_errors_total counter\n\
mindvault_grpc_errors_total {}\n",
        snapshot.rest_requests_total,
        snapshot.rest_errors_total,
        snapshot.grpc_requests_total,
        snapshot.grpc_errors_total,
    );
    body.push_str(&format!(
        "# HELP mindvault_vault_sealed_http_requests_blocked_total Total HTTP requests blocked because vault is sealed\n\
# TYPE mindvault_vault_sealed_http_requests_blocked_total counter\n\
mindvault_vault_sealed_http_requests_blocked_total {}\n\
# HELP mindvault_vault_sealed_grpc_requests_blocked_total Total gRPC requests blocked because vault is sealed\n\
# TYPE mindvault_vault_sealed_grpc_requests_blocked_total counter\n\
mindvault_vault_sealed_grpc_requests_blocked_total {}\n\
# HELP mindvault_vault_unseal_failures_total Total failed vault unseal attempts\n\
# TYPE mindvault_vault_unseal_failures_total counter\n\
mindvault_vault_unseal_failures_total {}\n\
# HELP mindvault_vault_unseal_rate_limited_total Total unseal attempts blocked by rate limiting\n\
# TYPE mindvault_vault_unseal_rate_limited_total counter\n\
mindvault_vault_unseal_rate_limited_total {}\n\
# HELP mindvault_vault_sealed_migration_success_total Total successful post-unseal sealed migrations\n\
# TYPE mindvault_vault_sealed_migration_success_total counter\n\
mindvault_vault_sealed_migration_success_total {}\n\
# HELP mindvault_vault_sealed_migration_failures_total Total failed post-unseal sealed migrations\n\
# TYPE mindvault_vault_sealed_migration_failures_total counter\n\
mindvault_vault_sealed_migration_failures_total {}\n\
# HELP mindvault_vault_runtime_rebuild_success_total Total successful post-unseal runtime index rebuilds\n\
# TYPE mindvault_vault_runtime_rebuild_success_total counter\n\
mindvault_vault_runtime_rebuild_success_total {}\n\
# HELP mindvault_vault_runtime_rebuild_failures_total Total failed post-unseal runtime index rebuilds\n\
# TYPE mindvault_vault_runtime_rebuild_failures_total counter\n\
mindvault_vault_runtime_rebuild_failures_total {}\n",
        snapshot.vault_sealed_http_requests_blocked_total,
        snapshot.vault_sealed_grpc_requests_blocked_total,
        snapshot.vault_unseal_failures_total,
        snapshot.vault_unseal_rate_limited_total,
        snapshot.vault_sealed_migration_success_total,
        snapshot.vault_sealed_migration_failures_total,
        snapshot.vault_runtime_rebuild_success_total,
        snapshot.vault_runtime_rebuild_failures_total,
    ));

    // Aggregate latency histogram (backward compatible)
    append_histogram(
        &mut body,
        "mindvault_rest_request_duration_seconds",
        "REST request latency",
        &m.rest_latency,
        None,
    );

    // Per-API-group latency histograms for core API groups.
    // Emit HELP/TYPE once, then labeled series only.
    body.push_str(
        "# HELP mindvault_rest_request_duration_seconds_by_group REST request latency by API group\n\
# TYPE mindvault_rest_request_duration_seconds_by_group histogram\n",
    );
    for group in ApiGroup::ALL {
        let hist = &m.group_latency[MetricsCounters::group_index(*group)];
        let label_prefix = format!("api_group=\"{}\",", group.as_str());
        for (i, &bound_ms) in LATENCY_BUCKETS_MS.iter().enumerate() {
            let count = hist.buckets[i].load(Ordering::Relaxed);
            let bound_s = bound_ms as f64 / 1000.0;
            body.push_str(&format!(
                "mindvault_rest_request_duration_seconds_by_group_bucket{{{label_prefix}le=\"{bound_s}\"}} {count}\n"
            ));
        }
        let inf_count = hist.buckets.last().unwrap().load(Ordering::Relaxed);
        body.push_str(&format!(
            "mindvault_rest_request_duration_seconds_by_group_bucket{{{label_prefix}le=\"+Inf\"}} {inf_count}\n"
        ));
        let sum_s = hist.sum_us.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let count = hist.count.load(Ordering::Relaxed);
        body.push_str(&format!(
            "mindvault_rest_request_duration_seconds_by_group_sum{{api_group=\"{}\"}} {sum_s}\n\
mindvault_rest_request_duration_seconds_by_group_count{{api_group=\"{}\"}} {count}\n",
            group.as_str(),
            group.as_str()
        ));
    }

    let mut response = (StatusCode::OK, body).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    response
}

/// Render Prometheus text for a local counters instance (tests / tooling).
pub fn render_prometheus_text(m: &MetricsCounters) -> String {
    let snapshot = m.snapshot();
    let mut body = format!(
        "mindvault_rest_requests_total {}\nmindvault_rest_errors_total {}\n",
        snapshot.rest_requests_total, snapshot.rest_errors_total
    );
    append_histogram(
        &mut body,
        "mindvault_rest_request_duration_seconds",
        "REST request latency",
        &m.rest_latency,
        None,
    );
    body.push_str(
        "# HELP mindvault_rest_request_duration_seconds_by_group REST request latency by API group\n\
# TYPE mindvault_rest_request_duration_seconds_by_group histogram\n",
    );
    for group in ApiGroup::ALL {
        let hist = &m.group_latency[MetricsCounters::group_index(*group)];
        if hist.count.load(Ordering::Relaxed) == 0 {
            continue;
        }
        let label_prefix = format!("api_group=\"{}\",", group.as_str());
        for (i, &bound_ms) in LATENCY_BUCKETS_MS.iter().enumerate() {
            let count = hist.buckets[i].load(Ordering::Relaxed);
            let bound_s = bound_ms as f64 / 1000.0;
            body.push_str(&format!(
                "mindvault_rest_request_duration_seconds_by_group_bucket{{{label_prefix}le=\"{bound_s}\"}} {count}\n"
            ));
        }
        let inf_count = hist.buckets.last().unwrap().load(Ordering::Relaxed);
        body.push_str(&format!(
            "mindvault_rest_request_duration_seconds_by_group_bucket{{{label_prefix}le=\"+Inf\"}} {inf_count}\n"
        ));
        let sum_s = hist.sum_us.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let count = hist.count.load(Ordering::Relaxed);
        body.push_str(&format!(
            "mindvault_rest_request_duration_seconds_by_group_sum{{api_group=\"{}\"}} {sum_s}\n\
mindvault_rest_request_duration_seconds_by_group_count{{api_group=\"{}\"}} {count}\n",
            group.as_str(),
            group.as_str()
        ));
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_counters_increment_and_snapshot() {
        let counters = MetricsCounters::default();
        counters.incr_rest_request();
        counters.incr_rest_error();
        counters.incr_grpc_request();
        counters.incr_vault_sealed_http_blocked();
        counters.incr_vault_sealed_grpc_blocked();
        counters.incr_vault_unseal_failure();
        counters.incr_vault_unseal_rate_limited();
        counters.incr_vault_migration_success();
        counters.incr_vault_migration_failure();
        counters.incr_vault_rebuild_success();
        counters.incr_vault_rebuild_failure();

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.rest_requests_total, 1);
        assert_eq!(snapshot.rest_errors_total, 1);
        assert_eq!(snapshot.grpc_requests_total, 1);
        assert_eq!(snapshot.grpc_errors_total, 0);
        assert_eq!(snapshot.vault_sealed_http_requests_blocked_total, 1);
        assert_eq!(snapshot.vault_sealed_grpc_requests_blocked_total, 1);
        assert_eq!(snapshot.vault_unseal_failures_total, 1);
        assert_eq!(snapshot.vault_unseal_rate_limited_total, 1);
        assert_eq!(snapshot.vault_sealed_migration_success_total, 1);
        assert_eq!(snapshot.vault_sealed_migration_failures_total, 1);
        assert_eq!(snapshot.vault_runtime_rebuild_success_total, 1);
        assert_eq!(snapshot.vault_runtime_rebuild_failures_total, 1);
    }

    #[test]
    fn latency_histogram_buckets() {
        let counters = MetricsCounters::default();
        counters.observe_rest_latency_us(50_000);

        let buckets = &counters.rest_latency.buckets;
        assert_eq!(buckets[0].load(Ordering::Relaxed), 0); // 5ms
        assert_eq!(buckets[1].load(Ordering::Relaxed), 0); // 10ms
        assert_eq!(buckets[2].load(Ordering::Relaxed), 0); // 25ms
        assert_eq!(buckets[3].load(Ordering::Relaxed), 1); // 50ms
        assert_eq!(buckets[4].load(Ordering::Relaxed), 1); // 100ms
        assert_eq!(buckets.last().unwrap().load(Ordering::Relaxed), 1);
        assert_eq!(counters.rest_latency.count.load(Ordering::Relaxed), 1);
        assert_eq!(counters.rest_latency.sum_us.load(Ordering::Relaxed), 50_000);
    }

    #[test]
    fn api_group_from_path_classifies_core_routes() {
        assert_eq!(ApiGroup::from_path("/api/v1/health"), ApiGroup::Health);
        assert_eq!(ApiGroup::from_path("/api/v1/nodes/abc"), ApiGroup::Nodes);
        assert_eq!(ApiGroup::from_path("/api/v1/recall"), ApiGroup::Recall);
        assert_eq!(ApiGroup::from_path("/api/v1/search?q=x"), ApiGroup::Search);
        assert_eq!(
            ApiGroup::from_path("/api/v1/keychain/status"),
            ApiGroup::Keychain
        );
        assert_eq!(
            ApiGroup::from_path("/api/v1/metrics/summary"),
            ApiGroup::Metrics
        );
        assert_eq!(ApiGroup::from_path("/metrics"), ApiGroup::Metrics);
        assert_eq!(ApiGroup::from_path("/api/v1/unknown"), ApiGroup::Other);
    }

    #[test]
    fn per_api_group_latency_histograms_are_isolated() {
        let counters = MetricsCounters::default();
        counters.observe_rest_latency_for_group_us(ApiGroup::Nodes, 40_000);
        counters.observe_rest_latency_for_group_us(ApiGroup::Recall, 900_000);

        let nodes = &counters.group_latency[MetricsCounters::group_index(ApiGroup::Nodes)];
        let recall = &counters.group_latency[MetricsCounters::group_index(ApiGroup::Recall)];
        let search = &counters.group_latency[MetricsCounters::group_index(ApiGroup::Search)];

        assert_eq!(nodes.count.load(Ordering::Relaxed), 1);
        assert_eq!(recall.count.load(Ordering::Relaxed), 1);
        assert_eq!(search.count.load(Ordering::Relaxed), 0);
        assert_eq!(counters.rest_latency.count.load(Ordering::Relaxed), 2);

        let text = render_prometheus_text(&counters);
        assert!(text.contains("api_group=\"nodes\""));
        assert!(text.contains("api_group=\"recall\""));
        assert!(!text.contains("api_group=\"search\""));
    }

    #[test]
    fn health_report_healthy_when_within_thresholds() {
        let counters = MetricsCounters::default();
        for _ in 0..10 {
            counters.incr_rest_request();
            counters.observe_rest_latency_for_group_us(ApiGroup::Health, 5_000);
        }
        let report = counters.health_report(AlertThresholds::default());
        assert_eq!(report.status, HealthStatus::Healthy);
        assert!(report.hints.iter().any(|h| h.code == "ok"));
        assert!(report.rest_error_rate < 0.01);
    }

    #[test]
    fn health_report_degraded_on_high_error_rate() {
        let counters = MetricsCounters::default();
        for _ in 0..10 {
            counters.incr_rest_request();
        }
        // 10% error rate is above warning (5%) but below critical (20%).
        counters.incr_rest_error();
        for _ in 0..10 {
            counters.observe_rest_latency_for_group_us(ApiGroup::Nodes, 10_000);
        }
        let report = counters.health_report(AlertThresholds::default());
        assert_eq!(report.status, HealthStatus::Degraded);
        assert!(report
            .hints
            .iter()
            .any(|h| h.code == "rest_error_rate_warning"));
    }

    #[test]
    fn health_report_unhealthy_on_critical_latency_and_migration_failure() {
        let mut thresholds = AlertThresholds::default();
        thresholds.min_samples_for_latency_alerts = 1;
        thresholds.p95_latency_ms_critical = 100.0;

        let counters = MetricsCounters::default();
        counters.incr_rest_request();
        // 250ms observation lands in >=250ms bucket → p95 >= 250
        counters.observe_rest_latency_for_group_us(ApiGroup::Recall, 250_000);
        counters.incr_vault_migration_failure();

        let report = counters.health_report(thresholds);
        assert_eq!(report.status, HealthStatus::Unhealthy);
        assert!(report
            .hints
            .iter()
            .any(|h| h.code == "rest_p95_latency_critical"));
        assert!(report
            .hints
            .iter()
            .any(|h| h.code == "vault_migration_failures"));
    }

    #[test]
    fn percentile_from_empty_histogram_is_none() {
        let counters = MetricsCounters::default();
        let snap = counters.latency_snapshot();
        assert_eq!(snap.count, 0);
        assert_eq!(snap.p50_ms, None);
        assert_eq!(snap.p95_ms, None);
    }
}
