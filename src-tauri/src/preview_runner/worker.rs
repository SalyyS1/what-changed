use super::{
    coordinator::{Control, Pending},
    invalid,
};
use crate::app::data_lifecycle::MutationLease;
use crate::dto::owner_evidence_dto::*;
use crate::owner_workspace::{EvidenceFreshness, OwnerStore, RunLifecycle};
use std::path::PathBuf;
use std::sync::{atomic::Ordering, Arc};

pub(super) fn run(
    control: Arc<Control>,
    store: Arc<OwnerStore>,
    admission: MutationLease,
    resource_root: PathBuf,
    data_root: PathBuf,
    pending: Pending,
    run: OwnerManualRunDto,
) {
    #[cfg(windows)]
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute(
            &control,
            &store,
            &admission,
            &resource_root,
            &data_root,
            &pending,
            &run,
        )
    }))
    .unwrap_or_else(|_| Err(invalid("The preview worker stopped unexpectedly")));
    #[cfg(not(windows))]
    let outcome: crate::error::AppResult<Option<OwnerExecutionSummaryDto>> =
        Err(invalid("The packaged preview runner requires Windows"));
    let cancelled = control.cancelled.load(Ordering::Acquire);
    let lifecycle = if cancelled {
        RunLifecycle::Cancelled
    } else if outcome.as_ref().is_ok_and(Option::is_some) {
        RunLifecycle::Completed
    } else {
        RunLifecycle::Interrupted
    };
    let execution = outcome.as_ref().ok().and_then(Clone::clone);
    let finish = store.finish_browser_run(
        FinishOwnerManualRunInput {
            operation_id: uuid::Uuid::new_v4().to_string(),
            app_id: run.app_id.clone(),
            run_id: run.id.clone(),
            expected_run_version: run.version,
            lifecycle,
        },
        execution,
    );
    let mut status = control.status.lock().unwrap_or_else(|p| p.into_inner());
    status.can_continue = false;
    status.lifecycle = if finish.is_err() {
        "interrupted"
    } else {
        match lifecycle {
            RunLifecycle::Completed => "completed",
            RunLifecycle::Cancelled => "cancelled",
            _ => "interrupted",
        }
    }
    .into();
    status.error=outcome.err().map(|error|match error{crate::error::AppError::InvalidInput{what:"preview runner",reason}=>reason,_=>"The session ended without complete authorized evidence. Review saved observations and prepare a new scope before retrying.".into()});
    if finish.is_err() {
        status.error=Some("The session stopped, but its final state could not be saved. Reload history; startup will reconcile interrupted execution.".into());
    }
    control.finished.store(true, Ordering::Release);
}

#[cfg(windows)]
fn execute(
    control: &Arc<Control>,
    store: &OwnerStore,
    admission: &MutationLease,
    resource_root: &std::path::Path,
    data_root: &std::path::Path,
    pending: &Pending,
    run: &OwnerManualRunDto,
) -> crate::error::AppResult<Option<OwnerExecutionSummaryDto>> {
    use super::{
        bundle::Bundle,
        coordinator::PreviewRunner,
        protocol::{Action, Transcript},
        windows_job::WindowsJob,
    };
    use std::io::{BufRead, BufReader, Read};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    admission.validate()?;
    let bundle = Bundle::open(resource_root)?;
    let _profile_guard =
        crate::app::data_lifecycle::DataLifecycle::runner_profile_guard(data_root)?;
    crate::app::data_lifecycle::DataLifecycle::recover_runner_profiles(data_root)?;
    super::captures::expire(data_root)?;
    let profile = PreviewRunner::run_root(data_root, &run.id);
    crate::app::data_lifecycle::DataLifecycle::prepare_runner_profile(data_root, &run.id)?;
    struct Cleanup<'a> {
        control: &'a Arc<Control>,
        data_root: &'a std::path::Path,
        run_id: &'a str,
    }
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            if let Some(job) = self
                .control
                .job
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .take()
            {
                job.terminate();
                let _ = job.wait(5000);
            }
            let _ = crate::app::data_lifecycle::DataLifecycle::cleanup_runner_profile(
                self.data_root,
                self.run_id,
            );
        }
    }
    let _cleanup = Cleanup {
        control,
        data_root,
        run_id: &run.id,
    };
    let system_root =
        std::env::var("SystemRoot").map_err(|_| invalid("Windows runtime path unavailable"))?;
    let system_drive = system_root
        .get(..2)
        .filter(|drive| drive.as_bytes()[0].is_ascii_alphabetic() && drive.ends_with(':'))
        .ok_or_else(|| invalid("Windows system drive is unavailable"))?
        .to_owned();
    let environment = vec![
        ("SystemRoot", system_root.clone()),
        ("WINDIR", system_root),
        ("SystemDrive", system_drive),
        ("TEMP", profile.to_string_lossy().into_owned()),
        ("TMP", profile.to_string_lossy().into_owned()),
        ("USERPROFILE", profile.to_string_lossy().into_owned()),
        ("LOCALAPPDATA", profile.to_string_lossy().into_owned()),
        (
            "OWNVERO_BUNDLE_ROOT",
            bundle.root.to_string_lossy().into_owned(),
        ),
    ];
    if control.cancelled.load(Ordering::Acquire) {
        return Err(invalid("Cancelled before browser launch"));
    }
    let (owned, output, error) = WindowsJob::spawn(
        &bundle.root.join("node.exe"),
        &bundle.root.join("src/main.mjs"),
        &profile,
        &environment,
    )
    .map_err(|_| invalid("The packaged browser process could not start"))?;
    let job = Arc::new(owned);
    *control.job.lock().unwrap_or_else(|p| p.into_inner()) = Some(job.clone());
    let nonce = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let request = serde_json::json!({"protocol":1,"type":"run","run_id":run.id,"nonce":nonce,"manifest_digest":run.manifest_digest,"manifest":run.manifest,"fixture":pending.input.fixture,"mode":pending.input.mode,
        "consent":{"manifest_digest":run.manifest_digest,"authorized_test_environment":true,"isolated_test_data":true,"capture_screenshots":pending.input.capture_screenshots,"routes":pending.input.routes}});
    let (sender, receiver) = mpsc::sync_channel::<Result<Vec<u8>, ()>>(16);
    let output_thread = std::thread::spawn(move || {
        let mut reader = BufReader::new(output);
        let mut total = 0usize;
        loop {
            let mut frame = Vec::new();
            match (&mut reader)
                .take(1024 * 1024 + 1)
                .read_until(b'\n', &mut frame)
            {
                Ok(0) => break,
                Ok(_) => {
                    total += frame.len();
                    if total > 48 * 1024 * 1024
                        || frame.len() > 1024 * 1024
                        || frame.last() != Some(&b'\n')
                    {
                        let _ = sender.send(Err(()));
                        break;
                    }
                    if sender.send(Ok(frame)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = sender.send(Err(()));
                    break;
                }
            }
        }
    });
    let stderr_job = job.clone();
    let error_thread = std::thread::spawn(move || {
        let mut reader = error;
        let mut buffer = [0u8; 4096];
        let mut total = 0;
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            total += count;
            if total > 65536 {
                stderr_job.terminate();
                break;
            }
        }
    });
    let mut transcript = Transcript::new(
        run.clone(),
        nonce.clone(),
        pending.plan.scope_digest.clone(),
        pending.input.mode.clone(),
    );
    let mut completion = None;
    let mut failure = None;
    let started = Instant::now();
    let mut continued = false;
    let mut committed = std::collections::BTreeSet::new();
    let mut clock = super::clock_guard::ClockGuard::new()?;
    let result = (|| -> crate::error::AppResult<()> {
        job.send(&request.to_string())
            .map_err(|_| invalid("Runner request could not be sent"))?;
        loop {
            admission.validate()?;
            clock.validate()?;
            if control.cancelled.load(Ordering::Acquire)
                || started.elapsed() > Duration::from_secs(600)
            {
                return Err(invalid("Run cancelled or deadline reached"));
            }
            if !continued && transcript.login_roles.len() == pending.plan.roles.len() {
                let status = control.status.lock().unwrap_or_else(|p| p.into_inner());
                if status.lifecycle == "running" {
                    drop(status);
                    job.send(&serde_json::json!({"protocol":1,"manifest_digest":run.manifest_digest,"type":"continue","run_id":run.id,"nonce":nonce}).to_string()).map_err(|_|invalid("Login confirmation could not be sent"))?;
                    continued = true;
                }
            }
            if let Some((id, role)) = control
                .capture_request
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .take()
            {
                transcript.request_capture(id.clone(), role.clone())?;
                job.send(&serde_json::json!({"protocol":1,"manifest_digest":run.manifest_digest,"type":"capture","run_id":run.id,"nonce":nonce,"capture_id":id,"role":role}).to_string()).map_err(|_|invalid("Capture request could not be sent"))?;
            }
            let frame = match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(frame)) => frame,
                Ok(Err(())) => return Err(invalid("Runner output limit or framing error")),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let frame: serde_json::Value =
                serde_json::from_slice(&frame).map_err(|_| invalid("Malformed runner frame"))?;
            let (event, action) = transcript.accept(&frame)?;
            {
                let mut status = control.status.lock().unwrap_or_else(|p| p.into_inner());
                if event.kind == "needs_login"
                    && transcript.login_roles.len() == pending.plan.roles.len()
                {
                    status.lifecycle = "awaiting_login".into();
                    status.can_continue = true;
                }
                if event.kind == "recording" {
                    status.lifecycle = "recording".into();
                }
                if status.events.len() == 200 {
                    status.events.remove(0);
                }
                status.events.push(event);
            }
            match action {
                Action::Authorize(step_id) => {
                    if !continued || pending.input.mode != "run" {
                        return Err(invalid(
                            "The owner has not confirmed test login for execution",
                        ));
                    }
                    if store.get_manual_run(&run.app_id, &run.id)?.freshness
                        == EvidenceFreshness::Stale
                    {
                        return Err(invalid("Frozen candidate, permission or scenario changed"));
                    }
                    admission.validate()?;
                    job.send(&serde_json::json!({"protocol":1,"manifest_digest":run.manifest_digest,"type":"allow_step","run_id":run.id,"nonce":nonce,"step_id":step_id}).to_string()).map_err(|_|invalid("Step permission could not be sent"))?;
                }
                Action::Complete(mut execution) => {
                    execution.scope_digest =
                        super::digest(&(&pending.plan.scope_digest, &bundle.digest))?;
                    completion = Some(execution);
                }
                Action::Capture(capture) => {
                    if let Some(capture) = capture {
                        control
                            .status
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .captures
                            .push(capture);
                    }
                    control.capture_pending.store(false, Ordering::Release);
                }
                Action::None => {}
            }
            for scenario in &run.manifest.scenarios {
                let expected = scenario
                    .recipe
                    .steps
                    .iter()
                    .map(|step| step.assertions.len())
                    .sum::<usize>();
                let result = &transcript.results[&scenario.id];
                if result.assertions.len() == expected && !committed.contains(&scenario.id) {
                    store.record_browser_result(result.clone())?;
                    committed.insert(scenario.id.clone());
                }
            }
        }
        if !transcript.completed || job.wait(5000) != Some(0) {
            return Err(invalid("Runner ended without a verified terminal receipt"));
        }
        Ok(())
    })();
    if let Err(error) = result {
        failure = Some(error);
    }
    // Kill the entire owned tree on every terminal path, including a clean Node exit.
    job.terminate();
    let exited = job.wait(5000).is_some();
    drop(receiver);
    let _ = output_thread.join();
    let _ = error_thread.join();
    control.job.lock().unwrap_or_else(|p| p.into_inner()).take();
    for (id, result) in transcript.results {
        if !committed.contains(&id) && !result.assertions.is_empty() {
            if let Err(error) = store.record_browser_result(result) {
                failure.get_or_insert(error);
            }
        }
    }
    if !exited {
        failure.get_or_insert_with(|| invalid("The owned preview process did not exit"));
    }
    drop(job);
    drop(bundle);
    if let Err(error) =
        crate::app::data_lifecycle::DataLifecycle::cleanup_runner_profile(data_root, &run.id)
    {
        failure.get_or_insert(error);
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(completion)
}
