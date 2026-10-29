use super::Stage;
use std::time::{Duration, Instant};

pub(super) struct Active {
    started: Instant,
    pub(super) cursor: Instant,
    stage: Stage,
    durations: [Duration; 10],
    pub(super) input_started: Option<Instant>,
    pub(super) input_total: Option<Duration>,
    pub(super) complete: bool,
}

impl Active {
    pub(super) fn new() -> Self {
        let started = Instant::now();
        Self {
            started,
            cursor: started,
            stage: Stage::RequestSetup,
            durations: [Duration::ZERO; 10],
            input_started: None,
            input_total: None,
            complete: false,
        }
    }

    pub(super) fn enter(&mut self, stage: Stage) {
        let now = Instant::now();
        self.durations[self.stage as usize] += now - self.cursor;
        self.cursor = now;
        self.stage = stage;
    }

    pub(super) fn record(&self) -> Record {
        let now = Instant::now();
        let mut durations = self.durations;
        durations[self.stage as usize] += now - self.cursor;
        Record {
            total: now - self.started,
            durations,
            complete: self.complete,
            input: self
                .input_total
                .or_else(|| self.input_started.map(|start| now - start))
                .unwrap_or_default(),
        }
    }
}

pub(super) struct Record {
    total: Duration,
    input: Duration,
    durations: [Duration; 10],
    complete: bool,
}

impl std::fmt::Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.complete { "complete" } else { "error" };
        write!(f, "DIAGNOSTIC investigation_stages status={status} total_inclusive_ms={:.3} input_inclusive_ms={:.3}",
            self.total.as_secs_f64() * 1000.0, self.input.as_secs_f64() * 1000.0)?;
        for (label, duration) in [
            "request_setup",
            "window_hydration",
            "boundary_queries",
            "entity_load",
            "relationships",
            "recurrence",
            "input_finalize",
            "analysis_scoring",
            "result_sanitization",
            "persistence",
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
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::services::investigation_service::stage_timings::StageTimings;

    #[test]
    fn disabled_has_no_active_record() {
        let mut timings = StageTimings::with_enabled(false);
        timings.begin_input();
        timings.enter(Stage::BoundaryQueries);
        timings.finish_input(true);
        timings.complete(true);
        assert!(timings.active.is_none());
    }

    #[test]
    fn record_has_positive_aggregate_static_fields_and_error_until_complete() {
        let mut timings = StageTimings::with_enabled(true);
        if let Some(active) = &mut timings.active {
            active.started -= Duration::from_millis(2);
            active.cursor = active.started;
            active.enter(Stage::WindowHydration);
            active.input_started = Some(active.cursor - Duration::from_millis(1));
            let record = active.record();
            assert!(record.total > Duration::ZERO);
            assert!(record.input > Duration::ZERO);
            assert_eq!(record.durations.iter().sum::<Duration>(), record.total);
            let formatted = record.to_string();
            assert!(formatted.contains("status=error"));
            assert_eq!(formatted.split_whitespace().count(), 15);
            for field in formatted.split_whitespace().skip(3) {
                assert!(field
                    .split('=')
                    .next()
                    .is_some_and(|label| label.ends_with("_ms")));
            }
            active.complete = true;
            assert!(active.record().to_string().contains("status=complete"));
        } else {
            panic!("explicit opt-in must enable diagnostics");
        }
    }
}
