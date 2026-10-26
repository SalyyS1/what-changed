//! Pinned projection for checkpoint lookup and ordered snapshot-item reads only.

use super::{
    CheckpointExplainOutcome, CheckpointPlanAccess, CheckpointPlanConstraint, CheckpointPlanNode,
    CheckpointPlanNodeKind,
};
use crate::storage::page_plan_capture::{PageAccessKind, PageExplainError};
use rusqlite::{types::Value, Connection};

fn access(operation: PageAccessKind, metadata: bool, raw: &str) -> Option<CheckpointPlanNodeKind> {
    let (index, covering, terms) = if raw.is_empty() && operation == PageAccessKind::Scan {
        (None, false, "")
    } else if metadata {
        (
            Some("INTEGER PRIMARY KEY"),
            false,
            raw.strip_prefix(" USING INTEGER PRIMARY KEY")?,
        )
    } else {
        let (raw, covering) = if let Some(raw) = raw.strip_prefix(" USING COVERING INDEX ") {
            (raw, true)
        } else {
            (raw.strip_prefix(" USING INDEX ")?, false)
        };
        let (name, terms) = raw
            .find(' ')
            .map_or((raw, ""), |position| raw.split_at(position));
        let index = [
            "idx_snapshots_checkpoint",
            "idx_snapshots_checkpoint_collector",
        ]
        .into_iter()
        .find(|index| *index == name)?;
        (Some(index), covering, terms)
    };
    let constraint = match (metadata, terms) {
        (_, "") => None,
        (true, " (rowid=?)") => Some(CheckpointPlanConstraint::RowIdEqual),
        (false, " (checkpoint_id=?)") => Some(CheckpointPlanConstraint::CheckpointIdEqual),
        _ => return None,
    };
    let access = CheckpointPlanAccess {
        operation,
        index,
        covering,
        constraint,
    };
    Some(if metadata {
        CheckpointPlanNodeKind::Metadata(access)
    } else {
        CheckpointPlanNodeKind::Items(access)
    })
}

fn parse(raw: &str) -> Option<CheckpointPlanNodeKind> {
    for (prefix, operation, metadata) in [
        ("SEARCH checkpoints", PageAccessKind::Search, true),
        ("SCAN checkpoints", PageAccessKind::Scan, true),
        ("SEARCH snapshots", PageAccessKind::Search, false),
        ("SCAN snapshots", PageAccessKind::Scan, false),
    ] {
        if let Some(raw) = raw.strip_prefix(prefix) {
            return access(operation, metadata, raw);
        }
    }
    if raw == "USE TEMP B-TREE FOR ORDER BY" {
        return Some(CheckpointPlanNodeKind::OrderByTempBTree { last_terms: None });
    }
    if raw == "USE TEMP B-TREE FOR RIGHT PART OF ORDER BY" {
        return Some(CheckpointPlanNodeKind::OrderByRightPartTempBTree);
    }
    if raw == "USE TEMP B-TREE FOR LAST TERM OF ORDER BY" {
        return Some(CheckpointPlanNodeKind::OrderByTempBTree {
            last_terms: Some(1),
        });
    }
    raw.strip_prefix("USE TEMP B-TREE FOR LAST ")
        .and_then(|raw| raw.strip_suffix(" TERMS OF ORDER BY"))
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|terms| (1..=3).contains(terms))
        .map(|terms| CheckpointPlanNodeKind::OrderByTempBTree {
            last_terms: Some(terms),
        })
}

fn rows(
    conn: &Connection,
    sql: &str,
    parameters: &[Value; 1],
) -> Result<Vec<CheckpointPlanNode>, PageExplainError> {
    let mut statement = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .map_err(|_| PageExplainError::Sqlite)?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(parameters.iter()))
        .map_err(|_| PageExplainError::Sqlite)?;
    let mut plan = Vec::new();
    while let Some(row) = rows.next().map_err(|_| PageExplainError::Sqlite)? {
        let id = row.get(0).map_err(|_| PageExplainError::Sqlite)?;
        let parent = row.get(1).map_err(|_| PageExplainError::Sqlite)?;
        let raw: String = row.get(3).map_err(|_| PageExplainError::Sqlite)?;
        let kind = parse(&raw).ok_or(PageExplainError::UnsupportedVocabulary)?;
        plan.push(CheckpointPlanNode { id, parent, kind });
    }
    // Any unknown row invalidates the whole plan: never silently truncate it.
    Ok(plan)
}

pub(super) fn explain(
    conn: &Connection,
    sql: &str,
    parameters: &[Value; 1],
) -> CheckpointExplainOutcome {
    match rows(conn, sql, parameters) {
        Ok(nodes) => CheckpointExplainOutcome::Complete(nodes),
        Err(error) => CheckpointExplainOutcome::Failed(error),
    }
}

#[cfg(test)]
#[path = "checkpoint_compare_plan_explain_tests.rs"]
mod tests;
