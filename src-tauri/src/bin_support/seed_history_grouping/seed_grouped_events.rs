//! Private fixed-five grouping; CLI execution still uses independent batches.

use std::collections::HashSet;
use std::time::Instant;

use what_changed_lib::domain::NormalizedEvent;
use what_changed_lib::privacy::Sanitized;
use what_changed_lib::storage::Db;

use super::{
    make_batch, relationship_selection, selected_relation_inputs, system_payload_selection,
    Options, RelationSeedInput, SeedTimings, BATCH_SIZE, ENTITY_COUNT,
};

#[path = "seed_grouped_failure.rs"]
mod seed_grouped_failure;
#[path = "seed_grouped_fixture.rs"]
mod seed_grouped_fixture;
#[path = "seed_grouped_persisted_parity.rs"]
mod seed_grouped_persisted_parity;
#[path = "seed_grouped_timings.rs"]
mod seed_grouped_timings;
#[path = "seed_grouped_transaction.rs"]
mod seed_grouped_transaction;

use seed_grouped_failure::GroupFailure;
use seed_grouped_timings::GroupTimings;
use seed_grouped_transaction::execute_group;

const UNITS_PER_GROUP: usize = 5;

pub(super) struct GroupState {
    pub inserted: usize,
    pub owned_entities: HashSet<String>,
    known_entities: HashSet<String>,
    relation_ordinals: Vec<u64>,
}

impl Default for GroupState {
    fn default() -> Self {
        Self {
            inserted: 0,
            owned_entities: HashSet::new(),
            known_entities: HashSet::new(),
            relation_ordinals: vec![0; ENTITY_COUNT as usize],
        }
    }
}

struct PreparedUnit {
    events: Vec<Sanitized<NormalizedEvent>>,
    relations: Option<Vec<RelationSeedInput>>,
}

struct PreparedGroup {
    units: Vec<PreparedUnit>,
    next_ordinals: Vec<u64>,
    input_rows: usize,
}

/// Only scalar committed progress survives an error; reusable owned state does not.
#[derive(Clone, Copy, Default)]
pub(super) struct PublishedProgress {
    pub rows: usize,
    pub groups: usize,
    pub notifications: usize,
}

pub(super) struct GroupedRun {
    pub value: Result<GroupState, GroupFailure>,
    pub attribution: GroupTimings,
    pub progress: PublishedProgress,
}

fn prepare_group(
    state: &GroupState,
    options: &Options,
    end_at: i64,
    selected: &[bool],
    system_payloads: &[bool],
) -> PreparedGroup {
    let mut next_ordinals = state.relation_ordinals.clone();
    let mut units = Vec::with_capacity(UNITS_PER_GROUP);
    let mut first = state.inserted;
    for _ in 0..UNITS_PER_GROUP {
        if first == options.events {
            break;
        }
        let count = BATCH_SIZE.min(options.events - first);
        let events = make_batch(
            first,
            count,
            options,
            end_at,
            selected,
            system_payloads,
            &mut next_ordinals,
        );
        let relations = options.relations.then(|| selected_relation_inputs(&events));
        units.push(PreparedUnit { events, relations });
        first += count;
    }
    PreparedGroup {
        units,
        next_ordinals,
        input_rows: first - state.inserted,
    }
}

pub(super) fn seed_grouped_events(
    db: &Db,
    options: &Options,
    end_at: i64,
    coarse: &mut SeedTimings,
) -> GroupedRun {
    let mut attribution = GroupTimings::default();
    let mut progress = PublishedProgress::default();
    let value = (|| -> Result<GroupState, GroupFailure> {
        let selected = if options.relations {
            relationship_selection(options.events, options.seed)?
        } else {
            vec![false; options.events]
        };
        let system_payloads = if options.collector_payload_sensitivity {
            system_payload_selection(options.events, options.seed)
        } else {
            Vec::new()
        };
        let mut state = GroupState::default();
        while state.inserted < options.events {
            let generated = Instant::now();
            let prepared = prepare_group(&state, options, end_at, &selected, &system_payloads);
            coarse.make_batch += generated.elapsed();
            let capture = options.insertion_timings;
            // Infrastructure/admission failure is outer: no attempted group was
            // recorded unless the actual writer-local transaction job ran.
            let outcome =
                db.write(move |conn| Ok(execute_group(conn, prepared, state, capture)))?;
            attribution.accumulate(outcome.timings);
            state = outcome.value?;
            progress.rows = state.inserted;
            progress.groups += 1;
            if state.inserted % 10_000 == 0 || state.inserted == options.events {
                progress.notifications += 1;
                println!(
                    "seeded {}/{} synthetic events",
                    state.inserted, options.events
                );
            }
        }
        Ok(state)
    })();
    GroupedRun {
        value,
        attribution,
        progress,
    }
}

#[path = "seed_grouped_cleanup_tests.rs"]
mod cleanup_tests;
#[path = "seed_grouped_failure_tests.rs"]
mod failure_tests;
#[path = "seed_grouped_parity_tests.rs"]
mod parity_tests;
#[path = "seed_grouped_purge_tests.rs"]
mod purge_tests;

#[path = "seed_grouped_measurement.rs"]
mod measurement;
