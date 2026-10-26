//! A pinned, page-specific projection, not a general SQLite EQP parser.

use rusqlite::{types::Value, Connection};

use super::page_plan_evidence::{
    PageAccessKind, PageConstraintKind, PageExplainError, PageExplainOutcome, PagePlanAccess,
    PagePlanConstraint, PagePlanNode, PagePlanNodeKind,
};

// Only schema-owned names are retained. An externally named index must not
// turn a structural diagnostic into an identity/secret logging surface.
const INDEXES: [&str; 10] = [
    "idx_events_timestamp",
    "idx_events_category_timestamp",
    "idx_events_entity_timestamp",
    "idx_events_event_type_timestamp",
    "idx_events_importance_timestamp",
    "idx_events_analysis_window",
    "idx_events_recurrence",
    "idx_events_timestamp_importance",
    "idx_events_process_started_path",
    "idx_events_process_started_entity_path",
];

fn column(raw: &str) -> Option<&'static str> {
    [
        "timestamp",
        "category",
        "entity_id",
        "importance",
        "event_type",
        "id",
        "rowid",
    ]
    .into_iter()
    .find(|column| *column == raw)
}

fn constraint(raw: &str) -> Option<PagePlanConstraint> {
    if let Some(raw) = raw
        .strip_prefix("ANY(")
        .and_then(|raw| raw.strip_suffix(')'))
    {
        return Some(PagePlanConstraint {
            column: column(raw)?,
            kind: PageConstraintKind::Any,
        });
    }
    for (suffix, kind) in [
        ("=?", PageConstraintKind::Equal),
        (">?", PageConstraintKind::Lower),
        ("<?", PageConstraintKind::Upper),
    ] {
        if let Some(raw) = raw.strip_suffix(suffix) {
            return Some(PagePlanConstraint {
                column: column(raw)?,
                kind,
            });
        }
    }
    None
}

fn constraints(raw: &str) -> Option<Vec<PagePlanConstraint>> {
    if raw.is_empty() {
        return Some(Vec::new());
    }
    let inner = raw.strip_prefix(" (")?.strip_suffix(')')?;
    inner.split(" AND ").map(constraint).collect()
}

fn access(operation: PageAccessKind, raw: &str) -> Option<PagePlanNodeKind> {
    let (index, covering, terms) = if raw.is_empty() && operation == PageAccessKind::Scan {
        (None, false, "")
    } else if let Some(terms) = raw.strip_prefix(" USING INTEGER PRIMARY KEY") {
        (Some("INTEGER PRIMARY KEY"), false, terms)
    } else {
        let (raw, covering) = if let Some(raw) = raw.strip_prefix(" USING COVERING INDEX ") {
            (raw, true)
        } else {
            (raw.strip_prefix(" USING INDEX ")?, false)
        };
        let (name, terms) = raw
            .find(' ')
            .map_or((raw, ""), |position| raw.split_at(position));
        let index = INDEXES.into_iter().find(|index| *index == name)?;
        (Some(index), covering, terms)
    };
    Some(PagePlanNodeKind::Events(PagePlanAccess {
        operation,
        index,
        covering,
        constraints: constraints(terms)?,
    }))
}

fn parse(raw: &str) -> Option<PagePlanNodeKind> {
    if let Some(raw) = raw.strip_prefix("SEARCH events") {
        return access(PageAccessKind::Search, raw);
    }
    if let Some(raw) = raw.strip_prefix("SCAN events") {
        return access(PageAccessKind::Scan, raw);
    }
    if raw == "MULTI-INDEX OR" {
        return Some(PagePlanNodeKind::MultiIndexOr);
    }
    if let Some(ordinal) = raw
        .strip_prefix("INDEX ")
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|ordinal| (1..=2).contains(ordinal))
    {
        return Some(PagePlanNodeKind::IndexBranch { ordinal });
    }
    if raw == "USE TEMP B-TREE FOR ORDER BY" {
        return Some(PagePlanNodeKind::OrderByTempBTree { last_terms: None });
    }
    if raw == "USE TEMP B-TREE FOR LAST TERM OF ORDER BY" {
        return Some(PagePlanNodeKind::OrderByTempBTree {
            last_terms: Some(1),
        });
    }
    if let Some(last_terms) = raw
        .strip_prefix("USE TEMP B-TREE FOR LAST ")
        .and_then(|raw| raw.strip_suffix(" TERMS OF ORDER BY"))
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|terms| (1..=2).contains(terms))
    {
        return Some(PagePlanNodeKind::OrderByTempBTree {
            last_terms: Some(last_terms),
        });
    }
    None
}

fn explain_rows(
    conn: &Connection,
    sql: &str,
    params: &[Value],
) -> Result<Vec<PagePlanNode>, PageExplainError> {
    let mut statement = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .map_err(|_| PageExplainError::Sqlite)?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(params.iter()))
        .map_err(|_| PageExplainError::Sqlite)?;
    let mut plan = Vec::new();
    while let Some(row) = rows.next().map_err(|_| PageExplainError::Sqlite)? {
        let id = row.get(0).map_err(|_| PageExplainError::Sqlite)?;
        let parent = row.get(1).map_err(|_| PageExplainError::Sqlite)?;
        let raw: String = row.get(3).map_err(|_| PageExplainError::Sqlite)?;
        let kind = parse(&raw).ok_or(PageExplainError::UnsupportedVocabulary)?;
        plan.push(PagePlanNode { id, parent, kind });
    }
    // Unknown rows never yield a fabricated success or silently shortened tree.
    Ok(plan)
}

pub(super) fn explain(conn: &Connection, sql: &str, params: &[Value]) -> PageExplainOutcome {
    match explain_rows(conn, sql, params) {
        Ok(rows) => PageExplainOutcome::Complete(rows),
        Err(error) => PageExplainOutcome::Failed(error),
    }
}

#[cfg(test)]
#[path = "page_plan_explain_tests.rs"]
mod tests;
