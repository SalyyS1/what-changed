//! Independently derive the actual generator's cross-boundary graph witness.

use std::collections::{HashMap, HashSet};

use super::super::super::{make_batch, relationship_selection, Options, ENTITY_COUNT};

pub(super) struct Witness {
    pub target: String,
    pub edge_index: usize,
    pub edge_timestamp: i64,
    pub subject_index: usize,
}

pub(super) fn witness(options: &Options, end_at: i64) -> Witness {
    let selected = relationship_selection(options.events, options.seed).unwrap();
    let mut ordinals = vec![0; ENTITY_COUNT as usize];
    let batch = make_batch(0, 5_000, options, end_at, &selected, &[], &mut ordinals);
    let mut first_subject = HashMap::new();
    for (index, event) in batch.iter().enumerate() {
        first_subject
            .entry(event.get().entity_id.as_str())
            .or_insert(index);
    }
    let mut present = batch[..1_000]
        .iter()
        .map(|event| event.get().entity_id.as_str())
        .collect::<HashSet<_>>();
    for (edge_index, event) in batch[..1_000].iter().enumerate() {
        for target in &event.get().related_entity_ids {
            if !present.contains(target.as_str()) {
                if let Some(&subject_index) = first_subject.get(target.as_str()) {
                    if subject_index >= 1_000
                        && event.get().timestamp < batch[subject_index].get().timestamp
                    {
                        return Witness {
                            target: target.clone(),
                            edge_index,
                            edge_timestamp: event.get().timestamp,
                            subject_index,
                        };
                    }
                }
            }
            present.insert(target.as_str());
        }
    }
    panic!("actual generator did not exercise the missing endpoint across logical units");
}
