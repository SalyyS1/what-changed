#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use what_changed_lib::collectors::testing::{live_context, LiveContext};
use what_changed_lib::collectors::updates::{
    self, UpdateCollector, UpdateHistoryProvider, UpdateHistoryRow, OPERATION_INSTALLATION,
    OPERATION_UNINSTALLATION, RESULT_ABORTED, RESULT_FAILED, RESULT_SUCCEEDED,
};
use what_changed_lib::collectors::{
    Collector, CollectorSnapshot, ObservationClock, RawObservation, SnapshotContext,
};
use what_changed_lib::domain::{EventType, Importance};
use what_changed_lib::error::AppResult;
use what_changed_lib::storage::CollectorCommitReceipt;

#[derive(Clone, Default)]
struct ScriptedHistory(Arc<Mutex<Vec<UpdateHistoryRow>>>);

impl ScriptedHistory {
    fn set(&self, rows: Vec<UpdateHistoryRow>) {
        *self.0.lock().unwrap() = rows;
    }
}

impl UpdateHistoryProvider for ScriptedHistory {
    fn history(&self) -> AppResult<Vec<UpdateHistoryRow>> {
        Ok(self.0.lock().unwrap().clone())
    }
}

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    fn at(now_ms: i64) -> Self {
        Self(Arc::new(AtomicI64::new(now_ms)))
    }

    fn set(&self, now_ms: i64) {
        self.0.store(now_ms, Ordering::Release);
    }
}

impl ObservationClock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

type UnderTest = UpdateCollector<ScriptedHistory, ManualClock>;

fn row(index: usize, title: &str) -> UpdateHistoryRow {
    UpdateHistoryRow {
        update_id: format!("update-{index:03}"),
        revision: 1,
        title: title.to_owned(),
        date_ms: 1_700_000_000_000 + index as i64 * 1_000,
        result_code: RESULT_SUCCEEDED,
        operation: OPERATION_INSTALLATION,
        hresult: 0,
        client_application_id: "Windows Update".to_owned(),
        support_url: "https://support.microsoft.com/help/example".to_owned(),
    }
}

fn started(rows: Vec<UpdateHistoryRow>) -> (UnderTest, ScriptedHistory, LiveContext) {
    let provider = ScriptedHistory::default();
    provider.set(rows);
    let mut collector = UpdateCollector::with_clock(
        provider.clone(),
        updates::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_600_000),
    );
    let live = live_context(updates::ID).unwrap();
    collector.start(live.context.clone()).unwrap();
    (collector, provider, live)
}

fn snapshot(collector: &mut UnderTest, token_value: u64) -> CollectorSnapshot {
    collector
        .snapshot_batch(SnapshotContext::for_devtools(token_value))
        .unwrap()
}

fn cycle(collector: &mut UnderTest, token_value: u64) -> CollectorSnapshot {
    let snapshot = snapshot(collector, token_value);
    if let CollectorSnapshot::Atomic { token, .. } = &snapshot {
        collector
            .commit_succeeded(
                *token,
                &CollectorCommitReceipt {
                    event_ids: Vec::new(),
                },
            )
            .unwrap();
    }
    snapshot
}

fn observations(snapshot: &CollectorSnapshot) -> &[RawObservation] {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations,
        CollectorSnapshot::Noop => &[],
        other => panic!("expected atomic/noop snapshot, got {other:?}"),
    }
}

#[test]
fn a_133_row_baseline_is_silent_and_the_134th_entry_is_emitted_once() {
    let baseline: Vec<_> = (0..133)
        .map(|index| {
            row(
                index,
                &format!("Security Update (KB{:07})", 5_000_000 + index),
            )
        })
        .collect();
    let (mut collector, provider, _live) = started(baseline.clone());
    assert!(observations(&cycle(&mut collector, 1)).is_empty());
    assert!(observations(&cycle(&mut collector, 2)).is_empty());

    let mut next = baseline;
    next.push(row(133, "Cumulative Update (KB5034123)"));
    provider.set(next);
    let detected = cycle(&mut collector, 3);
    assert_eq!(observations(&detected).len(), 1);
    assert_eq!(
        observations(&detected)[0].kind.user_friendly_summary,
        "Windows Update installed KB5034123"
    );
    assert_eq!(
        observations(&detected)[0].occurred_at,
        row(133, "x").date_ms
    );
    assert!(observations(&cycle(&mut collector, 4)).is_empty());
}

#[test]
fn failed_and_aborted_results_use_failed_phrasing_and_important_classification() {
    let (mut collector, provider, _live) = started(Vec::new());
    cycle(&mut collector, 1);
    let mut failed = row(1, "Security Update (KB5034123)");
    failed.result_code = RESULT_FAILED;
    failed.hresult = 0x8007_0002u32 as i32;
    provider.set(vec![failed]);

    let detected = cycle(&mut collector, 2);
    let observation = &observations(&detected)[0];
    assert_eq!(observation.kind.event_type, EventType::Failed);
    assert_eq!(
        observation.kind.user_friendly_summary,
        "Windows Update failed: KB5034123 (error 0x80070002)"
    );
    let normalized =
        what_changed_lib::collectors::normalize::Normalizer::new().normalize(observation.clone());
    assert_eq!(normalized.get().importance, Importance::Important);

    let mut aborted = row(2, "Security Update (KB5034124)");
    aborted.result_code = RESULT_ABORTED;
    aborted.hresult = 0x8024_000Cu32 as i32;
    provider.set(vec![row(1, "Security Update (KB5034123)"), aborted]);
    let aborted = cycle(&mut collector, 3);
    assert_eq!(observations(&aborted)[0].kind.event_type, EventType::Failed);
    assert_eq!(
        observations(&aborted)[0].kind.user_friendly_summary,
        "Windows Update failed: KB5034124 (error 0x8024000C)"
    );
}

#[test]
fn successful_uninstall_maps_to_update_uninstalled() {
    let (mut collector, provider, _live) = started(Vec::new());
    cycle(&mut collector, 1);
    let mut removed = row(2, "Servicing Stack Update (KB5034441)");
    removed.operation = OPERATION_UNINSTALLATION;
    provider.set(vec![removed]);
    let detected = cycle(&mut collector, 2);
    assert_eq!(
        observations(&detected)[0].kind.event_type,
        EventType::Uninstalled
    );
    assert_eq!(
        observations(&detected)[0].kind.user_friendly_summary,
        "Windows Update uninstalled KB5034441"
    );
}

#[test]
fn successful_unknown_operation_is_acknowledged_without_inventing_an_install() {
    let (mut collector, provider, _live) = started(Vec::new());
    cycle(&mut collector, 1);
    let mut unknown = row(9, "Unknown WUA operation (KB5034999)");
    unknown.operation = 99;
    provider.set(vec![unknown]);
    assert!(observations(&cycle(&mut collector, 2)).is_empty());
    assert!(observations(&cycle(&mut collector, 3)).is_empty());
}

#[test]
fn future_wua_timestamp_is_clamped_without_poisoning_the_seen_baseline() {
    const DETECTED_AT: i64 = 1_700_000_600_000;
    let provider = ScriptedHistory::default();
    let clock = ManualClock::at(DETECTED_AT);
    let mut collector = UpdateCollector::with_clock(
        provider.clone(),
        updates::catalog_descriptor().unwrap(),
        clock.clone(),
    );
    let live = live_context(updates::ID).unwrap();
    collector.start(live.context.clone()).unwrap();
    cycle(&mut collector, 1);

    let mut future = row(10, "Future-dated Update (KB5035010)");
    future.date_ms = DETECTED_AT + 3_600_000;
    provider.set(vec![future]);

    let detected = cycle(&mut collector, 2);
    let (emitted, baseline) = match &detected {
        CollectorSnapshot::Atomic {
            observations,
            baseline: Some(baseline),
            ..
        } => (observations.as_slice(), baseline),
        other => panic!("expected atomic snapshot with baseline, got {other:?}"),
    };
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].occurred_at, DETECTED_AT);
    assert!(emitted[0].occurred_at_is_uncertain);
    assert!(baseline.captured_at >= emitted[0].occurred_at);

    // A backward clock correction must not change the WUA source identity.
    // The original future timestamp remains in `seen`, so the same history row
    // is not rediscovered with a new, clamped key on later cycles.
    clock.set(DETECTED_AT - 60_000);
    assert!(observations(&cycle(&mut collector, 3)).is_empty());
    clock.set(DETECTED_AT + 60_000);
    assert!(observations(&cycle(&mut collector, 4)).is_empty());
}

#[test]
fn kb_less_entries_use_a_stable_hash_of_update_id() {
    let (mut collector, provider, _live) = started(Vec::new());
    cycle(&mut collector, 1);
    let driver = row(3, "NVIDIA display driver 566.45");
    let expected = format!(
        "update:{}",
        what_changed_lib::domain::ids::short_hash(&driver.update_id)
    );
    provider.set(vec![driver]);
    let detected = cycle(&mut collector, 2);
    let normalized = what_changed_lib::collectors::normalize::Normalizer::new()
        .normalize(observations(&detected)[0].clone());
    assert_eq!(normalized.get().entity_id, expected);
}

#[test]
fn rejected_atomic_commit_does_not_advance_seen_history() {
    let (mut collector, provider, _live) = started(Vec::new());
    cycle(&mut collector, 1);
    provider.set(vec![row(4, "Security Update (KB5035000)")]);
    let first = snapshot(&mut collector, 2);
    let token = match &first {
        CollectorSnapshot::Atomic { token, .. } => *token,
        other => panic!("expected atomic snapshot, got {other:?}"),
    };
    collector.commit_failed(token).unwrap();
    assert_eq!(observations(&first).len(), 1);
    assert_eq!(observations(&cycle(&mut collector, 3)).len(), 1);
    assert!(observations(&cycle(&mut collector, 4)).is_empty());
}

#[test]
fn kb_extraction_requires_parentheses_and_six_or_seven_digits() {
    assert_eq!(
        updates::extract_kb("Security Update (KB5034123)"),
        Some("KB5034123".to_owned())
    );
    assert_eq!(
        updates::extract_kb("Older Update (KB123456)"),
        Some("KB123456".to_owned())
    );
    assert_eq!(updates::extract_kb("Update KB5034123"), None);
    assert_eq!(updates::extract_kb("Update (KB12345)"), None);
    assert_eq!(updates::extract_kb("Update (kb5034123)"), None);
}
