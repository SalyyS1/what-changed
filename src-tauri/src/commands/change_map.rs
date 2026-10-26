//! Read-only Change Map command.

use std::sync::Arc;

use tauri::State;

use crate::analysis::change_map::{self, ChangeMapData, ChangeMapRequest};
use crate::app::{validate, AppState};
use crate::domain::{Category, InvestigationWindow};
use crate::dto::CommandResult;
use crate::error::AppError;
use crate::services::investigation_service::InvestigationService;
use crate::storage::event_repository;

#[tauri::command]
pub async fn get_change_map(
    state: State<'_, AppState>,
    request: ChangeMapRequest,
) -> CommandResult<ChangeMapData> {
    validate_request(&request)?;
    let db = Arc::clone(&state.db);
    tauri::async_runtime::spawn_blocking(move || {
        let service = InvestigationService::new(Arc::clone(&db));
        match request {
            ChangeMapRequest::Investigation { investigation_id } => {
                let investigation = service.get(investigation_id)?;
                let source_events = db.read(|conn| {
                    event_repository::analysis_window(
                        conn,
                        investigation.resolved_window.from_ms,
                        investigation.resolved_window.to_ms,
                    )
                    .map(|window| window.events)
                })?;
                Ok::<ChangeMapData, AppError>(change_map::build(&investigation, &source_events))
            }
            ChangeMapRequest::Window {
                range,
                mut categories,
                importance_min,
                entity_id,
            } => {
                let (from_ms, to_ms) = crate::services::query_service::resolve_range_for_db(
                    &db,
                    &range,
                    crate::domain::now_ms(),
                )?;
                categories.sort();
                categories.dedup();
                let (investigation, source_events) = service.analyze_filtered_window(
                    InvestigationWindow::Custom { from_ms, to_ms },
                    &categories,
                    importance_min,
                    entity_id.as_deref(),
                )?;
                Ok(change_map::build_window(
                    &investigation,
                    &source_events,
                    u64::try_from(investigation.total_events_in_window).unwrap_or(u64::MAX),
                    u64::try_from(investigation.dropped_events).unwrap_or(u64::MAX),
                ))
            }
        }
    })
    .await
    .map_err(|_| background_failed())?
    .map_err(Into::into)
}

fn validate_request(request: &ChangeMapRequest) -> crate::error::AppResult<()> {
    match request {
        ChangeMapRequest::Investigation { investigation_id } => {
            validate::in_range("investigation id", *investigation_id, 1, i64::MAX).map(|_| ())
        }
        ChangeMapRequest::Window {
            range,
            categories,
            entity_id,
            ..
        } => {
            if let crate::domain::TimeRange::Custom { from, to } = range {
                validate::time_range(*from, *to)?;
            }
            if let Some(entity_id) = entity_id {
                let _ = validate::text("entity id", entity_id, 2_048)?;
            }
            if categories.len() > Category::ALL.len() {
                return Err(AppError::InvalidInput {
                    what: "change map categories",
                    reason: format!("must contain at most {} entries", Category::ALL.len()),
                });
            }
            Ok(())
        }
    }
}

fn background_failed() -> crate::dto::AppErrorDto {
    AppError::InvalidInput {
        what: "change map",
        reason: "the background task stopped before returning a result".to_owned(),
    }
    .into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::TimeRange;

    #[test]
    fn category_filter_is_bounded_before_sql_placeholder_construction() {
        let request = ChangeMapRequest::Window {
            range: TimeRange::Custom { from: 1, to: 2 },
            categories: vec![Category::System; Category::ALL.len() + 1],
            importance_min: None,
            entity_id: None,
        };
        assert!(validate_request(&request).is_err());

        let bounded = ChangeMapRequest::Window {
            range: TimeRange::Custom { from: 1, to: 2 },
            categories: vec![Category::System; Category::ALL.len()],
            importance_min: None,
            entity_id: None,
        };
        validate_request(&bounded).unwrap();
    }
}
