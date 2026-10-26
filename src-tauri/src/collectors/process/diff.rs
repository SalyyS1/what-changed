//! The difference between what storage knows and what the machine is doing.
//!
//! A pid is unique among live processes at one instant and reused afterwards, so
//! it is a map key and never an identity. Continuity is decided by creation time
//! when both sides have one and by the carried-forward id when neither does —
//! which is what stops a recycled pid from forging a stop/start pair for two
//! unrelated programs, and stops a path read flipping from forging one for the
//! same program.

use std::collections::BTreeMap;

use crate::collectors::process::state::TrackedInstance;

/// One indivisible difference between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A pid that is running and was not.
    Started(TrackedInstance),
    /// A pid that was running and is not.
    Stopped(TrackedInstance),
    /// One pid reused: the old instance ended and a different one began.
    ///
    /// Both halves persist in one transition. Splitting them would let a crash
    /// between the two commits record a program that stopped and never came
    /// back, which is the false fact the pairing exists to prevent.
    Reused {
        /// The instance that ended.
        stopped: TrackedInstance,
        /// The instance that began on the same pid.
        started: TrackedInstance,
    },
}

impl Change {
    /// Event rows this change writes.
    pub const fn observation_count(&self) -> usize {
        match self {
            Change::Reused { .. } => 2,
            Change::Started(_) | Change::Stopped(_) => 1,
        }
    }
}

/// Every difference between the acknowledged view and this cycle's snapshot.
///
/// Ordered by pid, so a burst arrives in a stable order and a remainder left
/// over from one cycle resumes in the same order on the next.
pub fn diff(
    acknowledged: &BTreeMap<u32, TrackedInstance>,
    current: &BTreeMap<u32, TrackedInstance>,
) -> Vec<Change> {
    let mut changes = Vec::new();
    for (pid, instance) in current {
        match acknowledged.get(pid) {
            None => changes.push(Change::Started(instance.clone())),
            Some(previous) if is_continuation(previous, instance) => {}
            Some(previous) => changes.push(Change::Reused {
                stopped: previous.clone(),
                started: instance.clone(),
            }),
        }
    }
    for (pid, instance) in acknowledged {
        if !current.contains_key(pid) {
            changes.push(Change::Stopped(instance.clone()));
        }
    }
    changes
}

/// Whether one pid still names the process it named last cycle.
pub(super) fn is_continuation(previous: &TrackedInstance, current: &TrackedInstance) -> bool {
    same_identity(
        previous.creation,
        &previous.entity_id,
        current.creation,
        &current.entity_id,
    )
}

/// Collector-owned process continuity rule reused by checkpoint comparison.
///
/// A newly readable creation timestamp promotes a weak identity without
/// inventing a restart. Once both timestamps are authoritative, however, a
/// difference is PID reuse even when the executable identity is unchanged.
pub(crate) fn same_identity(
    previous_creation: Option<u64>,
    previous_entity: &crate::domain::ids::ProcessId,
    current_creation: Option<u64>,
    current_entity: &crate::domain::ids::ProcessId,
) -> bool {
    match (previous_creation, current_creation) {
        // Authoritative when both sides could open a handle. The same pid
        // running the same executable again is still a different instance, and
        // the identity alone cannot say so.
        (Some(before), Some(after)) => before == after,
        // Otherwise the id carried forward by the snapshot builder decides, so
        // a readability flip is continuity rather than a restart.
        _ => previous_entity == current_entity,
    }
}

/// Record one change in the acknowledged view.
pub fn apply(acknowledged: &mut BTreeMap<u32, TrackedInstance>, change: &Change) {
    match change {
        Change::Started(instance) => insert_committed(acknowledged, instance),
        Change::Stopped(instance) => {
            acknowledged.remove(&instance.pid);
        }
        Change::Reused { stopped, started } => {
            acknowledged.remove(&stopped.pid);
            insert_committed(acknowledged, started);
        }
    }
}

/// Store a committed instance, marking its start as observed and dropping the
/// executable path.
///
/// The path has done its work by now: it is in the start event's details and
/// hashed into the id. Keeping it here would hold one string per running
/// process for the life of the app, and would be the only place a path could
/// outlive the cycle that read it.
fn insert_committed(acknowledged: &mut BTreeMap<u32, TrackedInstance>, instance: &TrackedInstance) {
    acknowledged.insert(
        instance.pid,
        TrackedInstance {
            observed_start: true,
            executable: None,
            ..instance.clone()
        },
    );
}

/// How many leading changes fit inside `budget` event rows.
///
/// A change is admitted whole or not at all, so a burst larger than one commit
/// makes progress over several cycles instead of truncating or retrying an
/// oversized batch forever. A two-row pid reuse waits when only one row remains;
/// taking it anyway would split or overflow the indivisible transition.
pub fn fitting(changes: &[Change], budget: usize) -> usize {
    let mut used = 0usize;
    let mut count = 0usize;
    for change in changes {
        let next = used.saturating_add(change.observation_count());
        if next > budget {
            break;
        }
        used = next;
        count += 1;
    }
    count
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::ids::ProcessId;
    use crate::storage::MAX_ATOMIC_OBSERVATIONS;

    fn instance(pid: u32, image: &str, creation: Option<u64>) -> TrackedInstance {
        TrackedInstance {
            pid,
            creation,
            creation_available: creation.is_some(),
            image: image.to_owned(),
            entity_id: ProcessId::from_executable(image),
            identity_path_based: true,
            path_available: true,
            executable: Some(format!(r"C:\dev\{image}")),
            browser_subtree: false,
            observed_start: false,
        }
    }

    fn map(instances: &[TrackedInstance]) -> BTreeMap<u32, TrackedInstance> {
        instances
            .iter()
            .map(|instance| (instance.pid, instance.clone()))
            .collect()
    }

    #[test]
    fn a_recycled_pid_running_the_same_program_is_still_a_different_instance() {
        let acknowledged = map(&[instance(1_234, "api.exe", Some(10))]);
        let current = map(&[instance(1_234, "api.exe", Some(11))]);
        let changes = diff(&acknowledged, &current);
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            Change::Reused { stopped, started } => {
                assert_eq!(stopped.creation, Some(10));
                assert_eq!(started.creation, Some(11));
                assert_eq!(
                    stopped.entity_id, started.entity_id,
                    "the same executable is the same entity, restarted"
                );
            }
            other => panic!("expected a reuse, got {other:?}"),
        }
        assert_eq!(changes[0].observation_count(), 2);
    }

    #[test]
    fn a_path_read_flipping_is_continuity_rather_than_a_restart() {
        let acknowledged = map(&[instance(1, "api.exe", None)]);
        let mut flipped = instance(1, "api.exe", None);
        flipped.executable = None;
        flipped.path_available = false;
        let changes = diff(&acknowledged, &map(&[flipped]));
        assert!(changes.is_empty(), "nothing about the program changed");
    }

    #[test]
    fn starts_and_stops_are_reported_separately_and_in_pid_order() {
        let acknowledged = map(&[
            instance(5, "gone.exe", Some(1)),
            instance(9, "stays.exe", Some(2)),
        ]);
        let current = map(&[
            instance(2, "new.exe", Some(3)),
            instance(9, "stays.exe", Some(2)),
        ]);
        let changes = diff(&acknowledged, &current);
        assert_eq!(
            changes,
            vec![
                Change::Started(instance(2, "new.exe", Some(3))),
                Change::Stopped(instance(5, "gone.exe", Some(1))),
            ]
        );
    }

    #[test]
    fn applying_a_change_makes_the_acknowledged_view_match_and_drops_the_path() {
        let mut acknowledged = map(&[instance(5, "gone.exe", Some(1))]);
        let started = instance(7, "new.exe", Some(4));
        let reused = Change::Reused {
            stopped: instance(5, "gone.exe", Some(1)),
            started: instance(5, "other.exe", Some(9)),
        };

        apply(&mut acknowledged, &Change::Started(started.clone()));
        apply(&mut acknowledged, &reused);
        assert_eq!(acknowledged.len(), 2, "one new pid and one recycled one");

        let committed = &acknowledged[&7];
        assert!(committed.observed_start, "storage was told it started");
        assert_eq!(
            committed.executable, None,
            "the path does not outlive the cycle"
        );
        assert_eq!(committed.entity_id, started.entity_id);

        let recycled = &acknowledged[&5];
        assert_eq!(
            recycled.image, "other.exe",
            "the reuse replaced the instance"
        );
        assert_eq!(recycled.creation, Some(9));
        assert!(recycled.observed_start);
        assert_eq!(recycled.executable, None);
    }

    #[test]
    fn a_burst_larger_than_one_commit_is_drained_a_commit_at_a_time() {
        let changes: Vec<Change> = (0..(MAX_ATOMIC_OBSERVATIONS * 2))
            .map(|index| Change::Started(instance(index as u32, "x.exe", Some(index as u64))))
            .collect();
        let first = fitting(&changes, MAX_ATOMIC_OBSERVATIONS);
        assert_eq!(first, MAX_ATOMIC_OBSERVATIONS);
        assert!(
            fitting(&changes[first..], MAX_ATOMIC_OBSERVATIONS) > 0,
            "the remainder makes progress instead of retrying the same batch"
        );
        assert_eq!(fitting(&[], MAX_ATOMIC_OBSERVATIONS), 0);
        assert_eq!(
            fitting(&changes, 0),
            0,
            "no room at all is not a reason to overflow the bound"
        );
        assert_eq!(
            fitting(&changes[..1], 1),
            1,
            "one change is always taken when a row fits"
        );
    }

    #[test]
    fn a_reuse_costs_two_rows_and_never_splits_across_a_commit_boundary() {
        // 199 single-row changes plus one reuse: the reuse would be the 200th
        // and 201st rows, so it must wait whole for the next cycle.
        let mut changes: Vec<Change> = (0..(MAX_ATOMIC_OBSERVATIONS - 1))
            .map(|index| Change::Started(instance(index as u32, "x.exe", Some(1))))
            .collect();
        changes.push(Change::Reused {
            stopped: instance(900, "x.exe", Some(1)),
            started: instance(900, "x.exe", Some(2)),
        });
        let taken = fitting(&changes, MAX_ATOMIC_OBSERVATIONS);
        assert_eq!(taken, MAX_ATOMIC_OBSERVATIONS - 1);
        assert!(matches!(changes[taken], Change::Reused { .. }));
        assert_eq!(fitting(&changes[taken..], MAX_ATOMIC_OBSERVATIONS), 1);
        assert_eq!(
            fitting(&changes[taken..], 1),
            0,
            "one remaining row cannot split or overflow a two-row reuse"
        );
    }
}
