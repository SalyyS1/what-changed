//! Scoped plan evidence for one executed sorted recurrence invocation.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use rusqlite::Connection;

use super::recurrence_rolling_counts::SortedRecurrenceCountKind;
use super::{sorted_recurrence_params, PriorOccurrenceQuery};

#[path = "recurrence_execution_capture.rs"]
mod recurrence_execution_capture;
pub(super) use recurrence_execution_capture::ExecutionCapture;

#[path = "recurrence_window_statistics.rs"]
mod recurrence_window_statistics;
pub use recurrence_window_statistics::SortedRecurrenceWindowStatistics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortedRecurrenceCaptureError {
    Nested,
    Missing,
    Multiple,
    Explain,
}

#[derive(Debug)]
pub struct SortedRecurrenceBoundPlan {
    pub kind: SortedRecurrenceCountKind,
    pub details: Vec<String>,
}

#[derive(Debug)]
pub struct SortedRecurrencePlan {
    pub input_queries: usize,
    /// Successful SQL executions, including full and delta probes.
    pub completed_executions: usize,
    pub completed_full_executions: usize,
    pub completed_delta_executions: usize,
    pub completed_input_queries: usize,
    pub completed_rolling_queries: usize,
    pub identical_reuses: usize,
    pub window_statistics: SortedRecurrenceWindowStatistics,
    pub full_plan_kind: SortedRecurrenceCountKind,
    /// The first full query retains the existing details surface.
    pub details: Vec<String>,
    pub delta_plan: Option<SortedRecurrenceBoundPlan>,
}

pub struct CapturedRecurrence<R> {
    pub value: R,
    pub plan: Result<SortedRecurrencePlan, SortedRecurrenceCaptureError>,
}

#[derive(Default)]
struct State {
    invocations: usize,
    outcome: Option<Result<SortedRecurrencePlan, SortedRecurrenceCaptureError>>,
}

thread_local! {
    static CAPTURE: RefCell<Option<State>> = const { RefCell::new(None) };
}

struct Scope(PhantomData<Rc<()>>);

impl Drop for Scope {
    fn drop(&mut self) {
        CAPTURE.with(|state| *state.borrow_mut() = None);
    }
}

/// Explain the first full and, when executed, delta ranges on this thread.
/// The operation's result is independent of the owned, parameter-free evidence.
/// Nested scopes are rejected without changing the outer capture. State is
/// cleared on normal return and unwinding; no capture runs without this scope.
pub fn capture_sorted_recurrence_plan<R>(
    operation: impl FnOnce() -> R,
) -> Result<CapturedRecurrence<R>, SortedRecurrenceCaptureError> {
    CAPTURE.with(|state| {
        let mut state = state.borrow_mut();
        if state.is_some() {
            return Err(SortedRecurrenceCaptureError::Nested);
        }
        *state = Some(State::default());
        Ok(())
    })?;
    let scope = Scope(PhantomData);
    let value = operation();
    let plan = CAPTURE.with(|state| {
        state
            .borrow_mut()
            .take()
            .and_then(|state| state.outcome)
            .unwrap_or(Err(SortedRecurrenceCaptureError::Missing))
    });
    drop(scope);
    Ok(CapturedRecurrence { value, plan })
}

pub(super) fn begin(
    conn: &Connection,
    sql: &str,
    queries: &[PriorOccurrenceQuery],
    order: &[usize],
) -> Option<ExecutionCapture> {
    CAPTURE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        state.invocations += 1;
        if state.invocations != 1 {
            state.outcome = Some(Err(SortedRecurrenceCaptureError::Multiple));
            return None;
        }
        Some(())
    })?;
    // Claim the explicit scope before walking borrowed inputs. The first
    // sorted input always takes a full COUNT in both execution paths.
    let window_statistics = recurrence_window_statistics::summarize(queries, order);
    let query = &queries[*order.first()?];
    // No TLS borrow crosses SQLite work, including its callbacks.
    let outcome = explain(conn, sql, sorted_recurrence_params(query))
        .map(|details| SortedRecurrencePlan {
            input_queries: window_statistics.input_queries,
            completed_executions: 0,
            completed_full_executions: 0,
            completed_delta_executions: 0,
            completed_input_queries: 0,
            completed_rolling_queries: 0,
            identical_reuses: 0,
            window_statistics,
            full_plan_kind: SortedRecurrenceCountKind::Full,
            details,
            delta_plan: None,
        })
        // Raw SQLite errors can include SQL or paths. Retain only the category.
        .map_err(|_| SortedRecurrenceCaptureError::Explain);
    CAPTURE.with(|state| {
        if let Some(state) = &mut *state.borrow_mut() {
            if state.invocations == 1 {
                state.outcome = Some(outcome);
            }
        }
    });
    Some(ExecutionCapture::default())
}

fn explain(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> rusqlite::Result<Vec<String>> {
    let mut statement = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
    let rows = statement.query_map(params, |row| row.get(3))?;
    rows.collect()
}

#[cfg(test)]
#[path = "recurrence_plan_capture_tests.rs"]
mod tests;
