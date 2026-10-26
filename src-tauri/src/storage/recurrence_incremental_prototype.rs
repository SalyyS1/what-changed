//! Independent old-path baseline and evidence wrapper for production rolling counts.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeMap;

use super::recurrence_rolling_counts::{self, can_roll, checked_delta, checked_sum, ProbeCounts};
use super::{prior_occurrences_batch, PriorOccurrenceQuery};
use crate::error::{AppResult, DbError};
use rusqlite::Connection;

struct CandidateRun {
    counts: BTreeMap<i64, u64>,
    // None means the unchanged repository owns SQL execution through fallback.
    probes: Option<ProbeCounts>,
}

/// Force the original execution and its independent aggregation even in a
/// transaction. Do not route the baseline through the new public dispatcher.
fn original_batch(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<BTreeMap<i64, u64>> {
    if queries.len() < 2_000 {
        super::prior_occurrences_indexed_batches(conn, queries)
    } else {
        super::prior_occurrences_sorted_full_counts(conn, queries)
    }
}

fn incremental_batch(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<CandidateRun> {
    if queries.len() < 2_000 || conn.is_autocommit() {
        return Ok(CandidateRun {
            counts: prior_occurrences_batch(conn, queries)?,
            probes: None,
        });
    }
    let result = recurrence_rolling_counts::counted(conn, queries)?;
    Ok(CandidateRun {
        counts: result.counts,
        probes: Some(result.probes),
    })
}

#[path = "recurrence_incremental_prototype_benchmark.rs"]
mod benchmark;
#[path = "recurrence_incremental_prototype_fixture.rs"]
mod fixture;
#[path = "recurrence_incremental_prototype_parity.rs"]
mod parity;
