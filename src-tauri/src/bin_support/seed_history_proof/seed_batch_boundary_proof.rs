//! Rejection proof only: no checkpoint/investigation or performance harness.

use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::types::Value;
use rusqlite::Connection;

use super::{make_batch, relationship_selection, seed_events, Options, SeedBatchSize, SeedTimings};
use what_changed_lib::storage::entity_repository;

#[path = "seed_batch_boundary_fixture.rs"]
mod seed_batch_boundary_fixture;
use seed_batch_boundary_fixture::OwnedFixture;

const END_AT: i64 = 1_800_000_000_000;

struct Witness {
    target: String,
    edge_index: usize,
    edge_timestamp: i64,
    first_subject_index: usize,
    first_subject_timestamp: i64,
}

/// Derive an actual canonical edge whose missing target is first mentioned by
/// a subject event after the standard boundary but inside the candidate batch.
/// No constructed event replaces generator output or changes graph selection.
fn witness(options: &Options) -> Witness {
    let selected = relationship_selection(options.events, options.seed).unwrap();
    let mut ordinals = vec![0; super::ENTITY_COUNT as usize];
    let batch = make_batch(
        0,
        SeedBatchSize::Candidate.rows(),
        options,
        END_AT,
        &selected,
        &[],
        &mut ordinals,
    );
    let mut first_subject = HashMap::new();
    for (index, event) in batch.iter().enumerate() {
        first_subject
            .entry(event.get().entity_id.as_str())
            .or_insert((index, event.get().timestamp));
    }
    // All standard event subjects exist before its first graph transaction.
    let mut present = batch[..SeedBatchSize::Standard.rows()]
        .iter()
        .map(|event| event.get().entity_id.as_str())
        .collect::<HashSet<_>>();
    for (edge_index, event) in batch[..SeedBatchSize::Standard.rows()].iter().enumerate() {
        for target in &event.get().related_entity_ids {
            if !present.contains(target.as_str()) {
                if let Some(&(first_subject_index, first_subject_timestamp)) =
                    first_subject.get(target.as_str())
                {
                    if first_subject_index >= SeedBatchSize::Standard.rows()
                        && event.get().timestamp < first_subject_timestamp
                    {
                        return Witness {
                            target: target.clone(),
                            edge_index,
                            edge_timestamp: event.get().timestamp,
                            first_subject_index,
                            first_subject_timestamp,
                        };
                    }
                }
            }
            present.insert(target.as_str());
        }
    }
    panic!("canonical generator did not exercise the suspected cross-boundary endpoint branch");
}

#[derive(Default)]
struct Difference {
    rows: usize,
    mismatched_rows: usize,
    fields: BTreeMap<String, usize>,
}

/// Compare every persisted column and row, including physical IDs and order.
/// One row per connection is buffered; no history-proportional event vector.
fn compare_table(left: &Connection, right: &Connection, table: &str) -> Difference {
    let sql = format!("SELECT * FROM {table} ORDER BY id");
    let mut left_statement = left.prepare(&sql).unwrap();
    let mut right_statement = right.prepare(&sql).unwrap();
    let columns = left_statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(columns
        .iter()
        .map(String::as_str)
        .eq(right_statement.column_names()));
    let mut left_rows = left_statement.query([]).unwrap();
    let mut right_rows = right_statement.query([]).unwrap();
    let mut difference = Difference::default();
    loop {
        match (left_rows.next().unwrap(), right_rows.next().unwrap()) {
            (None, None) => break,
            (Some(left), Some(right)) => {
                difference.rows += 1;
                let mut mismatched = false;
                for (index, name) in columns.iter().enumerate() {
                    if left.get::<_, Value>(index).unwrap() != right.get::<_, Value>(index).unwrap()
                    {
                        *difference.fields.entry(name.clone()).or_default() += 1;
                        mismatched = true;
                    }
                }
                difference.mismatched_rows += usize::from(mismatched);
            }
            _ => panic!("batch proof persisted row counts differ"),
        }
    }
    difference
}

#[test]
#[ignore = "Explicit proof-only rejection: canonical entity times must not be normalized or promoted"]
fn canonical_cross_boundary_endpoint_rejects_larger_batch_parity() {
    let options = Options {
        insertion_timings: true,
        end_at: Some(END_AT),
        ..Options::default()
    };
    assert_eq!(options.events, 10_000);
    assert_eq!(options.days, 90);
    assert!(options.relations && !options.collector_payload_sensitivity);
    let witness = witness(&options);
    assert!(witness.edge_index < 1_000);
    assert!((1_000..5_000).contains(&witness.first_subject_index));
    let baseline = OwnedFixture::new();
    let candidate = OwnedFixture::new();
    let mut baseline_timings = SeedTimings::default();
    let mut candidate_timings = SeedTimings::default();
    let (baseline_rows, baseline_owned) = seed_events(
        baseline.db(),
        &options,
        END_AT,
        &mut baseline_timings,
        SeedBatchSize::Standard,
    )
    .unwrap();
    let (candidate_rows, candidate_owned) = seed_events(
        candidate.db(),
        &options,
        END_AT,
        &mut candidate_timings,
        SeedBatchSize::Candidate,
    )
    .unwrap();
    assert_eq!((baseline_rows, candidate_rows), (10_000, 10_000));
    assert!(
        baseline_owned == candidate_owned,
        "complete graph/event ownership sets differ"
    );
    assert!(baseline_owned.contains(&witness.target));
    baseline.db().read(|left| candidate.db().read(|right| {
        let events = compare_table(left, right, "events");
        let entities = compare_table(left, right, "entities");
        let relationships = compare_table(left, right, "entity_relationships");
        assert_eq!(events.rows, 10_000);
        assert_eq!(relationships.rows, 1_500);
        assert_eq!(entities.rows, baseline_owned.len());
        let old_target = entity_repository::by_id(left, &witness.target).unwrap().unwrap();
        let new_target = entity_repository::by_id(right, &witness.target).unwrap().unwrap();
        assert_eq!(old_target.first_seen, witness.edge_timestamp);
        assert_eq!(new_target.first_seen, witness.first_subject_timestamp);
        assert!(old_target.first_seen < new_target.first_seen);
        assert_eq!(old_target.last_seen, new_target.last_seen);
        assert!(old_target.display_name == new_target.display_name, "witness display names differ");
        assert!(old_target.metadata == new_target.metadata, "witness metadata differs");
        println!("DIAGNOSTIC seed_batch_boundary_proof rows={} events_mismatched={} entity_rows={} entities_mismatched={} relationship_rows={} relationships_mismatched={} ownership_equal=1 witness_edge_index={} witness_first_subject_index={} witness_first_seen_delta_ms={} status=rejected_semantic_parity",
            events.rows, events.mismatched_rows, entities.rows, entities.mismatched_rows,
            relationships.rows, relationships.mismatched_rows, witness.edge_index,
            witness.first_subject_index, new_target.first_seen - old_target.first_seen);
        for (field, count) in &entities.fields {
            println!("DIAGNOSTIC seed_batch_boundary_entity_field field={field} mismatches={count}");
        }
        assert_eq!(events.mismatched_rows, 0, "complete persisted event content/order differs");
        assert_eq!(relationships.mismatched_rows, 0, "complete relationship evidence differs");
        assert!(entities.mismatched_rows > 0, "suspected semantic mismatch was not exercised");
        assert!(entities.fields.get("first_seen").copied().unwrap_or(0) > 0);
        Ok(())
    })).unwrap();
    assert_eq!(baseline_timings.insertion.committed_batches, 10);
    assert_eq!(candidate_timings.insertion.committed_batches, 2);
    assert_eq!(baseline_timings.insertion.committed_rows, 10_000);
    assert_eq!(candidate_timings.insertion.committed_rows, 10_000);
    // Expected rejection proves the original experiment cannot pass semantic
    // parity. No normalization, repair, checkpoint/performance run or promotion.
}
