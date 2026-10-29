//! Guarded rolling counts within the caller's existing recurrence snapshot.

use std::collections::BTreeMap;

use rusqlite::{CachedStatement, Connection};

use super::{
    count_column, sorted_recurrence_range_params, PriorOccurrenceQuery, SORTED_RECURRENCE_SQL,
};
use crate::error::{AppResult, DbError};
use crate::storage::SqlContext;

#[cfg(feature = "devtools")]
use super::recurrence_plan_capture::{self, ExecutionCapture};

/// Classifies the actual bound range used by a recurrence COUNT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortedRecurrenceCountKind {
    Full,
    Delta,
}

#[cfg(feature = "devtools")]
impl SortedRecurrenceCountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Delta => "delta",
        }
    }
}

#[derive(Clone, Copy)]
enum Resolution {
    Full,
    Rolled,
    Reused,
}

#[cfg(test)]
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ProbeCounts {
    pub(super) full: usize,
    pub(super) delta: usize,
    pub(super) identical_reuses: usize,
}

pub(super) struct CountedRecurrence {
    pub(super) counts: BTreeMap<i64, u64>,
    #[cfg(test)]
    pub(super) probes: ProbeCounts,
}

struct Counter<'conn> {
    statement: CachedStatement<'conn>,
    #[cfg(feature = "devtools")]
    conn: &'conn Connection,
    #[cfg(feature = "devtools")]
    capture: Option<ExecutionCapture>,
    #[cfg(test)]
    probes: ProbeCounts,
}

impl Counter<'_> {
    fn count(
        &mut self,
        query: &PriorOccurrenceQuery,
        since: i64,
        before: i64,
        _kind: SortedRecurrenceCountKind,
    ) -> AppResult<u64> {
        #[cfg(feature = "devtools")]
        if _kind == SortedRecurrenceCountKind::Delta {
            if let Some(capture) = &mut self.capture {
                capture.explain_delta_once(self.conn, SORTED_RECURRENCE_SQL, query, since, before);
            }
        }
        let count = self
            .statement
            .query_row(
                sorted_recurrence_range_params(query, since, before),
                |row| count_column(row, 0),
            )
            .during("count sorted prior occurrences")?;
        #[cfg(feature = "devtools")]
        if let Some(capture) = &mut self.capture {
            capture.completed_sql(_kind);
        }
        #[cfg(test)]
        match _kind {
            SortedRecurrenceCountKind::Full => self.probes.full += 1,
            SortedRecurrenceCountKind::Delta => self.probes.delta += 1,
        }
        Ok(count)
    }

    fn resolved(&mut self, _resolution: Resolution) {
        #[cfg(feature = "devtools")]
        if let Some(capture) = &mut self.capture {
            match _resolution {
                Resolution::Full => capture.resolved_full(),
                Resolution::Rolled => capture.resolved_rolling(),
                Resolution::Reused => capture.resolved_reuse(),
            }
        }
        #[cfg(test)]
        if matches!(_resolution, Resolution::Reused) {
            self.probes.identical_reuses += 1;
        }
    }
}

pub(super) fn prior_occurrences(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<BTreeMap<i64, u64>> {
    counted(conn, queries).map(|result| result.counts)
}

/// The public repository and test evidence wrapper both call this exact helper
/// only for sorted-size inputs in an already open transaction.
pub(super) fn counted(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<CountedRecurrence> {
    let mut order: Vec<_> = (0..queries.len()).collect();
    order.sort_unstable_by_key(|&index| {
        let query = &queries[index];
        (
            &query.signature.entity_id,
            query.signature.category.as_str(),
            query.signature.event_type.as_str(),
            query.since,
            query.before,
        )
    });
    let statement = conn
        .prepare_cached(SORTED_RECURRENCE_SQL)
        .during("prepare sorted recurrence counts")?;
    let mut counter = Counter {
        statement,
        #[cfg(feature = "devtools")]
        conn,
        #[cfg(feature = "devtools")]
        capture: recurrence_plan_capture::begin(conn, SORTED_RECURRENCE_SQL, queries, &order),
        #[cfg(test)]
        probes: ProbeCounts::default(),
    };
    let mut by_query = vec![0_u64; queries.len()];
    let mut previous: Option<(&PriorOccurrenceQuery, u64)> = None;
    for index in order {
        let current = &queries[index];
        let (count, resolution) = match previous.filter(|(last, _)| can_roll(last, current)) {
            Some((last, prior)) if last.since == current.since && last.before == current.before => {
                (prior, Resolution::Reused)
            }
            Some((last, prior)) => {
                let removed = if last.since == current.since {
                    0
                } else {
                    counter.count(
                        current,
                        last.since,
                        current.since,
                        SortedRecurrenceCountKind::Delta,
                    )?
                };
                let added = if last.before == current.before {
                    0
                } else {
                    counter.count(
                        current,
                        last.before,
                        current.before,
                        SortedRecurrenceCountKind::Delta,
                    )?
                };
                (checked_delta(prior, removed, added)?, Resolution::Rolled)
            }
            None => (
                counter.count(
                    current,
                    current.since,
                    current.before,
                    SortedRecurrenceCountKind::Full,
                )?,
                Resolution::Full,
            ),
        };
        by_query[index] = count;
        previous = Some((current, count));
        counter.resolved(resolution);
    }
    let mut counts = BTreeMap::new();
    for (batch_index, batch) in queries.chunks(150).enumerate() {
        let mut batch_counts = BTreeMap::<i64, u64>::new();
        for (index, query) in batch.iter().enumerate() {
            let entry = batch_counts.entry(query.event_id).or_default();
            *entry = checked_sum(*entry, by_query[batch_index * 150 + index])?;
        }
        counts.extend(batch_counts);
    }
    Ok(CountedRecurrence {
        counts,
        #[cfg(test)]
        probes: counter.probes,
    })
}

pub(super) fn can_roll(last: &PriorOccurrenceQuery, current: &PriorOccurrenceQuery) -> bool {
    last.signature == current.signature
        && last.since < last.before
        && current.since < current.before
        && current.since >= last.since
        && current.before >= last.before
        && last.since.max(current.since) < last.before.min(current.before)
        && {
            // Time spans are a probe-cost heuristic, not a row-density or
            // no-regression guarantee. Widen every endpoint before subtracting.
            let removed_span = i128::from(current.since) - i128::from(last.since);
            let added_span = i128::from(current.before) - i128::from(last.before);
            let full_span = i128::from(current.before) - i128::from(current.since);
            removed_span + added_span < full_span
        }
}

pub(super) fn checked_delta(prior: u64, removed: u64, added: u64) -> AppResult<u64> {
    let retained = prior
        .checked_sub(removed)
        .ok_or_else(|| DbError::Decoding {
            reason: "recurrence removal exceeds prior count".to_owned(),
        })?;
    checked_sum(retained, added)
}

pub(super) fn checked_sum(left: u64, right: u64) -> AppResult<u64> {
    left.checked_add(right).ok_or_else(|| {
        DbError::Decoding {
            reason: "recurrence count exceeds u64".to_owned(),
        }
        .into()
    })
}
