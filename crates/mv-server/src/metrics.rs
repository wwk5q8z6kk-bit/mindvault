//! Lightweight Prometheus-style metrics for MindVault server.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use axum::{
    extract::Request,
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

#[derive(Debug, Default)]
pub struct MetricsCounters {
    rest_requests_total: AtomicU64,
    rest_errors_total: AtomicU64,
    grpc_requests_total: AtomicU64,
    grpc_errors_total: AtomicU64,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct MetricsSnapshot {
    pub rest_requests_total: u64,
    pub rest_errors_total: u64,
    pub grpc_requests_total: u64,
    pub grpc_errors_total: u64,
}

static METRICS: OnceLock<MetricsCounters> = OnceLock::new();

pub fn init_metrics() {
    let _ = METRICS.get_or_init(MetricsCounters::default);
}

pub fn get_metrics() -> &'static MetricsCounters {
    METRICS.get_or_init(MetricsCounters::default)
}

impl MetricsCounters {
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

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            rest_requests_total: self.rest_requests_total.load(Ordering::Relaxed),
            rest_errors_total: self.rest_errors_total.load(Ordering::Relaxed),
            grpc_requests_total: self.grpc_requests_total.load(Ordering::Relaxed),
            grpc_errors_total: self.grpc_errors_total.load(Ordering::Relaxed),
        }
    }
}

pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let metrics = get_metrics();
    metrics.incr_rest_request();
    let response = next.run(request).await;
    if !response.status().is_success() {
        metrics.incr_rest_error();
    }
    response
}

pub async fn metrics_handler() -> impl IntoResponse {
    let snapshot = get_metrics().snapshot();
    let body = format!(
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

    let mut response = (StatusCode::OK, body).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    response
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

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.rest_requests_total, 1);
        assert_eq!(snapshot.rest_errors_total, 1);
        assert_eq!(snapshot.grpc_requests_total, 1);
        assert_eq!(snapshot.grpc_errors_total, 0);
    }
}
