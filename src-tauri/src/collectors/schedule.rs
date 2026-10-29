//! Cost-aware scheduling and the UI-query gate.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::domain::{CollectorCadence, CollectorCost, MonitoringProfile};

#[derive(Clone, Debug, Default)]
pub struct QueryBusy(Arc<AtomicUsize>);

impl QueryBusy {
    pub fn enter(&self) -> QueryGuard {
        self.0.fetch_add(1, Ordering::AcqRel);
        QueryGuard(self.clone())
    }
    pub fn is_busy(&self) -> bool {
        self.0.load(Ordering::Acquire) != 0
    }
}

pub struct QueryGuard(QueryBusy);
impl Drop for QueryGuard {
    fn drop(&mut self) {
        self.0 .0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub fn interval(cadence: CollectorCadence, profile: MonitoringProfile) -> Duration {
    let base = match cadence {
        CollectorCadence::Poll(value) | CollectorCadence::NotifyPlusSweep(value) => value,
        CollectorCadence::Notify => Duration::from_secs(60 * 60),
    };
    match profile {
        MonitoringProfile::Minimal => base.saturating_mul(2),
        MonitoringProfile::Balanced => base,
        MonitoringProfile::Thorough => base.div_f32(2.0).max(Duration::from_millis(10)),
    }
}

/// Effective cadence after profile scaling and the expensive-sweep floor.
pub fn collector_interval(
    cadence: CollectorCadence,
    cost: CollectorCost,
    profile: MonitoringProfile,
) -> Duration {
    let interval = interval(cadence, profile);
    if cost == CollectorCost::Expensive {
        interval.max(Duration::from_secs(30))
    } else {
        interval
    }
}

pub fn expensive_stagger(id: &str, cost: CollectorCost) -> Duration {
    if cost != CollectorCost::Expensive {
        return Duration::ZERO;
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hash);
    Duration::from_secs(hash.finish() % 30)
}

/// Assign collision-free positions inside the 30-second expensive window.
///
/// Sorting makes the assignment stable even if registry construction order
/// changes. A product registry with at most 30 expensive collectors receives
/// one distinct whole-second slot per collector.
pub fn expensive_staggers(ids: &[&str]) -> BTreeMap<String, Duration> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let count = ids.len();
    if count == 0 {
        return BTreeMap::new();
    }
    ids.into_iter()
        .enumerate()
        .map(|(index, id)| {
            let slot = if count <= 30 {
                index * 30 / count
            } else {
                index % 30
            };
            (id.to_owned(), Duration::from_secs(slot as u64))
        })
        .collect()
}
