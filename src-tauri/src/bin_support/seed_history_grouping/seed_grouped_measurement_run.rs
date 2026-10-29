//! Identical full-seed clock boundaries and distinct truthful attribution.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use what_changed_lib::storage::maintenance;

use super::super::seed_grouped_fixture::OwnedFixture;
use super::super::seed_grouped_timings::GroupTimings;
use super::super::{seed_grouped_events, PublishedProgress};
use super::END_AT;
use crate::{
    finish_seed, seed_events, Options, SeedBatchSize, SeedSummary, SeedTimings, BATCH_SIZE,
};

#[derive(Clone, Copy)]
pub(super) enum Route {
    Baseline,
    FixedFive,
}

impl Route {
    pub fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::FixedFive => "fixed_five",
        }
    }
}

pub(super) struct MeasuredSeed {
    pub elapsed: Duration,
    pub owned: HashSet<String>,
    summary: SeedSummary,
    coarse: SeedTimings,
    group: Option<(GroupTimings, PublishedProgress, Vec<u64>)>,
}

pub(super) fn measure(fixture: &OwnedFixture, options: &Options, route: Route) -> MeasuredSeed {
    let mut coarse = SeedTimings::default();
    // Both clocks start immediately before the SAME real marker registration
    // and finish after the SAME inventory/checkpoint/investigation tail and
    // actual production flush barrier. Fixture opening/closing is excluded.
    // The same bounded ownership snapshot is included in both clocks.
    let started = Instant::now();
    let marker = maintenance::register_synthetic_run(fixture.db()).unwrap();
    let (inserted, owned, group) = match route {
        Route::Baseline => {
            let (rows, owned) = seed_events(
                fixture.db(),
                options,
                END_AT,
                &mut coarse,
                SeedBatchSize::Standard,
            )
            .unwrap();
            (rows, owned, None)
        }
        Route::FixedFive => {
            let run = seed_grouped_events(fixture.db(), options, END_AT, &mut coarse);
            let state = run.value.unwrap();
            (
                state.inserted,
                state.owned_entities,
                Some((run.attribution, run.progress, state.relation_ordinals)),
            )
        }
    };
    let summary = finish_seed(
        fixture.db(),
        options,
        &mut coarse,
        END_AT,
        &marker,
        inserted,
        owned.clone(),
    )
    .unwrap();
    fixture.db().flush().unwrap();
    let elapsed = started.elapsed();
    MeasuredSeed {
        elapsed,
        owned,
        summary,
        coarse,
        group,
    }
}

impl MeasuredSeed {
    pub fn validate(&self, rows: usize) {
        assert_eq!(
            (
                self.summary.events,
                self.summary.entities,
                self.summary.checkpoints,
                self.summary.investigations,
                self.summary.end_at
            ),
            (rows, 3_000, 40, 20, END_AT)
        );
        assert!(self.owned.len() <= 3_000);
        let units = rows.div_ceil(BATCH_SIZE);
        if let Some((timings, progress, ordinals)) = &self.group {
            let groups = units.div_ceil(5);
            assert_eq!(
                (
                    timings.attempted_groups,
                    timings.committed_groups,
                    timings.failed_groups
                ),
                (groups, groups, 0)
            );
            assert_eq!(
                (
                    timings.attempted_input_rows,
                    timings.completed_body_rows,
                    timings.committed_input_rows
                ),
                (rows, rows, rows)
            );
            assert_eq!(
                (timings.started_units, timings.completed_units),
                (units, units)
            );
            assert_eq!(
                (
                    timings.body.bodies,
                    timings.body.attempted_rows,
                    timings.body.completed_bodies,
                    timings.body.completed_rows
                ),
                (units, rows, units, rows)
            );
            assert_eq!(
                (
                    timings.body.failed_bodies,
                    timings.body.empty_bodies,
                    timings.body.completed_phases
                ),
                (0, 0, 3 * units)
            );
            assert_eq!(
                (progress.rows, progress.groups, progress.notifications),
                (rows, groups, rows / 10_000)
            );
            assert_eq!(ordinals.len(), 3_000);
            assert_eq!(
                ordinals.iter().sum::<u64>(),
                u64::try_from(rows * 15 / 100).unwrap()
            );
            assert!(
                timings.group_total
                    >= timings.begin + timings.body.body_total + timings.graph + timings.commit
            );
            assert_eq!(
                self.coarse.insertion.batches, 0,
                "group records must not masquerade as repository commits"
            );
        } else {
            let timings = self.coarse.insertion;
            assert_eq!(
                (
                    timings.batches,
                    timings.committed_batches,
                    timings.failed_batches
                ),
                (units, units, 0)
            );
            assert_eq!(
                (timings.attempted_rows, timings.committed_rows),
                (rows, rows)
            );
            assert_eq!(
                (timings.empty_batches, timings.completed_phases),
                (0, 4 * units)
            );
        }
    }

    pub fn print(&self, rows: usize, route: Route) {
        let route = route.label();
        let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
        println!("seed grouping full seed: events={rows} route={route} elapsed_ms={:.3} clock=before_marker_registration_through_finish_seed_and_flush ownership_snapshot_included=1 fixture_open_settings_close_validation_digest_report_cleanup_excluded=1", ms(self.elapsed));
        if let Some((timings, progress, _)) = &self.group {
            println!("seed grouping actual groups: events={rows} route={route} attempted_groups={} committed_groups={} failed_groups={} attempted_input_rows={} started_units={} completed_body_rows={} completed_units={} committed_input_rows={} progress_rows={} progress_groups={} progress_notifications={} begin_ms={:.3} graph_ms={:.3} commit_ms={:.3} group_total_ms={:.3}",
                timings.attempted_groups, timings.committed_groups, timings.failed_groups,
                timings.attempted_input_rows, timings.started_units, timings.completed_body_rows,
                timings.completed_units, timings.committed_input_rows,
                progress.rows, progress.groups, progress.notifications,
                ms(timings.begin), ms(timings.graph), ms(timings.commit), ms(timings.group_total));
            let body = timings.body;
            println!("seed grouping actual bodies: events={rows} route={route} bodies={} attempted_rows={} completed_bodies={} completed_rows={} failed_bodies={} empty_bodies={} completed_phases={} preparation_ms={:.3} event_entity_loop_ms={:.3} reconciliation_ms={:.3} body_total_ms={:.3} body_success_is_not_commit=1",
                body.bodies, body.attempted_rows, body.completed_bodies, body.completed_rows,
                body.failed_bodies, body.empty_bodies, body.completed_phases,
                ms(body.preparation), ms(body.event_entity_loop), ms(body.reconciliation), ms(body.body_total));
            println!("seed grouping coarse: events={rows} route={route} make_batch_ms={:.3} inventory_ms={:.3} checkpoints_ms={:.3} investigations_ms={:.3} legacy_insert_graph_fields_not_reinterpreted=1", ms(self.coarse.make_batch), ms(self.coarse.inventory), ms(self.coarse.checkpoints), ms(self.coarse.investigations));
        } else {
            self.coarse.print_insertion();
            self.coarse.print(self.elapsed);
        }
    }
}
