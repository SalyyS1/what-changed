use crate::app::monitoring::Monitoring;
use crate::domain::EventSummary;
use crate::dto::{CheckpointRef, DashboardCountsDto, DashboardSummaryDto, MonitoringDto};
use crate::error::AppResult;
use crate::storage::{checkpoint_repository, event_repository, Db};

pub fn summary(db: &Db, from: i64, to: i64) -> AppResult<DashboardSummaryDto> {
    summary_with_snapshot(db, from, to, Monitoring::running().snapshot())
}

pub fn summary_with_monitoring(
    db: &Db,
    from: i64,
    to: i64,
    monitoring: &Monitoring,
) -> AppResult<DashboardSummaryDto> {
    summary_with_snapshot(db, from, to, monitoring.snapshot())
}

fn summary_with_snapshot(
    db: &Db,
    from: i64,
    to: i64,
    monitoring: MonitoringDto,
) -> AppResult<DashboardSummaryDto> {
    db.read(|conn| {
        let counts = event_repository::counts_for_range(conn, from, to)?;
        let top_important = event_repository::top_important(conn, from, to, 3)?;
        let last_checkpoint = checkpoint_repository::latest(conn)?;
        Ok(build_summary(
            counts,
            top_important,
            monitoring,
            last_checkpoint,
        ))
    })
}

fn build_summary(
    counts: event_repository::ImportanceCounts,
    top_important: Vec<EventSummary>,
    monitoring: MonitoringDto,
    last_checkpoint: Option<CheckpointRef>,
) -> DashboardSummaryDto {
    let attention = counts.important + counts.critical;
    let status_line = if attention == 0 {
        "Everything looks normal.".to_owned()
    } else {
        format!("{attention} recent changes may need attention.")
    };
    let today = DashboardCountsDto {
        total: counts.total(),
        notable: counts.notable,
        important: counts.important,
        critical: counts.critical,
    };
    DashboardSummaryDto {
        total: today.total,
        notable: today.notable,
        important: today.important,
        critical: today.critical,
        today,
        status_line,
        top_important,
        monitoring,
        last_checkpoint,
    }
}
