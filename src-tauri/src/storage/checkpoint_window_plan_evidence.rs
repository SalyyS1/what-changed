//! Parameter-free evidence for just the checkpoint event-ID window statement.

use super::super::page_plan_capture::PageExplainOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointWindowCaptureError {
    Nested,
    Missing,
    Multiple,
    BindingMismatch,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CheckpointWindowPlanEvidence {
    pub invocations: usize,
    pub prepared: bool,
    pub bindings: Option<usize>,
    pub statement_parameters: Option<usize>,
    pub explain: PageExplainOutcome,
    pub query_started: bool,
    /// True only after the entire bounded ID collection succeeds, including empty.
    pub fully_decoded: bool,
    /// None for any incomplete read or decode; zero for a successful empty window.
    pub decoded_rows: Option<usize>,
}

pub struct CapturedCheckpointWindow<R> {
    pub value: R,
    pub evidence: Result<CheckpointWindowPlanEvidence, CheckpointWindowCaptureError>,
}
