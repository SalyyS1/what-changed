//! Isolated actual compare event-window evidence, never a whole-compare/p95 gate.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::storage::checkpoint_window_plan_capture::capture_checkpoint_window_plan;
use what_changed_lib::storage::page_plan_capture::{
    PageAccessKind, PageExplainOutcome, PagePlanNodeKind,
};

#[path = "support/perf_fixture.rs"]
mod perf_fixture;

#[test]
#[ignore = "one unchanged canonical10k actual checkpoint event-window diagnostic; root executes after gates"]
fn canonical_10k_actual_checkpoint_compare_event_window_plan() {
    let (fixture, corpus) = perf_fixture::PerfFixture::new_seeded_app_state(10_000);
    assert_eq!(corpus.payload_profile, "canonical");
    let [left, right] = corpus.checkpoint_ids;
    let expected = fixture
        .state()
        .checkpoints
        .compare(left, Some(right))
        .unwrap();
    let captured =
        capture_checkpoint_window_plan(|| fixture.state().checkpoints.compare(left, Some(right)))
            .unwrap();
    let actual = captured.value.unwrap();
    assert!(
        actual == expected,
        "complete comparison return differs; no raw diff is logged"
    );
    assert!(actual.window_from <= actual.window_to);
    assert!(!actual.event_ids_in_window.is_empty());
    assert!(actual.event_ids_in_window.len() <= 10_000);
    let evidence = captured.evidence.unwrap();
    assert_eq!(evidence.invocations, 1);
    assert_eq!(evidence.bindings, Some(3));
    assert_eq!(evidence.statement_parameters, Some(3));
    assert!(evidence.prepared && evidence.query_started && evidence.fully_decoded);
    assert_eq!(
        evidence.decoded_rows,
        Some(actual.event_ids_in_window.len())
    );
    let PageExplainOutcome::Complete(nodes) = &evidence.explain else {
        panic!("actual event-window EQP must be complete");
    };
    assert!(nodes.iter().any(|node| matches!(&node.kind, PagePlanNodeKind::Events(access) if access.operation==PageAccessKind::Search && access.index.is_some())));
    assert!(nodes.iter().all(|node| !matches!(&node.kind, PagePlanNodeKind::Events(access) if access.operation==PageAccessKind::Scan && access.index.is_none())));
    println!("actual checkpoint event-window plan: corpus_rows=10000 comparison_calls=2 captured_window_invocations=1 decoded_rows={} full_return_parity=1 whole_compare_plan_claimed=0 timing_acceptance=0 evidence={evidence:?}", actual.event_ids_in_window.len());
    drop(fixture); // Unchanged guard stops/joins AppState and deletes only its owned root.
}

#[test]
#[ignore = "one unchanged canonical10k complete Some comparison diagnostic; root executes after gates"]
fn canonical_10k_actual_complete_checkpoint_compare_plans() {
    use what_changed_lib::storage::checkpoint_compare_plan_capture::{
        capture_checkpoint_compare_plan, CheckpointCompareOperation, CheckpointComparePlan,
        CheckpointExplainOutcome, CheckpointPlanNodeKind,
    };
    let (fixture, corpus) = perf_fixture::PerfFixture::new_seeded_app_state(10_000);
    assert_eq!(corpus.payload_profile, "canonical");
    let [left, right] = corpus.checkpoint_ids;
    let original = fixture
        .state()
        .checkpoints
        .compare(left, Some(right))
        .unwrap();
    let captured =
        capture_checkpoint_compare_plan(|| fixture.state().checkpoints.compare(left, Some(right)))
            .unwrap();
    let actual = captured.value.unwrap();
    assert!(
        actual == original,
        "complete comparison return differs; no raw diff is logged"
    );
    let evidence = captured.evidence;
    assert_eq!(evidence.failure, None);
    assert_eq!(
        (
            evidence.invocations,
            evidence.metadata_invocations,
            evidence.items_invocations,
            evidence.window_invocations
        ),
        (5, 2, 2, 1)
    );
    assert_eq!(
        evidence
            .reads
            .iter()
            .map(|read| read.operation)
            .collect::<Vec<_>>(),
        [
            CheckpointCompareOperation::Metadata,
            CheckpointCompareOperation::Items,
            CheckpointCompareOperation::Metadata,
            CheckpointCompareOperation::Items,
            CheckpointCompareOperation::EventWindow
        ]
    );
    let counts = [1, 3_000, 1, 3_000, actual.event_ids_in_window.len()];
    for (index, read) in evidence.reads.iter().enumerate() {
        assert!(read.prepared && read.query_started && read.fully_decoded);
        assert_eq!(read.bindings, Some(if index == 4 { 3 } else { 1 }));
        assert_eq!(read.statement_parameters, read.bindings);
        assert_eq!(read.decoded_rows, Some(counts[index]));
        match &read.plan {
            CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::Complete(nodes)) => {
                assert!(nodes.iter().any(|node| match &node.kind {
                    CheckpointPlanNodeKind::Metadata(access)
                    | CheckpointPlanNodeKind::Items(access) =>
                        access.operation == PageAccessKind::Search && access.index.is_some(),
                    _ => false,
                }));
                assert!(nodes.iter().all(|node| !matches!(&node.kind,
                    CheckpointPlanNodeKind::Metadata(access) | CheckpointPlanNodeKind::Items(access)
                    if access.operation==PageAccessKind::Scan && access.index.is_none())));
            }
            CheckpointComparePlan::EventWindow(PageExplainOutcome::Complete(nodes)) => {
                assert!(nodes
                    .iter()
                    .any(|node| matches!(&node.kind, PagePlanNodeKind::Events(access)
                    if access.operation==PageAccessKind::Search && access.index.is_some())));
                assert!(nodes.iter().all(
                    |node| !matches!(&node.kind, PagePlanNodeKind::Events(access)
                    if access.operation==PageAccessKind::Scan && access.index.is_none())
                ));
            }
            _ => panic!("complete actual comparison plan evidence is required"),
        }
    }
    println!("actual complete checkpoint comparison plans: corpus_rows=10000 comparison_calls=2 metadata_reads=2 item_reads=2 items_each=3000 window_reads=1 decoded_window_rows={} full_return_parity=1 shared_snapshot_claimed=0 timing_acceptance=0 evidence={evidence:?}", actual.event_ids_in_window.len());
    drop(fixture);
}
