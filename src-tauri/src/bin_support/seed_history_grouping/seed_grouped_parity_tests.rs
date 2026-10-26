use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{entity_repository, event_repository};

use super::seed_grouped_fixture::OwnedFixture;
use super::seed_grouped_persisted_parity::assert_complete_corpus;
use super::*;

#[path = "seed_grouped_generated_witness.rs"]
mod seed_grouped_generated_witness;
use seed_grouped_generated_witness::witness;
#[path = "seed_grouped_full_seed_fixture.rs"]
mod seed_grouped_full_seed_fixture;
use seed_grouped_full_seed_fixture::full_seed;

const END_AT: i64 = 1_800_000_000_000;

fn options(events: usize) -> Options {
    Options {
        events,
        insertion_timings: true,
        end_at: Some(END_AT),
        ..Options::default()
    }
}

fn assert_full_parity(events: usize) {
    let options = options(events);
    let baseline = OwnedFixture::new();
    let grouped = OwnedFixture::new();
    let expected = full_seed(&baseline, &options, false);
    let actual = full_seed(&grouped, &options, true);
    assert!(actual == expected, "complete event/graph ownership differs");
    baseline
        .db()
        .read(|left| {
            grouped.db().read(|right| {
                assert_complete_corpus(left, right);
                Ok(())
            })
        })
        .unwrap();
    if events == 10_000 {
        let witness = witness(&options, END_AT);
        assert_eq!((witness.edge_index, witness.subject_index), (986, 3_186));
        for fixture in [&baseline, &grouped] {
            fixture
                .db()
                .read(|conn| {
                    let target = entity_repository::by_id(conn, &witness.target)
                        .unwrap()
                        .unwrap();
                    assert_eq!(target.first_seen, witness.edge_timestamp);
                    assert!(actual.contains(&witness.target));
                    Ok(())
                })
                .unwrap();
        }
    }
}

#[test]
fn canonical_full_seed_preserves_all_persisted_rows_and_cross_boundary_endpoint() {
    assert_full_parity(10_000);
}

#[test]
fn nondivisible_tail_preserves_full_seed_and_committed_progress() {
    assert_full_parity(11_001);
}

#[test]
fn real_entity_collision_and_repeated_owned_updates_preserve_complete_parity() {
    let options = options(10_000);
    let baseline = OwnedFixture::new();
    let grouped = OwnedFixture::new();
    let selected = relationship_selection(options.events, options.seed).unwrap();
    let mut ordinals = vec![0; ENTITY_COUNT as usize];
    let batch = make_batch(0, 5_000, &options, END_AT, &selected, &[], &mut ordinals);
    let witness = witness(&options, END_AT);
    for fixture in [&baseline, &grouped] {
        let mut real = batch[witness.subject_index].get().clone();
        real.source_collector = "real-fixture".to_owned();
        real.timestamp = 1_600_000_000_000;
        real.entity_display_name = "Real immutable evidence".to_owned();
        real.related_entity_ids.clear();
        fixture
            .db()
            .insert_events(vec![Sanitizer::new().sanitize(real).0])
            .unwrap();
        let id = witness.target.clone();
        fixture
            .db()
            .write(move |conn| {
                conn.execute("UPDATE entities SET metadata_json='{}' WHERE id=?1", [&id])
                    .unwrap();
                Ok(())
            })
            .unwrap();
    }
    let before = grouped
        .db()
        .read(|conn| {
            Ok(entity_repository::by_id(conn, &witness.target)
                .unwrap()
                .unwrap())
        })
        .unwrap();
    let expected = full_seed(&baseline, &options, false);
    let actual = full_seed(&grouped, &options, true);
    assert!(
        expected == actual,
        "complete ownership differs with a real collision"
    );
    assert!(!actual.contains(&witness.target));
    // Generated history must actually repeat at least one owned subject.
    let repeated = batch
        .iter()
        .filter(|event| actual.contains(&event.get().entity_id))
        .fold(std::collections::HashMap::new(), |mut counts, event| {
            *counts
                .entry(event.get().entity_id.as_str())
                .or_insert(0usize) += 1;
            counts
        });
    assert!(repeated.values().any(|count| *count > 1));
    baseline
        .db()
        .read(|left| {
            grouped.db().read(|right| {
                assert_complete_corpus(left, right);
                let after = entity_repository::by_id(right, &witness.target)
                    .unwrap()
                    .unwrap();
                assert!(
                    serde_json::to_value(&after).unwrap() == serde_json::to_value(&before).unwrap()
                );
                for (id, count) in &repeated {
                    if *count < 2 {
                        continue;
                    }
                    assert!(
                        serde_json::to_value(entity_repository::by_id(left, id).unwrap()).unwrap()
                            == serde_json::to_value(entity_repository::by_id(right, id).unwrap())
                                .unwrap()
                    );
                }
                Ok(())
            })
        })
        .unwrap();
    // Keep event persistence API in the parity path explicit, not a raw insert.
    assert_eq!(grouped.db().read(event_repository::count).unwrap(), 10_001);
}
