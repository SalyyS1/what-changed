//! Parameter-free data contracts shared by page capture and its pinned EQP projection.

use crate::domain::{EventQuery, Importance};

/// Only SQL structure, never category words, entity identities or bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePlanVariant {
    /// Categories=1, importance=2, entity=4, cursor=8.
    pub mask: u8,
    pub category_arity: usize,
    pub importance_arity: usize,
}

impl PagePlanVariant {
    pub fn for_query(query: &EventQuery) -> Self {
        Self {
            mask: u8::from(!query.categories.is_empty())
                | (u8::from(query.importance_min.is_some()) << 1)
                | (u8::from(query.entity_id.is_some()) << 2)
                | (u8::from(query.cursor.is_some()) << 3),
            category_arity: query.categories.len(),
            importance_arity: query.importance_min.map_or(0, |minimum| {
                Importance::ALL
                    .iter()
                    .filter(|importance| **importance >= minimum)
                    .count()
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCaptureError {
    Nested,
    Missing,
    Multiple,
    VariantMismatch,
    BindingMismatch,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PageExecutionEvidence {
    pub variant: PagePlanVariant,
    pub invocations: usize,
    pub prepared: bool,
    pub bindings: Option<usize>,
    pub statement_parameters: Option<usize>,
    pub explain: PageExplainOutcome,
    pub query_started: bool,
    /// True only after cursor exhaustion and every summary, including lookahead, decoded.
    pub fully_decoded: bool,
    /// None on incomplete consumption; zero on a successful empty page.
    pub decoded_rows: Option<usize>,
}

pub struct CapturedPage<R> {
    pub value: R,
    pub evidence: Result<PageExecutionEvidence, PageCaptureError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageAccessKind {
    Search,
    Scan,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageConstraintKind {
    Equal,
    Lower,
    Upper,
    Any,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PagePlanConstraint {
    pub column: &'static str,
    pub kind: PageConstraintKind,
}
#[derive(Debug, PartialEq, Eq)]
pub struct PagePlanAccess {
    pub operation: PageAccessKind,
    /// Known migration identity, or the built-in INTEGER PRIMARY KEY.
    pub index: Option<&'static str>,
    pub covering: bool,
    pub constraints: Vec<PagePlanConstraint>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PagePlanNodeKind {
    Events(PagePlanAccess),
    MultiIndexOr,
    IndexBranch { ordinal: u32 },
    OrderByTempBTree { last_terms: Option<u32> },
}
#[derive(Debug, PartialEq, Eq)]
pub struct PagePlanNode {
    pub id: i64,
    pub parent: i64,
    pub kind: PagePlanNodeKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageExplainError {
    Sqlite,
    UnsupportedVocabulary,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PageExplainOutcome {
    NotAttempted,
    Complete(Vec<PagePlanNode>),
    Failed(PageExplainError),
}
