//! Phase 06 attention-budget contract for a realistic synthetic day.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::domain::{Category, EventType, Importance};
use what_changed_lib::testing::synthetic::{self, SyntheticProfile};

#[test]
fn a_synthetic_day_is_calm_and_application_updates_never_escalate() {
    let events = synthetic::profile(SyntheticProfile::Day, 0xCA11, 1_700_000_000_000);
    let informational = events
        .iter()
        .filter(|event| event.get().importance == Importance::Informational)
        .count();
    let critical = events
        .iter()
        .filter(|event| event.get().importance == Importance::Critical)
        .count();

    assert!(
        informational * 100 >= events.len() * 80,
        "only {informational}/{} events were informational",
        events.len()
    );
    assert!(
        critical * 100 <= events.len() * 2,
        "{critical}/{} events were critical",
        events.len()
    );
    assert!(events.iter().all(|event| {
        let event = event.get();
        event.category != Category::Application
            || event.event_type != EventType::Updated
            || event.importance == Importance::Informational
    }));
}
