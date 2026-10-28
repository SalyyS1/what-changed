use super::{invalid, types::*, SEMANTICS};
use crate::app::data_lifecycle::DataLifecycle;
use crate::error::AppResult;
use crate::owner_workspace::OwnerStore;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(super) struct Pending {
    pub input: PreparePreviewRunInput,
    pub plan: PreviewRunPlanDto,
    pub expires: Instant,
}
pub(super) struct Control {
    pub status: Mutex<PreviewRunnerStatusDto>,
    pub cancelled: AtomicBool,
    pub finished: AtomicBool,
    #[cfg(windows)]
    pub job: Mutex<Option<Arc<super::windows_job::WindowsJob>>>,
    pub thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    pub capture_request: Mutex<Option<(String, String)>>,
    pub capture_pending: AtomicBool,
}
impl Control {
    pub fn stop(&self) {
        self.cancelled.store(true, Ordering::Release);
        #[cfg(windows)]
        if let Some(job) = self.job.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            job.terminate();
        }
    }
    pub fn join(&self) {
        if let Some(thread) = self.thread.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = thread.join();
        }
    }
}
#[derive(Default)]
pub struct PreviewRunner {
    pub(super) pending: Mutex<Option<Pending>>,
    pub(super) active: Mutex<Option<Arc<Control>>>,
}

impl PreviewRunner {
    pub fn start(
        &self,
        store: Arc<OwnerStore>,
        lifecycle: Arc<DataLifecycle>,
        resource_root: PathBuf,
        data_root: PathBuf,
        grant: GrantPreviewRunInput,
    ) -> AppResult<PreviewRunnerStatusDto> {
        if !grant.authorized_test_environment
            || !grant.isolated_test_data
            || !grant.acknowledge_temporary_auth
        {
            return Err(invalid(
                "Confirm ownership, isolated test data and temporary login storage before starting",
            ));
        }
        let mut active = self.active.lock().unwrap_or_else(|p| p.into_inner());
        if active
            .as_ref()
            .is_some_and(|control| !control.finished.load(Ordering::Acquire))
        {
            return Err(invalid(
                "Another preview session is active; cancel it before starting a new one",
            ));
        }
        if let Some(old) = active.take() {
            old.join();
        }
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let entry = pending
            .as_ref()
            .ok_or_else(|| invalid("Review the run scope before granting permission"))?;
        if entry.expires < Instant::now()
            || entry.plan.consent_id != grant.consent_id
            || entry.plan.scope_digest != grant.scope_digest
        {
            return Err(invalid(
                "Permission expired or scope changed; review it again",
            ));
        }
        let admission = lifecycle.admit()?;
        let current = store.preview_browser_manifest(&entry.input.run)?;
        let mut expected = current;
        expected.semantics_version = SEMANTICS.into();
        if expected != entry.plan.manifest {
            return Err(invalid(
                "The candidate, rules or environment changed; review a fresh run scope",
            ));
        }
        let entry = pending
            .take()
            .ok_or_else(|| invalid("Permission is no longer available"))?;
        drop(pending);
        let run = store.start_browser_run(entry.input.run.clone(), entry.plan.manifest.clone())?;
        let status = PreviewRunnerStatusDto {
            app_id: run.app_id.clone(),
            run_id: run.id.clone(),
            lifecycle: "starting".into(),
            roles: entry.plan.roles.clone(),
            can_continue: false,
            mode: entry.input.mode.clone(),
            events: vec![],
            error: None,
            capture_screenshots: entry.input.capture_screenshots,
            captures: vec![],
        };
        let control = Arc::new(Control {
            status: Mutex::new(status.clone()),
            cancelled: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            #[cfg(windows)]
            job: Mutex::new(None),
            thread: Mutex::new(None),
            capture_request: Mutex::new(None),
            capture_pending: AtomicBool::new(false),
        });
        let owned = control.clone();
        let fallback_store = store.clone();
        let fallback_run = run.clone();
        let thread = std::thread::Builder::new()
            .name("preview-runner".into())
            .spawn(move || {
                super::worker::run(
                    owned,
                    store,
                    admission,
                    resource_root,
                    data_root,
                    entry,
                    run,
                )
            });
        match thread {
            Ok(thread) => *control.thread.lock().unwrap_or_else(|p| p.into_inner()) = Some(thread),
            Err(_) => {
                let _ = fallback_store.finish_browser_run(
                    crate::dto::owner_evidence_dto::FinishOwnerManualRunInput {
                        operation_id: uuid::Uuid::new_v4().to_string(),
                        app_id: fallback_run.app_id,
                        run_id: fallback_run.id,
                        expected_run_version: fallback_run.version,
                        lifecycle: crate::owner_workspace::RunLifecycle::Interrupted,
                    },
                    None,
                );
                return Err(invalid(
                    "The preview worker could not start; this attempt was interrupted",
                ));
            }
        }
        *active = Some(control);
        Ok(status)
    }
    pub fn stop_all(&self) {
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(control) = self
            .active
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
        {
            control.stop();
            control.join();
        }
    }
    pub fn resource_root(app: &tauri::AppHandle) -> AppResult<PathBuf> {
        use tauri::Manager;
        app.path()
            .resource_dir()
            .map_err(|_| invalid("Application resources unavailable"))
    }
    pub(super) fn run_root(data_root: &Path, id: &str) -> PathBuf {
        data_root.join("owner-temp-profiles").join(id)
    }
}
impl Drop for PreviewRunner {
    fn drop(&mut self) {
        self.stop_all();
    }
}
