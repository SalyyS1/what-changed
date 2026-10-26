//! Shared machinery for collectors that diff a durable baseline.
//!
//! A stateful collector has to answer the same three questions every cycle: how
//! much of what it just found fits in one atomic commit, whether the gap since
//! the last durable snapshot makes occurrence times unknowable, and what to
//! persist so a recreated instance resumes instead of restarting. The answers
//! live here so two collectors cannot drift apart on them.

use crate::collectors::RawBaseline;
use crate::error::{AppError, AppResult};
use crate::storage::{
    baseline_repository, Baseline, MAX_ATOMIC_OBSERVATIONS, MAX_ATOMIC_RELATED_ENTITIES,
    MAX_ATOMIC_RELATIONSHIPS,
};

/// A provider snapshot that could not fit in one atomic commit.
///
/// The complete target and the facts about when it was observed are durable.
/// Later cycles drain against this target instead of re-diffing a newer OS
/// view, which is what preserves short-lived changes and keeps every chunk on
/// the timestamp that actually detected the burst.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingTarget<T> {
    /// The complete provider view this drain is converging toward.
    pub target: T,
    /// Detection time of that provider view, in epoch milliseconds UTC.
    pub observed_at: i64,
    /// Whether the view was first observed across recorder downtime.
    pub occurred_at_is_uncertain: bool,
}

/// A gap longer than this between a durable baseline and now means the changes
/// it implies happened while nothing was watching.
///
/// Ten seconds is ten process polls and five listener polls: long enough that
/// an ordinary scheduling delay or one slow cycle is not mistaken for downtime,
/// short enough that a pause or a restart is caught.
pub const DOWNTIME_GAP_MS: i64 = 10_000;

/// Whether a change discovered against a baseline captured at `captured_at` has
/// an unknowable occurrence time.
///
/// The *detection* time is still known and is what the event carries. What is
/// unknown is where inside the gap the change actually happened, and the
/// analysis engine scores an uncertain timestamp at zero rather than reasoning
/// over a fabricated one.
pub fn spans_downtime(captured_at: i64, now_ms: i64) -> bool {
    now_ms.saturating_sub(captured_at) > DOWNTIME_GAP_MS
}

/// Adopt a durable baseline, and say whether the next diff spans a gap.
///
/// The gap is reported beside the state rather than stored inside it, because it
/// describes this instance's first cycle and not the machine: persisting it
/// would let a stale `true` survive into a session that has been watching
/// continuously.
///
/// A baseline that cannot be read is a reason to re-baseline silently, not to
/// fail. The alternative is a collector that parks itself on one corrupt row and
/// never records anything again, which trades a missing history for no history.
pub fn hydrate<T>(baseline: Option<Baseline>, now_ms: i64, describe: &'static str) -> (T, bool)
where
    T: serde::de::DeserializeOwned + Default,
{
    let Some(baseline) = baseline.filter(|stored| !stored.truncated) else {
        return (T::default(), false);
    };
    let downtime = spans_downtime(baseline.captured_at, now_ms);
    match serde_json::from_value::<T>(baseline.snapshot.clone()) {
        Ok(state) => (state, downtime),
        Err(error) => {
            tracing::warn!(
                reason = %error,
                state = describe,
                "a stored collector baseline could not be read; re-baselining silently"
            );
            (T::default(), false)
        }
    }
}

/// Describe the durable half of one atomic cycle.
///
/// Serialized before the caller stages anything, so a state that cannot be
/// represented leaves no candidate behind it.
pub fn baseline_for(
    entity_kind: &str,
    state: &impl serde::Serialize,
    captured_at: i64,
) -> AppResult<RawBaseline> {
    let snapshot = serde_json::to_value(state).map_err(|_| AppError::InvalidInput {
        what: "collector baseline",
        reason: "the collector state could not be serialized".to_owned(),
    })?;
    let bytes = serde_json::to_vec(&snapshot).map_err(|_| AppError::InvalidInput {
        what: "collector baseline",
        reason: "the collector state could not be measured".to_owned(),
    })?;
    if bytes.len() > baseline_repository::MAX_SNAPSHOT_BYTES {
        return Err(AppError::InvalidInput {
            what: "collector baseline",
            reason: "the collector state is larger than the atomic commit limit".to_owned(),
        });
    }
    Ok(RawBaseline {
        entity_kind: entity_kind.to_owned(),
        snapshot,
        captured_at,
    })
}

/// Build a durable baseline when a provider view is larger than the storage
/// envelope.  The complete state remains in memory for the current process,
/// while the persisted marker makes the next process quietly re-baseline
/// instead of retrying the same oversized transaction forever.  The marker
/// contains only its bounded byte count; it never contains a provider value or
/// a digest derived from one.
pub fn baseline_for_or_marker(
    entity_kind: &str,
    state: &impl serde::Serialize,
    captured_at: i64,
) -> AppResult<RawBaseline> {
    match baseline_for(entity_kind, state, captured_at) {
        Ok(baseline) => Ok(baseline),
        Err(AppError::InvalidInput {
            what: "collector baseline",
            ..
        }) => {
            let bytes = serde_json::to_vec(state).map_err(|_| AppError::InvalidInput {
                what: "collector baseline",
                reason: "the collector state could not be serialized".to_owned(),
            })?;
            Ok(RawBaseline {
                entity_kind: entity_kind.to_owned(),
                snapshot: serde_json::json!({
                    "truncated": true,
                    "bytes": bytes.len(),
                }),
                captured_at,
            })
        }
        Err(error) => Err(error),
    }
}

/// Remaining room inside one atomic collector commit.
///
/// A cycle that found more change than one commit may carry delivers what fits
/// and leaves the rest to the next cycle. That makes progress on a burst
/// without truncating anything and without retrying an oversized batch forever;
/// delivery simply takes extra cycles. The three bounds belong to the storage
/// layer and are read from it rather than restated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitCapacity {
    observations: usize,
    relationships: usize,
    related_entities: usize,
}

impl CommitCapacity {
    /// A full commit's worth of room.
    pub const fn full() -> Self {
        Self {
            observations: MAX_ATOMIC_OBSERVATIONS,
            relationships: MAX_ATOMIC_RELATIONSHIPS,
            related_entities: MAX_ATOMIC_RELATED_ENTITIES,
        }
    }

    /// Whether one more indivisible change still fits.
    ///
    /// A change is admitted whole or not at all. Splitting a pid reuse into a
    /// stop in one commit and a start in the next would let a crash between the
    /// two record a program that stopped and never came back, which is exactly
    /// the false fact the pairing exists to prevent.
    pub const fn fits(
        &self,
        observations: usize,
        relationships: usize,
        related_entities: usize,
    ) -> bool {
        self.observations >= observations
            && self.relationships >= relationships
            && self.related_entities >= related_entities
    }

    /// Reserve room for one indivisible change.
    ///
    /// Reserves nothing when the change does not fit, so a caller that stops at
    /// the first refusal has not part-paid for the change it declined.
    pub fn spend(
        &mut self,
        observations: usize,
        relationships: usize,
        related_entities: usize,
    ) -> bool {
        if !self.fits(observations, relationships, related_entities) {
            return false;
        }
        self.observations -= observations;
        self.relationships -= relationships;
        self.related_entities -= related_entities;
        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
    struct Probe {
        established: bool,
    }

    fn stored(snapshot: serde_json::Value, captured_at: i64, truncated: bool) -> Baseline {
        Baseline {
            collector_id: "test".to_owned(),
            entity_kind: "kind".to_owned(),
            snapshot,
            captured_at,
            truncated,
        }
    }

    #[test]
    fn a_gap_has_to_outlast_ordinary_scheduling_delay_to_count_as_downtime() {
        assert!(!spans_downtime(1_000, 1_000));
        assert!(!spans_downtime(1_000, 1_000 + DOWNTIME_GAP_MS));
        assert!(spans_downtime(1_000, 1_000 + DOWNTIME_GAP_MS + 1));
        assert!(
            !spans_downtime(2_000, 1_000),
            "a baseline from the future is not a gap"
        );
    }

    #[test]
    fn hydration_reports_the_gap_beside_the_state_and_never_inside_it() {
        let snapshot = serde_json::json!({ "established": true });
        let (state, downtime) = hydrate::<Probe>(
            Some(stored(snapshot.clone(), 1_000, false)),
            1_000 + DOWNTIME_GAP_MS,
            "test",
        );
        assert!(!downtime);
        assert!(state.established);

        let (_, downtime) = hydrate::<Probe>(
            Some(stored(snapshot, 1_000, false)),
            1_000 + DOWNTIME_GAP_MS + 1,
            "test",
        );
        assert!(downtime, "a restart or a long pause is a gap");
    }

    #[test]
    fn an_unreadable_baseline_re_baselines_instead_of_parking_the_collector() {
        let corrupt = stored(serde_json::json!("not a state"), 1_000, false);
        assert_eq!(
            hydrate::<Probe>(Some(corrupt), 2_000, "test"),
            (Probe::default(), false)
        );

        let marker = stored(
            serde_json::json!({ "truncated": true, "bytes": 9 }),
            1_000,
            true,
        );
        assert_eq!(
            hydrate::<Probe>(Some(marker), 2_000, "test"),
            (Probe::default(), false),
            "a marker standing in for an oversized snapshot is not a state"
        );

        assert_eq!(
            hydrate::<Probe>(None, 2_000, "test"),
            (Probe::default(), false)
        );
    }

    #[test]
    fn a_baseline_describes_exactly_the_state_it_was_given() {
        let baseline = baseline_for("kind", &Probe { established: true }, 5_000).unwrap();
        assert_eq!(baseline.entity_kind, "kind");
        assert_eq!(baseline.captured_at, 5_000);
        assert_eq!(baseline.snapshot["established"], true);
    }

    #[test]
    fn an_oversized_baseline_is_rejected_before_a_collector_can_stage_it() {
        let oversized = "x".repeat(baseline_repository::MAX_SNAPSHOT_BYTES + 1);
        let error = baseline_for("kind", &oversized, 5_000).unwrap_err();
        assert!(matches!(
            error,
            AppError::InvalidInput {
                what: "collector baseline",
                ..
            }
        ));
    }

    #[test]
    fn an_oversized_baseline_can_be_acknowledged_as_a_bounded_marker() {
        let oversized = "x".repeat(baseline_repository::MAX_SNAPSHOT_BYTES + 1);
        let baseline = baseline_for_or_marker("kind", &oversized, 5_000).unwrap();
        assert_eq!(baseline.snapshot["truncated"], true);
        assert_eq!(
            baseline.snapshot["bytes"],
            serde_json::to_vec(&oversized).unwrap().len()
        );
        assert!(serde_json::to_vec(&baseline.snapshot).unwrap().len() < 128);
    }

    #[test]
    fn capacity_admits_a_whole_change_at_the_boundary_and_refuses_one_more() {
        let capacity = CommitCapacity::full();
        assert!(capacity.fits(MAX_ATOMIC_OBSERVATIONS, 0, 0));
        assert!(!capacity.fits(MAX_ATOMIC_OBSERVATIONS + 1, 0, 0));
        assert!(capacity.fits(1, MAX_ATOMIC_RELATIONSHIPS, 0));
        assert!(!capacity.fits(1, MAX_ATOMIC_RELATIONSHIPS + 1, 0));
        assert!(capacity.fits(1, 0, MAX_ATOMIC_RELATED_ENTITIES));
        assert!(!capacity.fits(1, 0, MAX_ATOMIC_RELATED_ENTITIES + 1));
    }

    #[test]
    fn one_listened_port_with_two_directions_of_evidence_fits_inside_a_full_commit() {
        // The listener collector's worst ordinary case: every observation in
        // the batch is a newly opened port carrying both graph directions and
        // one related process fact.
        let capacity = CommitCapacity::full();
        assert!(capacity.fits(
            MAX_ATOMIC_OBSERVATIONS,
            MAX_ATOMIC_OBSERVATIONS * 2,
            MAX_ATOMIC_OBSERVATIONS
        ));
    }

    #[test]
    fn spending_reserves_whole_changes_and_refuses_the_one_that_overflows() {
        let mut capacity = CommitCapacity::full();
        for _ in 0..MAX_ATOMIC_OBSERVATIONS {
            assert!(capacity.spend(1, 2, 1), "one opened port with both edges");
        }
        assert!(
            !capacity.spend(1, 2, 1),
            "the relationship bound is reached at the same time"
        );
        assert!(
            !capacity.fits(1, 0, 0),
            "a refusal reserves nothing, and the room really is gone"
        );

        let mut relationship_limited = CommitCapacity::full();
        assert!(relationship_limited.spend(1, MAX_ATOMIC_RELATIONSHIPS, 1));
        assert!(!relationship_limited.spend(1, 1, 0));
    }
}
