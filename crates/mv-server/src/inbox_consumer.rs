//! Server spawn for the IK-006 inbox consumer.

use std::sync::Arc;

use mv_engine::engine::{
    spawn_inbox_consumer, InboxConsumerConfig, InboxDomainHandler, LocalIndexHandler,
};
use tokio::sync::broadcast;

use crate::state::AppState;

pub fn spawn_inbox_consuming(state: Arc<AppState>, shutdown_rx: broadcast::Receiver<()>) {
    let config = InboxConsumerConfig::from_env();
    let handler: Arc<dyn InboxDomainHandler> = Arc::new(LocalIndexHandler);
    spawn_inbox_consumer(Arc::clone(&state.engine), shutdown_rx, config, handler);
}
