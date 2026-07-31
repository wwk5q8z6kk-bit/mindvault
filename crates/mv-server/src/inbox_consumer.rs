//! Server spawn for the IK-006 inbox consumer runtime.

use std::sync::Arc;

use mv_engine::engine::{
    spawn_inbox_consumer, InboxConsumerConfig, LocalProjectionHandler,
};
use tokio::sync::broadcast;

use crate::state::AppState;

pub fn spawn_inbox_consuming(state: Arc<AppState>, shutdown_rx: broadcast::Receiver<()>) {
    let config = InboxConsumerConfig::from_env();
    spawn_inbox_consumer(
        Arc::clone(&state.engine),
        shutdown_rx,
        config,
        Arc::new(LocalProjectionHandler),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_consumer_env_gate_defaults_off_and_enables_explicitly() {
        let previous = std::env::var("MINDVAULT_INBOX_CONSUMER_ENABLED").ok();

        std::env::remove_var("MINDVAULT_INBOX_CONSUMER_ENABLED");
        assert!(!InboxConsumerConfig::from_env().enabled);

        std::env::set_var("MINDVAULT_INBOX_CONSUMER_ENABLED", "1");
        assert!(InboxConsumerConfig::from_env().enabled);

        match previous {
            Some(value) => std::env::set_var("MINDVAULT_INBOX_CONSUMER_ENABLED", value),
            None => std::env::remove_var("MINDVAULT_INBOX_CONSUMER_ENABLED"),
        }
    }
}
