//! The app owns and joins its artifact expiry worker; it never executes a target.
use crate::app::data_lifecycle::DataLifecycle;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;
pub struct CaptureRetention {
    stop: mpsc::SyncSender<()>,
    thread: Option<JoinHandle<()>>,
}
impl CaptureRetention {
    pub fn start(data: PathBuf, lifecycle: Arc<DataLifecycle>) -> crate::error::AppResult<Self> {
        let (stop, receiver) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("preview-artifact-expiry".into())
            .stack_size(128 * 1024)
            .spawn(move || loop {
                // Periodic deletion is covered by the same barrier as explicit purge.
                if let Ok(lease) = lifecycle.admit() {
                    if lease.validate().is_ok() {
                        if let Err(error) = super::captures::expire(&data) {
                            tracing::warn!(%error,"preview artifact expiry remains incomplete");
                        }
                    }
                }
                match receiver.recv_timeout(Duration::from_secs(60)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            })
            .map_err(|_| super::invalid("Artifact retention worker could not start"))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}
impl Drop for CaptureRetention {
    fn drop(&mut self) {
        let _ = self.stop.try_send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
