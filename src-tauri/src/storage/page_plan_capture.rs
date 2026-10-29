//! Explicit thread-scoped evidence for one actually executed page invocation.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use rusqlite::{types::Value, Connection};

use crate::domain::EventQuery;

#[path = "page_plan_evidence.rs"]
mod page_plan_evidence;
pub use page_plan_evidence::*;

#[path = "page_plan_explain.rs"]
mod page_plan_explain;

/// Reuse the pinned privacy projection for the fixed checkpoint event window.
/// This is crate-private, not an arbitrary public query-capture API.
pub(crate) fn explain_checkpoint_window_statement(
    conn: &Connection,
    sql: &str,
    params: &[Value; 3],
) -> PageExplainOutcome {
    page_plan_explain::explain(conn, sql, params)
}

struct State {
    expected: PagePlanVariant,
    invocations: usize,
    evidence: Option<PageExecutionEvidence>,
    error: Option<PageCaptureError>,
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

/// No environment switch or worker-thread inheritance. Nested scopes do not
/// run their closure or disturb the outer scope. Every evidence failure is
/// independent of the unchanged operation result; unwinding clears the scope.
pub fn capture_page_plan<R>(
    expected: PagePlanVariant,
    operation: impl FnOnce() -> R,
) -> Result<CapturedPage<R>, PageCaptureError> {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err(PageCaptureError::Nested);
        }
        *slot = Some(State {
            expected,
            invocations: 0,
            evidence: None,
            error: None,
        });
        Ok(())
    })?;
    let scope = Scope(PhantomData);
    let value = operation();
    let evidence = CAPTURE.with(|slot| {
        let state = slot.borrow_mut().take().ok_or(PageCaptureError::Missing)?;
        if let Some(error) = state.error {
            return Err(error);
        }
        state.evidence.ok_or(PageCaptureError::Missing)
    });
    drop(scope);
    Ok(CapturedPage { value, evidence })
}

pub(crate) struct ExecutionCapture(PhantomData<Rc<()>>);

pub(crate) fn begin(query: &EventQuery) -> Option<ExecutionCapture> {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let state = slot.as_mut()?; // Inactive: stop before classification/allocation.
        state.invocations += 1;
        if state.invocations != 1 {
            state.error = Some(PageCaptureError::Multiple);
            return None;
        }
        let variant = PagePlanVariant::for_query(query);
        if variant != state.expected {
            state.error = Some(PageCaptureError::VariantMismatch);
            return None;
        }
        state.evidence = Some(PageExecutionEvidence {
            variant,
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

fn update(operation: impl FnOnce(&mut PageExecutionEvidence)) -> bool {
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
        params: &[Value],
        parameter_count: usize,
    ) {
        if !update(|evidence| {
            evidence.prepared = true;
            evidence.bindings = Some(params.len());
            evidence.statement_parameters = Some(parameter_count);
        }) {
            return;
        }
        if parameter_count != params.len() {
            CAPTURE.with(|slot| {
                if let Some(state) = &mut *slot.borrow_mut() {
                    state.error = Some(PageCaptureError::BindingMismatch);
                }
            });
            return;
        }
        // Neither TLS borrow nor EXPLAIN statement/rows survives SQLite work
        // into the real query. Raw SQL, params, details and errors are not retained.
        let explained = page_plan_explain::explain(conn, sql, params);
        update(|evidence| evidence.explain = explained);
    }

    pub(crate) fn started(&self) {
        update(|evidence| evidence.query_started = true);
    }
    pub(crate) fn completed(&self, decoded_rows: usize) {
        update(|evidence| {
            evidence.fully_decoded = true;
            evidence.decoded_rows = Some(decoded_rows);
        });
    }
}

#[cfg(test)]
#[path = "page_capture_test_fixture.rs"]
pub(crate) mod test_fixture;
#[cfg(test)]
#[path = "page_plan_capture_tests.rs"]
mod tests;
