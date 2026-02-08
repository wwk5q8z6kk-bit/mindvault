pub mod audit;
pub mod auth;
pub mod email;
pub mod grpc;
pub mod limits;
pub mod metrics;
pub mod openapi;
pub mod rest;
pub mod state;
pub mod validation;
pub mod websocket;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use mv_core::ChronicleEntry;
use mv_engine::config::EngineConfig;
use mv_engine::engine::MindVaultEngine;
use mv_engine::intent::IntentEngine;
use mv_engine::watcher::WatcherAgent;
use state::AppState;
use uuid::Uuid;

pub struct ServerConfig {
    pub bind_host: String,
    pub rest_port: u16,
    pub grpc_port: u16,
    pub socket_path: Option<String>,
    pub cors_allowed_origins: Vec<String>,
    pub engine_config: EngineConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_host: "127.0.0.1".into(),
            rest_port: 9470,
            grpc_port: 50051,
            socket_path: Some(shellexpand("~/.mindvault/mindvault.sock")),
            cors_allowed_origins: Vec::new(),
            engine_config: EngineConfig::default(),
        }
    }
}

/// Start the MindVault server with all transports.
pub async fn start_server(
    config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mv_server=debug,mv_engine=debug".parse().unwrap()),
        )
        .init();
    rest::init_observability();

    tracing::info!("initializing MindVault engine...");
    let mut engine = MindVaultEngine::init(config.engine_config).await?;

    // Create broadcast channel early so enrichment can use it
    let (change_tx, _) = tokio::sync::broadcast::channel::<state::ChangeNotification>(256);

    // Set up enrichment pipeline (before wrapping in Arc)
    let enrichment_worker = engine.setup_enrichment(change_tx.clone());

    let engine = Arc::new(engine);
    engine.proactive.set_engine(Arc::clone(&engine));

    ensure_today_daily_note_on_startup_best_effort(&engine).await;
    spawn_daily_note_scheduler(Arc::clone(&engine));

    // Spawn enrichment worker if enabled
    if let Some(worker) = enrichment_worker {
        tokio::spawn(worker.run());
        tracing::info!("enrichment worker spawned");
    }

    // Shutdown broadcast for background agents
    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

    // Spawn watcher agent
    spawn_watcher_agent(Arc::clone(&engine), shutdown_tx.subscribe());

    let state = Arc::new(AppState::new_with_change_tx(engine, change_tx));
    spawn_agent_change_processor(Arc::clone(&state), shutdown_tx.subscribe());
    spawn_recurrence_and_reminder_scheduler(Arc::clone(&state));
    email::spawn_email_adapter(Arc::clone(&state), shutdown_tx.subscribe());

    // REST + WebSocket server
    let rest_state = Arc::clone(&state);
    let ws_state = Arc::clone(&state);
    let bind_host = config.bind_host.clone();
    let cors_allowed_origins = config.cors_allowed_origins.clone();
    let rest_port = config.rest_port;
    let rest_handle = tokio::spawn(async move {
        let app = rest::create_router_with_cors(rest_state, &cors_allowed_origins)
            .merge(websocket::ws_router(ws_state));
        tracing::info!("REST API listening on {bind_host}:{rest_port}");
        let listener = tokio::net::TcpListener::bind(format!("{bind_host}:{rest_port}"))
            .await
            .expect("failed to bind REST port");
        axum::serve(listener, app).await.ok();
    });

    // gRPC server
    let grpc_state = Arc::clone(&state);
    let grpc_bind_host = config.bind_host.clone();
    let grpc_port = config.grpc_port;
    let grpc_handle = tokio::spawn(async move {
        tracing::info!("gRPC API listening on {grpc_bind_host}:{grpc_port}");
        let addr = format!("{grpc_bind_host}:{grpc_port}").parse().unwrap();
        let service = grpc::MindVaultGrpc::new(Arc::clone(&grpc_state));
        let keychain_service = grpc::KeychainGrpc::new(grpc_state);
        tonic::transport::Server::builder()
            .add_service(
                grpc::proto::mind_vault_service_server::MindVaultServiceServer::with_interceptor(
                    service,
                    grpc::auth_interceptor,
                ),
            )
            .add_service(
                grpc::proto::keychain_service_server::KeychainServiceServer::with_interceptor(
                    keychain_service,
                    grpc::auth_interceptor,
                ),
            )
            .serve(addr)
            .await
            .ok();
    });

    // Unix Domain Socket (REST API over UDS)
    if let Some(ref sock_path) = config.socket_path {
        let uds_state = Arc::clone(&state);
        let sock = sock_path.clone();
        tokio::spawn(async move {
            let _ = std::fs::remove_file(&sock);
            if let Some(parent) = std::path::Path::new(&sock).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            tracing::info!("UDS listening on {sock}");
            let uds_listener = match tokio::net::UnixListener::bind(&sock) {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("failed to bind UDS at {sock}: {e}");
                    return;
                }
            };

            let app = rest::create_router(uds_state);
            loop {
                match uds_listener.accept().await {
                    Ok((stream, _addr)) => {
                        let app = app.clone();
                        tokio::spawn(async move {
                            let io = hyper_util::rt::TokioIo::new(stream);
                            let service = hyper::service::service_fn(move |req| {
                                let app = app.clone();
                                async move {
                                    let resp = tower::ServiceExt::oneshot(app, req).await;
                                    resp
                                }
                            });
                            if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                                hyper_util::rt::TokioExecutor::new(),
                            )
                            .serve_connection(io, service)
                            .await
                            {
                                tracing::error!("UDS connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("UDS accept error: {e}");
                    }
                }
            }
        });
    }

    tracing::info!("MindVault server started");

    tokio::select! {
        _ = rest_handle => {},
        _ = grpc_handle => {},
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down...");
            let _ = shutdown_tx.send(());
        }
    }

    Ok(())
}

fn spawn_watcher_agent(
    engine: Arc<MindVaultEngine>,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let watcher_config = engine.config.watcher.clone();
    if !watcher_config.enabled {
        tracing::info!("watcher agent disabled by config");
        return;
    }

    let intent_engine = IntentEngine::new(Arc::clone(&engine.store));
    let proactive_engine = Arc::clone(&engine.proactive);

    let agent = Arc::new(WatcherAgent::new(
        Arc::clone(&engine),
        intent_engine,
        proactive_engine,
        watcher_config,
    ));

    tokio::spawn(async move {
        agent.run_loop(shutdown_rx).await;
    });

    tracing::info!("watcher agent spawned");
}

fn spawn_agent_change_processor(
    state: Arc<AppState>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    if !state.engine.config.watcher.enabled {
        tracing::info!("agent change processor disabled by config");
        return;
    }

    let intent_engine = IntentEngine::new(Arc::clone(&state.engine.store));
    let mut change_rx = state.change_tx.subscribe();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("agent change processor shutting down");
                    break;
                }
                event = change_rx.recv() => {
                    match event {
                        Ok(notification) => {
                            process_agent_change_notification(&state, &intent_engine, notification).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("agent change processor lagged by {n} events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    tracing::info!("agent change processor spawned");
}

async fn process_agent_change_notification(
    state: &Arc<AppState>,
    intent_engine: &IntentEngine,
    notification: state::ChangeNotification,
) {
    match notification.operation.as_str() {
        "create" | "update" | "enriched" => {}
        _ => return,
    }

    let node_id = match Uuid::parse_str(&notification.node_id) {
        Ok(id) => id,
        Err(err) => {
            tracing::warn!(
                node_id = notification.node_id,
                error = %err,
                "agent change processor received invalid node id"
            );
            return;
        }
    };

    let node = match state.engine.get_node(node_id).await {
        Ok(Some(node)) => node,
        Ok(None) => return,
        Err(err) => {
            tracing::warn!(node_id = %node_id, error = %err, "agent change processor failed to load node");
            return;
        }
    };

    let namespace = Some(node.namespace.clone());

    if notification.operation == "enriched" {
        state.notify_agent(state::AgentNotification::NodeEnriched {
            node_id: node.id.to_string(),
            namespace,
        });
        return;
    }

    let observed_entry = ChronicleEntry::new(
        "agent_change_observed",
        format!(
            "Observed {} operation on node {}",
            notification.operation, notification.node_id
        ),
    )
    .with_node(node.id);

    if let Err(err) = state.engine.log_chronicle(&observed_entry).await {
        tracing::warn!(node_id = %node.id, error = %err, "failed to log agent change chronicle");
    } else {
        state.notify_agent(state::AgentNotification::Chronicle {
            entry: observed_entry,
            namespace: Some(node.namespace.clone()),
        });
    }

    match intent_engine.extract_intents_and_store(&node).await {
        Ok(intents) => {
            for intent in intents {
                state.notify_agent(state::AgentNotification::Intent {
                    intent: intent.clone(),
                    namespace: Some(node.namespace.clone()),
                });

                let entry = ChronicleEntry::new(
                    "intent_detected",
                    format!(
                        "Detected {} intent with {:.0}% confidence",
                        intent.intent_type,
                        intent.confidence * 100.0
                    ),
                )
                .with_node(node.id);
                if let Err(err) = state.engine.log_chronicle(&entry).await {
                    tracing::warn!(node_id = %node.id, error = %err, "failed to log intent chronicle");
                } else {
                    state.notify_agent(state::AgentNotification::Chronicle {
                        entry,
                        namespace: Some(node.namespace.clone()),
                    });
                }
            }
        }
        Err(err) => {
            tracing::warn!(node_id = %node.id, error = %err, "intent extraction failed");
        }
    }

    match state
        .engine
        .proactive
        .find_related_context(node.id, 5)
        .await
    {
        Ok(related) => {
            let nodes: Vec<state::AgentRelatedNode> = related
                .into_iter()
                .map(|related_node| state::AgentRelatedNode {
                    id: related_node.id.to_string(),
                    title: related_node.title.unwrap_or_else(|| "Untitled".to_string()),
                    updated_at: related_node.temporal.updated_at.to_rfc3339(),
                })
                .collect();

            if !nodes.is_empty() {
                state.notify_agent(state::AgentNotification::RelatedContext {
                    nodes,
                    namespace: Some(node.namespace.clone()),
                });
            }
        }
        Err(err) => {
            tracing::warn!(node_id = %node.id, error = %err, "related context discovery failed");
        }
    }

    match state
        .engine
        .proactive
        .generate_insights(node.namespace.clone())
        .await
    {
        Ok(insights) => {
            for insight in insights {
                state.notify_agent(state::AgentNotification::InsightDiscovered {
                    insight: insight.clone(),
                    namespace: Some(node.namespace.clone()),
                });

                let entry = ChronicleEntry::new(
                    "insight_generated",
                    format!("{}: {}", insight.insight_type, insight.title),
                )
                .with_node(node.id);
                if let Err(err) = state.engine.log_chronicle(&entry).await {
                    tracing::warn!(node_id = %node.id, error = %err, "failed to log insight chronicle");
                } else {
                    state.notify_agent(state::AgentNotification::Chronicle {
                        entry,
                        namespace: Some(node.namespace.clone()),
                    });
                }
            }
        }
        Err(err) => {
            tracing::warn!(node_id = %node.id, error = %err, "proactive insight generation failed");
        }
    }
}

async fn ensure_today_daily_note_on_startup_best_effort(engine: &Arc<MindVaultEngine>) {
    if !engine.config.daily_notes.enabled {
        return;
    }

    let today = Utc::now().date_naive();
    match engine.ensure_daily_note(today, None).await {
        Ok((_node, created)) => {
            tracing::info!(
                date = %today,
                created,
                namespace = %engine.config.daily_notes.namespace,
                "mindvault_daily_note_startup_ensure_complete"
            );
        }
        Err(err) => {
            tracing::warn!(
                date = %today,
                namespace = %engine.config.daily_notes.namespace,
                error = %err,
                "mindvault_daily_note_startup_ensure_failed"
            );
        }
    }
}

fn spawn_daily_note_scheduler(engine: Arc<MindVaultEngine>) {
    if !daily_note_scheduler_enabled(&engine.config) {
        if engine.config.daily_notes.enabled
            && !engine.config.daily_notes.midnight_scheduler_enabled
        {
            tracing::info!(
                namespace = %engine.config.daily_notes.namespace,
                "mindvault_daily_note_scheduler_disabled_by_config"
            );
        }
        return;
    }

    tokio::spawn(async move {
        loop {
            let sleep_duration = duration_until_next_utc_midnight(Utc::now());
            tracing::info!(
                sleep_seconds = sleep_duration.as_secs(),
                namespace = %engine.config.daily_notes.namespace,
                "mindvault_daily_note_scheduler_sleep_until_next_utc_midnight"
            );
            tokio::time::sleep(sleep_duration).await;

            let today = Utc::now().date_naive();
            match engine.ensure_daily_note(today, None).await {
                Ok((_node, created)) => {
                    tracing::info!(
                        date = %today,
                        created,
                        namespace = %engine.config.daily_notes.namespace,
                        "mindvault_daily_note_scheduler_ensure_complete"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        date = %today,
                        namespace = %engine.config.daily_notes.namespace,
                        error = %err,
                        "mindvault_daily_note_scheduler_ensure_failed"
                    );
                }
            }
        }
    });
}

fn daily_note_scheduler_enabled(config: &EngineConfig) -> bool {
    config.daily_notes.enabled && config.daily_notes.midnight_scheduler_enabled
}

fn spawn_recurrence_and_reminder_scheduler(state: Arc<AppState>) {
    if !state.engine.config.recurrence.enabled {
        return;
    }

    let interval_secs = state
        .engine
        .config
        .recurrence
        .scheduler_interval_secs
        .max(30);
    let max_instances_per_template = state
        .engine
        .config
        .recurrence
        .max_instances_per_template
        .max(1);
    tokio::spawn(async move {
        loop {
            let now = Utc::now();
            match state
                .engine
                .rollforward_recurring_tasks(now, max_instances_per_template)
                .await
            {
                Ok(stats) => {
                    tracing::info!(
                        scanned_tasks = stats.scanned_tasks,
                        recurring_templates = stats.recurring_templates,
                        generated_instances = stats.generated_instances,
                        updated_templates = stats.updated_templates,
                        errors = stats.errors,
                        interval_secs,
                        "mindvault_recurrence_rollforward_cycle_complete"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        interval_secs,
                        "mindvault_recurrence_rollforward_cycle_failed"
                    );
                }
            }

            // Dispatch task reminders with WebSocket/webhook notifications
            dispatch_task_reminders_with_notifications(&state, now, interval_secs).await;

            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    });
}

async fn dispatch_task_reminders_with_notifications(
    state: &Arc<AppState>,
    now: DateTime<Utc>,
    interval_secs: u64,
) {
    use mv_engine::recurrence::{
        parse_optional_metadata_datetime, TASK_DUE_AT_METADATA_KEY,
        TASK_REMINDER_SENT_AT_METADATA_KEY,
    };

    // First, get the list of due tasks before marking them
    let due_tasks = match state.engine.list_due_tasks(now, None, 500, false).await {
        Ok(tasks) => tasks,
        Err(err) => {
            tracing::warn!(
                error = %err,
                interval_secs,
                "mindvault_task_reminder_list_due_failed"
            );
            return;
        }
    };

    // Filter to those not yet notified
    let mut tasks_to_notify = Vec::new();
    for task in due_tasks {
        let already_sent =
            parse_optional_metadata_datetime(&task.metadata, TASK_REMINDER_SENT_AT_METADATA_KEY)
                .map(|v| v.is_some())
                .unwrap_or(false);

        if !already_sent {
            tasks_to_notify.push(task);
        }
    }

    // Now dispatch reminders (marks them as sent)
    match state.engine.dispatch_due_task_reminders(now, 500).await {
        Ok(stats) => {
            tracing::info!(
                scanned_tasks = stats.scanned_tasks,
                due_tasks = stats.due_tasks,
                reminders_marked_sent = stats.reminders_marked_sent,
                errors = stats.errors,
                interval_secs,
                "mindvault_task_reminder_dispatch_cycle_complete"
            );

            // Send WebSocket/webhook notifications for tasks we just marked
            for task in tasks_to_notify {
                let due_at =
                    parse_optional_metadata_datetime(&task.metadata, TASK_DUE_AT_METADATA_KEY)
                        .ok()
                        .flatten();

                let content_preview = task.content.chars().take(200).collect::<String>();

                let notification = state::ReminderNotification {
                    node_id: task.id.to_string(),
                    title: task.title.clone(),
                    content_preview,
                    due_at,
                    namespace: Some(task.namespace.clone()),
                    timestamp: now,
                    notification_type: "task_due".to_string(),
                };

                state.notify_reminder(notification);
            }
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                interval_secs,
                "mindvault_task_reminder_dispatch_cycle_failed"
            );
        }
    }
}

fn duration_until_next_utc_midnight(now: DateTime<Utc>) -> std::time::Duration {
    let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
    let next_midnight_naive = tomorrow
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| now.naive_utc());
    let next_midnight = DateTime::<Utc>::from_naive_utc_and_offset(next_midnight_naive, Utc);
    let seconds = (next_midnight - now).num_seconds().max(1) as u64;
    std::time::Duration::from_secs(seconds)
}

fn shellexpand(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn daily_note_scheduler_respects_enabled_flags() {
        let mut config = EngineConfig::default();
        config.daily_notes.enabled = true;
        config.daily_notes.midnight_scheduler_enabled = true;
        assert!(daily_note_scheduler_enabled(&config));

        config.daily_notes.midnight_scheduler_enabled = false;
        assert!(!daily_note_scheduler_enabled(&config));

        config.daily_notes.enabled = false;
        config.daily_notes.midnight_scheduler_enabled = true;
        assert!(!daily_note_scheduler_enabled(&config));
    }

    #[test]
    fn duration_until_next_midnight_from_midday_is_half_day() {
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 12, 0, 0)
            .single()
            .expect("valid datetime");
        let duration = duration_until_next_utc_midnight(now);
        assert_eq!(duration.as_secs(), 12 * 60 * 60);
    }

    #[test]
    fn duration_until_next_midnight_from_exact_midnight_is_full_day() {
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 0, 0, 0)
            .single()
            .expect("valid datetime");
        let duration = duration_until_next_utc_midnight(now);
        assert_eq!(duration.as_secs(), 24 * 60 * 60);
    }

    #[test]
    fn duration_until_next_midnight_never_zero() {
        let now = Utc
            .with_ymd_and_hms(2026, 2, 6, 23, 59, 59)
            .single()
            .expect("valid datetime");
        let duration = duration_until_next_utc_midnight(now);
        assert!(duration.as_secs() >= 1);
    }
}
