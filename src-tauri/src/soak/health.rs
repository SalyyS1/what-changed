//! Sticky run-local evidence; recovered failures cannot erase this record.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use serde::Serialize;

use crate::app::AppState;
use crate::collectors::runtime::RuntimeStatusSink;
use crate::dto::{CollectorState, CollectorStatusDto, MonitoringDto};

#[derive(Clone, Default, Serialize)]
pub(super) struct Outcomes {
    pub successful_snapshots: BTreeMap<String, u64>,
    pub failure_observations: BTreeMap<String, u64>,
    pub monitoring_paused: bool,
}

#[derive(Default)]
pub(super) struct Evidence(Mutex<Outcomes>);

impl Evidence {
    pub fn snapshot(&self) -> Outcomes {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl RuntimeStatusSink for Evidence {
    fn collector_changed(&self, _: &str) {}
    fn monitoring_changed(&self, monitoring: MonitoringDto) {
        if monitoring.paused {
            self.0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .monitoring_paused = true;
        }
    }
    fn collector_outcome_observed(&self, id: &str, success: bool) {
        let mut evidence = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let counts = if success {
            &mut evidence.successful_snapshots
        } else {
            &mut evidence.failure_observations
        };
        *counts.entry(id.to_owned()).or_default() += 1;
    }
}

pub(super) fn inspect_rows(
    rows: &[CollectorStatusDto],
    expected: &[String],
    outcomes: &Outcomes,
) -> Result<bool, String> {
    let expected = expected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let actual = rows
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    if expected.is_empty() || actual != expected || rows.len() != expected.len() {
        return Err("collector registry does not match the full production denominator".to_owned());
    }
    if outcomes
        .successful_snapshots
        .keys()
        .chain(outcomes.failure_observations.keys())
        .any(|id| !expected.contains(id.as_str()))
    {
        return Err("unknown collector outcome was observed".to_owned());
    }
    if outcomes.monitoring_paused || !outcomes.failure_observations.is_empty() {
        return Err("collector failure or pause was observed during this run".to_owned());
    }
    for row in rows {
        if !row.enabled
            || !matches!(row.state, CollectorState::Running | CollectorState::Idle)
            || row.consecutive_failures > 0
            || row.last_error.is_some()
            || row.queue_drops > 0
            || row.failed_commit_rows > 0
            || row.cycle_admission_drops > 0
            || row.failed_cycles > 0
        {
            return Err(format!("enabled collector is unhealthy: {}", row.id));
        }
    }
    Ok(expected
        .iter()
        .all(|id| outcomes.successful_snapshots.get(*id).copied().unwrap_or(0) > 0))
}

pub(super) fn writer_loss(state: &AppState) -> u64 {
    state
        .db
        .dropped_writes()
        .saturating_add(state.db.failed_commit_rows().values().copied().sum::<u64>())
        .saturating_add(
            state
                .db
                .collector_cycle_admission_drops()
                .values()
                .copied()
                .sum::<u64>(),
        )
        .saturating_add(
            state
                .db
                .failed_collector_cycles()
                .values()
                .copied()
                .sum::<u64>(),
        )
}

pub(super) fn inspect(
    state: &AppState,
    expected: &[String],
    evidence: &Evidence,
) -> Result<bool, String> {
    if writer_loss(state) > 0 {
        return Err("writer lifetime loss counters are nonzero".to_owned());
    }
    inspect_rows(
        &state.collectors.list_collector_status(),
        expected,
        &evidence.snapshot(),
    )
}
