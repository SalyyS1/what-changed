//! Successful probe counts and the first bound delta plan for one scope.

use std::marker::PhantomData;
use std::rc::Rc;

use rusqlite::Connection;

use super::super::recurrence_rolling_counts::SortedRecurrenceCountKind;
use super::super::{sorted_recurrence_range_params, PriorOccurrenceQuery};
use super::{explain, SortedRecurrenceBoundPlan, SortedRecurrenceCaptureError, State, CAPTURE};

#[derive(Default)]
pub(crate) struct ExecutionCapture {
    completed_full: usize,
    completed_delta: usize,
    completed_inputs: usize,
    completed_rolling: usize,
    identical_reuses: usize,
    delta_explained: bool,
    _thread_bound: PhantomData<Rc<()>>,
}

impl ExecutionCapture {
    /// Preserve the unchanged full-count loop's single completion hook.
    pub(crate) fn completed_one(&mut self) {
        self.completed_sql(SortedRecurrenceCountKind::Full);
        self.resolved_full();
    }

    pub(crate) fn completed_sql(&mut self, kind: SortedRecurrenceCountKind) {
        match kind {
            SortedRecurrenceCountKind::Full => self.completed_full += 1,
            SortedRecurrenceCountKind::Delta => self.completed_delta += 1,
        }
    }

    pub(crate) fn resolved_full(&mut self) {
        self.completed_inputs += 1;
    }

    pub(crate) fn resolved_rolling(&mut self) {
        self.completed_inputs += 1;
        self.completed_rolling += 1;
    }

    pub(crate) fn resolved_reuse(&mut self) {
        self.completed_inputs += 1;
        self.identical_reuses += 1;
    }

    pub(crate) fn explain_delta_once(
        &mut self,
        conn: &Connection,
        sql: &str,
        query: &PriorOccurrenceQuery,
        since: i64,
        before: i64,
    ) {
        if self.delta_explained {
            return;
        }
        self.delta_explained = true;
        // Borrow actual delta bindings only for EXPLAIN. No TLS borrow crosses
        // SQLite or callbacks; never retain bounds, identities or raw errors.
        let outcome = explain(
            conn,
            sql,
            sorted_recurrence_range_params(query, since, before),
        )
        .map(|details| SortedRecurrenceBoundPlan {
            kind: SortedRecurrenceCountKind::Delta,
            details,
        })
        .map_err(|_| SortedRecurrenceCaptureError::Explain);
        CAPTURE.with(|slot| {
            if let Some(state) = &mut *slot.borrow_mut() {
                if state.invocations == 1 {
                    match outcome {
                        Ok(delta) => {
                            if let Some(Ok(plan)) = &mut state.outcome {
                                plan.delta_plan = Some(delta);
                            }
                        }
                        Err(error) => state.outcome = Some(Err(error)),
                    }
                }
            }
        });
    }
}

impl Drop for ExecutionCapture {
    fn drop(&mut self) {
        CAPTURE.with(|slot| {
            if let Some(State {
                outcome: Some(Ok(plan)),
                ..
            }) = &mut *slot.borrow_mut()
            {
                plan.completed_full_executions = self.completed_full;
                plan.completed_delta_executions = self.completed_delta;
                plan.completed_executions = self.completed_full + self.completed_delta;
                plan.completed_input_queries = self.completed_inputs;
                plan.completed_rolling_queries = self.completed_rolling;
                plan.identical_reuses = self.identical_reuses;
            }
        });
    }
}
