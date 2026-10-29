use super::Stage;
use std::time::{Duration, Instant};

pub(super) struct Active {
    started: Instant,
    cursor: Instant,
    stage: Stage,
    durations: [Duration; 7],
    pub(super) complete: bool,
}

impl Active {
    pub(super) fn new() -> Self {
        let started = Instant::now();
        Self {
            started,
            cursor: started,
            stage: Stage::Preprocess,
            durations: [Duration::ZERO; 7],
            complete: false,
        }
    }

    pub(super) fn enter(&mut self, stage: Stage) {
        self.enter_at(stage, Instant::now());
    }

    fn enter_at(&mut self, stage: Stage, now: Instant) {
        self.durations[self.stage as usize] += now - self.cursor;
        self.cursor = now;
        self.stage = stage;
    }

    pub(super) fn record(&self) -> Record {
        self.record_at(Instant::now(), std::thread::panicking())
    }

    fn record_at(&self, now: Instant, unwinding: bool) -> Record {
        let mut durations = self.durations;
        durations[self.stage as usize] += now - self.cursor;
        Record {
            total: now - self.started,
            durations,
            complete: self.complete && !unwinding,
        }
    }
}

pub(super) struct Record {
    total: Duration,
    durations: [Duration; 7],
    complete: bool,
}

impl std::fmt::Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.complete { "complete" } else { "error" };
        write!(
            f,
            "DIAGNOSTIC analysis_stages status={status} total_inclusive_ms={:.3}",
            self.total.as_secs_f64() * 1000.0
        )?;
        for (label, duration) in [
            "preprocess",
            "structural_graph",
            "chain_index",
            "failure_index",
            "candidate_preparation",
            "candidate_scoring",
            "selection_explanations",
        ]
        .into_iter()
        .zip(self.durations)
        {
            write!(
                f,
                " {label}_exclusive_ms={:.3}",
                duration.as_secs_f64() * 1000.0
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::StageTimings;
    use super::*;

    #[test]
    fn disabled_has_no_active_record() {
        let mut timings = StageTimings::with_enabled(false);
        timings.enter(Stage::StructuralGraph);
        timings.enter(Stage::CandidateScoring);
        timings.complete();
        assert!(timings.active.is_none());
    }

    #[test]
    fn active_record_has_exact_static_fields_and_exclusive_sum() {
        let timings = StageTimings::with_enabled(true);
        assert!(timings.active.is_some());
        let mut active = Active::new();
        let started = active.started;
        for (index, stage) in [
            Stage::StructuralGraph,
            Stage::ChainIndex,
            Stage::FailureIndex,
            Stage::CandidatePreparation,
            Stage::CandidateScoring,
            Stage::SelectionExplanations,
        ]
        .into_iter()
        .enumerate()
        {
            active.enter_at(stage, started + Duration::from_millis(index as u64 + 1));
        }
        active.complete = true;
        let record = active.record_at(started + Duration::from_millis(7), false);
        assert_eq!(record.total, Duration::from_millis(7));
        assert_eq!(record.durations, [Duration::from_millis(1); 7]);
        assert_eq!(record.durations.iter().sum::<Duration>(), record.total);
        let formatted = record.to_string();
        assert!(formatted.starts_with("DIAGNOSTIC analysis_stages status=complete "));
        let fields = formatted
            .split_whitespace()
            .skip(3)
            .map(|field| {
                let (label, value) = field.split_once('=').expect("duration field");
                assert!(value.parse::<f64>().is_ok());
                label
            })
            .collect::<Vec<_>>();
        assert_eq!(
            fields,
            [
                "total_inclusive_ms",
                "preprocess_exclusive_ms",
                "structural_graph_exclusive_ms",
                "chain_index_exclusive_ms",
                "failure_index_exclusive_ms",
                "candidate_preparation_exclusive_ms",
                "candidate_scoring_exclusive_ms",
                "selection_explanations_exclusive_ms"
            ]
        );
    }

    #[test]
    fn incomplete_or_unwinding_records_are_errors() {
        let mut active = Active::new();
        let now = active.started + Duration::from_millis(1);
        assert!(!active.record_at(now, false).complete);
        active.complete = true;
        assert!(active.record_at(now, false).complete);
        let unwinding = active.record_at(now, true);
        assert!(!unwinding.complete);
        assert!(unwinding.to_string().contains("status=error"));
    }
}
