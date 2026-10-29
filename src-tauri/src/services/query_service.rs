//! Time ranges, bounded paging and event context assembled from repositories.

use crate::domain::{EventQuery, SystemEvent, TimeRange, MAX_PAGE_SIZE, MAX_RANGE_MS};
use crate::dto::EventDetailsDto;
use crate::error::{AppError, AppResult};
use crate::services::cluster::{cluster, ClusterRef, TimelineItem};
use crate::storage::{event_repository, Db};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct TimelinePage {
    pub items: Vec<TimelineItem>,
    pub next: Option<crate::domain::Cursor>,
}

/// Resolve a range without storage context. Stored timestamps are UTC; the
/// restart range uses local midnight as the deterministic pre-Phase-13 fallback.
pub fn resolve_range(range: &TimeRange, now: i64) -> AppResult<(i64, i64)> {
    resolve_range_with_midnight(range, now, |days_ago| local_midnight(now, days_ago))
}

/// Resolve a range against persisted evidence when the range needs it.
pub fn resolve_range_for_db(db: &Db, range: &TimeRange, now: i64) -> AppResult<(i64, i64)> {
    if matches!(range, TimeRange::SinceLastRestart) {
        let boot = db.read(|conn| {
            event_repository::latest_timestamp(
                conn,
                crate::domain::Category::System,
                crate::domain::EventType::Started,
                &crate::domain::ids::system(),
                "eventlog",
            )
        })?;
        // Event 1074 records a restart/shutdown request, not proof that the
        // current boot ended; it may be cancelled or delayed. A later real
        // boot always has its own 6005, so the latest 6005 is the boundary.
        if let Some(boot) = boot {
            return Ok((boot.min(now), now));
        }
    }
    resolve_range(range, now)
}

fn resolve_range_with_midnight(
    range: &TimeRange,
    now: i64,
    mut midnight_for: impl FnMut(u64) -> Option<i64>,
) -> AppResult<(i64, i64)> {
    let day = 24 * 60 * 60 * 1_000i64;
    let midnight = midnight_for(0).unwrap_or_else(|| now.div_euclid(day) * day);
    let bounds = match range {
        TimeRange::Today => (midnight, now),
        TimeRange::Yesterday => (
            midnight_for(1).unwrap_or_else(|| midnight.saturating_sub(day)),
            midnight.saturating_sub(1),
        ),
        TimeRange::Last7Days => (
            midnight_for(6).unwrap_or_else(|| midnight.saturating_sub(6 * day)),
            now,
        ),
        TimeRange::SinceLastRestart => (midnight, now),
        TimeRange::Custom { from, to } => {
            if *from == i64::MIN && *to == i64::MAX {
                return Ok((*from, *to));
            }
            if *from > *to {
                return Err(AppError::InvalidInput {
                    what: "range",
                    reason: "from must not be after to".to_owned(),
                });
            }
            if to.saturating_sub(*from) > MAX_RANGE_MS {
                return Err(AppError::InvalidInput {
                    what: "range",
                    reason: "custom range is too wide".to_owned(),
                });
            }
            (*from, *to)
        }
    };
    Ok(bounds)
}

/// Start of the local calendar day `days_ago` before `now`.
///
/// Resolving each date independently is important around daylight-saving
/// transitions: "Yesterday" can be 23 or 25 hours and is not `now - 24h`.
fn local_midnight(now: i64, days_ago: u64) -> Option<i64> {
    use chrono::{Local, LocalResult, TimeZone};

    let instant = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now)?;
    let date = instant
        .with_timezone(&Local)
        .date_naive()
        .checked_sub_days(chrono::Days::new(days_ago))?;
    let midnight = date.and_hms_opt(0, 0, 0)?;
    match Local.from_local_datetime(&midnight) {
        LocalResult::Single(value) => Some(value.timestamp_millis()),
        LocalResult::Ambiguous(first, second) => {
            Some(first.timestamp_millis().min(second.timestamp_millis()))
        }
        LocalResult::None => None,
    }
}

pub fn page(db: &Db, mut query: EventQuery) -> AppResult<TimelinePage> {
    let (from, to) = resolve_range_for_db(db, &query.range, crate::domain::now_ms())?;
    query.from = from;
    query.to = to;
    query.limit = query.effective_limit().min(MAX_PAGE_SIZE);
    let raw = db.read(|conn| event_repository::page(conn, &query))?;
    Ok(TimelinePage {
        next: raw.next,
        items: cluster(raw.items),
    })
}

pub fn expand_cluster(
    db: &Db,
    cluster: &ClusterRef,
) -> AppResult<Vec<crate::domain::EventSummary>> {
    if !cluster.is_consistent() {
        return Err(AppError::InvalidInput {
            what: "cluster",
            reason: "cluster reference is invalid".to_owned(),
        });
    }
    db.read(|conn| {
        let query = event_repository::ClusterMembersQuery {
            category: cluster.category.as_str(),
            coarse_entity: &cluster.coarse_entity,
            first: (cluster.first_ts, cluster.first_id),
            last: (cluster.last_ts, cluster.last_id),
            limit: MAX_PAGE_SIZE,
        };
        event_repository::cluster_members(conn, &query)
    })
}

pub fn details(db: &Db, event_id: i64) -> AppResult<EventDetailsDto> {
    let mut event = db
        .read(|conn| event_repository::by_id(conn, event_id))?
        .ok_or_else(|| AppError::InvalidInput {
            what: "event id",
            reason: "that event no longer exists".to_owned(),
        })?;
    crate::analysis::evidence::corroborate_stored_event(db, &mut event);
    details_for(db, event)
}
fn details_for(db: &Db, event: SystemEvent) -> AppResult<EventDetailsDto> {
    let mut entity_ids = vec![event.event.entity_id.clone()];
    for id in &event.event.related_entity_ids {
        if !entity_ids.contains(id) {
            entity_ids.push(id.clone());
        }
    }
    let timestamp = event.event.timestamp;
    let related = db.read(|conn| {
        event_repository::related(
            conn,
            event.id,
            &entity_ids,
            timestamp.saturating_sub(5 * 60_000),
            timestamp.saturating_add(5 * 60_000),
            50,
        )
    })?;
    let prior_occurrences = db.read(|conn| {
        event_repository::prior_occurrences(conn, &event.event.signature(), i64::MIN, timestamp)
    })?;
    Ok(EventDetailsDto {
        event,
        related,
        prior_occurrences,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn calendar_ranges_resolve_each_midnight_independently() {
        let hour = 60 * 60 * 1_000i64;
        let today = 10_000_000_000;
        let yesterday = today - 23 * hour;
        let six_days_ago = yesterday - 5 * 24 * hour;
        let now = today + 12 * hour;
        let midnight = |days_ago| match days_ago {
            0 => Some(today),
            1 => Some(yesterday),
            6 => Some(six_days_ago),
            _ => None,
        };

        assert_eq!(
            resolve_range_with_midnight(&TimeRange::Yesterday, now, midnight).unwrap(),
            (yesterday, today - 1)
        );
        assert_eq!(
            today - yesterday,
            23 * hour,
            "the fixture is a DST-short day"
        );
        assert_eq!(
            resolve_range_with_midnight(&TimeRange::Last7Days, now, midnight).unwrap(),
            (six_days_ago, now)
        );
    }

    #[test]
    fn custom_range_accepts_the_limit_and_rejects_backwards_or_wider_ranges() {
        let now = 1_000_000;
        assert_eq!(
            resolve_range(&TimeRange::Custom { from: 10, to: 20 }, now).unwrap(),
            (10, 20)
        );
        assert!(resolve_range(&TimeRange::Custom { from: 20, to: 10 }, now).is_err());
        assert!(resolve_range(
            &TimeRange::Custom {
                from: 0,
                to: MAX_RANGE_MS + 1,
            },
            now,
        )
        .is_err());
    }
}
