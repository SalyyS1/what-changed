//! Real, bounded owner corpus and deterministic queue/pool saturation fixtures.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[path = "owner_workspace_capacity/backpressure.rs"]
mod backpressure;
#[path = "owner_workspace_capacity/fixture.rs"]
mod fixture;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use fixture::{app_input, Fixture};
use what_changed_lib::owner_workspace::{OwnerPageInput, MAX_APPS};

#[test]
fn twenty_apps_and_one_hundred_scenarios_keep_selected_app_pages_bounded() {
    let fixture = Fixture::new();
    let corpus = fixture.corpus();
    assert_eq!(corpus.len(), 20);
    assert_eq!(fixture.store.list_apps(false).unwrap().len(), MAX_APPS);
    assert!(fixture.store.save_app(app_input("Overflow")).is_err());
    assert_eq!(fixture.store.list_apps(false).unwrap().len(), 20);

    let mut samples = Vec::new();
    for _round in 0..3 {
        let mut all_scenarios = BTreeSet::new();
        for (app_id, expected) in &corpus {
            let mut after = None;
            let mut actual = Vec::new();
            let mut page_count = 0;
            loop {
                let started = Instant::now();
                let page = fixture
                    .store
                    .list_scenarios(
                        app_id,
                        OwnerPageInput {
                            after: after.clone(),
                            limit: 2,
                        },
                    )
                    .unwrap();
                samples.push(started.elapsed());
                page_count += 1;
                assert!(!page.items.is_empty());
                assert!(page.items.len() <= 2);
                for scenario in &page.items {
                    assert_eq!(&scenario.app_id, app_id);
                    assert_eq!(scenario.revision, 1);
                    assert!(all_scenarios.insert(scenario.id.clone()));
                    actual.push(scenario.id.clone());
                }
                match page.next_cursor {
                    Some(cursor) => {
                        assert!(after.as_ref().is_none_or(|prior| prior < &cursor));
                        assert_eq!(
                            cursor,
                            format!("{}:{:010}", page.items.last().unwrap().id, 1)
                        );
                        after = Some(cursor);
                    }
                    None => break,
                }
                assert!(page_count < 4, "selected-app cursor must advance");
            }
            assert_eq!(page_count, 3);
            assert_eq!(&actual, expected);
        }
        assert_eq!(all_scenarios.len(), 100);
    }
    assert_eq!(samples.len(), 180);
    print_timings(&mut samples);
}

fn print_timings(samples: &mut [Duration]) {
    samples.sort_unstable();
    let percentile = |percent: usize| {
        let index = (samples.len() * percent).div_ceil(100) - 1;
        samples[index].as_secs_f64() * 1000.0
    };
    println!(
        "owner selected-app scenarios: apps=20 total_scenarios=100 page_limit=2 rounds=3 samples={} p50_ms={:.3} p95_ms={:.3} max_ms={:.3}",
        samples.len(), percentile(50), percentile(95), percentile(100)
    );
}
