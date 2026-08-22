//! Server spawn for the IK-004/IK-005 outbox dispatcher.

use std::sync::Arc;

use mv_core::StableUri;
use mv_engine::engine::{
    spawn_outbox_dispatcher, HttpOutboxPublisher, LocalAckPublisher, OutboxDispatcherConfig,
    OutboxPublisher,
};
use tokio::sync::broadcast;

use crate::state::AppState;

pub fn spawn_outbox_dispatching(state: Arc<AppState>, shutdown_rx: broadcast::Receiver<()>) {
    let mut config = OutboxDispatcherConfig::from_env();
    let publisher: Arc<dyn OutboxPublisher> = match HttpOutboxPublisher::from_env() {
        Ok(Some(http)) => {
            config.destination = StableUri::parse("mindvault://destinations/http")
                .expect("http destination URI");
            tracing::info!("outbox dispatcher using HttpOutboxPublisher (IK-005)");
            Arc::new(http)
        }
        Ok(None) => Arc::new(LocalAckPublisher),
        Err(error) => {
            tracing::error!(
                %error,
                "invalid MINDVAULT_OUTBOX_HTTP_* configuration; falling back to local-ack"
            );
            Arc::new(LocalAckPublisher)
        }
    };
    spawn_outbox_dispatcher(Arc::clone(&state.engine), shutdown_rx, config, publisher);
}
