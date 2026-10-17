use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use mv_engine::workspace::decode_workspace_descriptor;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Instant, MissedTickBehavior};
use uuid::Uuid;

use crate::state::AppState;

const EVENT_QUEUE_CAPACITY: usize = 1_024;
const DEFAULT_DEBOUNCE_MS: u64 = 750;
const DEFAULT_DISCOVERY_INTERVAL_SECS: u64 = 10;
const DEFAULT_FULL_SCAN_INTERVAL_SECS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceWatchConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
    pub discovery_interval_secs: u64,
    pub full_scan_interval_secs: u64,
}

impl WorkspaceWatchConfig {
    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let enabled = parse_bool(lookup("MINDVAULT_WORKSPACE_WATCH_ENABLED"), true);
        let debounce_ms = parse_u64(
            lookup("MINDVAULT_WORKSPACE_WATCH_DEBOUNCE_MS"),
            DEFAULT_DEBOUNCE_MS,
        )
        .max(50);
        let discovery_interval_secs = parse_u64(
            lookup("MINDVAULT_WORKSPACE_WATCH_DISCOVERY_INTERVAL_SECS"),
            DEFAULT_DISCOVERY_INTERVAL_SECS,
        )
        .max(2);
        let full_scan_interval_secs = parse_u64(
            lookup("MINDVAULT_WORKSPACE_FULL_SCAN_INTERVAL_SECS"),
            DEFAULT_FULL_SCAN_INTERVAL_SECS,
        )
        .max(15);

        Self {
            enabled,
            debounce_ms,
            discovery_interval_secs,
            full_scan_interval_secs,
        }
    }
}

#[derive(Debug, Clone)]
struct KnownWorkspace {
    namespace: String,
    root: PathBuf,
    watched: bool,
    watch_error_reported: bool,
}

pub fn spawn_workspace_watcher(
    state: Arc<AppState>,
    shutdown_rx: broadcast::Receiver<()>,
) -> Option<tokio::task::JoinHandle<()>> {
    let config = WorkspaceWatchConfig::from_env();
    if !config.enabled {
        tracing::info!("knowledge workspace filesystem watcher disabled");
        return None;
    }

    tracing::info!(
        debounce_ms = config.debounce_ms,
        discovery_interval_secs = config.discovery_interval_secs,
        full_scan_interval_secs = config.full_scan_interval_secs,
        event_queue_capacity = EVENT_QUEUE_CAPACITY,
        "knowledge workspace filesystem watcher spawning"
    );

    Some(tokio::spawn(run_workspace_watcher(
        state,
        config,
        shutdown_rx,
    )))
}

async fn run_workspace_watcher(
    state: Arc<AppState>,
    config: WorkspaceWatchConfig,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let (event_tx, mut event_rx) = mpsc::channel::<notify::Result<Event>>(EVENT_QUEUE_CAPACITY);
    let event_queue_overflowed = Arc::new(AtomicBool::new(false));
    let callback_overflowed = Arc::clone(&event_queue_overflowed);
    let mut native_watcher =
        match notify::recommended_watcher(move |event: notify::Result<Event>| {
            match event_tx.try_send(event) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    callback_overflowed.store(true, Ordering::Release);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {}
            }
        }) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "native workspace watcher unavailable; periodic full scans remain active"
                );
                None
            }
        };
    let mut native_events_available = native_watcher.is_some();
    let mut known = HashMap::new();
    let mut pending = HashMap::<Uuid, Instant>::new();

    refresh_known_workspaces(&state, &mut native_watcher, &mut known).await;

    let mut discovery_tick =
        tokio::time::interval(Duration::from_secs(config.discovery_interval_secs));
    discovery_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    discovery_tick.tick().await;

    let mut full_scan_tick =
        tokio::time::interval(Duration::from_secs(config.full_scan_interval_secs));
    full_scan_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    full_scan_tick.tick().await;

    let pending_tick_ms = config.debounce_ms.clamp(50, 250);
    let mut pending_tick = tokio::time::interval(Duration::from_millis(pending_tick_ms));
    pending_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    pending_tick.tick().await;

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("knowledge workspace filesystem watcher shutting down");
                break;
            }
            event = event_rx.recv(), if native_events_available => {
                match event {
                    Some(Ok(event)) => {
                        let affected = affected_workspace_ids(&event, &known);
                        schedule_workspaces(
                            &mut pending,
                            affected,
                            Duration::from_millis(config.debounce_ms),
                        );
                    }
                    Some(Err(error)) => {
                        tracing::warn!(
                            error = %error,
                            "native workspace watcher reported an error; scheduling fallback reconciliation"
                        );
                        schedule_all_now(&mut pending, &known);
                    }
                    None => {
                        tracing::warn!(
                            "native workspace event channel closed; periodic full scans remain active"
                        );
                        native_events_available = false;
                    }
                }
            }
            _ = pending_tick.tick() => {
                if event_queue_overflowed.swap(false, Ordering::AcqRel) {
                    tracing::warn!(
                        "workspace event queue overflowed; scheduling all workspaces for reconciliation"
                    );
                    schedule_all_now(&mut pending, &known);
                }
                reconcile_due(&state, &known, &mut pending).await;
            }
            _ = discovery_tick.tick() => {
                refresh_known_workspaces(&state, &mut native_watcher, &mut known).await;
            }
            _ = full_scan_tick.tick() => {
                refresh_known_workspaces(&state, &mut native_watcher, &mut known).await;
                reconcile_all(&state, &known, &mut pending).await;
            }
        }
    }
}

async fn refresh_known_workspaces(
    state: &AppState,
    watcher: &mut Option<RecommendedWatcher>,
    known: &mut HashMap<Uuid, KnownWorkspace>,
) {
    let workspaces = match state.engine.list_knowledge_workspaces(None).await {
        Ok(workspaces) => workspaces,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "could not discover mounted knowledge workspaces"
            );
            return;
        }
    };

    let mut previous = std::mem::take(known);
    let mut next = HashMap::with_capacity(workspaces.len());

    for workspace in workspaces {
        let descriptor = match decode_workspace_descriptor(&workspace) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                tracing::warn!(
                    workspace_id = %workspace.id,
                    error = %error,
                    "workspace descriptor could not be decoded for filesystem watching"
                );
                continue;
            }
        };
        let root = PathBuf::from(descriptor.root_path);
        let mut entry = match previous.remove(&workspace.id) {
            Some(existing) if existing.root == root => KnownWorkspace {
                namespace: workspace.namespace,
                ..existing
            },
            Some(existing) => {
                unwatch_workspace(watcher, workspace.id, &existing);
                KnownWorkspace {
                    namespace: workspace.namespace,
                    root,
                    watched: false,
                    watch_error_reported: false,
                }
            }
            None => KnownWorkspace {
                namespace: workspace.namespace,
                root,
                watched: false,
                watch_error_reported: false,
            },
        };

        if entry.watched && !entry.root.is_dir() {
            unwatch_workspace(watcher, workspace.id, &entry);
            entry.watched = false;
        }

        if !entry.watched {
            try_watch_workspace(watcher, workspace.id, &mut entry);
        }

        next.insert(workspace.id, entry);
    }

    for (workspace_id, entry) in previous {
        unwatch_workspace(watcher, workspace_id, &entry);
    }
    *known = next;
}

fn try_watch_workspace(
    watcher: &mut Option<RecommendedWatcher>,
    workspace_id: Uuid,
    entry: &mut KnownWorkspace,
) {
    let Some(watcher) = watcher.as_mut() else {
        return;
    };
    match watcher.watch(&entry.root, RecursiveMode::Recursive) {
        Ok(()) => {
            entry.watched = true;
            entry.watch_error_reported = false;
            tracing::info!(
                workspace_id = %workspace_id,
                root = %entry.root.display(),
                "watching mounted knowledge workspace"
            );
        }
        Err(error) => {
            if !entry.watch_error_reported {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    root = %entry.root.display(),
                    error = %error,
                    "could not watch workspace root; periodic full scans remain active"
                );
                entry.watch_error_reported = true;
            } else {
                tracing::debug!(
                    workspace_id = %workspace_id,
                    root = %entry.root.display(),
                    error = %error,
                    "workspace root is still unavailable to the native watcher"
                );
            }
        }
    }
}

fn unwatch_workspace(
    watcher: &mut Option<RecommendedWatcher>,
    workspace_id: Uuid,
    entry: &KnownWorkspace,
) {
    if !entry.watched {
        return;
    }
    if let Some(watcher) = watcher.as_mut() {
        if let Err(error) = watcher.unwatch(&entry.root) {
            tracing::debug!(
                workspace_id = %workspace_id,
                root = %entry.root.display(),
                error = %error,
                "workspace root could not be cleanly unwatched"
            );
        }
    }
}

fn affected_workspace_ids(event: &Event, known: &HashMap<Uuid, KnownWorkspace>) -> Vec<Uuid> {
    if !is_mutating_event(event.kind) {
        return Vec::new();
    }
    if event.paths.is_empty() || event.paths.iter().any(|path| !path.is_absolute()) {
        return known.keys().copied().collect();
    }

    known
        .iter()
        .filter_map(|(workspace_id, workspace)| {
            event
                .paths
                .iter()
                .any(|path| is_relevant_workspace_path(path, &workspace.root))
                .then_some(*workspace_id)
        })
        .collect()
}

fn is_mutating_event(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn is_relevant_workspace_path(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    if relative.as_os_str().is_empty() {
        return true;
    }
    if relative.components().any(is_reserved_component) {
        return false;
    }
    if path.is_dir() {
        return true;
    }

    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
    {
        Some(extension) => extension == "md" || extension == "markdown",
        None => true,
    }
}

fn is_reserved_component(component: Component<'_>) -> bool {
    let Component::Normal(value) = component else {
        return false;
    };
    let value = value.to_string_lossy().to_ascii_lowercase();
    value == ".git" || value == ".mindvault" || value.starts_with(".mindvault-")
}

fn schedule_workspaces(
    pending: &mut HashMap<Uuid, Instant>,
    workspace_ids: impl IntoIterator<Item = Uuid>,
    debounce: Duration,
) {
    let deadline = Instant::now() + debounce;
    for workspace_id in workspace_ids {
        pending.entry(workspace_id).or_insert(deadline);
    }
}

fn schedule_all_now(pending: &mut HashMap<Uuid, Instant>, known: &HashMap<Uuid, KnownWorkspace>) {
    let now = Instant::now();
    for workspace_id in known.keys() {
        pending.insert(*workspace_id, now);
    }
}

async fn reconcile_due(
    state: &AppState,
    known: &HashMap<Uuid, KnownWorkspace>,
    pending: &mut HashMap<Uuid, Instant>,
) {
    let now = Instant::now();
    let due = pending
        .iter()
        .filter_map(|(workspace_id, deadline)| (*deadline <= now).then_some(*workspace_id))
        .collect::<Vec<_>>();
    for workspace_id in due {
        pending.remove(&workspace_id);
        if let Some(workspace) = known.get(&workspace_id) {
            reconcile_one(state, workspace_id, workspace, "filesystem_event").await;
        }
    }
}

async fn reconcile_all(
    state: &AppState,
    known: &HashMap<Uuid, KnownWorkspace>,
    pending: &mut HashMap<Uuid, Instant>,
) {
    for (workspace_id, workspace) in known {
        pending.remove(workspace_id);
        reconcile_one(state, *workspace_id, workspace, "periodic_full_scan").await;
    }
}

async fn reconcile_one(
    state: &AppState,
    workspace_id: Uuid,
    workspace: &KnownWorkspace,
    trigger: &'static str,
) {
    let started_at = Instant::now();
    match state
        .engine
        .reconcile_knowledge_workspace(workspace_id)
        .await
    {
        Ok(outcome) if outcome.applied => {
            state.notify_change(
                &workspace_id.to_string(),
                "workspace_reconciled",
                Some(&workspace.namespace),
            );
            tracing::info!(
                workspace_id = %workspace_id,
                trigger,
                inserted_documents = outcome.inserted_documents,
                updated_documents = outcome.updated_documents,
                unchanged_documents = outcome.unchanged_documents,
                renamed_documents = outcome.renamed_documents,
                diagnostics = outcome.diagnostics.len(),
                duration_ms = started_at.elapsed().as_millis() as u64,
                "knowledge workspace reconciliation completed"
            );
        }
        Ok(_) => {
            tracing::debug!(
                workspace_id = %workspace_id,
                trigger,
                "workspace reconciliation lost an optimistic revision; a later scan will retry"
            );
        }
        Err(error) => {
            tracing::warn!(
                workspace_id = %workspace_id,
                trigger,
                error = %error,
                duration_ms = started_at.elapsed().as_millis() as u64,
                "knowledge workspace reconciliation failed"
            );
        }
    }
}

fn parse_bool(value: Option<String>, default: bool) -> bool {
    match value.as_deref().map(str::trim).map(str::to_ascii_lowercase) {
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

fn parse_u64(value: Option<String>, default: u64) -> u64 {
    value
        .as_deref()
        .map(str::trim)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_engine::config::EngineConfig;
    use mv_engine::engine::MindVaultEngine;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn config_from(values: &[(&str, &str)]) -> WorkspaceWatchConfig {
        let values = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>();
        WorkspaceWatchConfig::from_lookup(|key| values.get(key).cloned())
    }

    fn known(root: PathBuf) -> KnownWorkspace {
        KnownWorkspace {
            namespace: "personal".into(),
            root,
            watched: true,
            watch_error_reported: false,
        }
    }

    #[test]
    fn config_defaults_are_bounded_and_enabled() {
        assert_eq!(
            config_from(&[]),
            WorkspaceWatchConfig {
                enabled: true,
                debounce_ms: DEFAULT_DEBOUNCE_MS,
                discovery_interval_secs: DEFAULT_DISCOVERY_INTERVAL_SECS,
                full_scan_interval_secs: DEFAULT_FULL_SCAN_INTERVAL_SECS,
            }
        );
    }

    #[test]
    fn config_parses_switches_and_clamps_busy_intervals() {
        let config = config_from(&[
            ("MINDVAULT_WORKSPACE_WATCH_ENABLED", "off"),
            ("MINDVAULT_WORKSPACE_WATCH_DEBOUNCE_MS", "1"),
            ("MINDVAULT_WORKSPACE_WATCH_DISCOVERY_INTERVAL_SECS", "0"),
            ("MINDVAULT_WORKSPACE_FULL_SCAN_INTERVAL_SECS", "2"),
        ]);
        assert!(!config.enabled);
        assert_eq!(config.debounce_ms, 50);
        assert_eq!(config.discovery_interval_secs, 2);
        assert_eq!(config.full_scan_interval_secs, 15);
    }

    #[test]
    fn markdown_and_directory_events_target_every_containing_workspace() {
        let directory = tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let nested = root.join("Projects");
        std::fs::create_dir(&nested).unwrap();
        let document = nested.join("plan.md");

        let parent_id = Uuid::now_v7();
        let nested_id = Uuid::now_v7();
        let workspaces = HashMap::from([(parent_id, known(root)), (nested_id, known(nested))]);
        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(document);
        let affected = affected_workspace_ids(&event, &workspaces);

        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&parent_id));
        assert!(affected.contains(&nested_id));
    }

    #[test]
    fn non_markdown_and_reserved_events_are_filtered() {
        let directory = tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let workspace_id = Uuid::now_v7();
        let workspaces = HashMap::from([(workspace_id, known(root.clone()))]);

        for path in [root.join("image.png"), root.join(".git/index")] {
            let event = Event::new(EventKind::Create(CreateKind::Any)).add_path(path);
            assert!(affected_workspace_ids(&event, &workspaces).is_empty());
        }
    }

    #[test]
    fn access_and_other_events_do_not_schedule_reconciliation() {
        let directory = tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let workspace_id = Uuid::now_v7();
        let workspaces = HashMap::from([(workspace_id, known(root.clone()))]);

        for kind in [EventKind::Access(AccessKind::Any), EventKind::Other] {
            let event = Event::new(kind).add_path(root.join("note.md"));
            assert!(affected_workspace_ids(&event, &workspaces).is_empty());
        }
    }

    #[test]
    fn remove_events_without_paths_fall_back_to_every_workspace() {
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        let workspaces = HashMap::from([
            (first, known(PathBuf::from("/tmp/first"))),
            (second, known(PathBuf::from("/tmp/second"))),
        ]);
        let event = Event::new(EventKind::Remove(RemoveKind::Any));
        let affected = affected_workspace_ids(&event, &workspaces);

        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&first));
        assert!(affected.contains(&second));
    }

    #[tokio::test(start_paused = true)]
    async fn debounce_uses_the_first_deadline_to_avoid_starvation() {
        let workspace_id = Uuid::now_v7();
        let mut pending = HashMap::new();
        schedule_workspaces(&mut pending, [workspace_id], Duration::from_millis(750));
        let first_deadline = pending[&workspace_id];

        tokio::time::advance(Duration::from_millis(500)).await;
        schedule_workspaces(&mut pending, [workspace_id], Duration::from_millis(750));

        assert_eq!(pending[&workspace_id], first_deadline);
    }

    #[tokio::test]
    async fn watcher_reconciles_a_real_mounted_workspace_after_a_file_change() {
        let directory = tempdir().unwrap();
        let root = directory.path().join("knowledge");
        let data_dir = directory.path().join("data");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("existing.md"), "# Existing").unwrap();

        let mut engine_config = EngineConfig {
            data_dir: data_dir.to_string_lossy().into_owned(),
            ..Default::default()
        };
        engine_config.embedding.provider = "noop".into();
        let engine = Arc::new(MindVaultEngine::init(engine_config).await.unwrap());
        let mounted = engine
            .mount_knowledge_workspace(&root, "personal", "Watcher test")
            .await
            .unwrap();
        let workspace_id = mounted.workspace.id;
        let state = Arc::new(AppState::new(Arc::clone(&engine)));
        let mut changes = state.change_tx.subscribe();
        let (shutdown_tx, _) = broadcast::channel(1);

        let watcher_task = tokio::spawn(run_workspace_watcher(
            Arc::clone(&state),
            WorkspaceWatchConfig {
                enabled: true,
                debounce_ms: 50,
                discovery_interval_secs: 1,
                // Keep the test correct on hosts where native notifications
                // are unavailable while still exercising the same coordinator.
                full_scan_interval_secs: 2,
            },
            shutdown_tx.subscribe(),
        ));

        // The coordinator performs initial discovery before entering its
        // receive loop. Give that synchronous watcher registration a bounded
        // opportunity to complete before producing the filesystem event.
        tokio::time::sleep(Duration::from_millis(250)).await;
        std::fs::write(root.join("added.md"), "# Added\n\nDetected automatically.").unwrap();

        let notification = tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                let notification = changes.recv().await.unwrap();
                if notification.node_id == workspace_id.to_string()
                    && notification.operation == "workspace_reconciled"
                {
                    break notification;
                }
            }
        })
        .await
        .expect("watcher should reconcile the changed workspace");

        assert_eq!(notification.namespace.as_deref(), Some("personal"));
        let workspace = engine.get_knowledge_workspace(workspace_id).await.unwrap();
        assert!(workspace.revision > mounted.workspace.revision);
        let tree = engine.knowledge_workspace_tree(workspace_id).await.unwrap();
        let added = tree
            .entries
            .iter()
            .find(|entry| entry.relative_path == "added.md")
            .expect("new document should be visible in the reconciled tree");
        assert!(added.document_id.is_some());

        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), watcher_task)
            .await
            .expect("watcher should stop after shutdown")
            .unwrap();
    }
}
