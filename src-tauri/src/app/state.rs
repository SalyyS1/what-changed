//! Shared application state and the crash-loop guard.
//!
//! `AppState` is what every command reaches the backend through. The
//! `StartupGuard` counts unclean starts in a small file beside the database:
//! three in a row and the app boots with collectors off, so a bad state
//! cannot become a crash loop that locks the user out of their own history.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::data_lifecycle::{self, DataLifecycle};
use super::event_bus::EventBus;
use super::monitoring::Monitoring;
use super::paths::AppPaths;
use super::rate_limit::RateLimiter;
use crate::checkpoint::{auto::AutomaticCheckpointManager, CheckpointService};
use crate::collectors::registry::CollectorRegistry;
use crate::collectors::runtime::CollectorSupervisor;
use crate::collectors::schedule::QueryBusy;
use crate::dto::{AppStatusDto, SettingsDto, SettingsUpdateDto};
use crate::error::{AppError, AppResult};
use crate::owner_workspace::OwnerStore;
use crate::storage::{settings_repository, Db, MaintenanceHandle};

/// Unclean starts in a row that trigger safe mode.
pub const SAFE_MODE_THRESHOLD: u32 = 3;

/// The marker file's contents.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct StartupMarker {
    /// Starts since the last clean shutdown.
    unclean_starts: u32,
    /// When the marker was last written, epoch milliseconds UTC.
    last_start_at: i64,
}

/// What the marker said when the app started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupGuard {
    /// How many starts preceded this one without a clean shutdown.
    pub previous_unclean_starts: u32,
    /// Whether collectors should stay off this run.
    pub safe_mode: bool,
}

impl StartupGuard {
    /// Read the marker, bump it, and write it back. Call once at startup,
    /// before anything that might crash.
    pub fn arm(marker: &Path) -> AppResult<StartupGuard> {
        let previous = fs::read_to_string(marker)
            .ok()
            .and_then(|text| serde_json::from_str::<StartupMarker>(&text).ok())
            .unwrap_or_default();
        let next = StartupMarker {
            unclean_starts: previous.unclean_starts.saturating_add(1),
            last_start_at: crate::domain::now_ms(),
        };
        write_marker(marker, &next)?;
        let safe_mode = previous.unclean_starts >= SAFE_MODE_THRESHOLD;
        if safe_mode {
            tracing::warn!(
                unclean_starts = previous.unclean_starts,
                "starting in safe mode: collectors stay off until the next clean shutdown"
            );
        }
        Ok(StartupGuard {
            previous_unclean_starts: previous.unclean_starts,
            safe_mode,
        })
    }

    /// Record a clean shutdown. Call from the quit path.
    pub fn disarm(marker: &Path) -> AppResult<()> {
        write_marker(
            marker,
            &StartupMarker {
                unclean_starts: 0,
                last_start_at: crate::domain::now_ms(),
            },
        )
    }
}

fn write_marker(marker: &Path, contents: &StartupMarker) -> AppResult<()> {
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::Directory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let text = serde_json::to_string(contents).unwrap_or_else(|_| "{}".to_owned());
    fs::write(marker, text).map_err(|source| AppError::File {
        operation: "write",
        path: marker.to_path_buf(),
        source,
    })
}

/// What the tray menu needs in order to reflect monitoring.
///
/// Boxed behind a trait so `AppState` does not carry Tauri's `Runtime`
/// parameter: a state generic over the runtime would infect every command
/// signature and every test that constructs one.
pub trait TrayReflector: Send + Sync {
    /// Enable exactly the applicable one of Pause and Resume.
    fn reflect(&self, monitoring: crate::dto::MonitoringDto);
}

/// Everything a command can reach.
pub struct AppState {
    /// Where the files are.
    pub paths: AppPaths,
    /// Cross-store lifecycle barrier and durable purge journal.
    pub data_lifecycle: Arc<DataLifecycle>,
    /// Independent owner-workspace store. Its failure is surfaced in its own
    /// status and never prevents legacy history from opening.
    pub owner_store: Arc<OwnerStore>,
    pub owner_portability: crate::owner_workspace::portability::cache::PortabilityReviewCache,
    pub preview_runner: crate::preview_runner::PreviewRunner,
    pub(crate) preview_retention: Mutex<Option<crate::preview_runner::CaptureRetention>>,
    /// The database, in whatever state it opened in.
    pub db: Arc<Db>,
    /// What the crash-loop guard found at startup.
    pub startup: StartupGuard,
    /// Whether collection is running.
    pub monitoring: Arc<Monitoring>,
    /// Gate used to defer expensive sweeps while a visible query is running.
    pub query_busy: QueryBusy,
    /// Platform sleep/resume watcher. Dropped before the supervisor.
    resume_monitor: Mutex<Option<Box<dyn Send>>>,
    /// The cross-platform collector supervisor.
    pub collectors: CollectorSupervisor,
    /// Known-good checkpoint orchestration over the live collector workers.
    pub checkpoints: CheckpointService,
    checkpoint_automation: Mutex<Option<AutomaticCheckpointManager>>,
    /// Keep the application sink when automation is paused for a purge.
    checkpoint_sink: Mutex<Option<Arc<dyn crate::checkpoint::auto::CheckpointSavedSink>>>,
    /// A failed purge can be retried without forgetting a detached watcher.
    resume_after_purge: AtomicBool,
    /// Serializes settings persistence with the matching runtime transition.
    collector_config: Mutex<()>,
    /// The cap on the frontend's own error reporting.
    pub ui_errors: RateLimiter,
    /// The tray menu, once it has been built. `None` means the tray failed to
    /// build, which is what keeps the window closable.
    tray: Mutex<Option<Box<dyn TrayReflector>>>,
    /// Set once the clean-shutdown marker has been written, so quitting
    /// through the tray and the exit event do not both do it.
    shutdown_recorded: AtomicBool,
    /// The coalescing thread. Held so it runs for the life of the app.
    bus: Mutex<Option<EventBus>>,
    /// The retention schedule. Held so it runs for the life of the app.
    maintenance: Mutex<Option<MaintenanceHandle>>,
}

impl AppState {
    /// Open the database under `paths` and start maintenance.
    ///
    /// Never fails on corruption: the database opens into a state the UI
    /// can show, and the app boots either way.
    pub fn open(paths: AppPaths) -> AppResult<AppState> {
        Self::open_with_registry(paths, CollectorRegistry::production())
    }

    /// Open with a platform-composed collector registry.
    ///
    /// The ordinary path deliberately uses the all-unavailable canonical
    /// catalog. Platform startup code injects activated factories here while
    /// tests can remain deterministic on every host.
    pub fn open_with_registry(paths: AppPaths, registry: CollectorRegistry) -> AppResult<AppState> {
        // Recover a pending purge before either SQLite store is opened or
        // migrated. This is intentionally the first storage action at boot.
        let data_lifecycle = Arc::new(DataLifecycle::prepare(&paths)?);
        let owner_store = Arc::new(OwnerStore::open(&paths.owner_workspace())?);
        let startup = StartupGuard::arm(&paths.startup_marker)?;
        let db = Arc::new(Db::open(&paths.db)?);
        data_lifecycle.finish_startup(&db, &owner_store)?;
        // The same exclusive guard is held during browser execution. A second
        // app process cannot clean a live session owned by the first instance.
        match DataLifecycle::runner_profile_guard(&paths.data) {
            Ok(_guard) => {
                for result in [
                    DataLifecycle::recover_runner_profiles(&paths.data),
                    owner_store.reconcile_browser_runs(),
                    crate::preview_runner::captures::expire(&paths.data),
                ] {
                    if let Err(error) = result {
                        tracing::warn!(%error,"preview recovery remains incomplete");
                    }
                }
            }
            Err(error) => tracing::debug!(%error,"preview recovery is owned by another process"),
        }
        let state = db.state();
        if !state.is_ready() {
            tracing::error!(?state, "the database did not open cleanly");
        }
        let maintenance = MaintenanceHandle::spawn(Arc::clone(&db));
        let preview_retention = crate::preview_runner::CaptureRetention::start(
            paths.data.clone(),
            Arc::clone(&data_lifecycle),
        )?;
        let (mut enabled, monitoring_profile, onboarding_completed) = match db
            .read(crate::storage::settings_repository::load)
        {
            Ok(settings) => (
                settings.collectors,
                settings.monitoring_profile,
                settings.onboarding_completed,
            ),
            Err(error) => {
                tracing::warn!(%error, "collector settings are unavailable; using safe defaults");
                let disabled = registry
                    .entries()
                    .iter()
                    .map(|entry| (entry.descriptor.id.to_owned(), false))
                    .collect();
                (disabled, crate::domain::MonitoringProfile::Balanced, false)
            }
        };
        let monitoring = Arc::new(Monitoring::running());
        let query_busy = QueryBusy::default();
        // A fresh install must not collect before the user finishes or skips
        // the local-only onboarding promise. Keep the persisted preferences
        // intact so completion can apply them without surprising defaults.
        if startup.safe_mode || !onboarding_completed {
            enabled.values_mut().for_each(|value| *value = false);
        }
        let collectors = CollectorSupervisor::new(
            registry,
            Arc::clone(&db),
            Arc::clone(&monitoring),
            query_busy.clone(),
            &enabled,
            monitoring_profile,
        );
        let checkpoints = CheckpointService::new(Arc::clone(&db), collectors.capture_handle());
        Ok(AppState {
            paths,
            data_lifecycle,
            owner_store,
            owner_portability: Default::default(),
            preview_runner: Default::default(),
            preview_retention: Mutex::new(Some(preview_retention)),
            db,
            startup,
            monitoring,
            query_busy,
            resume_monitor: Mutex::new(None),
            collectors,
            checkpoints,
            checkpoint_automation: Mutex::new(None),
            checkpoint_sink: Mutex::new(None),
            resume_after_purge: AtomicBool::new(false),
            collector_config: Mutex::new(()),
            ui_errors: RateLimiter::default(),
            tray: Mutex::new(None),
            shutdown_recorded: AtomicBool::new(false),
            bus: Mutex::new(None),
            maintenance: Mutex::new(Some(maintenance)),
        })
    }

    /// Record the tray once it has been built.
    pub fn attach_tray(&self, tray: Box<dyn TrayReflector>) {
        tray.reflect(self.monitoring.snapshot());
        *lock(&self.tray) = Some(tray);
    }

    /// Hold the coalescing thread for the life of the app.
    pub fn attach_event_bus(&self, bus: EventBus) {
        *lock(&self.bus) = Some(bus);
    }

    /// Hold the platform resume watcher for the application lifetime.
    pub fn attach_resume_monitor(&self, monitor: Box<dyn Send>) {
        *lock(&self.resume_monitor) = Some(monitor);
    }

    /// Start automatic checkpoint scheduling after collector workers exist.
    pub fn start_checkpoint_automation(
        &self,
        saved_sink: Arc<dyn crate::checkpoint::auto::CheckpointSavedSink>,
    ) -> AppResult<()> {
        *lock(&self.checkpoint_sink) = Some(Arc::clone(&saved_sink));
        let mut slot = lock(&self.checkpoint_automation);
        if slot.is_some() {
            return Ok(());
        }
        let manager = AutomaticCheckpointManager::spawn(
            self.checkpoints.clone(),
            Arc::clone(&self.db),
            saved_sink,
        )?;
        self.collectors
            .attach_automatic_checkpoint_sink(manager.sink());
        *slot = Some(manager);
        Ok(())
    }

    pub fn stop_checkpoint_automation(&self) {
        drop(lock(&self.checkpoint_automation).take());
    }

    /// Drop joins the maintenance worker before either store can be replaced.
    pub(crate) fn stop_maintenance(&self) {
        drop(lock(&self.maintenance).take());
    }

    pub(crate) fn stop_resume_monitor(&self) -> bool {
        let monitor = lock(&self.resume_monitor).take();
        let existed = monitor.is_some();
        drop(monitor);
        existed
    }

    fn resume_after_successful_purge(&self) -> AppResult<()> {
        self.checkpoints.resume_admission()?;
        let saved_sink = lock(&self.checkpoint_sink).clone();
        if let Some(saved_sink) = saved_sink {
            if let Err(error) = self.start_checkpoint_automation(saved_sink) {
                // Scheduling and the platform watcher are best-effort at normal
                // startup as well. Manual checkpoints remain usable on failure.
                tracing::warn!(%error, "automatic checkpoints could not restart after deletion");
            }
        }
        let mut maintenance = lock(&self.maintenance);
        if maintenance.is_none() {
            *maintenance = Some(MaintenanceHandle::spawn(Arc::clone(&self.db)));
        }
        drop(maintenance);
        if self.resume_after_purge.swap(false, Ordering::AcqRel) {
            match crate::platform::start_resume_monitor(self.collectors.resume_wake_handle()) {
                Ok(monitor) => self.attach_resume_monitor(monitor),
                Err(error) => {
                    tracing::warn!(%error, "resume watcher could not restart after deletion")
                }
            }
        }
        self.data_lifecycle.reopen_global()
    }

    /// Whether a tray icon exists to reach the window from.
    pub fn tray_available(&self) -> bool {
        lock(&self.tray).is_some()
    }

    /// Update the tray menu's enabled states, if there is a tray.
    pub fn reflect_monitoring(&self, monitoring: crate::dto::MonitoringDto) {
        if let Some(tray) = lock(&self.tray).as_ref() {
            tray.reflect(monitoring);
        }
    }

    /// Persist collector/profile settings and apply the exact returned state
    /// to the supervisor as one serialized configuration operation.
    pub fn update_collector_settings(&self, update: SettingsUpdateDto) -> AppResult<SettingsDto> {
        let _config = lock(&self.collector_config);
        if let Some(changes) = update.collectors.as_ref() {
            self.collectors
                .validate_enabled(changes.keys().map(String::as_str))?;
        }
        let collector_changes = update.collectors.clone();
        let changed = self
            .db
            .write(move |conn| settings_repository::update(conn, &update))?;
        self.collectors.set_profile(changed.monitoring_profile);
        if let Some(changes) = collector_changes {
            // Onboarding is a hard collection fence. Settings may be edited
            // by tests or a future shell before consent, but those desired
            // values stay persisted and are applied only by completion.
            if !changed.onboarding_completed {
                for id in changes.keys() {
                    self.collectors.set_enabled(id, false)?;
                }
                return Ok(changed);
            }
            for (id, enabled) in changes {
                // The registry is immutable and all ids were validated while
                // holding `collector_config`, so this cannot diverge after
                // the database transaction commits.
                self.collectors.set_enabled(&id, enabled)?;
            }
        }
        Ok(changed)
    }

    /// Read settings on the same serialization boundary as runtime updates.
    pub fn settings(&self) -> AppResult<SettingsDto> {
        let _config = lock(&self.collector_config);
        self.db.read(settings_repository::load)
    }

    /// Serialize onboarding's multi-step runtime/database transition with
    /// ordinary collector settings updates and concurrent finish/reopen calls.
    pub(crate) fn lock_collector_config(&self) -> std::sync::MutexGuard<'_, ()> {
        lock(&self.collector_config)
    }

    /// Fence every collector and discard its in-memory instance after a full
    /// history purge. The next onboarding completion creates fresh instances,
    /// so no pre-purge baseline can be compared against post-purge history.
    pub fn reset_collectors_after_history_purge(&self) -> AppResult<()> {
        let _config = lock(&self.collector_config);
        let ids: Vec<String> = self
            .collectors
            .list_collector_status()
            .into_iter()
            .map(|status| status.id)
            .collect();
        for id in ids {
            self.collectors.set_enabled(&id, false)?;
        }
        Ok(())
    }

    /// What the shell needs before it can draw.
    pub fn status(&self) -> AppStatusDto {
        AppStatusDto {
            db: self.db.state(),
            monitoring: self.monitoring.snapshot(),
            tray_available: self.tray_available(),
            safe_mode: self.startup.safe_mode,
            dropped_writes: self.db.dropped_writes(),
        }
    }

    /// Record a clean shutdown. Idempotent: the quit path and the exit event
    /// both call it, and the second call does nothing.
    pub fn mark_clean_shutdown(&self) {
        if self.shutdown_recorded.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Err(error) = StartupGuard::disarm(&self.paths.startup_marker) {
            tracing::warn!(%error, "could not record a clean shutdown");
        }
    }

    /// Flush the write queue and record a clean shutdown.
    pub fn shutdown_cleanly(&self) {
        super::lifecycle::shutdown(self);
    }

    /// Purge both application stores under one durable journal. The caller has
    /// already captured the legacy counts and appearance preferences so this
    /// method does not expose legacy DTOs from the lifecycle layer.
    pub fn purge_all_stores(
        &self,
        preserved_theme: &str,
        preserved_visualization: &str,
    ) -> AppResult<()> {
        self.preview_runner.stop_all();
        let _operation = self.data_lifecycle.lock_operation();
        data_lifecycle::purge_stores(
            &self.data_lifecycle,
            &self.paths,
            &self.db,
            &self.owner_store,
            || {
                self.checkpoints.pause_admission();
                self.stop_maintenance();
                if self.stop_resume_monitor() {
                    self.resume_after_purge.store(true, Ordering::Release);
                }
                self.stop_checkpoint_automation();
                // Command workers may have persisted settings immediately
                // before the write fence closed. Drain their runtime updates
                // before disabling collectors, so none can re-enable an old
                // in-memory instance behind the reset.
                self.data_lifecycle.wait_for_idle()?;
                self.owner_portability.clear();
                self.reset_collectors_after_history_purge()?;
                self.checkpoints.wait_for_idle();
                Ok(())
            },
            preserved_theme,
            preserved_visualization,
        )?;
        // Collector worker threads survive. Fresh onboarding can enable them;
        // terminal stop_all is reserved for actual application shutdown.
        self.resume_after_successful_purge()
    }
}

/// Lock, recovering the value if a holder panicked. A poisoned tray handle is
/// still a usable tray handle.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn three_unclean_starts_trigger_safe_mode_and_a_clean_shutdown_resets_it() {
        let dir = crate::testing::temp_dir("startup-guard");
        let marker = dir.join("startup.json");

        for expected in 0..SAFE_MODE_THRESHOLD {
            let guard = StartupGuard::arm(&marker).unwrap();
            assert_eq!(guard.previous_unclean_starts, expected);
            assert!(!guard.safe_mode);
        }
        let guard = StartupGuard::arm(&marker).unwrap();
        assert!(
            guard.safe_mode,
            "the fourth start follows three unclean ones"
        );

        StartupGuard::disarm(&marker).unwrap();
        let guard = StartupGuard::arm(&marker).unwrap();
        assert_eq!(guard.previous_unclean_starts, 0);
        assert!(!guard.safe_mode);
    }

    #[test]
    fn a_garbled_marker_counts_as_zero_rather_than_failing() {
        let dir = crate::testing::temp_dir("startup-garbled");
        let marker = dir.join("startup.json");
        fs::write(&marker, "not json").unwrap();
        let guard = StartupGuard::arm(&marker).unwrap();
        assert_eq!(guard.previous_unclean_starts, 0);
    }

    #[test]
    fn app_state_opens_and_shuts_down_cleanly() {
        let dir = crate::testing::temp_dir("app-state");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        assert!(state.db.state().is_ready());
        assert!(!state.startup.safe_mode);
        state.shutdown_cleanly();
        let marker: StartupMarker =
            serde_json::from_str(&fs::read_to_string(&state.paths.startup_marker).unwrap())
                .unwrap();
        assert_eq!(marker.unclean_starts, 0);
    }

    #[test]
    fn full_purge_restarts_maintenance_and_checkpoint_admission_for_fresh_onboarding() {
        struct SavedSink;
        impl crate::checkpoint::auto::CheckpointSavedSink for SavedSink {
            fn checkpoint_saved(&self, _: i64) {}
        }
        let dir = crate::testing::temp_dir("app-state-purge-resume");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        state
            .update_collector_settings(SettingsUpdateDto {
                theme: Some(crate::domain::Theme::Dark),
                visualization: Some(crate::domain::VisualizationMode::TwoD),
                onboarding_completed: Some(true),
                ..Default::default()
            })
            .unwrap();
        state.collectors.set_enabled("process", true).unwrap();
        state
            .checkpoints
            .create(
                Some("Before purge".into()),
                crate::domain::CheckpointKind::Manual,
                None,
                |_| {},
            )
            .unwrap();
        state
            .start_checkpoint_automation(Arc::new(SavedSink))
            .unwrap();

        state.purge_all_stores("\"dark\"", "\"2d\"").unwrap();
        assert!(!state.data_lifecycle.is_closed());
        assert!(lock(&state.maintenance).is_some());
        assert!(lock(&state.checkpoint_automation).is_some());
        assert!(lock(&state.checkpoint_sink).is_some());
        assert!(state
            .collectors
            .list_collector_status()
            .iter()
            .all(|row| !row.enabled));
        let settings = state.settings().unwrap();
        assert!(!settings.onboarding_completed);
        assert_eq!(settings.theme, crate::domain::Theme::Dark);
        assert_eq!(
            settings.visualization,
            crate::domain::VisualizationMode::TwoD
        );
        state.stop_checkpoint_automation();
        state
            .checkpoints
            .create(
                Some("After purge".into()),
                crate::domain::CheckpointKind::Manual,
                None,
                |_| {},
            )
            .unwrap();
        state
            .update_collector_settings(SettingsUpdateDto {
                onboarding_completed: Some(true),
                collectors: Some(std::collections::BTreeMap::from([(
                    "process".to_owned(),
                    true,
                )])),
                ..Default::default()
            })
            .unwrap();
        assert!(
            state
                .collectors
                .list_collector_status()
                .iter()
                .find(|row| row.id == "process")
                .unwrap()
                .enabled
        );
        state.shutdown_cleanly();
    }

    #[test]
    fn shutdown_joins_owned_background_handles_without_creating_purge_intent() {
        struct ResumeWatcher(Arc<AtomicBool>);
        impl Drop for ResumeWatcher {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let dir = crate::testing::temp_dir("app-state-worker-shutdown");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        let dropped = Arc::new(AtomicBool::new(false));
        state.attach_resume_monitor(Box::new(ResumeWatcher(Arc::clone(&dropped))));
        state.shutdown_cleanly();
        assert!(dropped.load(Ordering::Acquire));
        assert!(lock(&state.maintenance).is_none());
        assert!(lock(&state.resume_monitor).is_none());
        assert!(state.data_lifecycle.is_closed());
        assert!(!state.paths.purge_journal().exists());
    }

    #[test]
    fn a_state_with_no_tray_says_so_which_is_what_keeps_the_window_closable() {
        let dir = crate::testing::temp_dir("app-state-tray");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        assert!(!state.tray_available());
        let status = state.status();
        assert!(!status.tray_available);
        assert!(status.db.is_ready());
        assert!(!status.monitoring.paused);
        assert!(!status.safe_mode);
    }

    #[test]
    fn attaching_a_tray_hands_it_the_current_monitoring_state() {
        use std::sync::atomic::AtomicU8;

        struct Spy(Arc<AtomicU8>);
        impl TrayReflector for Spy {
            fn reflect(&self, monitoring: crate::dto::MonitoringDto) {
                self.0.store(u8::from(monitoring.paused), Ordering::SeqCst);
            }
        }

        let dir = crate::testing::temp_dir("app-state-reflect");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        state.monitoring.pause(None);

        let seen = Arc::new(AtomicU8::new(9));
        state.attach_tray(Box::new(Spy(Arc::clone(&seen))));
        assert_eq!(seen.load(Ordering::SeqCst), 1, "attached while paused");
        assert!(state.tray_available());
        assert!(state.status().monitoring.paused);

        state.monitoring.resume();
        state.reflect_monitoring(crate::dto::MonitoringDto {
            paused: false,
            resume_at: None,
        });
        assert_eq!(seen.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_clean_shutdown_marker_is_written_once_however_many_paths_ask() {
        let dir = crate::testing::temp_dir("app-state-idempotent");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        state.shutdown_cleanly();
        // A second start would bump the counter; if the second shutdown call
        // rewrote the marker it would mask that, so it must not.
        let guard = StartupGuard::arm(&state.paths.startup_marker).unwrap();
        assert_eq!(guard.previous_unclean_starts, 0);
        state.shutdown_cleanly();
        let marker: StartupMarker =
            serde_json::from_str(&fs::read_to_string(&state.paths.startup_marker).unwrap())
                .unwrap();
        assert_eq!(
            marker.unclean_starts, 1,
            "the second call is a no-op, so the start armed above still counts"
        );
    }

    #[test]
    fn concurrent_collector_configuration_keeps_database_and_runtime_equal() {
        let dir = crate::testing::temp_dir("app-state-concurrent-config");
        let paths = AppPaths::under(&dir);
        // Runtime toggles are intentionally fenced until the local-only
        // onboarding promise is completed. Seed consent so this test covers
        // the concurrent settings transition itself rather than the fence.
        let setup = crate::storage::Db::open(&paths.db).unwrap();
        setup
            .write(|conn| {
                conn.execute(
                    "UPDATE settings SET value = 'true' WHERE key = 'onboarding_completed'",
                    [],
                )
                .map_err(|source| {
                    crate::error::AppError::Db(crate::error::DbError::Sqlite {
                        operation: "seed onboarding consent for concurrent settings test",
                        source,
                    })
                })?;
                Ok(())
            })
            .unwrap();
        drop(setup);
        let state = Arc::new(AppState::open(paths).unwrap());
        let mut threads = Vec::new();
        for index in 0..12 {
            let state = Arc::clone(&state);
            threads.push(std::thread::spawn(move || {
                state
                    .update_collector_settings(SettingsUpdateDto {
                        monitoring_profile: Some(if index % 2 == 0 {
                            crate::domain::MonitoringProfile::Minimal
                        } else {
                            crate::domain::MonitoringProfile::Thorough
                        }),
                        collectors: Some(std::collections::BTreeMap::from([(
                            "process".to_owned(),
                            index % 3 == 0,
                        )])),
                        ..Default::default()
                    })
                    .unwrap()
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }

        let stored = state.db.read(settings_repository::load).unwrap();
        let runtime = state.collectors.list_collector_status();
        let process = runtime.iter().find(|row| row.id == "process").unwrap();
        assert_eq!(
            process.enabled,
            stored.collectors.get("process").copied().unwrap()
        );
        assert_eq!(state.collectors.profile(), stored.monitoring_profile);
        state.shutdown_cleanly();
    }
}
