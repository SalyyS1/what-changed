use super::fixture::{connection, query};
use super::*;

pub(super) struct Recipe {
    pub label: &'static str,
    pub signatures: usize,
    pub history_per_signature: usize,
    pub window_width: i64,
    pub shift: i64,
    pub overlaps: usize,
    pub rolled_overlaps: usize,
    pub max_group: usize,
}

pub(super) const RECIPES: [Recipe; 4] = [
    // Only the input distribution matches measured 1M statistics. The fixed
    // 128-row/signature history and widths are synthetic moderate density.
    Recipe {
        label: "canonical_like_input",
        signatures: 3_429,
        history_per_signature: 128,
        window_width: 96,
        shift: 1,
        overlaps: 1_571,
        rolled_overlaps: 1_571,
        max_group: 18,
    },
    Recipe {
        label: "unique_sparse",
        signatures: 5_000,
        history_per_signature: 4,
        window_width: 96,
        shift: 1,
        overlaps: 0,
        rolled_overlaps: 0,
        max_group: 1,
    },
    Recipe {
        label: "dense_overlap",
        signatures: 64,
        history_per_signature: 2_048,
        window_width: 1_024,
        shift: 1,
        overlaps: 4_936,
        rolled_overlaps: 4_936,
        max_group: 79,
    },
    Recipe {
        label: "wide_shift_tiny_overlap",
        signatures: 1_000,
        history_per_signature: 1_280,
        window_width: 256,
        shift: 255,
        overlaps: 4_000,
        rolled_overlaps: 0,
        max_group: 5,
    },
];

fn group_size(recipe: &Recipe, index: usize) -> usize {
    match recipe.label {
        "canonical_like_input" if index == 0 => 18,
        "canonical_like_input" if index <= 1_554 => 2,
        "canonical_like_input" => 1,
        _ => 5_000 / recipe.signatures + usize::from(index < 5_000 % recipe.signatures),
    }
}

pub(super) fn seed(recipe: &Recipe) -> (Connection, Vec<PriorOccurrenceQuery>) {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    let mut queries = Vec::with_capacity(5_000);
    let mut insert = tx
        .prepare("INSERT INTO events VALUES (?1, 'service', 'stopped', ?2)")
        .unwrap();
    let mut maximum_group = 0;
    for index in 0..recipe.signatures {
        let entity = format!("synthetic-prototype-{index:05}");
        for timestamp in 0..recipe.history_per_signature {
            insert.execute((&entity, timestamp as i64)).unwrap();
        }
        let count = group_size(recipe, index);
        maximum_group = maximum_group.max(count);
        for ordinal in 0..count {
            let since = ordinal as i64 * recipe.shift;
            queries.push(query(
                queries.len() as i64,
                &entity,
                since,
                since + recipe.window_width,
            ));
        }
    }
    drop(insert);
    tx.commit().unwrap();
    assert_eq!(queries.len(), 5_000);
    assert_eq!(queries.len() - recipe.signatures, recipe.overlaps);
    assert_eq!(maximum_group, recipe.max_group);
    // Same fixed permutation in every round/family; IDs preserve input identity.
    queries.reverse();
    queries.rotate_left(173);
    (conn, queries)
}
