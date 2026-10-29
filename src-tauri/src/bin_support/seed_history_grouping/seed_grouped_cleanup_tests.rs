use super::failure_tests::seed_grouped_failure_fixtures::{install, options, FailureKind, END_AT};
use super::seed_grouped_fixture::OwnedFixture;
use super::seed_grouped_persisted_parity::{assert_exact_event_graph, count};
use super::*;

#[test]
fn disabled_group_attribution_reads_no_clocks_on_success_or_error() {
    for fail in [false, true] {
        let fixture = OwnedFixture::new();
        if fail {
            install(&fixture, FailureKind::Event);
        }
        fixture
            .db()
            .write(|_| {
                seed_grouped_timings::CLOCK_READS.with(|reads| reads.set(0));
                Ok(())
            })
            .unwrap();
        let mut options = options();
        options.insertion_timings = false;
        let run = seed_grouped_events(fixture.db(), &options, END_AT, &mut SeedTimings::default());
        assert_eq!(run.value.is_err(), fail);
        assert_eq!(run.attribution.attempted_groups, 2);
        assert_eq!(
            run.attribution.committed_input_rows,
            if fail { 5_000 } else { 10_000 }
        );
        assert_eq!(run.attribution.group_total, std::time::Duration::ZERO);
        assert_eq!(
            run.attribution.begin
                + run.attribution.graph
                + run.attribution.commit
                + run.attribution.body.body_total,
            std::time::Duration::ZERO
        );
        fixture
            .db()
            .write(|_| {
                assert_eq!(
                    seed_grouped_timings::CLOCK_READS.with(|reads| reads.get()),
                    0
                );
                Ok(())
            })
            .unwrap();
        if !fail {
            let captured = OwnedFixture::new();
            let active = seed_grouped_events(
                captured.db(),
                &self::options(),
                END_AT,
                &mut SeedTimings::default(),
            );
            let expected = active.value.unwrap();
            let actual = run.value.unwrap();
            assert!(expected.owned_entities == actual.owned_entities);
            captured
                .db()
                .read(|left| {
                    fixture.db().read(|right| {
                        assert_exact_event_graph(left, right);
                        Ok(())
                    })
                })
                .unwrap();
        }
    }
}

#[test]
fn grouped_owner_unwind_rolls_back_and_next_job_has_fresh_state() {
    let fixture = OwnedFixture::new();
    let options = options();
    let selected = relationship_selection(options.events, options.seed).unwrap();
    let state = GroupState::default();
    let mut prepared = prepare_group(&state, &options, END_AT, &selected, &[]);
    // A graph-input invariant panic occurs after real generated event/entity
    // work. This is a narrow unwind fixture, not a general injection callback.
    let first = &mut prepared.units[0];
    let invalid_index = first.events.len();
    first.relations.as_mut().unwrap()[0].batch_index = invalid_index;
    fixture
        .db()
        .write(move |conn| {
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execute_group(conn, prepared, state, true)
            }))
            .is_err());
            assert!(conn.is_autocommit());
            for table in ["events", "entities", "entity_relationships"] {
                assert_eq!(count(conn, table), 0);
            }
            Ok(())
        })
        .unwrap();
    let state = GroupState::default();
    let prepared = prepare_group(&state, &options, END_AT, &selected, &[]);
    let outcome = fixture
        .db()
        .write(move |conn| Ok(execute_group(conn, prepared, state, true)))
        .unwrap();
    assert_eq!(outcome.timings.attempted_groups, 1);
    assert_eq!(outcome.timings.failed_groups, 0);
    assert_eq!(outcome.timings.committed_input_rows, 5_000);
    assert_eq!(outcome.value.unwrap().inserted, 5_000);
}
