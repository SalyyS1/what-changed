//! Page-only assertions shared by service regression and isolated diagnostics.

use what_changed_lib::domain::{Cursor, EventQuery};
use what_changed_lib::services::{
    cluster::TimelineItem, query_service, query_service::TimelinePage,
};
use what_changed_lib::storage::{
    page_plan_capture::{
        PageAccessKind, PageExecutionEvidence, PageExplainOutcome, PagePlanNodeKind,
        PagePlanVariant,
    },
    Db,
};

pub fn assert_execution(
    evidence: &PageExecutionEvidence,
    variant: PagePlanVariant,
    decoded_rows: usize,
) {
    assert_eq!(evidence.variant, variant);
    assert_eq!(evidence.invocations, 1);
    assert!(evidence.prepared && evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(decoded_rows));
    let bindings = 3
        + variant.category_arity
        + variant.importance_arity
        + usize::from(variant.mask & 4 != 0)
        + 2 * usize::from(variant.mask & 8 != 0);
    assert_eq!(evidence.bindings, Some(bindings));
    assert_eq!(evidence.statement_parameters, Some(bindings));
    let PageExplainOutcome::Complete(nodes) = &evidence.explain else {
        panic!("actual page EQP must be complete");
    };
    assert!(nodes.iter().any(|node| matches!(&node.kind, PagePlanNodeKind::Events(access) if access.operation==PageAccessKind::Search && access.index.is_some())), "actual page must use indexed SEARCH events");
    assert!(nodes.iter().all(|node| !matches!(&node.kind, PagePlanNodeKind::Events(access) if access.operation==PageAccessKind::Scan && access.index.is_none())), "actual page must not scan unindexed events");
}

pub fn assert_service_page(
    db: &Db,
    query: &EventQuery,
    page: &TimelinePage,
    expected_rows: usize,
) -> Vec<(i64, i64)> {
    let mut keys = Vec::with_capacity(expected_rows);
    for item in &page.items {
        match item {
            TimelineItem::Event { event } => keys.push((event.timestamp, event.id)),
            TimelineItem::Cluster { cluster, count, .. } => {
                let expanded = query_service::expand_cluster(db, cluster).unwrap();
                assert_eq!(expanded.len(), *count);
                keys.extend(
                    expanded
                        .into_iter()
                        .map(|event| (event.timestamp, event.id)),
                );
            }
        }
    }
    assert_eq!(keys.len(), expected_rows);
    assert!(
        keys.windows(2).all(|pair| pair[0] > pair[1]),
        "raw timestamp/id order must remain strict across clusters"
    );
    if let Some(previous) = query.cursor {
        assert!(keys
            .iter()
            .all(|key| *key < (previous.timestamp, previous.id)));
    }
    if let Some(next) = page.next {
        let &(timestamp, id) = keys.last().unwrap();
        assert_eq!(next, Cursor { timestamp, id });
    }
    keys
}
