//! Explicit, writer-local insertion attribution. No ambient capture state.

use crate::error::AppResult;
use std::time::{Duration, Instant};

/// Aggregate durations include incomplete work on an error, not just successful
/// phases. Attempted rows are inputs; committed rows are inputs accepted by
/// successful nonempty transactions, not newly distinct persisted event IDs.
/// Nothing here retains event identities or error details.
#[derive(Debug, Clone, Copy, Default)]
pub struct BatchInsertionTimings {
    pub batches: usize,
    pub attempted_rows: usize,
    pub committed_batches: usize,
    pub committed_rows: usize,
    pub failed_batches: usize,
    pub empty_batches: usize,
    pub completed_phases: usize,
    pub begin_prepare: Duration,
    pub event_entity_loop: Duration,
    pub reconciliation: Duration,
    pub commit: Duration,
    pub repository_total: Duration,
}

impl BatchInsertionTimings {
    pub fn accumulate(&mut self, other: Self) {
        self.batches += other.batches;
        self.attempted_rows += other.attempted_rows;
        self.committed_batches += other.committed_batches;
        self.committed_rows += other.committed_rows;
        self.failed_batches += other.failed_batches;
        self.empty_batches += other.empty_batches;
        self.completed_phases += other.completed_phases;
        self.begin_prepare += other.begin_prepare;
        self.event_entity_loop += other.event_entity_loop;
        self.reconciliation += other.reconciliation;
        self.commit += other.commit;
        self.repository_total += other.repository_total;
    }
}

/// The ordinary operation result travels separately from numeric attribution.
/// Writer queue admission failures remain outer API errors, not fake batches.
pub struct CapturedBatchInsertion<R> {
    pub value: R,
    pub timings: BatchInsertionTimings,
}

pub type PreservingEntityInsertion = (Vec<i64>, std::collections::HashSet<String>);

pub type CapturedPreservingInsertion = CapturedBatchInsertion<AppResult<PreservingEntityInsertion>>;

/// Existing-transaction body attribution, not a commit acknowledgement.
/// Completed rows mean all input processing and reconciliation succeeded; they
/// remain provisional until the outer transaction commits. Durations include
/// incomplete phases on error, but exclude outer begin, commit and rollback.
#[derive(Debug, Clone, Copy, Default)]
pub struct BodyInsertionTimings {
    pub bodies: usize,
    pub attempted_rows: usize,
    pub completed_bodies: usize,
    pub completed_rows: usize,
    pub failed_bodies: usize,
    pub empty_bodies: usize,
    pub completed_phases: usize,
    pub preparation: Duration,
    pub event_entity_loop: Duration,
    pub reconciliation: Duration,
    pub body_total: Duration,
}

/// Provisional ownership exists only in the successful operation value, never
/// in numeric attribution or an error result. Capture is explicitly optional.
pub struct InsertionBodyOutcome {
    pub value: AppResult<PreservingEntityInsertion>,
    pub timings: Option<BodyInsertionTimings>,
}

#[derive(Clone, Copy)]
pub(super) enum Phase {
    BeginPrepare,
    EventEntityLoop,
    Reconciliation,
    Commit,
    Complete,
}

pub(super) struct Recorder {
    started: Instant,
    checkpoint: Instant,
    phase: Phase,
    timings: BatchInsertionTimings,
}

impl Recorder {
    pub(super) fn start(rows: usize) -> Self {
        let started = clock();
        Self {
            started,
            checkpoint: started,
            phase: Phase::BeginPrepare,
            timings: BatchInsertionTimings {
                batches: 1,
                attempted_rows: rows,
                empty_batches: usize::from(rows == 0),
                ..BatchInsertionTimings::default()
            },
        }
    }

    pub(super) fn advance(&mut self, next: Phase) {
        let now = clock();
        self.record_phase(now.duration_since(self.checkpoint));
        self.timings.completed_phases += 1;
        self.checkpoint = now;
        self.phase = next;
    }

    fn record_phase(&mut self, elapsed: Duration) {
        match self.phase {
            Phase::BeginPrepare => self.timings.begin_prepare += elapsed,
            Phase::EventEntityLoop => self.timings.event_entity_loop += elapsed,
            Phase::Reconciliation => self.timings.reconciliation += elapsed,
            Phase::Commit => self.timings.commit += elapsed,
            Phase::Complete => {}
        }
    }

    /// Called only after the repository returns, so total includes statement and
    /// transaction drops (including rollback on error). A panic drops this local
    /// recorder; no state survives for a subsequent call or another thread.
    pub(super) fn finish(mut self, success: bool) -> BatchInsertionTimings {
        let now = clock();
        self.record_phase(now.duration_since(self.checkpoint));
        self.timings.repository_total = now.duration_since(self.started);
        self.timings.failed_batches = usize::from(!success);
        if success && self.timings.attempted_rows > 0 {
            self.timings.committed_batches = 1;
            self.timings.committed_rows = self.timings.attempted_rows;
        }
        self.timings
    }

    /// The shared body has returned and dropped its statements, but its caller
    /// still owns the transaction. Never claim that this body committed rows.
    pub(super) fn finish_body(mut self, success: bool) -> BodyInsertionTimings {
        let now = clock();
        self.record_phase(now.duration_since(self.checkpoint));
        let completed = success && self.timings.attempted_rows > 0;
        BodyInsertionTimings {
            bodies: 1,
            attempted_rows: self.timings.attempted_rows,
            completed_bodies: usize::from(completed),
            completed_rows: if completed {
                self.timings.attempted_rows
            } else {
                0
            },
            failed_bodies: usize::from(!success),
            empty_bodies: self.timings.empty_batches,
            completed_phases: self.timings.completed_phases + usize::from(completed),
            preparation: self.timings.begin_prepare,
            event_entity_loop: self.timings.event_entity_loop,
            reconciliation: self.timings.reconciliation,
            body_total: now.duration_since(self.started),
        }
    }
}

fn clock() -> Instant {
    #[cfg(test)]
    CLOCK_READS.with(|reads| reads.set(reads.get() + 1));
    Instant::now()
}

#[cfg(test)]
thread_local! {
    static CLOCK_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "insertion_timing_capture_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "insertion_transaction_tests.rs"]
mod transaction_tests;
