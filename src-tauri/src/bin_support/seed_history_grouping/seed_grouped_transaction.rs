//! One writer job preserves each logical event-to-graph sequence.

use rusqlite::{Connection, Transaction};
use what_changed_lib::error::{AppError, AppResult, DbError};
use what_changed_lib::storage::event_repository::insert_batch_preserving_entities_in_transaction;

use super::seed_grouped_failure::{abandon, GroupFailure};
use super::seed_grouped_timings::{elapsed, start, GroupTimings};
use super::{GroupState, PreparedGroup, PreparedUnit};
use crate::seed_graph_body::insert_graph_body;

pub(super) struct GroupOutcome {
    pub value: Result<GroupState, GroupFailure>,
    pub timings: GroupTimings,
}

pub(super) fn execute_group(
    conn: &Connection,
    prepared: PreparedGroup,
    state: GroupState,
    capture: bool,
) -> GroupOutcome {
    let started = start(capture);
    let PreparedGroup {
        units,
        next_ordinals,
        input_rows,
    } = prepared;
    let mut timings = GroupTimings {
        attempted_groups: 1,
        attempted_input_rows: input_rows,
        ..GroupTimings::default()
    };
    let begin = start(capture);
    let transaction = conn
        .unchecked_transaction()
        .map_err(|source| DbError::Sqlite {
            operation: "start a grouped synthetic seed transaction",
            source,
        });
    timings.begin = elapsed(begin);
    let value = match transaction {
        Err(error) => {
            drop(units);
            drop(next_ordinals);
            drop(state);
            Err(GroupFailure::from(AppError::from(error)))
        }
        Ok(tx) => match insert_units(&tx, units, state, capture, &mut timings) {
            Err(primary) => {
                drop(next_ordinals);
                Err(abandon(tx, primary))
            }
            Ok(mut state) => {
                let commit = start(capture);
                // Keep the Transaction alive across COMMIT failure so explicit
                // rollback can report its own error. Default Drop remains the
                // unwind/retry fallback; no DropBehavior::Ignore is installed.
                let committed = tx
                    .execute_batch("COMMIT")
                    .map_err(|source| DbError::Sqlite {
                        operation: "commit a grouped synthetic seed transaction",
                        source,
                    });
                timings.commit = elapsed(commit);
                match committed {
                    Err(error) => {
                        drop(next_ordinals);
                        Err(abandon(tx, error.into()))
                    }
                    Ok(()) => {
                        drop(tx);
                        state.relation_ordinals = next_ordinals;
                        timings.committed_groups = 1;
                        timings.committed_input_rows = input_rows;
                        Ok(state)
                    }
                }
            }
        },
    };
    timings.failed_groups = usize::from(value.is_err());
    // All explicit cleanup/transaction disposal precedes publication/timing.
    timings.group_total = elapsed(started);
    GroupOutcome { value, timings }
}

fn insert_units(
    tx: &Transaction<'_>,
    units: Vec<PreparedUnit>,
    mut state: GroupState,
    capture: bool,
    timings: &mut GroupTimings,
) -> AppResult<GroupState> {
    for unit in units {
        timings.started_units += 1;
        let captured = insert_batch_preserving_entities_in_transaction(
            tx,
            &unit.events,
            std::mem::take(&mut state.owned_entities),
            capture,
        );
        if let Some(body) = captured.timings {
            timings.add_body(body);
        }
        let (ids, owned) = captured.value?;
        state.owned_entities = owned;
        timings.completed_body_rows += unit.events.len();
        if let Some(relations) = unit.relations {
            let graph = start(capture);
            let inserted = insert_graph_body(
                tx,
                &relations,
                &ids,
                std::mem::take(&mut state.known_entities),
                std::mem::take(&mut state.owned_entities),
            );
            timings.graph += elapsed(graph);
            let (known, owned) = inserted?;
            state.known_entities = known;
            state.owned_entities = owned;
        }
        state.inserted += ids.len();
        timings.completed_units += 1;
    }
    Ok(state)
}
