//! Server spawn for the IK-004 outbox dispatcher.

use std::sync::Arc;

use mv_engine::engine::{
    spawn_outbox_dispatcher, LocalAckPublisher, OutboxDispatcherConfig, OutboxPublisher,
};
use tokio::sync::broadcast;

use crate::state::AppState;

pub fn spawn_outbox_dispatching(
    state: Arc<AppState>,
    shutdown_rx: broadcast::Receiver<()>,
) -> Option<tokio::task::JoinHandle<()>> {
    let config = OutboxDispatcherConfig::from_env();
    let publisher: Arc<dyn OutboxPublisher> = Arc::new(LocalAckPublisher);
    spawn_outbox_dispatcher(Arc::clone(&state.engine), shutdown_rx, config, publisher)
}
