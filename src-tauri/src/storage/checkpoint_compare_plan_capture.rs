//! Calling-thread evidence across actual statements, not a shared read snapshot.

use super::checkpoint_window_plan_capture::{
    capture_checkpoint_window_plan, CheckpointWindowCaptureError,
};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

#[path = "checkpoint_compare_plan_evidence.rs"]
mod evidence;
pub use evidence::*;
#[path = "checkpoint_compare_plan_execution.rs"]
mod execution;
#[path = "checkpoint_compare_plan_explain.rs"]
mod explain;
pub(crate) use execution::{begin, window_invoked};

thread_local! {
    static CAPTURE: RefCell<Option<CheckpointComparePlanEvidence>> = const { RefCell::new(None) };
}

struct Scope(PhantomData<Rc<()>>);
impl Drop for Scope {
    fn drop(&mut self) {
        CAPTURE.with(|slot| *slot.borrow_mut() = None);
    }
}

/// Capture the concrete owner reads without changing operation errors or
/// short-circuits. Complete Some comparisons have five reads; partial/error
/// operations retain their observed prefix. Neither TLS scope spans SQLite
/// with a live borrow. Existing window capture supplies its one actual EQP.
pub fn capture_checkpoint_compare_plan<R>(
    operation: impl FnOnce() -> R,
) -> Result<CapturedCheckpointCompare<R>, CheckpointCompareCaptureError> {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err(CheckpointCompareCaptureError::Nested);
        }
        *slot = Some(CheckpointComparePlanEvidence {
            invocations: 0,
            metadata_invocations: 0,
            items_invocations: 0,
            window_invocations: 0,
            reads: Vec::with_capacity(5),
            failure: None,
        });
        Ok(())
    })?;
    let scope = Scope(PhantomData);
    // Opposite nesting with a public window scope rejects before operation,
    // and the local compare guard clears on this early-return path too.
    let captured = capture_checkpoint_window_plan(operation)
        .map_err(|_| CheckpointCompareCaptureError::Nested)?;
    let mut evidence = CAPTURE
        .with(|slot| slot.borrow_mut().take())
        .ok_or(CheckpointCompareCaptureError::Missing)?;
    match captured.evidence {
        Ok(window) => {
            if let Some(read) = evidence
                .reads
                .iter_mut()
                .find(|read| read.operation == CheckpointCompareOperation::EventWindow)
            {
                read.prepared = window.prepared;
                read.bindings = window.bindings;
                read.statement_parameters = window.statement_parameters;
                read.plan = CheckpointComparePlan::EventWindow(window.explain);
                read.query_started = window.query_started;
                read.fully_decoded = window.fully_decoded;
                read.decoded_rows = window.decoded_rows;
            } else {
                evidence
                    .failure
                    .get_or_insert(CheckpointCompareCaptureError::Missing);
            }
        }
        Err(CheckpointWindowCaptureError::Missing) if evidence.window_invocations == 0 => {}
        Err(error) => {
            evidence
                .failure
                .get_or_insert(CheckpointCompareCaptureError::Window(error));
        }
    }
    if evidence.invocations == 0 {
        evidence
            .failure
            .get_or_insert(CheckpointCompareCaptureError::Missing);
    }
    drop(scope);
    Ok(CapturedCheckpointCompare {
        value: captured.value,
        evidence,
    })
}

#[cfg(test)]
#[path = "checkpoint_compare_plan_owner_tests.rs"]
mod owner_tests;
#[cfg(test)]
#[path = "checkpoint_compare_plan_service_tests.rs"]
mod service_tests;
#[cfg(test)]
#[path = "checkpoint_compare_plan_test_fixture.rs"]
mod test_fixture;
#[cfg(test)]
#[path = "checkpoint_compare_plan_capture_tests.rs"]
mod tests;
