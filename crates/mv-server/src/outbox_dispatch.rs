//! Server spawn for the IK-004 outbox dispatcher and IK-005 HTTP publisher gate.

use std::sync::Arc;

use mv_core::StableUri;
use mv_engine::engine::{
    spawn_outbox_dispatcher, HttpOutboxPublisher, HttpOutboxPublisherConfig, LocalAckPublisher,
    OutboxDispatcherConfig, OutboxPublisher,
};
use tokio::sync::broadcast;

use crate::state::AppState;

pub fn spawn_outbox_dispatching(state: Arc<AppState>, shutdown_rx: broadcast::Receiver<()>) {
    let (config, publisher) = resolve_outbox_dispatch(state.as_ref());
    spawn_outbox_dispatcher(Arc::clone(&state.engine), shutdown_rx, config, publisher);
}

fn resolve_outbox_dispatch(state: &AppState) -> (OutboxDispatcherConfig, Arc<dyn OutboxPublisher>) {
    let mut config = OutboxDispatcherConfig::from_env();
    if let Some(http_config) =
        HttpOutboxPublisherConfig::from_env_if_admission_active(state.command_admission.is_active())
    {
        config.destination =
            StableUri::parse("mindvault://destinations/http").expect("http destination URI");
        match HttpOutboxPublisher::new(http_config) {
            Ok(publisher) => {
                tracing::info!("outbox dispatcher using HttpOutboxPublisher (env-gated)");
                return (config, Arc::new(publisher));
            }
            Err(error) => {
                tracing::warn!(%error, "HttpOutboxPublisher init failed; falling back to LocalAckPublisher");
            }
        }
    }
    (config, Arc::new(LocalAckPublisher))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_publisher_requires_admission_and_destination_url() {
        std::env::remove_var("MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED");
        std::env::remove_var("MINDVAULT_OUTBOX_HTTP_DESTINATION_URL");
        assert!(HttpOutboxPublisherConfig::from_env_if_admission_active(false).is_none());
        assert!(HttpOutboxPublisherConfig::from_env_if_admission_active(true).is_none());

        std::env::set_var("MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED", "1");
        assert!(HttpOutboxPublisherConfig::from_env_if_admission_active(false).is_none());
        assert!(HttpOutboxPublisherConfig::from_env_if_admission_active(true).is_none());

        std::env::set_var(
            "MINDVAULT_OUTBOX_HTTP_DESTINATION_URL",
            "http://127.0.0.1:8080/events",
        );
        let config = HttpOutboxPublisherConfig::from_env_if_admission_active(true)
            .expect("http config");
        assert_eq!(
            config.endpoint.as_str(),
            "http://127.0.0.1:8080/events"
        );

        std::env::remove_var("MINDVAULT_OUTBOX_HTTP_PUBLISHER_ENABLED");
        std::env::remove_var("MINDVAULT_OUTBOX_HTTP_DESTINATION_URL");
    }
}
