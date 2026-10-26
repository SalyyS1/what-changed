use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{health, paths, Options};
use crate::app::{AppPaths, AppState};
use crate::collectors::runtime::CollectorCaptureStatus;
use crate::commands::dev::simulate_event_for_state;
use crate::domain::{MonitoringProfile, SettingsUpdate};
use crate::error::{AppError, AppResult};
use crate::platform;
use crate::storage::{settings_repository, Db};

const POLL: Duration = Duration::from_millis(100);
const READINESS_TIMEOUT: Duration = Duration::from_secs(120);

fn failure(reason: impl Into<String>) -> AppError {
    AppError::Platform {
        operation: "backend soak",
        reason: reason.into(),
    }
}

pub(super) fn run(options: Options) -> AppResult<()> {
    let owned =
        paths::OwnedRoot::create(&options.data_dir, &options.trigger_file, &options.ack_file)?;
    let registry = platform::production_registry();
    let expected = registry
        .entries()
        .iter()
        .map(|entry| entry.descriptor.id.to_owned())
        .collect::<Vec<_>>();
    let enabled = expected
        .iter()
        .cloned()
        .map(|id| (id, true))
        .collect::<BTreeMap<_, _>>();
    let db = Db::open_in(&owned.root)?;
    db.write(move |conn| {
        settings_repository::update(
            conn,
            &SettingsUpdate {
                monitoring_profile: Some(MonitoringProfile::Thorough),
                collectors: Some(enabled),
                onboarding_completed: Some(true),
                developer_mode: Some(true),
                ..SettingsUpdate::default()
            },
        )
        .map(|_| ())
    })?;
    db.flush()?;
    db.close();
    let state = AppState::open_with_registry(AppPaths::under(&owned.root), registry)?;
    let evidence = Arc::new(health::Evidence::default());
    state.collectors.attach_status_sink(evidence.clone());
    let run_id = uuid::Uuid::new_v4().to_string();
    let mut ready = false;
    let mut started = None;
    let mut triggers_committed = 0_u64;
    let result = (|| {
        state.collectors.spawn_all()?;
        let waiting = Instant::now();
        loop {
            if health::inspect(&state, &expected, &evidence).map_err(failure)? {
                break;
            }
            if waiting.elapsed() >= READINESS_TIMEOUT {
                return Err(failure(
                    "readiness timed out before every collector completed a successful snapshot",
                ));
            }
            thread::sleep(POLL);
        }
        // Legacy notification drains enqueue writes; flush before readiness so
        // they cannot hide a failed transaction behind a successful drain.
        state.db.flush()?;
        health::inspect(&state, &expected, &evidence).map_err(failure)?;
        ready = true;
        started = Some(Instant::now());
        paths::write_new(
            &owned.root.join(paths::READY),
            &json!({
                "schema_version": 1, "mode": "backend-only", "run_id": run_id,
                "process_id": std::process::id(), "requested_seconds": options.seconds,
                "ready_at_ms": crate::domain::now_ms(), "expected_collectors": expected,
                "trigger_file": owned.trigger.file_name().and_then(|name| name.to_str()),
                "ack_file": owned.ack.file_name().and_then(|name| name.to_str())
            })
            .to_string(),
        )?;
        let began = started.ok_or_else(|| failure("missing readiness start"))?;
        while began.elapsed() < Duration::from_secs(options.seconds) {
            if !health::inspect(&state, &expected, &evidence).map_err(failure)? {
                return Err(failure("collector readiness was lost"));
            }
            if let Some(payload) = paths::read_trigger(&owned.trigger)? {
                let (nonce, kind) = payload
                    .trim()
                    .split_once('|')
                    .ok_or_else(|| failure("malformed trigger"))?;
                if nonce.is_empty()
                    || !nonce
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                {
                    return Err(failure("invalid trigger nonce"));
                }
                let simulated = simulate_event_for_state(&state, kind);
                let response = match &simulated {
                    Ok(id) => format!("{nonce}|ok|{id}"),
                    Err(error) => format!("{nonce}|error|{}", error.code.as_str()),
                };
                std::fs::remove_file(&owned.trigger)
                    .map_err(|_| failure("could not consume trigger"))?;
                paths::write_new(&owned.ack, &response)?;
                simulated.map_err(|_| failure("simulation failed"))?;
                triggers_committed += 1;
            }
            thread::sleep(POLL);
        }
        // A stalled collector can retain a healthy old snapshot. Require one
        // new complete requested cycle from every worker before finishing.
        let capture = state.collectors.capture_enabled(Duration::from_secs(60));
        if capture.collectors.len() != expected.len()
            || capture
                .collectors
                .iter()
                .any(|row| row.status != CollectorCaptureStatus::Complete)
        {
            return Err(failure("final collector capture was incomplete"));
        }
        health::inspect(&state, &expected, &evidence).map_err(failure)?;
        if triggers_committed == 0 {
            return Err(failure("no committed simulation trigger"));
        }
        Ok::<(), AppError>(())
    })();
    // Stop admission before the final drain and inspect lifetime counters after
    // shutdown, so late commit failures cannot escape the terminal report.
    state.shutdown_cleanly();
    let flush = state.db.flush();
    let outcomes = evidence.snapshot();
    let losses = health::writer_loss(&state);
    let worker_running = state
        .collectors
        .list_collector_status()
        .iter()
        .any(|row| row.state == crate::dto::CollectorState::Running);
    let shutdown_complete = !state.collectors.shutdown_incomplete() && !worker_running;
    let result = result.and(flush).and_then(|()| {
        if losses > 0
            || !outcomes.failure_observations.is_empty()
            || outcomes.monitoring_paused
            || !shutdown_complete
        {
            Err(failure(
                "loss or collector failure occurred before shutdown completed",
            ))
        } else {
            Ok(())
        }
    });
    paths::write_new(
        &owned.root.join(paths::REPORT),
        &json!({
            "schema_version": 1, "mode": "backend-only", "run_id": run_id,
            "process_id": std::process::id(), "requested_seconds": options.seconds,
            "expected_collectors": expected, "ready": ready,
            "observed_seconds": started.map(|start| start.elapsed().as_secs_f64()).unwrap_or(0.0),
            "status": if result.is_ok() { "pass" } else { "fail" },
            "writer_loss": losses, "outcomes": outcomes, "triggers_committed": triggers_committed,
            "shutdown_complete": shutdown_complete
        })
        .to_string(),
    )?;
    result
}
