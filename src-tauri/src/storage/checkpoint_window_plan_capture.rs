//! Explicit thread-scoped capture of one actual checkpoint event-window read.

use super::page_plan_capture::{explain_checkpoint_window_statement, PageExplainOutcome};
use rusqlite::{types::Value, Connection};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

#[path = "checkpoint_window_plan_evidence.rs"]
mod evidence;
pub use evidence::{
    CapturedCheckpointWindow, CheckpointWindowCaptureError, CheckpointWindowPlanEvidence,
};

struct State {
    invocations: usize,
    evidence: Option<CheckpointWindowPlanEvidence>,
    error: Option<CheckpointWindowCaptureError>,
}

thread_local! {
    static CAPTURE: RefCell<Option<State>> = const { RefCell::new(None) };
}

struct Scope(PhantomData<Rc<()>>);
impl Drop for Scope {
    fn drop(&mut self) {
        CAPTURE.with(|slot| *slot.borrow_mut() = None);
    }
}

/// Nested scopes reject before their closure runs; other evidence failures
/// never replace the original operation value. Panic propagates and clears
/// the scope. No worker-thread inheritance, clocks or environment activation.
pub fn capture_checkpoint_window_plan<R>(
    operation: impl FnOnce() -> R,
) -> Result<CapturedCheckpointWindow<R>, CheckpointWindowCaptureError> {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err(CheckpointWindowCaptureError::Nested);
        }
        *slot = Some(State {
            invocations: 0,
            evidence: None,
            error: None,
        });
        Ok(())
    })?;
    let scope = Scope(PhantomData);
    let value = operation();
    let evidence = CAPTURE.with(|slot| {
        let state = slot
            .borrow_mut()
            .take()
            .ok_or(CheckpointWindowCaptureError::Missing)?;
        if let Some(error) = state.error {
            return Err(error);
        }
        state.evidence.ok_or(CheckpointWindowCaptureError::Missing)
    });
    drop(scope);
    Ok(CapturedCheckpointWindow { value, evidence })
}

pub(crate) struct ExecutionCapture(PhantomData<Rc<()>>);

pub(crate) fn begin() -> Option<ExecutionCapture> {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let state = slot.as_mut()?; // Inactive: membership check only.
        state.invocations += 1;
        if state.invocations != 1 {
            state.error = Some(CheckpointWindowCaptureError::Multiple);
            return None;
        }
        state.evidence = Some(CheckpointWindowPlanEvidence {
            invocations: 1,
            prepared: false,
            bindings: None,
            statement_parameters: None,
            explain: PageExplainOutcome::NotAttempted,
            query_started: false,
            fully_decoded: false,
            decoded_rows: None,
        });
        Some(ExecutionCapture(PhantomData))
    })
}

fn update(operation: impl FnOnce(&mut CheckpointWindowPlanEvidence)) -> bool {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(state) = slot
            .as_mut()
            .filter(|state| state.invocations == 1 && state.error.is_none())
        {
            if let Some(evidence) = &mut state.evidence {
                operation(evidence);
                return true;
            }
        }
        false
    })
}

impl ExecutionCapture {
    pub(crate) fn prepared(
        &self,
        conn: &Connection,
        sql: &str,
        parameters: (i64, i64, i64),
        parameter_count: usize,
    ) {
        if !update(|evidence| {
            evidence.prepared = true;
            evidence.bindings = Some(3);
            evidence.statement_parameters = Some(parameter_count);
        }) {
            return;
        }
        if parameter_count != 3 {
            CAPTURE.with(|slot| {
                if let Some(state) = &mut *slot.borrow_mut() {
                    state.error = Some(CheckpointWindowCaptureError::BindingMismatch);
                }
            });
            return;
        }
        // Convert ONLY after a valid explicit active claim. The original
        // query_map uses the owner's same normalized tuple values.
        let values = [
            Value::Integer(parameters.0),
            Value::Integer(parameters.1),
            Value::Integer(parameters.2),
        ];
        // No TLS borrow or diagnostic statement survives into the real query.
        let explained = explain_checkpoint_window_statement(conn, sql, &values);
        update(|evidence| evidence.explain = explained);
    }

    pub(crate) fn started(&self) {
        update(|evidence| evidence.query_started = true);
    }
    pub(crate) fn completed(&self, rows: usize) {
        update(|evidence| {
            evidence.fully_decoded = true;
            evidence.decoded_rows = Some(rows);
        });
    }
}

#[cfg(test)]
#[path = "checkpoint_window_owner_tests.rs"]
mod owner_tests;
#[cfg(test)]
#[path = "checkpoint_window_plan_capture_tests.rs"]
mod tests;
