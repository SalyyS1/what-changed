//! Isolated actual-bound page-only smoke, never a p95 or larger-scale gate.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::domain::{EventQuery, TimeRange};
use what_changed_lib::services::query_service;
use what_changed_lib::storage::page_plan_capture::{capture_page_plan, PagePlanVariant};

#[path = "support/page_capture_assertions.rs"]
mod page_capture_assertions;
#[path = "support/perf_fixture.rs"]
mod perf_fixture;

#[test]
#[ignore = "one unchanged canonical10k actual page1/page10 diagnostic; root executes after review"]
fn canonical_10k_actual_first_and_tenth_page_plans() {
    let (fixture, corpus) = perf_fixture::PerfFixture::new_seeded_db(10_000);
    assert_eq!(corpus.payload_profile, "canonical");
    let mut query = EventQuery {
        range: TimeRange::Custom {
            from: corpus.from,
            to: corpus.to,
        },
        limit: 200,
        ..EventQuery::default()
    };
    let first = capture_page_plan(PagePlanVariant::for_query(&query), || {
        query_service::page(fixture.db(), query.clone())
    })
    .unwrap();
    let page = first.value.unwrap();
    let evidence = first.evidence.unwrap();
    page_capture_assertions::assert_execution(&evidence, PagePlanVariant::for_query(&query), 201);
    page_capture_assertions::assert_service_page(fixture.db(), &query, &page, 200);
    println!("actual page plan: corpus_rows=10000 page_number=1 timing_acceptance=0 evidence={evidence:?}");
    query.cursor = Some(page.next.unwrap());
    // Exactly eight uncaptured invocations: actual pages 2 through 9.
    for _page_number in 2..=9 {
        let page = query_service::page(fixture.db(), query.clone()).unwrap();
        page_capture_assertions::assert_service_page(fixture.db(), &query, &page, 200);
        query.cursor = Some(page.next.unwrap());
    }
    let tenth = capture_page_plan(PagePlanVariant::for_query(&query), || {
        query_service::page(fixture.db(), query.clone())
    })
    .unwrap();
    let page = tenth.value.unwrap();
    let evidence = tenth.evidence.unwrap();
    page_capture_assertions::assert_execution(&evidence, PagePlanVariant::for_query(&query), 201);
    page_capture_assertions::assert_service_page(fixture.db(), &query, &page, 200);
    assert!(page.next.is_some());
    println!("actual page plan: corpus_rows=10000 page_number=10 actual_page_invocations=10 timing_acceptance=0 evidence={evidence:?}");
    drop(fixture); // Existing exclusive guard closes/joins and reports cleanup failures.
}
