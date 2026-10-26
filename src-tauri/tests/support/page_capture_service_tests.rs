use super::*;
use what_changed_lib::domain::TimeRange;
use what_changed_lib::storage::page_plan_capture::{capture_page_plan, PagePlanVariant};

#[test]
fn captured_tied_cluster_pages_preserve_all_raw_ids_and_next_cursor() {
    let db = Db::open_in(&temp_dir("query-page-capture-ties")).unwrap();
    let ids = db
        .insert_events(sanitize(
            (0..8)
                .map(|index| {
                    let mut event = synthetic_event(index, 1_020_000);
                    event.category = Category::Docker;
                    event.entity_id = "container:page-capture-tied-burst".into();
                    event
                })
                .collect(),
        ))
        .unwrap();
    let mut query = EventQuery {
        range: TimeRange::Custom {
            from: 1_000_000,
            to: 1_100_000,
        },
        limit: 5,
        ..EventQuery::default()
    };
    let first = capture_page_plan(PagePlanVariant::for_query(&query), || {
        query_service::page(&db, query.clone())
    })
    .unwrap();
    let page = first.value.unwrap();
    page_capture_assertions::assert_execution(
        &first.evidence.unwrap(),
        PagePlanVariant::for_query(&query),
        6,
    );
    assert!(matches!(
        &page.items[..],
        [TimelineItem::Cluster { count: 5, .. }]
    ));
    let mut keys = page_capture_assertions::assert_service_page(&db, &query, &page, 5);
    query.cursor = Some(page.next.unwrap());
    let second = capture_page_plan(PagePlanVariant::for_query(&query), || {
        query_service::page(&db, query.clone())
    })
    .unwrap();
    let page = second.value.unwrap();
    page_capture_assertions::assert_execution(
        &second.evidence.unwrap(),
        PagePlanVariant::for_query(&query),
        3,
    );
    assert!(page.next.is_none());
    keys.extend(page_capture_assertions::assert_service_page(
        &db, &query, &page, 3,
    ));
    assert_eq!(
        keys.into_iter().map(|(_, id)| id).collect::<Vec<_>>(),
        ids.into_iter().rev().collect::<Vec<_>>()
    );
    db.close();
}
