//! Known-good checkpoint commands.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::app::event_bus::EVENT_CHECKPOINT_SAVED;
use crate::app::validate;
use crate::app::AppState;
use crate::checkpoint::CheckpointService;
use crate::collectors::runtime::{CollectorCaptureResult, CollectorCaptureStatus};
use crate::domain::CheckpointKind;
use crate::dto::{CheckpointDiffDto, CheckpointDto, CommandResult};
use crate::error::AppError;
use crate::privacy::Sanitizer;
use crate::storage::checkpoint_repository;

/// Longest name a checkpoint may carry.
const MAX_NAME: usize = 120;

/// Every checkpoint, newest first.
#[tauri::command]
pub fn list_checkpoints(state: State<'_, AppState>) -> CommandResult<Vec<CheckpointDto>> {
    Ok(state
        .db
        .read(checkpoint_repository::list)
        .map(|rows| rows.into_iter().map(Into::into).collect())?)
}

/// Capture the current state of every enabled collector.
#[tauri::command]
pub async fn create_checkpoint(
    app: AppHandle,
    state: State<'_, AppState>,
    name: Option<String>,
    reason: Option<String>,
) -> CommandResult<CheckpointDto> {
    let (name, reason) = normalize_checkpoint_text(name, reason)?;
    let total = state
        .collectors
        .list_collector_status()
        .into_iter()
        .filter(|collector| {
            collector.enabled && collector.state != crate::dto::CollectorState::Unavailable
        })
        .count();
    let service = state.checkpoints.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        create_manual_with_progress(app, service, name, reason, total)
    })
    .await
    .map_err(|_| background_task_failed())?;
    Ok(result?)
}

/// Rename one.
#[tauri::command]
pub fn rename_checkpoint(
    state: State<'_, AppState>,
    checkpoint_id: i64,
    name: String,
) -> CommandResult<CheckpointDto> {
    validate::in_range("checkpoint id", checkpoint_id, 1, i64::MAX)?;
    let name = validate::text("checkpoint name", &name, MAX_NAME)?;
    let (name, _) = Sanitizer::new().sanitize(name);
    Ok(state.db.write(move |conn| {
        if !checkpoint_repository::rename(conn, checkpoint_id, &name)? {
            return Err(not_found("checkpoint id"));
        }
        checkpoint_repository::by_id(conn, checkpoint_id)?
            .map(Into::into)
            .ok_or_else(|| not_found("checkpoint id"))
    })?)
}

/// Delete one, and the snapshots it owns.
#[tauri::command]
pub fn delete_checkpoint(state: State<'_, AppState>, checkpoint_id: i64) -> CommandResult<()> {
    validate::in_range("checkpoint id", checkpoint_id, 1, i64::MAX)?;
    Ok(state.db.write(move |conn| {
        if checkpoint_repository::delete(conn, checkpoint_id)? {
            Ok(())
        } else {
            Err(not_found("checkpoint id"))
        }
    })?)
}

/// What differs between a checkpoint and now, or between two checkpoints.
#[tauri::command]
pub async fn compare_checkpoint(
    state: State<'_, AppState>,
    checkpoint_id: i64,
    against: Option<i64>,
) -> CommandResult<CheckpointDiffDto> {
    validate::in_range("checkpoint id", checkpoint_id, 1, i64::MAX)?;
    if let Some(other) = against {
        validate::in_range("comparison checkpoint id", other, 1, i64::MAX)?;
    }
    let service = state.checkpoints.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        service.compare(checkpoint_id, against).map(Into::into)
    })
    .await
    .map_err(|_| background_task_failed())?;
    Ok(result?)
}

const EVENT_CHECKPOINT_PROGRESS: &str = "checkpoint-progress";

/// Validate and canonicalize optional checkpoint text at every entry point.
pub(crate) fn normalize_checkpoint_text(
    name: Option<String>,
    reason: Option<String>,
) -> crate::error::AppResult<(Option<String>, Option<String>)> {
    let name = name
        .map(|value| validate::text("checkpoint name", &value, MAX_NAME))
        .transpose()?;
    let reason = reason
        .map(|value| validate::text("reason", &value, validate::MAX_TEXT))
        .transpose()?;
    Ok((name, reason))
}

/// Shared command/tray operation: capture, report progress and announce the
/// committed row so every open checkpoint view refreshes authoritative data.
pub(crate) fn create_manual_with_progress<R: tauri::Runtime>(
    app: AppHandle<R>,
    service: CheckpointService,
    name: Option<String>,
    reason: Option<String>,
    total: usize,
) -> crate::error::AppResult<CheckpointDto> {
    let (name, reason) = normalize_checkpoint_text(name, reason)?;
    let completed = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let heartbeat = spawn_progress_heartbeat(
        app.clone(),
        Arc::clone(&completed),
        Arc::clone(&stop),
        total,
    );
    let result = service.create(name, CheckpointKind::Manual, reason, |capture| {
        if !capture_counts_toward_progress(capture.status) {
            return;
        }
        let done = completed.fetch_add(1, Ordering::AcqRel).saturating_add(1);
        emit_progress(&app, capture, done, total);
    });
    stop.store(true, Ordering::Release);
    if let Some(thread) = heartbeat {
        let _ = thread.join();
    }
    emit_capture_terminal(
        &app,
        result.is_ok(),
        completed.load(Ordering::Acquire),
        total,
    );
    let checkpoint: CheckpointDto = result?.into();
    let _ = app.emit(EVENT_CHECKPOINT_SAVED, checkpoint.id);
    Ok(checkpoint)
}

fn capture_counts_toward_progress(status: CollectorCaptureStatus) -> bool {
    !matches!(
        status,
        CollectorCaptureStatus::Disabled | CollectorCaptureStatus::Unavailable
    )
}

#[derive(Clone, Serialize)]
struct CheckpointProgress<'a> {
    collector_id: &'a str,
    collector_name: &'a str,
    status: &'a str,
    completed: usize,
    total: usize,
}

fn emit_progress<R: tauri::Runtime>(
    app: &AppHandle<R>,
    capture: &CollectorCaptureResult,
    completed: usize,
    total: usize,
) {
    let status = match capture.status {
        CollectorCaptureStatus::Complete => "complete",
        CollectorCaptureStatus::Disabled | CollectorCaptureStatus::Unavailable => return,
        CollectorCaptureStatus::Paused
        | CollectorCaptureStatus::Failed
        | CollectorCaptureStatus::TimedOutWaitingForWorker
        | CollectorCaptureStatus::TimedOutDuringCycle => "failed",
    };
    let _ = app.emit(
        EVENT_CHECKPOINT_PROGRESS,
        CheckpointProgress {
            collector_id: &capture.collector_id,
            collector_name: &capture.display_name,
            status,
            completed: completed.min(total),
            total,
        },
    );
}

fn emit_capture_terminal<R: tauri::Runtime>(
    app: &AppHandle<R>,
    succeeded: bool,
    completed: usize,
    total: usize,
) {
    let _ = app.emit(
        EVENT_CHECKPOINT_PROGRESS,
        CheckpointProgress {
            collector_id: "capture",
            collector_name: "Capturing system state",
            status: if succeeded { "complete" } else { "failed" },
            completed: completed.min(total),
            total,
        },
    );
}

fn spawn_progress_heartbeat<R: tauri::Runtime>(
    app: AppHandle<R>,
    completed: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    total: usize,
) -> Option<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("checkpoint-progress".to_owned())
        .spawn(move || {
            while !stop.load(Ordering::Acquire) {
                let _ = app.emit(
                    EVENT_CHECKPOINT_PROGRESS,
                    CheckpointProgress {
                        collector_id: "capture",
                        collector_name: "Capturing system state",
                        status: "running",
                        completed: completed.load(Ordering::Acquire).min(total),
                        total,
                    },
                );
                std::thread::sleep(Duration::from_millis(450));
            }
        })
        .ok()
}

fn not_found(what: &'static str) -> AppError {
    AppError::InvalidInput {
        what,
        reason: "checkpoint was not found".to_owned(),
    }
}

fn background_task_failed() -> crate::dto::AppErrorDto {
    AppError::InvalidInput {
        what: "checkpoint task",
        reason: "the background task stopped unexpectedly".to_owned(),
    }
    .into()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_text_is_canonicalized_once_for_button_and_tray_paths() {
        let (name, reason) = normalize_checkpoint_text(
            Some("  Before upgrade  ".to_owned()),
            Some("  Installing SDK  ".to_owned()),
        )
        .expect("valid checkpoint text");
        assert_eq!(name.as_deref(), Some("Before upgrade"));
        assert_eq!(reason.as_deref(), Some("Installing SDK"));
    }

    #[test]
    fn disabled_and_unavailable_results_do_not_inflate_enabled_progress() {
        assert!(capture_counts_toward_progress(
            CollectorCaptureStatus::Complete
        ));
        assert!(capture_counts_toward_progress(
            CollectorCaptureStatus::Failed
        ));
        assert!(!capture_counts_toward_progress(
            CollectorCaptureStatus::Disabled
        ));
        assert!(!capture_counts_toward_progress(
            CollectorCaptureStatus::Unavailable
        ));
    }
}
