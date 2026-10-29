//! Bounded automatic checkpoint scheduling.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::collectors::runtime::AutomaticCheckpointSink;
use crate::collectors::AutomaticCheckpointTrigger;
use crate::domain::{CheckpointKind, DAY_MS};
use crate::error::{AppError, AppResult};
use crate::storage::{checkpoint_repository, Db};

use super::CheckpointService;

const TRIGGER_CAPACITY: usize = 8;
const INSTALLER_DEBOUNCE: Duration = Duration::from_secs(10 * 60);
const WEEKLY_MS: i64 = 7 * DAY_MS;

/// Frontend notification boundary kept injectable for scheduler tests.
pub trait CheckpointSavedSink: Send + Sync + 'static {
    fn checkpoint_saved(&self, checkpoint_id: i64);
}

impl<R: tauri::Runtime> CheckpointSavedSink for tauri::AppHandle<R> {
    fn checkpoint_saved(&self, checkpoint_id: i64) {
        use tauri::Emitter;
        if let Err(error) = self.emit(crate::app::event_bus::EVENT_CHECKPOINT_SAVED, checkpoint_id)
        {
            tracing::debug!(%error, "automatic checkpoint notice could not be delivered");
        }
    }
}

pub struct AutomaticCheckpointManager {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    submitter: Arc<AutomaticCheckpointSubmitter>,
}

impl AutomaticCheckpointManager {
    pub fn spawn(
        service: CheckpointService,
        db: Arc<Db>,
        saved_sink: Arc<dyn CheckpointSavedSink>,
    ) -> AppResult<Self> {
        let (sender, receiver) = sync_channel(TRIGGER_CAPACITY);
        let submitter = Arc::new(AutomaticCheckpointSubmitter { sender });
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("automatic-checkpoints".to_owned())
            .spawn(move || run(service, db, receiver, &worker_stop, &saved_sink))
            .map_err(|source| AppError::File {
                operation: "start automatic checkpoint thread",
                path: std::path::PathBuf::from("automatic-checkpoints"),
                source,
            })?;
        Ok(Self {
            stop,
            thread: Some(thread),
            submitter,
        })
    }

    pub fn sink(&self) -> Arc<dyn AutomaticCheckpointSink> {
        self.submitter.clone()
    }
}

impl Drop for AutomaticCheckpointManager {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct AutomaticCheckpointSubmitter {
    sender: SyncSender<AutomaticCheckpointTrigger>,
}

impl AutomaticCheckpointSink for AutomaticCheckpointSubmitter {
    fn checkpoint_requested(&self, trigger: AutomaticCheckpointTrigger) {
        let _ = self.sender.try_send(trigger);
    }
}

fn run(
    service: CheckpointService,
    db: Arc<Db>,
    receiver: Receiver<AutomaticCheckpointTrigger>,
    stop: &AtomicBool,
    saved_sink: &Arc<dyn CheckpointSavedSink>,
) {
    let mut last_installer: Option<Instant> = None;
    maybe_create_weekly(&service, &db, saved_sink);
    let mut last_weekly_check = Instant::now();
    while !stop.load(Ordering::Acquire) {
        match receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(AutomaticCheckpointTrigger::InstallerProcessStarted) => {
                if !installer_trigger_due(last_installer, Instant::now()) {
                    continue;
                }
                if create_best_effort(
                    &service,
                    "Before installer activity",
                    "An installer process started",
                    saved_sink,
                ) {
                    last_installer = Some(Instant::now());
                }
            }
            Ok(AutomaticCheckpointTrigger::WindowsUpdateInstallStarted) => {
                create_best_effort(
                    &service,
                    "Before Windows Update",
                    "Windows Update reported installation started",
                    saved_sink,
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if last_weekly_check.elapsed() >= Duration::from_secs(60 * 60) {
            maybe_create_weekly(&service, &db, saved_sink);
            last_weekly_check = Instant::now();
        }
    }
}

fn installer_trigger_due(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|previous| now.saturating_duration_since(previous) >= INSTALLER_DEBOUNCE)
}

fn maybe_create_weekly(
    service: &CheckpointService,
    db: &Db,
    saved_sink: &Arc<dyn CheckpointSavedSink>,
) {
    let now = crate::domain::now_ms();
    let needs_checkpoint = match db.read(checkpoint_repository::latest) {
        Ok(latest) => latest.is_none_or(|row| now.saturating_sub(row.created_at) >= WEEKLY_MS),
        Err(error) => {
            tracing::warn!(%error, "weekly checkpoint eligibility could not be read");
            return;
        }
    };
    if needs_checkpoint {
        create_best_effort(
            service,
            "Weekly automatic checkpoint",
            "No checkpoint had been captured in seven days",
            saved_sink,
        );
    }
}

fn create_best_effort(
    service: &CheckpointService,
    name: &str,
    reason: &str,
    saved_sink: &Arc<dyn CheckpointSavedSink>,
) -> bool {
    match service.create_queued(
        Some(name.to_owned()),
        CheckpointKind::Automatic,
        Some(reason.to_owned()),
        |_| {},
    ) {
        Ok(checkpoint) => {
            saved_sink.checkpoint_saved(checkpoint.id);
            true
        }
        Err(error) => {
            tracing::warn!(%error, "automatic checkpoint could not be created");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_queue_is_bounded_and_nonblocking() {
        let (sender, _receiver) = sync_channel(1);
        let submitter = AutomaticCheckpointSubmitter { sender };
        submitter.checkpoint_requested(AutomaticCheckpointTrigger::InstallerProcessStarted);
        submitter.checkpoint_requested(AutomaticCheckpointTrigger::InstallerProcessStarted);
    }

    #[test]
    fn five_installer_starts_in_ten_minutes_fire_once() {
        let began = Instant::now();
        let mut last = None;
        let mut fired = 0;
        for offset in [0, 30, 60, 120, 599] {
            let now = began + Duration::from_secs(offset);
            if installer_trigger_due(last, now) {
                last = Some(now);
                fired += 1;
            }
        }
        assert_eq!(fired, 1);
        assert!(installer_trigger_due(last, began + INSTALLER_DEBOUNCE));
    }
}
