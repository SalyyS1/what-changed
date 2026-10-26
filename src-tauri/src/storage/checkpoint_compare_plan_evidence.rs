//! Bounded, parameter-free evidence for the five concrete comparison reads.

use super::super::checkpoint_window_plan_capture::CheckpointWindowCaptureError;
use super::super::page_plan_capture::{PageAccessKind, PageExplainError, PageExplainOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointCompareOperation {
    Metadata,
    Items,
    EventWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointCompareCaptureError {
    Nested,
    Missing,
    Multiple,
    BindingMismatch,
    Window(CheckpointWindowCaptureError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointPlanConstraint {
    RowIdEqual,
    CheckpointIdEqual,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CheckpointPlanAccess {
    pub operation: PageAccessKind,
    pub index: Option<&'static str>,
    pub covering: bool,
    pub constraint: Option<CheckpointPlanConstraint>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckpointPlanNodeKind {
    Metadata(CheckpointPlanAccess),
    Items(CheckpointPlanAccess),
    OrderByTempBTree { last_terms: Option<u32> },
    OrderByRightPartTempBTree,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CheckpointPlanNode {
    pub id: i64,
    pub parent: i64,
    pub kind: CheckpointPlanNodeKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckpointExplainOutcome {
    NotAttempted,
    Complete(Vec<CheckpointPlanNode>),
    Failed(PageExplainError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckpointComparePlan {
    Checkpoint(CheckpointExplainOutcome),
    EventWindow(PageExplainOutcome),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CheckpointCompareReadEvidence {
    pub operation: CheckpointCompareOperation,
    pub prepared: bool,
    pub bindings: Option<usize>,
    pub statement_parameters: Option<usize>,
    pub plan: CheckpointComparePlan,
    /// Metadata: a row was stepped before raw decoding, or optional absence
    /// succeeded. Items/window: the original query_map succeeded.
    pub query_started: bool,
    /// Includes domain/kind/manifests and state JSON, not merely SQLite rows.
    pub fully_decoded: bool,
    pub decoded_rows: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CheckpointComparePlanEvidence {
    pub invocations: usize,
    pub metadata_invocations: usize,
    pub items_invocations: usize,
    pub window_invocations: usize,
    /// Actual invocation order, bounded to the first five reads even on misuse.
    pub reads: Vec<CheckpointCompareReadEvidence>,
    /// Partial reads remain available on any diagnostic or operation failure.
    pub failure: Option<CheckpointCompareCaptureError>,
}

pub struct CapturedCheckpointCompare<R> {
    pub value: R,
    pub evidence: CheckpointComparePlanEvidence,
}
