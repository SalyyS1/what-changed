//! Integer-only evidence from the already sorted, borrowed recurrence inputs.

use super::PriorOccurrenceQuery;

/// Exact signatures include entity, category and event type. Distinct windows
/// also include both bounds, even when those bounds describe an empty interval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SortedRecurrenceWindowStatistics {
    pub input_queries: usize,
    pub distinct_signatures: usize,
    pub distinct_windows: usize,
    pub identical_window_repeats: usize,
    pub empty_or_reversed_windows: usize,
    pub max_queries_per_signature: usize,
    pub adjacent_overlapping_windows: usize,
    pub monotone_before_overlapping_windows: usize,
}

/// The repository orders by exact signature, since, then before. Equal keys
/// are consequently adjacent. Retain no signatures, bounds or event IDs.
pub(super) fn summarize(
    queries: &[PriorOccurrenceQuery],
    order: &[usize],
) -> SortedRecurrenceWindowStatistics {
    let mut statistics = SortedRecurrenceWindowStatistics {
        input_queries: queries.len(),
        ..Default::default()
    };
    let mut previous: Option<&PriorOccurrenceQuery> = None;
    let mut group_queries = 0;
    for &index in order {
        let current = &queries[index];
        let same_signature = previous.is_some_and(|last| last.signature == current.signature);
        if same_signature {
            group_queries += 1;
        } else {
            statistics.distinct_signatures += 1;
            group_queries = 1;
        }
        statistics.max_queries_per_signature =
            statistics.max_queries_per_signature.max(group_queries);
        if !same_signature
            || previous
                .is_some_and(|last| last.since != current.since || last.before != current.before)
        {
            statistics.distinct_windows += 1;
        }
        if current.since >= current.before {
            statistics.empty_or_reversed_windows += 1;
        }
        if let Some(last) = previous.filter(|_| same_signature) {
            // Compare only immediate neighbors, with half-open bounds. An
            // invalid window interrupts adjacency rather than being skipped.
            if last.since < last.before
                && current.since < current.before
                && last.since.max(current.since) < last.before.min(current.before)
            {
                statistics.adjacent_overlapping_windows += 1;
                if current.before >= last.before {
                    statistics.monotone_before_overlapping_windows += 1;
                }
            }
        }
        previous = Some(current);
    }
    statistics.identical_window_repeats = statistics.input_queries - statistics.distinct_windows;
    statistics
}

#[cfg(test)]
#[path = "recurrence_window_statistics_tests.rs"]
mod tests;
