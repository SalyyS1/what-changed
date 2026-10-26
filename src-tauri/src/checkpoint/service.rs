//! Application checkpoint orchestration over live collector workers and SQLite.

use std::collections::BTreeSet;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::collectors::runtime::{
    CollectorCaptureHandle, CollectorCaptureResult, CollectorCaptureStatus,
};
use crate::domain::{Checkpoint, CheckpointDiff, CheckpointKind};
use crate::error::{AppError, AppResult};
use crate::privacy::Sanitizer;
use crate::storage::{checkpoint_repository, Db};

use super::{compare_item_sets, prepare_capture, state_items};

const COLLECTOR_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone)]
pub struct CheckpointService {
    db: Arc<Db>,
    capture: CollectorCaptureHandle,
    in_flight: Arc<Mutex<Option<Arc<Flight>>>>,
    activity: Arc<ServiceActivity>,
}

impl CheckpointService {
    pub fn new(db: Arc<Db>, capture: CollectorCaptureHandle) -> Self {
        Self {
            db,
            capture,
            in_flight: Arc::new(Mutex::new(None)),
            activity: Arc::new(ServiceActivity::default()),
        }
    }

    pub fn create(
        &self,
        name: Option<String>,
        kind: CheckpointKind,
        reason: Option<String>,
        progress: impl FnMut(&CollectorCaptureResult),
    ) -> AppResult<Checkpoint> {
        self.create_with_policy(name, kind, reason, progress, false)
    }

    /// Queue an automatic intent behind a different active checkpoint. This
    /// is deliberately separate from the interactive API, which reports Busy
    /// immediately so a person never sees one request silently become another.
    pub fn create_queued(
        &self,
        name: Option<String>,
        kind: CheckpointKind,
        reason: Option<String>,
        progress: impl FnMut(&CollectorCaptureResult),
    ) -> AppResult<Checkpoint> {
        self.create_with_policy(name, kind, reason, progress, true)
    }

    fn create_with_policy(
        &self,
        name: Option<String>,
        kind: CheckpointKind,
        reason: Option<String>,
        mut progress: impl FnMut(&CollectorCaptureResult),
        queue_distinct: bool,
    ) -> AppResult<Checkpoint> {
        loop {
            let activity = self.activity.enter()?;
            let intent = CheckpointIntent {
                name: name.clone(),
                kind,
                reason: reason.clone(),
            };
            let (flight, leader) = match self.flight(intent) {
                Ok(decision) => decision,
                Err(_error) if queue_distinct => {
                    let active = lock(&self.in_flight).as_ref().cloned();
                    drop(activity);
                    if let Some(active) = active {
                        let _ = active.wait();
                    }
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !leader {
                let id = flight.wait()?;
                return self.db.read(|conn| {
                    checkpoint_repository::by_id(conn, id)?.ok_or_else(|| AppError::InvalidInput {
                        what: "checkpoint",
                        reason: "the in-flight checkpoint no longer exists".to_owned(),
                    })
                });
            }

            let result = self.create_leader(name, kind, reason, &mut progress);
            match &result {
                Ok(checkpoint) => flight.finish(Ok(checkpoint.id)),
                Err(error) => flight.finish(Err(error.to_string())),
            }
            let mut slot = lock(&self.in_flight);
            if slot
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &flight))
            {
                *slot = None;
            }
            return result;
        }
    }

    fn create_leader(
        &self,
        name: Option<String>,
        kind: CheckpointKind,
        reason: Option<String>,
        progress: &mut impl FnMut(&CollectorCaptureResult),
    ) -> AppResult<Checkpoint> {
        let began = Instant::now();
        let manifest = self
            .capture
            .capture_enabled_with_progress(COLLECTOR_TIMEOUT, progress);
        let (mut captured, mut missing) = collector_manifests(&manifest.collectors);
        let durable = self.db.read(|conn| {
            Ok(state_items::from_durable_state(
                crate::storage::baseline_repository::all(conn)?,
                crate::storage::config_snapshot_repository::all(conn)?,
            ))
        })?;
        reconcile_durable_state(&mut captured, &mut missing, &durable);
        let captured_set: BTreeSet<&str> = captured.iter().map(String::as_str).collect();
        let items = durable
            .items
            .into_iter()
            .filter(|item| captured_set.contains(item.collector_id.as_str()))
            .collect();
        let (capture, _) = prepare_capture(
            name,
            kind,
            reason,
            manifest.completed_at,
            u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX),
            captured,
            missing,
            items,
            &Sanitizer::new(),
        );
        self.db
            .write(move |conn| checkpoint_repository::create(conn, &capture))
    }

    pub fn compare(&self, checkpoint_id: i64, against: Option<i64>) -> AppResult<CheckpointDiff> {
        let (left, left_items) = self.db.read(|conn| {
            let checkpoint =
                checkpoint_repository::by_id(conn, checkpoint_id)?.ok_or_else(|| {
                    AppError::InvalidInput {
                        what: "checkpoint id",
                        reason: "checkpoint was not found".to_owned(),
                    }
                })?;
            let items = checkpoint_repository::items(conn, checkpoint_id)?;
            Ok((checkpoint, items))
        })?;

        let (right_items, right_captured, right_missing, right_at) = match against {
            Some(other_id) => self.db.read(|conn| {
                let other = checkpoint_repository::by_id(conn, other_id)?.ok_or_else(|| {
                    AppError::InvalidInput {
                        what: "comparison checkpoint id",
                        reason: "checkpoint was not found".to_owned(),
                    }
                })?;
                Ok((
                    checkpoint_repository::items(conn, other_id)?,
                    other.captured_collectors,
                    other.missing_collectors,
                    other.created_at,
                ))
            })?,
            None => {
                let manifest = self.capture.current_manifest();
                let (mut captured, mut missing) = collector_manifests(&manifest.collectors);
                let durable = self.db.read(|conn| {
                    Ok(state_items::from_durable_state(
                        crate::storage::baseline_repository::all(conn)?,
                        crate::storage::config_snapshot_repository::all(conn)?,
                    ))
                })?;
                reconcile_durable_state(&mut captured, &mut missing, &durable);
                let captured_set: BTreeSet<&str> = captured.iter().map(String::as_str).collect();
                let items = durable
                    .items
                    .into_iter()
                    .filter(|item| captured_set.contains(item.collector_id.as_str()))
                    .collect();
                (items, captured, missing, manifest.completed_at)
            }
        };
        let event_ids = self.db.read(|conn| {
            checkpoint_repository::event_ids_in_window(conn, left.created_at, right_at)
        })?;
        let mut diff = compare_item_sets(
            &left_items,
            &left.captured_collectors,
            &left.missing_collectors,
            &right_items,
            &right_captured,
            &right_missing,
            event_ids,
        );
        diff.left_missing_collectors = side_missing(
            &left.missing_collectors,
            &left.captured_collectors,
            &right_captured,
        );
        diff.right_missing_collectors =
            side_missing(&right_missing, &right_captured, &left.captured_collectors);
        diff.window_from = left.created_at.min(right_at);
        diff.window_to = left.created_at.max(right_at);
        Ok(diff)
    }

    /// Stop admitting checkpoint writes. Existing callers remain tracked until
    /// they finish, allowing shutdown to flush only after every detached
    /// command/tray task has left the service.
    pub fn begin_shutdown(&self) {
        self.activity.begin_shutdown();
    }

    /// Wait until all checkpoint creation callers (leaders and followers)
    /// have returned. New creation is rejected after `begin_shutdown`.
    pub fn wait_for_idle(&self) {
        self.activity.wait_for_idle();
    }

    fn flight(&self, intent: CheckpointIntent) -> AppResult<(Arc<Flight>, bool)> {
        let mut slot = lock(&self.in_flight);
        if let Some(flight) = slot.as_ref() {
            if flight.intent == intent {
                return Ok((Arc::clone(flight), false));
            }
            return Err(AppError::InvalidInput {
                what: "checkpoint creation",
                reason: "another checkpoint with different details is already being created"
                    .to_owned(),
            });
        }
        let flight = Arc::new(Flight::new(intent));
        *slot = Some(Arc::clone(&flight));
        Ok((flight, true))
    }
}

fn collector_manifests(results: &[CollectorCaptureResult]) -> (Vec<String>, Vec<String>) {
    let mut captured = Vec::new();
    let mut missing = Vec::new();
    for result in results {
        match result.status {
            CollectorCaptureStatus::Complete => captured.push(result.collector_id.clone()),
            CollectorCaptureStatus::Disabled | CollectorCaptureStatus::Unavailable => {}
            CollectorCaptureStatus::Paused
            | CollectorCaptureStatus::Failed
            | CollectorCaptureStatus::TimedOutWaitingForWorker
            | CollectorCaptureStatus::TimedOutDuringCycle => {
                missing.push(result.collector_id.clone());
            }
        }
    }
    (captured, missing)
}

fn reconcile_durable_state(
    captured: &mut Vec<String>,
    missing: &mut Vec<String>,
    durable: &state_items::DurableStateItems,
) {
    // Disabled and unavailable collectors are deliberately absent from both
    // manifests. Their stale durable rows must not make a new checkpoint
    // partial until the collector is enabled and participates again.
    let participating: BTreeSet<String> = captured.iter().chain(missing.iter()).cloned().collect();
    let incomplete: BTreeSet<&str> = durable
        .incomplete_collectors
        .iter()
        .map(String::as_str)
        .collect();
    let complete_baselines: BTreeSet<&str> = durable
        .complete_baseline_collectors
        .iter()
        .map(String::as_str)
        .collect();
    let absent_baselines = captured
        .iter()
        .filter(|collector| {
            state_items::expected_baseline_kind(collector).is_some()
                && !complete_baselines.contains(collector.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    captured.retain(|collector| {
        !incomplete.contains(collector.as_str()) && !absent_baselines.contains(collector)
    });
    missing.extend(
        incomplete
            .into_iter()
            .filter(|collector| participating.contains(*collector))
            .map(str::to_owned),
    );
    missing.extend(absent_baselines);
    captured.sort();
    captured.dedup();
    missing.sort();
    missing.dedup();
}

fn side_missing(
    explicit: &[String],
    captured: &[String],
    other_captured: &[String],
) -> BTreeSet<String> {
    let captured: BTreeSet<&str> = captured.iter().map(String::as_str).collect();
    explicit
        .iter()
        .cloned()
        .chain(
            other_captured
                .iter()
                .filter(|collector| !captured.contains(collector.as_str()))
                .cloned(),
        )
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckpointIntent {
    name: Option<String>,
    kind: CheckpointKind,
    reason: Option<String>,
}

struct Flight {
    intent: CheckpointIntent,
    outcome: Mutex<Option<Result<i64, String>>>,
    changed: Condvar,
}

impl Flight {
    fn new(intent: CheckpointIntent) -> Self {
        Self {
            intent,
            outcome: Mutex::new(None),
            changed: Condvar::new(),
        }
    }

    fn finish(&self, outcome: Result<i64, String>) {
        *lock(&self.outcome) = Some(outcome);
        self.changed.notify_all();
    }

    fn wait(&self) -> AppResult<i64> {
        let mut outcome = lock(&self.outcome);
        while outcome.is_none() {
            outcome = self
                .changed
                .wait(outcome)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        match outcome.as_ref() {
            Some(Ok(id)) => Ok(*id),
            Some(Err(reason)) => Err(AppError::InvalidInput {
                what: "checkpoint creation",
                reason: reason.clone(),
            }),
            None => Err(AppError::InvalidInput {
                what: "checkpoint creation",
                reason: "the in-flight checkpoint ended without a result".to_owned(),
            }),
        }
    }
}

#[derive(Default)]
struct ServiceActivity {
    state: Mutex<ActivityState>,
    changed: Condvar,
}

#[derive(Default)]
struct ActivityState {
    closing: bool,
    active: usize,
}

impl ServiceActivity {
    fn enter(self: &Arc<Self>) -> AppResult<ActivityGuard> {
        let mut state = lock(&self.state);
        if state.closing {
            return Err(AppError::InvalidInput {
                what: "checkpoint creation",
                reason: "the application is shutting down".to_owned(),
            });
        }
        state.active = state.active.saturating_add(1);
        Ok(ActivityGuard {
            activity: Arc::clone(self),
        })
    }

    fn begin_shutdown(&self) {
        lock(&self.state).closing = true;
    }

    fn wait_for_idle(&self) {
        let mut state = lock(&self.state);
        while state.active > 0 {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

struct ActivityGuard {
    activity: Arc<ServiceActivity>,
}

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        let mut state = lock(&self.activity.state);
        state.active = state.active.saturating_sub(1);
        if state.active == 0 {
            self.activity.changed.notify_all();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn disabled_and_unavailable_collectors_are_skipped_not_missing() {
        let make = |id: &str, status| CollectorCaptureResult {
            collector_id: id.to_owned(),
            display_name: id.to_owned(),
            cost: crate::domain::CollectorCost::Cheap,
            status,
            requested_at: 1,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            events_committed: 0,
            error: None,
        };
        let (captured, missing) = collector_manifests(&[
            make("process", CollectorCaptureStatus::Complete),
            make("docker", CollectorCaptureStatus::Disabled),
            make("dev_env", CollectorCaptureStatus::Unavailable),
            make("drivers", CollectorCaptureStatus::TimedOutDuringCycle),
        ]);
        assert_eq!(captured, ["process"]);
        assert_eq!(missing, ["drivers"]);

        let durable = state_items::DurableStateItems {
            incomplete_collectors: vec!["services".to_owned()],
            ..state_items::DurableStateItems::default()
        };
        let (mut captured, mut missing) =
            collector_manifests(&[make("services", CollectorCaptureStatus::Disabled)]);
        reconcile_durable_state(&mut captured, &mut missing, &durable);
        assert!(captured.is_empty());
        assert!(missing.is_empty(), "disabled stale state must stay ignored");

        let (mut captured, mut missing) =
            collector_manifests(&[make("services", CollectorCaptureStatus::Complete)]);
        reconcile_durable_state(&mut captured, &mut missing, &durable);
        assert!(captured.is_empty());
        assert_eq!(missing, ["services"]);
    }

    #[test]
    fn pending_durable_cycle_stays_missing_until_the_final_chunk_is_committed() {
        let capture_result = |status| CollectorCaptureResult {
            collector_id: "services".to_owned(),
            display_name: "Services".to_owned(),
            cost: crate::domain::CollectorCost::Cheap,
            status,
            requested_at: 1,
            started_at: Some(1),
            completed_at: Some(2),
            duration_ms: Some(1),
            events_committed: 1,
            error: None,
        };
        let baseline = |state: &str, pending: serde_json::Value| crate::storage::Baseline {
            collector_id: "services".to_owned(),
            entity_kind: "service_instances".to_owned(),
            snapshot: serde_json::json!({
                "acknowledged": { "spooler": {
                    "name": "spooler", "display_name": "Print Spooler",
                    "state": state, "start_type": "automatic",
                    "config_available": true, "process_id": 10
                }},
                "pending": pending,
                "established": true
            }),
            captured_at: 1,
            truncated: false,
        };

        let pending = state_items::from_durable_state(
            vec![baseline(
                "stopped",
                serde_json::json!({
                    "target": { "spooler": {
                        "name": "spooler", "display_name": "Print Spooler",
                        "state": "running", "start_type": "automatic",
                        "config_available": true, "process_id": 10
                    }},
                    "observed_at": 2,
                    "occurred_at_is_uncertain": false
                }),
            )],
            vec![],
        );
        let (mut captured, mut missing) =
            collector_manifests(&[capture_result(CollectorCaptureStatus::Complete)]);
        reconcile_durable_state(&mut captured, &mut missing, &pending);
        assert!(captured.is_empty());
        assert_eq!(missing, ["services"]);
        assert!(pending.items.is_empty());

        let drained = state_items::from_durable_state(
            vec![baseline("running", serde_json::Value::Null)],
            vec![],
        );
        let (mut captured, mut missing) =
            collector_manifests(&[capture_result(CollectorCaptureStatus::Complete)]);
        reconcile_durable_state(&mut captured, &mut missing, &drained);
        assert_eq!(captured, ["services"]);
        assert!(missing.is_empty());
        assert_eq!(drained.items.len(), 1);
        assert_eq!(drained.items[0].state["state"], "running");

        let (mut captured, mut missing) =
            collector_manifests(&[capture_result(CollectorCaptureStatus::TimedOutDuringCycle)]);
        reconcile_durable_state(&mut captured, &mut missing, &pending);
        assert!(captured.is_empty());
        assert_eq!(missing, ["services"]);
    }

    #[test]
    fn shutdown_gate_rejects_new_work_and_waits_for_active_work() {
        let activity = Arc::new(ServiceActivity::default());
        let guard = activity.enter().expect("first operation is admitted");
        activity.begin_shutdown();
        assert!(activity.enter().is_err());

        let waiter_activity = Arc::clone(&activity);
        let waited = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let waiter_flag = Arc::clone(&waited);
        let waiter = std::thread::spawn(move || {
            waiter_activity.wait_for_idle();
            waiter_flag.store(true, std::sync::atomic::Ordering::Release);
        });
        std::thread::sleep(Duration::from_millis(10));
        assert!(!waited.load(std::sync::atomic::Ordering::Acquire));
        drop(guard);
        waiter.join().expect("waiter exits");
        assert!(waited.load(std::sync::atomic::Ordering::Acquire));
    }

    #[test]
    fn checkpoint_intents_only_coalesce_when_all_details_match() {
        use crate::app::Monitoring;
        use crate::collectors::registry::CollectorRegistry;
        use crate::collectors::runtime::CollectorSupervisor;
        use crate::collectors::schedule::QueryBusy;
        use crate::domain::MonitoringProfile;

        let first = CheckpointIntent {
            name: Some("Manual baseline".to_owned()),
            kind: CheckpointKind::Manual,
            reason: None,
        };
        let duplicate = first.clone();
        let automatic = CheckpointIntent {
            name: Some("Before update".to_owned()),
            kind: CheckpointKind::Automatic,
            reason: Some("update started".to_owned()),
        };
        assert_eq!(first, duplicate);
        assert_ne!(first, automatic);

        let db = Arc::new(
            Db::open_in(&crate::testing::temp_dir("checkpoint-intent-flight"))
                .expect("database opens"),
        );
        let supervisor = CollectorSupervisor::new(
            CollectorRegistry::new(Vec::new()),
            Arc::clone(&db),
            Arc::new(Monitoring::running()),
            QueryBusy::default(),
            &std::collections::BTreeMap::new(),
            MonitoringProfile::Balanced,
        );
        let service = CheckpointService::new(db, supervisor.capture_handle());
        let (leader, is_leader) = service.flight(first).expect("first intent is admitted");
        assert!(is_leader);
        let (follower, is_leader) = service
            .flight(duplicate)
            .expect("identical intent follows the flight");
        assert!(!is_leader);
        assert!(Arc::ptr_eq(&leader, &follower));
        assert!(service.flight(automatic).is_err());
    }

    #[test]
    fn side_specific_missing_includes_manifest_asymmetry() {
        let missing = side_missing(
            &["drivers".to_owned()],
            &["services".to_owned()],
            &["services".to_owned(), "network".to_owned()],
        );
        assert_eq!(
            missing,
            BTreeSet::from(["drivers".to_owned(), "network".to_owned()])
        );
    }

    #[test]
    fn queued_automatic_intent_runs_after_a_different_active_flight() {
        use crate::app::Monitoring;
        use crate::collectors::registry::CollectorRegistry;
        use crate::collectors::runtime::CollectorSupervisor;
        use crate::collectors::schedule::QueryBusy;
        use crate::domain::MonitoringProfile;

        let db = Arc::new(
            Db::open_in(&crate::testing::temp_dir("checkpoint-queued-intent"))
                .expect("database opens"),
        );
        let supervisor = CollectorSupervisor::new(
            CollectorRegistry::new(Vec::new()),
            Arc::clone(&db),
            Arc::new(Monitoring::running()),
            QueryBusy::default(),
            &std::collections::BTreeMap::new(),
            MonitoringProfile::Balanced,
        );
        let service = CheckpointService::new(Arc::clone(&db), supervisor.capture_handle());
        let manual = CheckpointIntent {
            name: Some("Manual".to_owned()),
            kind: CheckpointKind::Manual,
            reason: None,
        };
        let (manual_flight, leader) = service.flight(manual).expect("manual flight starts");
        assert!(leader);

        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker_service = service.clone();
        let worker_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            worker_service.create_queued(
                Some("Before update".to_owned()),
                CheckpointKind::Automatic,
                Some("update started".to_owned()),
                |_| {},
            )
        });
        barrier.wait();
        std::thread::sleep(Duration::from_millis(10));
        assert!(db.read(checkpoint_repository::latest).unwrap().is_none());

        manual_flight.finish(Err("manual test flight ended".to_owned()));
        *lock(&service.in_flight) = None;
        let automatic = worker.join().expect("automatic worker exits").unwrap();
        assert_eq!(automatic.kind, CheckpointKind::Automatic);
        assert_eq!(automatic.name, "Before update");
    }
}
