use std::collections::BTreeSet;

use crate::domain::{Importance, Relationship, SystemEvent};

use super::score;

pub const MAX_EVENTS: usize = 5_000;

pub fn downsample(
    mut events: Vec<SystemEvent>,
    relationships: &[Relationship],
) -> (Vec<SystemEvent>, usize) {
    if events.len() <= MAX_EVENTS {
        return (events, 0);
    }
    let structural = relationships
        .iter()
        .flat_map(|edge| [&edge.source_id, &edge.target_id])
        .cloned()
        .collect::<BTreeSet<_>>();
    events.sort_by(|left, right| {
        let left_keep =
            structural.contains(&left.event.entity_id) || score::is_failure_signal(left);
        let right_keep =
            structural.contains(&right.event.entity_id) || score::is_failure_signal(right);
        right_keep
            .cmp(&left_keep)
            .then_with(|| right.event.importance.cmp(&left.event.importance))
            .then_with(|| right.event.timestamp.cmp(&left.event.timestamp))
            .then_with(|| left.id.cmp(&right.id))
    });
    let dropped = events.len() - MAX_EVENTS;
    events.truncate(MAX_EVENTS);
    events.sort_by_key(|event| (event.event.timestamp, event.id));
    (events, dropped)
}

pub fn is_routine(event: &SystemEvent) -> bool {
    event.event.importance == Importance::Informational
        && !event.event.metadata.as_ref().is_some_and(|metadata| {
            metadata
                .get("reportable")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
}
