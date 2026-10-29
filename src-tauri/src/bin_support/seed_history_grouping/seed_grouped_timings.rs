//! Group attribution is separate from legacy repository commit records.

use std::time::{Duration, Instant};

use what_changed_lib::storage::event_repository::BodyInsertionTimings;

#[derive(Clone, Copy, Default)]
pub(crate) struct GroupTimings {
    pub attempted_groups: usize,
    pub committed_groups: usize,
    pub failed_groups: usize,
    /// Submitted group inputs, including units not reached after a failure.
    pub attempted_input_rows: usize,
    pub started_units: usize,
    pub completed_body_rows: usize,
    /// Event and graph bodies completed; still provisional before commit.
    pub completed_units: usize,
    pub committed_input_rows: usize,
    pub body: BodyInsertionTimings,
    pub begin: Duration,
    pub graph: Duration,
    pub commit: Duration,
    /// Writer-local total including explicit rollback and transaction disposal.
    pub group_total: Duration,
}

impl GroupTimings {
    pub fn accumulate(&mut self, other: Self) {
        self.attempted_groups += other.attempted_groups;
        self.committed_groups += other.committed_groups;
        self.failed_groups += other.failed_groups;
        self.attempted_input_rows += other.attempted_input_rows;
        self.started_units += other.started_units;
        self.completed_body_rows += other.completed_body_rows;
        self.completed_units += other.completed_units;
        self.committed_input_rows += other.committed_input_rows;
        self.add_body(other.body);
        self.begin += other.begin;
        self.graph += other.graph;
        self.commit += other.commit;
        self.group_total += other.group_total;
    }

    pub fn add_body(&mut self, body: BodyInsertionTimings) {
        self.body.bodies += body.bodies;
        self.body.attempted_rows += body.attempted_rows;
        self.body.completed_bodies += body.completed_bodies;
        self.body.completed_rows += body.completed_rows;
        self.body.failed_bodies += body.failed_bodies;
        self.body.empty_bodies += body.empty_bodies;
        self.body.completed_phases += body.completed_phases;
        self.body.preparation += body.preparation;
        self.body.event_entity_loop += body.event_entity_loop;
        self.body.reconciliation += body.reconciliation;
        self.body.body_total += body.body_total;
    }
}

pub(super) fn start(capture: bool) -> Option<Instant> {
    capture.then(clock)
}

pub(super) fn elapsed(started: Option<Instant>) -> Duration {
    started.map_or(Duration::ZERO, |started| clock().duration_since(started))
}

fn clock() -> Instant {
    CLOCK_READS.with(|reads| reads.set(reads.get() + 1));
    Instant::now()
}

thread_local! {
    pub(super) static CLOCK_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
