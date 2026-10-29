//! Collapse noisy runs into one timeline item without changing pagination.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{Category, EventSummary, Importance};

/// Events in the same coarse group and minute become a cluster above this size.
pub const CLUSTER_THRESHOLD: usize = 4;

/// An expandable reference to a cluster's exact raw-row scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ClusterRef {
    pub key: String,
    pub category: Category,
    pub coarse_entity: String,
    pub bucket: i64,
    /// Oldest row in the run. The id breaks timestamp ties at a page edge.
    pub first_ts: i64,
    pub first_id: i64,
    /// Newest row in the run. The id breaks timestamp ties at a page edge.
    pub last_ts: i64,
    pub last_id: i64,
}

impl ClusterRef {
    pub fn parse(
        key: &str,
        first_ts: i64,
        first_id: i64,
        last_ts: i64,
        last_id: i64,
    ) -> Option<Self> {
        let (head, bucket) = key.rsplit_once(':')?;
        let (category, coarse_entity) = head.split_once(':')?;
        let parsed = Self {
            key: key.to_owned(),
            category: category.parse().ok()?,
            coarse_entity: coarse_entity.to_owned(),
            bucket: bucket.parse().ok()?,
            first_ts,
            first_id,
            last_ts,
            last_id,
        };
        parsed.is_consistent().then_some(parsed)
    }

    /// Whether all caller-controlled fields still describe the key and minute
    /// the backend originally emitted.
    pub fn is_consistent(&self) -> bool {
        if self.coarse_entity.is_empty() || self.coarse_entity.len() > 256 {
            return false;
        }
        let expected = format!(
            "{}:{}:{}",
            self.category.as_str(),
            self.coarse_entity,
            self.bucket
        );
        let bucket_start = self.bucket.saturating_mul(60_000);
        let bucket_end = bucket_start.saturating_add(59_999);
        self.key == expected
            && self.first_id > 0
            && self.last_id > 0
            && (self.first_ts, self.first_id) <= (self.last_ts, self.last_id)
            && self.first_ts >= bucket_start
            && self.last_ts <= bucket_end
    }
}

/// One timeline item returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineItem {
    /// One ordinary event.
    Event { event: EventSummary },
    /// More than four related events in one sixty-second bucket.
    Cluster {
        key: String,
        cluster: ClusterRef,
        label: String,
        count: usize,
        first_ts: i64,
        last_ts: i64,
        highest_importance: Importance,
        category: Category,
    },
}

/// Cluster consecutive rows while preserving their newest-first order.
pub fn cluster(rows: Vec<EventSummary>) -> Vec<TimelineItem> {
    let mut out = Vec::new();
    let mut run = Vec::new();
    let mut current: Option<(Category, String, i64)> = None;

    for row in rows {
        let key = group_key(&row);
        if current.as_ref().is_some_and(|seen| seen == &key) {
            run.push(row);
        } else {
            flush(&mut out, &mut run);
            current = Some(key);
            run.push(row);
        }
    }
    flush(&mut out, &mut run);
    out
}

fn group_key(row: &EventSummary) -> (Category, String, i64) {
    (
        row.category,
        coarse_entity(&row.entity_id),
        row.timestamp.div_euclid(60_000),
    )
}

fn coarse_entity(id: &str) -> String {
    id.split(':').take(2).collect::<Vec<_>>().join(":")
}

fn flush(out: &mut Vec<TimelineItem>, run: &mut Vec<EventSummary>) {
    if run.is_empty() {
        return;
    }
    if run.len() > CLUSTER_THRESHOLD {
        let Some(first) = run.first() else {
            return;
        };
        let Some(last) = run.last() else {
            return;
        };
        let highest_importance = run
            .iter()
            .map(|event| event.importance)
            .max()
            .unwrap_or(Importance::Informational);
        let category = first.category;
        let count = run.len();
        let coarse = coarse_entity(&first.entity_id);
        let bucket = first.timestamp.div_euclid(60_000);
        let first_ts = last.timestamp;
        let first_id = last.id;
        let last_ts = first.timestamp;
        let last_id = first.id;
        let key = format!("{}:{coarse}:{bucket}", category.as_str());
        let cluster = ClusterRef {
            key: key.clone(),
            category,
            coarse_entity: coarse,
            bucket,
            first_ts,
            first_id,
            last_ts,
            last_id,
        };
        out.push(TimelineItem::Cluster {
            key,
            cluster,
            label: cluster_label(category, count),
            count,
            first_ts,
            last_ts,
            highest_importance,
            category,
        });
        run.clear();
        return;
    }
    out.extend(run.drain(..).map(|event| TimelineItem::Event { event }));
}

fn cluster_label(category: Category, count: usize) -> String {
    let noun = match category {
        Category::Docker => "Docker-related changes",
        Category::Filesystem | Category::Configuration => "file changes",
        Category::Application => "application changes",
        Category::Network => "network changes",
        Category::Service => "service changes",
        _ => "related changes",
    };
    format!("{count} {noun}")
}
