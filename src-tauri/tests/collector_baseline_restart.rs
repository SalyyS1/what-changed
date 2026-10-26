#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, RuntimeClock,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::updates::{
    self, UpdateCollector, UpdateHistoryProvider, UpdateHistoryRow, OPERATION_INSTALLATION,
    RESULT_SUCCEEDED,
};
use what_changed_lib::collectors::ObservationClock;
use what_changed_lib::domain::{CollectorCadence, MonitoringProfile};
use what_changed_lib::error::AppResult;

#[derive(Clone, Default)]
struct SharedHistory {
    rows: Arc<Mutex<Vec<UpdateHistoryRow>>>,
    calls: Arc<AtomicUsize>,
}

impl SharedHistory {
    fn set(&self, rows: Vec<UpdateHistoryRow>) {
        *self.rows.lock().unwrap() = rows;
    }
}

impl UpdateHistoryProvider for SharedHistory {
    fn history(&self) -> AppResult<Vec<UpdateHistoryRow>> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        Ok(self.rows.lock().unwrap().clone())
    }
}

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    fn at(now_ms: i64) -> Self {
        Self(Arc::new(AtomicI64::new(now_ms)))
    }
}

impl ObservationClock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

impl RuntimeClock for ManualClock {
    fn now(&self) -> Instant {
        static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let millis = self
            .0
            .load(Ordering::Acquire)
            .saturating_sub(1_700_000_000_000)
            .max(0) as u64;
        *BASE.get_or_init(Instant::now) + Duration::from_millis(millis)
    }

    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

struct FixedBackoff;

impl BackoffPolicy for FixedBackoff {
    fn delay(&self, _consecutive_failures: u64) -> Duration {
        Duration::from_millis(1)
    }
}

struct UpdateRig {
    db: Arc<what_changed_lib::storage::Db>,
    supervisor: CollectorSupervisor,
}

impl UpdateRig {
    fn at(directory: PathBuf, provider: SharedHistory, now_ms: i64) -> Self {
        let db = Arc::new(what_changed_lib::storage::Db::open_in(&directory).unwrap());
        let clock = ManualClock::at(now_ms);
        let mut descriptor = updates::catalog_descriptor().unwrap();
        descriptor.cadence = CollectorCadence::Poll(Duration::from_millis(10));
        let collector_descriptor = descriptor.clone();
        let collector_clock = clock.clone();
        let factory_provider = provider.clone();
        let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor, move || {
            Box::new(UpdateCollector::with_clock(
                factory_provider.clone(),
                collector_descriptor.clone(),
                collector_clock.clone(),
            ))
        })]);
        let supervisor = CollectorSupervisor::with_runtime(
            registry,
            CollectorRuntimeConfig {
                db: Arc::clone(&db),
                monitor: Arc::new(Monitoring::running()),
                query_busy: QueryBusy::default(),
                enabled: BTreeMap::from([(updates::ID.to_owned(), true)]),
                profile: MonitoringProfile::Balanced,
                clock: Arc::new(clock),
                backoff: Arc::new(FixedBackoff),
            },
        );
        supervisor.spawn_all().unwrap();
        let rig = Self { db, supervisor };
        rig.wait_for_cycle(now_ms);
        rig
    }

    fn wait_for_cycle(&self, target_ms: i64) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let ran = self
                .supervisor
                .list_collector_status()
                .into_iter()
                .find(|status| status.id == updates::ID)
                .and_then(|status| status.last_run_at)
                .is_some_and(|last_run| last_run >= target_ms);
            if ran {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("update collector did not finish its expected cycle");
    }

    fn event_count(&self) -> i64 {
        self.db
            .read(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM events WHERE source_collector = ?1",
                        [updates::ID],
                        |row| row.get(0),
                    )
                    .map_err(|source| {
                        what_changed_lib::error::DbError::Sqlite {
                            operation: "count restart-test update events",
                            source,
                        }
                        .into()
                    })
            })
            .unwrap()
    }

    fn events(&self) -> Vec<(String, bool)> {
        self.db
            .read(|connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT user_friendly_summary, occurred_at_is_uncertain \
                         FROM events WHERE source_collector = ?1 ORDER BY id",
                    )
                    .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                        operation: "prepare restart-test update events",
                        source,
                    })?;
                let rows = statement
                    .query_map([updates::ID], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
                    })
                    .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                        operation: "query restart-test update events",
                        source,
                    })?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|source| {
                    what_changed_lib::error::DbError::Sqlite {
                        operation: "read restart-test update events",
                        source,
                    }
                    .into()
                })
            })
            .unwrap()
    }

    fn stop(&self) {
        self.supervisor.stop_all();
    }
}

impl Drop for UpdateRig {
    fn drop(&mut self) {
        self.supervisor.stop_all();
    }
}

fn update(index: usize) -> UpdateHistoryRow {
    UpdateHistoryRow {
        update_id: format!("durable-update-{index}"),
        revision: 1,
        title: format!("Security Update (KB{:07})", 5_100_000 + index),
        date_ms: 1_700_000_000_000 + index as i64 * 1_000,
        result_code: RESULT_SUCCEEDED,
        operation: OPERATION_INSTALLATION,
        hresult: 0,
        client_application_id: "Windows Update".to_owned(),
        support_url: String::new(),
    }
}

#[test]
fn restarting_against_the_same_133_history_rows_does_not_replay_them() {
    let directory = what_changed_lib::testing::temp_dir("collector-updates-restart");
    let provider = SharedHistory::default();
    let mut rows: Vec<_> = (0..133).map(update).collect();
    provider.set(rows.clone());

    {
        let first = UpdateRig::at(directory.clone(), provider.clone(), 1_700_000_600_000);
        assert_eq!(first.event_count(), 0);
        first.stop();
    }

    {
        let resumed = UpdateRig::at(directory.clone(), provider.clone(), 1_700_001_200_000);
        assert_eq!(resumed.event_count(), 0);
        resumed.stop();
    }

    rows.push(update(133));
    provider.set(rows);
    let changed = UpdateRig::at(directory, provider, 1_700_001_800_000);
    assert_eq!(
        changed.events(),
        vec![("Windows Update installed KB5100133".to_owned(), true,)],
        "only the new entry is emitted, marked as discovered across downtime"
    );
}
