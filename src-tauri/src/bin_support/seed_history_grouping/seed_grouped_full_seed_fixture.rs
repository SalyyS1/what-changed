//! Both event routes finish through the identical inventory/artifact recipe.

use std::collections::HashSet;
use what_changed_lib::storage::maintenance;

use super::super::seed_grouped_fixture::OwnedFixture;
use super::super::seed_grouped_persisted_parity::count;
use super::super::{seed_grouped_events, UNITS_PER_GROUP};
use super::END_AT;
use crate::{
    finish_seed, seed_events, Options, SeedBatchSize, SeedTimings, BATCH_SIZE, ENTITY_COUNT,
};

pub(super) fn full_seed(
    fixture: &OwnedFixture,
    options: &Options,
    grouped: bool,
) -> HashSet<String> {
    let marker = maintenance::register_synthetic_run(fixture.db()).unwrap();
    let mut coarse = SeedTimings::default();
    let (inserted, owned) = if grouped {
        let run = seed_grouped_events(fixture.db(), options, END_AT, &mut coarse);
        let expected_units = options.events.div_ceil(BATCH_SIZE);
        let expected_groups = expected_units.div_ceil(UNITS_PER_GROUP);
        assert_eq!(run.attribution.attempted_groups, expected_groups);
        assert_eq!(run.attribution.committed_groups, expected_groups);
        assert_eq!(run.attribution.failed_groups, 0);
        assert_eq!(run.attribution.attempted_input_rows, options.events);
        assert_eq!(run.attribution.completed_body_rows, options.events);
        assert_eq!(run.attribution.committed_input_rows, options.events);
        assert_eq!(run.attribution.started_units, expected_units);
        assert_eq!(run.attribution.completed_units, expected_units);
        assert_eq!(run.attribution.body.bodies, expected_units);
        assert_eq!(run.attribution.body.completed_bodies, expected_units);
        assert_eq!(run.attribution.body.failed_bodies, 0);
        assert_eq!(run.attribution.body.completed_phases, 3 * expected_units);
        assert_eq!(run.attribution.body.completed_rows, options.events);
        assert_eq!(run.progress.rows, options.events);
        assert_eq!(run.progress.groups, expected_groups);
        assert_eq!(
            run.progress.notifications,
            if options.events == 10_000 { 1 } else { 2 }
        );
        assert!(
            run.attribution.group_total
                >= run.attribution.begin
                    + run.attribution.body.body_total
                    + run.attribution.graph
                    + run.attribution.commit
        );
        // No legacy insertion record is silently redefined by grouping.
        assert_eq!(coarse.insertion.batches, 0);
        let state = run.value.unwrap();
        assert_eq!(
            state.relation_ordinals.iter().sum::<u64>() as usize,
            options.events * 15 / 100
        );
        (state.inserted, state.owned_entities)
    } else {
        let value = seed_events(
            fixture.db(),
            options,
            END_AT,
            &mut coarse,
            SeedBatchSize::Standard,
        )
        .unwrap();
        assert_eq!(
            coarse.insertion.committed_batches,
            options.events.div_ceil(BATCH_SIZE)
        );
        assert_eq!(coarse.insertion.committed_rows, options.events);
        value
    };
    let ownership = owned.clone();
    let summary = finish_seed(
        fixture.db(),
        options,
        &mut coarse,
        END_AT,
        &marker,
        inserted,
        owned,
    )
    .unwrap();
    assert_eq!(summary.events, options.events);
    assert_eq!(summary.entities, ENTITY_COUNT as usize);
    assert_eq!(summary.checkpoints, crate::CHECKPOINT_COUNT);
    assert_eq!(summary.investigations, crate::INVESTIGATION_COUNT);
    fixture
        .db()
        .read(|conn| {
            assert_eq!(count(conn, "checkpoints"), 40);
            assert_eq!(count(conn, "snapshots"), 9_800);
            assert_eq!(count(conn, "investigations"), 20);
            assert_eq!(count(conn, "analysis_sessions"), 20);
            let marked: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM checkpoints WHERE reason=?1",
                    [&marker],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(marked, 40);
            let marked: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM investigations WHERE problem=?1",
                    [&marker],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(marked, 20);
            Ok(())
        })
        .unwrap();
    ownership
}
