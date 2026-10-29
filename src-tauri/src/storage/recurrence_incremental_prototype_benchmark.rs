use super::*;
use std::time::{Duration, Instant};

#[path = "recurrence_incremental_prototype_recipes.rs"]
mod recipes;
use recipes::{seed, Recipe, RECIPES};

fn old_timed(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> (BTreeMap<i64, u64>, Duration) {
    let started = Instant::now();
    let result = std::hint::black_box(original_batch(conn, queries));
    let elapsed = started.elapsed();
    (result.unwrap(), elapsed)
}

fn new_timed(conn: &Connection, queries: &[PriorOccurrenceQuery]) -> (CandidateRun, Duration) {
    let started = Instant::now();
    let result = std::hint::black_box(incremental_batch(conn, queries));
    let elapsed = started.elapsed();
    (result.unwrap(), elapsed)
}

fn assert_probes(run: &CandidateRun, recipe: &Recipe) {
    let probes = run.probes.as_ref().unwrap();
    assert_eq!(probes.full, 5_000 - recipe.rolled_overlaps);
    assert_eq!(probes.delta, recipe.rolled_overlaps * 2);
    assert_eq!(probes.identical_reuses, 0);
}

#[test]
#[ignore = "Explicit synthetic matched recurrence benchmark; two rounds of eleven pairs per family"]
fn two_rounds_eleven_alternating_pairs_compare_transaction_local_counts() {
    // Predeclared 4 families × 2 rounds × 11 matched pairs: emit all 88 pairs.
    // Fresh in-memory history per family/round; seeding, warm-up, result parity,
    // formatting/output and fixture teardown are all outside measured calls.
    // This isolated SQL microbenchmark does not establish service acceptance.
    for round in 1..=2 {
        for recipe in &RECIPES {
            let (conn, queries) = seed(recipe);
            let tx = conn.unchecked_transaction().unwrap();
            let warm_old = original_batch(&tx, &queries).unwrap();
            let warm_new = incremental_batch(&tx, &queries).unwrap();
            assert_eq!(warm_old, warm_new.counts);
            assert_probes(&warm_new, recipe);
            drop(warm_old);
            drop(warm_new);
            eprintln!(
                "DIAGNOSTIC recurrence_prototype_recipe round={round} family={} inputs=5000 signatures={} history_per_signature={} history_rows={} window_width={} shift={} potential_overlaps={} rolled_overlaps={} max_group={} identical_repeats=0 input_reverse=1 input_rotate=173 old_sql_executions=5000 new_sql_executions={}",
                recipe.label, recipe.signatures, recipe.history_per_signature,
                recipe.signatures * recipe.history_per_signature,
                recipe.window_width, recipe.shift, recipe.overlaps,
                recipe.rolled_overlaps, recipe.max_group,
                5_000 + recipe.rolled_overlaps,
            );
            for pair in 1..=11 {
                // Reverse the first method across rounds; alternate within each
                // round so one method does not always receive the warmer pages.
                let old_first = (round + pair) % 2 == 0;
                let (old, old_elapsed, new, new_elapsed) = if old_first {
                    let (old, old_elapsed) = old_timed(&tx, &queries);
                    let (new, new_elapsed) = new_timed(&tx, &queries);
                    (old, old_elapsed, new, new_elapsed)
                } else {
                    let (new, new_elapsed) = new_timed(&tx, &queries);
                    let (old, old_elapsed) = old_timed(&tx, &queries);
                    (old, old_elapsed, new, new_elapsed)
                };
                assert_eq!(old, new.counts, "complete benchmark map parity");
                assert_probes(&new, recipe);
                eprintln!(
                    "DIAGNOSTIC recurrence_prototype_pair round={round} family={} pair={pair} old_first={} old_ns={} new_ns={} old_sql_executions=5000 new_full_counts={} new_delta_counts={} new_identical_reuses={} new_sql_executions={}",
                    recipe.label, usize::from(old_first), old_elapsed.as_nanos(),
                    new_elapsed.as_nanos(), new.probes.as_ref().unwrap().full,
                    new.probes.as_ref().unwrap().delta,
                    new.probes.as_ref().unwrap().identical_reuses,
                    new.probes.as_ref().unwrap().full + new.probes.as_ref().unwrap().delta,
                );
                // Explicit drops are outside both clocks, including map teardown.
                drop(old);
                drop(new);
            }
            tx.commit().unwrap();
            drop(conn);
        }
    }
}
