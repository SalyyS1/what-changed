//! Concrete owner tokens and actual-bound EQP; scoped lifecycle lives separately.

use super::{
    explain, CheckpointCompareCaptureError, CheckpointCompareOperation, CheckpointComparePlan,
    CheckpointCompareReadEvidence, CheckpointExplainOutcome, CAPTURE,
};
use crate::storage::page_plan_capture::PageExplainOutcome;
use rusqlite::{types::Value, Connection};
use std::{marker::PhantomData, rc::Rc};

pub(crate) struct ExecutionCapture {
    ordinal: usize,
    _thread: PhantomData<Rc<()>>,
}

pub(crate) fn begin(operation: CheckpointCompareOperation) -> Option<ExecutionCapture> {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let evidence = slot.as_mut()?; // Inactive: membership check only.
        evidence.invocations += 1;
        match operation {
            CheckpointCompareOperation::Metadata => evidence.metadata_invocations += 1,
            CheckpointCompareOperation::Items => evidence.items_invocations += 1,
            CheckpointCompareOperation::EventWindow => evidence.window_invocations += 1,
        }
        if evidence.reads.len() == 5 {
            evidence
                .failure
                .get_or_insert(CheckpointCompareCaptureError::Multiple);
            return None;
        }
        let ordinal = evidence.reads.len();
        evidence.reads.push(CheckpointCompareReadEvidence {
            operation,
            prepared: false,
            bindings: None,
            statement_parameters: None,
            plan: if operation == CheckpointCompareOperation::EventWindow {
                CheckpointComparePlan::EventWindow(PageExplainOutcome::NotAttempted)
            } else {
                CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::NotAttempted)
            },
            query_started: false,
            fully_decoded: false,
            decoded_rows: None,
        });
        Some(ExecutionCapture {
            ordinal,
            _thread: PhantomData,
        })
    })
}

pub(crate) fn window_invoked() {
    let _ = begin(CheckpointCompareOperation::EventWindow);
}

impl ExecutionCapture {
    fn update(&self, operation: impl FnOnce(&mut CheckpointCompareReadEvidence)) -> bool {
        CAPTURE.with(|slot| {
            if let Some(read) = slot
                .borrow_mut()
                .as_mut()
                .and_then(|evidence| evidence.reads.get_mut(self.ordinal))
            {
                operation(read);
                true
            } else {
                false
            }
        })
    }

    pub(crate) fn prepared(
        &self,
        conn: &Connection,
        sql: &str,
        parameters: &[i64; 1],
        count: usize,
    ) {
        if !self.update(|read| {
            read.prepared = true;
            read.bindings = Some(1);
            read.statement_parameters = Some(count);
        }) {
            return;
        }
        if count != 1 {
            CAPTURE.with(|slot| {
                if let Some(evidence) = slot.borrow_mut().as_mut() {
                    evidence
                        .failure
                        .get_or_insert(CheckpointCompareCaptureError::BindingMismatch);
                }
            });
            return;
        }
        // The original fixed array is also passed to the actual owner query.
        let values = [Value::Integer(parameters[0])];
        let plan = explain::explain(conn, sql, &values);
        self.update(|read| read.plan = CheckpointComparePlan::Checkpoint(plan));
    }

    pub(crate) fn started(&self) {
        self.update(|read| read.query_started = true);
    }
    pub(crate) fn completed(&self, rows: usize) {
        self.update(|read| {
            read.fully_decoded = true;
            read.decoded_rows = Some(rows);
        });
    }
}
