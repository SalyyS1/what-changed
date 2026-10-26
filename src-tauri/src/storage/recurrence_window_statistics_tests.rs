use super::*;
use crate::domain::{Category, EventSignature, EventType};

fn query(entity: &str, since: i64, before: i64) -> PriorOccurrenceQuery {
    PriorOccurrenceQuery {
        event_id: 7,
        signature: EventSignature {
            entity_id: entity.to_owned(),
            category: Category::Service,
            event_type: EventType::Stopped,
        },
        since,
        before,
    }
}

fn statistics(queries: &[PriorOccurrenceQuery]) -> SortedRecurrenceWindowStatistics {
    summarize(queries, &(0..queries.len()).collect::<Vec<_>>())
}

#[test]
fn empty_inputs_have_zero_statistics() {
    assert_eq!(summarize(&[], &[]), Default::default());
}

#[test]
fn signature_boundaries_include_entity_category_and_event_type() {
    let mut first = query("a", 0, 4);
    first.signature.event_type = EventType::Started;
    let mut category_boundary = query("a", 0, 4);
    category_boundary.signature.category = Category::System;
    let queries = [
        first.clone(),
        first,
        query("a", 0, 4),
        category_boundary,
        query("b", 0, 4),
    ];
    assert_eq!(
        statistics(&queries),
        SortedRecurrenceWindowStatistics {
            input_queries: 5,
            distinct_signatures: 4,
            distinct_windows: 4,
            identical_window_repeats: 1,
            empty_or_reversed_windows: 0,
            max_queries_per_signature: 2,
            adjacent_overlapping_windows: 1,
            monotone_before_overlapping_windows: 1,
        }
    );
}

#[test]
fn identical_windows_ignore_event_ids_and_count_every_adjacent_overlap() {
    let mut second = query("a", 0, 4);
    second.event_id = 900;
    let result = statistics(&[query("a", 0, 4), second, query("a", 0, 4)]);
    assert_eq!(result.input_queries, 3);
    assert_eq!(result.distinct_windows, 1);
    assert_eq!(result.identical_window_repeats, 2);
    assert_eq!(result.adjacent_overlapping_windows, 2);
    assert_eq!(result.monotone_before_overlapping_windows, 2);
}

#[test]
fn touching_and_disjoint_half_open_windows_do_not_overlap() {
    let result = statistics(&[query("a", 0, 4), query("a", 4, 8), query("a", 9, 10)]);
    assert_eq!(result.distinct_windows, 3);
    assert_eq!(result.identical_window_repeats, 0);
    assert_eq!(result.adjacent_overlapping_windows, 0);
    assert_eq!(result.monotone_before_overlapping_windows, 0);
}

#[test]
fn nesting_and_decreasing_before_are_distinct_from_monotone_overlap() {
    let result = statistics(&[
        query("a", 0, 10),
        query("a", 1, 8),
        query("a", 2, 8),
        query("a", 3, 12),
    ]);
    assert_eq!(result.distinct_windows, 4);
    assert_eq!(result.adjacent_overlapping_windows, 3);
    assert_eq!(result.monotone_before_overlapping_windows, 2);
}

#[test]
fn invalid_windows_count_as_distinct_keys_and_interrupt_adjacency() {
    let result = statistics(&[
        query("a", 0, 10),
        query("a", 2, 2),
        query("a", 2, 2),
        query("a", 3, 1),
        query("a", 4, 8),
    ]);
    assert_eq!(result.input_queries, 5);
    assert_eq!(result.distinct_windows, 4);
    assert_eq!(result.identical_window_repeats, 1);
    assert_eq!(result.empty_or_reversed_windows, 3);
    assert_eq!(result.max_queries_per_signature, 5);
    assert_eq!(result.adjacent_overlapping_windows, 0);
    assert_eq!(result.monotone_before_overlapping_windows, 0);
}

#[test]
fn maximum_group_size_includes_repeated_queries_and_the_last_group() {
    let queries: Vec<_> = [("a", 3), ("b", 2), ("c", 4)]
        .into_iter()
        .flat_map(|(entity, count)| (0..count).map(move |_| query(entity, 0, 4)))
        .collect();
    let result = statistics(&queries);
    assert_eq!(result.distinct_signatures, 3);
    assert_eq!(result.distinct_windows, 3);
    assert_eq!(result.identical_window_repeats, 6);
    assert_eq!(result.max_queries_per_signature, 4);
    assert_eq!(result.adjacent_overlapping_windows, 6);
    assert_eq!(result.monotone_before_overlapping_windows, 6);
}

#[test]
fn statistics_walk_the_supplied_execution_order() {
    let queries = [query("a", 2, 12), query("a", 0, 10), query("a", 1, 8)];
    let result = summarize(&queries, &[1, 2, 0]);
    assert_eq!(result.input_queries, 3);
    assert_eq!(result.adjacent_overlapping_windows, 2);
    assert_eq!(result.monotone_before_overlapping_windows, 1);
}
