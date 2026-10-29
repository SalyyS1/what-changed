use super::fixture::*;
use super::*;
use crate::domain::{Category, EventType};
use crate::error::AppError;

#[test]
fn empty_and_threshold_inputs_match_in_one_read_transaction() {
    let conn = connection();
    for timestamp in 0..20 {
        insert(&conn, &signature("a"), timestamp);
    }
    let tx = conn.unchecked_transaction().unwrap();
    for len in [0, 1_999, 2_000, 2_001] {
        let queries = padded(vec![query(1, "a", 0, 10)], len);
        let queries = if len == 0 { Vec::new() } else { queries };
        let result = assert_parity(&tx, &queries);
        assert_eq!(result.probes.is_none(), len < 2_000);
    }
}

#[test]
fn half_open_extreme_and_invalid_windows_preserve_counts_and_probe_rules() {
    let conn = connection();
    for timestamp in [i64::MIN, -5, -5, 0, 2, 4, 5, 7, 9, 10, 20, i64::MAX] {
        insert(&conn, &signature("a"), timestamp);
    }
    // Expected delta/reuse counts also prove that noneligible pairs execute
    // full COUNTs, and that zero-width delta intervals are never executed.
    type EligibilityCase<'a> = (&'a str, &'a [(i64, i64)], usize, usize);
    let cases: &[EligibilityCase<'_>] = &[
        ("identical", &[(0, 10), (0, 10)], 0, 1),
        ("touching", &[(0, 5), (5, 10)], 0, 0),
        ("disjoint", &[(0, 4), (8, 12)], 0, 0),
        ("nested", &[(0, 10), (2, 8)], 0, 0),
        ("crossing", &[(0, 7), (3, 10)], 2, 0),
        ("decreasing-before", &[(0, 10), (2, 9)], 0, 0),
        ("equal-since", &[(0, 7), (0, 10)], 1, 0),
        ("equal-before", &[(0, 10), (2, 10)], 1, 0),
        ("empty", &[(2, 2), (2, 2)], 0, 0),
        ("reversed", &[(4, 1), (4, 1)], 0, 0),
        ("extremes", &[(i64::MIN, 0), (i64::MIN + 1, i64::MAX)], 2, 0),
        (
            "maximum-exclusive",
            &[(i64::MAX - 1, i64::MAX), (i64::MAX, i64::MAX)],
            0,
            0,
        ),
        ("invalid-interrupts", &[(0, 10), (2, 2), (3, 8)], 0, 0),
    ];
    let tx = conn.unchecked_transaction().unwrap();
    for &(label, windows, delta, reused) in cases {
        let queries = padded(
            windows
                .iter()
                .enumerate()
                .map(|(index, &(since, before))| query(index as i64 + 1, "a", since, before))
                .collect(),
            2_000,
        );
        let result = assert_parity(&tx, &queries);
        let probes = result.probes.unwrap();
        assert_eq!(probes.delta, delta, "{label}");
        assert_eq!(probes.identical_reuses, reused, "{label}");
        assert_eq!(
            probes.full,
            2_000 - reused - usize::from(delta > 0),
            "{label}"
        );
    }
}

#[test]
fn duplicate_timestamps_and_all_signature_fields_are_counted_exactly() {
    let conn = connection();
    let mut system = signature("a");
    system.category = Category::System;
    let mut started = signature("a");
    started.event_type = EventType::Started;
    let signatures = [
        (signature("a"), 2),
        (system, 3),
        (started, 5),
        (signature("b"), 7),
    ];
    let mut queries = Vec::new();
    for (index, (signature, multiplicity)) in signatures.iter().enumerate() {
        for _ in 0..*multiplicity {
            insert(&conn, signature, 4);
        }
        for offset in 0..2 {
            queries.push(PriorOccurrenceQuery {
                event_id: index as i64 * 2 + offset,
                signature: signature.clone(),
                since: offset,
                before: 5,
            });
        }
    }
    let tx = conn.unchecked_transaction().unwrap();
    let result = assert_parity(&tx, &padded(queries, 2_000));
    for (index, (_, multiplicity)) in signatures.iter().enumerate() {
        assert_eq!(
            result.counts.get(&(index as i64 * 2)),
            Some(&(*multiplicity as u64))
        );
        assert_eq!(
            result.counts.get(&(index as i64 * 2 + 1)),
            Some(&(*multiplicity as u64))
        );
    }
    assert_eq!(result.probes.unwrap().delta, 4);
}

#[test]
fn duplicate_ids_keep_original_chunks_and_later_chunk_overwrites() {
    let conn = connection();
    for timestamp in 0..32 {
        insert(&conn, &signature("a"), timestamp);
    }
    let mut queries = padded(Vec::new(), 2_001);
    for (index, id, before) in [
        (148, 7, 2),
        (149, 7, 3),
        (150, 7, 7),
        (298, 9, 7),
        (299, 9, 11),
        (300, 9, 13),
        (301, 9, 17),
    ] {
        queries[index] = query(id, "a", 0, before);
    }
    let tx = conn.unchecked_transaction().unwrap();
    let result = assert_parity(&tx, &queries);
    assert_eq!(result.counts.get(&7), Some(&7));
    assert_eq!(result.counts.get(&9), Some(&30));
}

#[test]
fn deterministic_histories_and_varied_input_orders_preserve_complete_maps() {
    let conn = connection();
    let mut state = 0x4D59_5DF4_D0F3_3173_u64;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        state
    };
    for entity in 0..13 {
        for _ in 0..150 {
            let timestamp = (next() % 161) as i64 - 80;
            insert(
                &conn,
                &signature(&format!("generated-{entity:02}")),
                timestamp,
            );
        }
    }
    let mut queries: Vec<_> = (0..2_111)
        .map(|index| {
            let entity = next() % 13;
            let since = (next() % 221) as i64 - 110;
            let width = (next() % 81) as i64 - 15;
            query(
                index % 307,
                &format!("generated-{entity:02}"),
                since,
                since + width,
            )
        })
        .collect();
    let tx = conn.unchecked_transaction().unwrap();
    for _ in 0..4 {
        assert_parity(&tx, &queries);
        queries.reverse();
        queries.rotate_left(173);
    }
}

#[test]
fn autocommit_falls_back_directly_without_starting_a_transaction() {
    let conn = connection();
    insert(&conn, &signature("a"), 4);
    let queries = vec![query(7, "a", 0, 10); 2_000];
    assert!(conn.is_autocommit());
    let result = incremental_batch(&conn, &queries).unwrap();
    assert_eq!(result.counts, original_batch(&conn, &queries).unwrap());
    assert_eq!(
        result.counts,
        prior_occurrences_batch(&conn, &queries).unwrap()
    );
    assert!(result.probes.is_none());
    assert!(conn.is_autocommit());
}

#[test]
fn wal_read_snapshot_survives_an_external_commit() {
    let directory = OwnedDirectory::new();
    {
        let path = directory.0.join("recurrence.sqlite");
        let writer = Connection::open(&path).unwrap();
        writer.pragma_update(None, "journal_mode", "WAL").unwrap();
        schema(&writer);
        insert(&writer, &signature("a"), 4);
        let reader = Connection::open(&path).unwrap();
        let tx = reader.unchecked_transaction().unwrap();
        let queries = padded(vec![query(1, "a", 0, 10), query(2, "a", 1, 11)], 2_000);
        let before = assert_parity(&tx, &queries).counts;
        insert(&writer, &signature("a"), 5);
        assert_eq!(assert_parity(&tx, &queries).counts, before);
        #[cfg(feature = "devtools")]
        {
            // Baseline/three-way parity above is outside this capture. Capture
            // exactly one public invocation on the already pinned WAL snapshot.
            let captured = super::super::capture_sorted_recurrence_plan(|| {
                prior_occurrences_batch(&tx, &queries)
            })
            .unwrap();
            assert_eq!(captured.value.unwrap(), before);
            let plan = captured.plan.unwrap();
            assert_eq!(plan.completed_input_queries, 2_000);
            assert_eq!(plan.completed_full_executions, 1_999);
            assert_eq!(plan.completed_delta_executions, 2);
            assert_eq!(plan.completed_rolling_queries, 1);
            assert_eq!(plan.identical_reuses, 0);
            assert_eq!(plan.completed_executions, 2_001);
            assert_eq!(
                plan.delta_plan.unwrap().kind,
                super::super::SortedRecurrenceCountKind::Delta
            );
        }
        tx.commit().unwrap();
        let after = prior_occurrences_batch(&reader, &queries).unwrap();
        assert_eq!(before.get(&1), Some(&1));
        assert_eq!(after.get(&1), Some(&2));
    }
    directory.cleanup();
}

#[test]
fn checked_arithmetic_rejects_underflow_and_overflow_without_saturation() {
    assert_eq!(checked_delta(5, 3, 7).unwrap(), 9);
    assert_eq!(
        checked_delta(u64::MAX, u64::MAX, u64::MAX).unwrap(),
        u64::MAX
    );
    for result in [
        checked_delta(1, 2, 0),
        checked_delta(u64::MAX, 0, 1),
        checked_sum(u64::MAX, 1),
    ] {
        assert!(matches!(
            result,
            Err(AppError::Db(DbError::Decoding { .. }))
        ));
    }
}

#[test]
fn missing_table_retains_the_existing_typed_prepare_error() {
    let conn = Connection::open_in_memory().unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    let queries = padded(Vec::new(), 2_000);
    let old = original_batch(&tx, &queries).unwrap_err();
    let actual = prior_occurrences_batch(&tx, &queries).unwrap_err();
    let candidate = incremental_batch(&tx, &queries).err().unwrap();
    for error in [old, actual, candidate] {
        assert!(matches!(
            error,
            AppError::Db(DbError::Sqlite {
                operation: "prepare sorted recurrence counts",
                ..
            })
        ));
    }
}

#[test]
fn span_guard_rolls_only_below_the_full_span_cost() {
    let conn = connection();
    for timestamp in -1..16 {
        insert(&conn, &signature("a"), timestamp);
    }
    let tx = conn.unchecked_transaction().unwrap();
    // Current width is eight; delta spans are respectively seven, eight and
    // nine. Every pair remains valid, monotone and strictly overlapping.
    for (old_before, should_roll) in [(9, true), (8, false), (7, false)] {
        let queries = padded(
            vec![query(1, "a", 0, old_before), query(2, "a", 4, 12)],
            2_000,
        );
        assert_eq!(can_roll(&queries[0], &queries[1]), should_roll);
        let result = assert_parity(&tx, &queries);
        let probes = result.probes.unwrap();
        assert_eq!(probes.full, 2_000 - usize::from(should_roll));
        assert_eq!(probes.delta, if should_roll { 2 } else { 0 });
        assert_eq!(probes.identical_reuses, 0);
        assert_eq!(result.counts.get(&2), Some(&8));
    }
}

#[test]
fn span_guard_retains_valid_duplicates_and_single_delta_probes() {
    let conn = connection();
    for timestamp in -1..16 {
        insert(&conn, &signature("a"), timestamp);
    }
    let tx = conn.unchecked_transaction().unwrap();
    for (windows, delta, reused) in [
        ([(4, 12), (4, 12)], 0, 1),
        ([(0, 8), (0, 9)], 1, 0),
        ([(0, 9), (1, 9)], 1, 0),
    ] {
        let queries = padded(
            windows
                .into_iter()
                .enumerate()
                .map(|(index, (since, before))| query(index as i64 + 1, "a", since, before))
                .collect(),
            2_000,
        );
        assert!(can_roll(&queries[0], &queries[1]));
        let result = assert_parity(&tx, &queries);
        let probes = result.probes.unwrap();
        assert_eq!(probes.full, 1_999);
        assert_eq!(probes.delta, delta);
        assert_eq!(probes.identical_reuses, reused);
    }
}

#[test]
fn widened_endpoints_preserve_guard_boundaries_at_i64_extremes() {
    let conn = connection();
    for timestamp in [i64::MIN, -1, 0, 1, i64::MAX - 2, i64::MAX - 1, i64::MAX] {
        insert(&conn, &signature("a"), timestamp);
    }
    let tx = conn.unchecked_transaction().unwrap();
    for (old_since, old_before, new_since, new_before, should_roll) in [
        (i64::MIN, i64::MAX, -1, i64::MAX, true),
        (i64::MIN, i64::MAX - 1, -1, i64::MAX - 1, false),
        (i64::MIN, i64::MAX, 0, i64::MAX, false),
        (i64::MIN, 0, i64::MIN + 1, i64::MAX, true),
        (i64::MIN, i64::MAX - 2, i64::MIN + 1, i64::MAX - 1, true),
    ] {
        let queries = padded(
            vec![
                query(1, "a", old_since, old_before),
                query(2, "a", new_since, new_before),
            ],
            2_000,
        );
        assert_eq!(can_roll(&queries[0], &queries[1]), should_roll);
        let result = assert_parity(&tx, &queries);
        let probes = result.probes.unwrap();
        let deltas = usize::from(old_since != new_since) + usize::from(old_before != new_before);
        assert_eq!(probes.full, 2_000 - usize::from(should_roll));
        assert_eq!(probes.delta, if should_roll { deltas } else { 0 });
        assert_eq!(probes.identical_reuses, 0);
    }
}

#[test]
fn clustered_edge_history_preserves_parity_despite_short_delta_spans() {
    let conn = connection();
    // Dense removed/added edges make two short probes visit 4,096 rows, while
    // the new full window contains only 2,049. Span cost cannot promise speed.
    for _ in 0..2_048 {
        insert(&conn, &signature("a"), 0);
        insert(&conn, &signature("a"), 100);
    }
    insert(&conn, &signature("a"), 50);
    let queries = padded(vec![query(1, "a", 0, 100), query(2, "a", 1, 101)], 2_000);
    assert!(can_roll(&queries[0], &queries[1]));
    let tx = conn.unchecked_transaction().unwrap();
    let result = assert_parity(&tx, &queries);
    let probes = result.probes.unwrap();
    assert_eq!(probes.full, 1_999);
    assert_eq!(probes.delta, 2);
    assert_eq!(probes.identical_reuses, 0);
    assert_eq!(result.counts.get(&1), Some(&2_049));
    assert_eq!(result.counts.get(&2), Some(&2_049));
}
