//! Reading the history. Bodies land in phase 06; the shapes and the validation
//! are here so the typed surface is closed from the start.

use tauri::State;

use crate::app::validate;
use crate::app::AppState;
use crate::domain::{EventQuery, TimeRange};
use crate::dto::{CommandResult, DashboardSummaryDto, EventDetailsDto, SearchResultDto};
use crate::services::cluster::ClusterRef;
use crate::services::query_service::TimelinePage;
use crate::storage::search_index::{MAX_QUERY_CHARS, MAX_SEARCH_RESULTS};

/// The Home screen's summary for a window.
#[tauri::command]
pub async fn get_dashboard_summary(
    state: State<'_, AppState>,
    range: TimeRange,
) -> CommandResult<DashboardSummaryDto> {
    let _query = state.query_busy.enter();
    let (from, to) = crate::services::query_service::resolve_range_for_db(
        &state.db,
        &range,
        crate::domain::now_ms(),
    )?;
    Ok(crate::services::summary_service::summary_with_monitoring(
        &state.db,
        from,
        to,
        &state.monitoring,
    )?)
}

/// One page of the timeline, newest first.
///
/// The query is re-validated even though the frontend built it from a page
/// this backend returned: a page object that came back through the webview is
/// a value the page could have edited.
#[tauri::command]
pub async fn get_events(
    state: State<'_, AppState>,
    mut query: EventQuery,
) -> CommandResult<TimelinePage> {
    let _query = state.query_busy.enter();
    validate::time_range(query.from, query.to)?;
    clamp_event_limit(&mut query);
    Ok(crate::services::query_service::page(&state.db, query)?)
}

/// Expand exactly the members represented by a timeline cluster.
#[tauri::command]
pub async fn expand_cluster(
    state: State<'_, AppState>,
    cluster: ClusterRef,
) -> CommandResult<Vec<crate::domain::EventSummary>> {
    let _query = state.query_busy.enter();
    if !cluster.is_consistent() {
        return Err(crate::error::AppError::InvalidInput {
            what: "cluster",
            reason: "cluster reference is invalid".to_owned(),
        }
        .into());
    }
    Ok(crate::services::query_service::expand_cluster(
        &state.db, &cluster,
    )?)
}

/// One event with its context.
#[tauri::command]
pub async fn get_event_details(
    state: State<'_, AppState>,
    event_id: i64,
) -> CommandResult<EventDetailsDto> {
    let _query = state.query_busy.enter();
    validate::in_range("event id", event_id, 1, i64::MAX)?;
    Ok(crate::services::query_service::details(
        &state.db, event_id,
    )?)
}

/// Free-text search over the history.
#[tauri::command]
pub async fn search_events(
    state: State<'_, AppState>,
    query: String,
    limit: u16,
) -> CommandResult<Vec<SearchResultDto>> {
    let _query = state.query_busy.enter();
    let query = validate::text("search", &query, MAX_QUERY_CHARS)?;
    let limit = clamp_search_limit(limit);
    Ok(crate::services::search_service::search(
        &state.db, &query, limit,
    )?)
}

fn clamp_event_limit(query: &mut EventQuery) {
    query.limit = query.effective_limit();
}

fn clamp_search_limit(limit: u16) -> u16 {
    limit.clamp(1, MAX_SEARCH_RESULTS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MAX_PAGE_SIZE;

    #[test]
    fn event_and_search_limits_are_clamped_at_the_command_boundary() {
        let mut empty_page = EventQuery {
            limit: 0,
            ..EventQuery::default()
        };
        clamp_event_limit(&mut empty_page);
        assert_eq!(empty_page.limit, 1);

        let mut oversized_page = EventQuery {
            limit: u16::MAX,
            ..EventQuery::default()
        };
        clamp_event_limit(&mut oversized_page);
        assert_eq!(oversized_page.limit, MAX_PAGE_SIZE);

        assert_eq!(clamp_search_limit(0), 1);
        assert_eq!(clamp_search_limit(u16::MAX), MAX_SEARCH_RESULTS);
    }
}
