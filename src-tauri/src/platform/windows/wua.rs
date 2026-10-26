//! Read-only Windows Update Agent history on an owned COM apartment thread.
//!
//! WUA is used because it exposes the complete update history and operation
//! result. `Win32_QuickFixEngineering` is intentionally not used: on the
//! measured machine it returned 4 rows while WUA returned 133.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::UpdateAgent::{IUpdateSearcher, IUpdateSession, UpdateSession};

use crate::collectors::updates::{UpdateHistoryProvider, UpdateHistoryRow};
use crate::error::{AppError, AppResult};

const PAGE_SIZE: i32 = 50;
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const OLE_AUTOMATION_UNIX_EPOCH_DAYS: f64 = 25_569.0;
const MILLIS_PER_DAY: f64 = 86_400_000.0;

enum Request {
    History(mpsc::SyncSender<Result<Vec<UpdateHistoryRow>, String>>),
    Stop,
}

/// Serializes WUA history calls across runtime-created provider instances.
///
/// WUA exposes a synchronous COM API, so a timed-out call may outlive the
/// collector instance that started it. Production composition shares this
/// coordinator through its factory: replacements report a no-op until the old
/// apartment exits its query instead of starting an overlapping COM call.
#[derive(Clone, Default)]
pub struct WuaQueryCoordinator {
    busy: Arc<AtomicBool>,
}

impl WuaQueryCoordinator {
    fn try_begin(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn finish(&self) {
        self.busy.store(false, Ordering::Release);
    }
}

/// WUA provider whose COM interfaces never leave its dedicated STA thread.
pub struct WindowsUpdateProvider {
    requests: Option<mpsc::Sender<Request>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    startup_error: Option<String>,
    response_timeout: Duration,
    query_coordinator: WuaQueryCoordinator,
}

impl WindowsUpdateProvider {
    /// Start the apartment thread. COM activation itself is reported by the
    /// first history request so one unavailable collector cannot abort startup.
    pub fn new() -> AppResult<Self> {
        Self::with_query_coordinator(WuaQueryCoordinator::default())
    }

    pub fn with_query_coordinator(query_coordinator: WuaQueryCoordinator) -> AppResult<Self> {
        let (requests, receiver) = mpsc::channel();
        let apartment_coordinator = query_coordinator.clone();
        let thread = thread::Builder::new()
            .name("what-changed-wua".to_owned())
            .spawn(move || apartment_main(receiver, apartment_coordinator))
            .map_err(|source| AppError::Platform {
                operation: "start the Windows Update history thread",
                reason: source.kind().to_string(),
            })?;
        Ok(Self {
            requests: Some(requests),
            thread: Mutex::new(Some(thread)),
            startup_error: None,
            response_timeout: RESPONSE_TIMEOUT,
            query_coordinator,
        })
    }

    /// Construct an inactive provider after the apartment thread could not be
    /// started. Keeping the same concrete type lets platform composition mark
    /// only this collector unhealthy without panicking or aborting app startup.
    pub fn failed(reason: String) -> Self {
        Self {
            requests: None,
            thread: Mutex::new(None),
            startup_error: Some(reason),
            response_timeout: RESPONSE_TIMEOUT,
            query_coordinator: WuaQueryCoordinator::default(),
        }
    }

    fn try_query_history(&self) -> AppResult<Option<Vec<UpdateHistoryRow>>> {
        if let Some(reason) = &self.startup_error {
            return Err(provider_error(reason));
        }
        if !self.query_coordinator.try_begin() {
            return Ok(None);
        }
        let Some(requests) = self.requests.as_ref() else {
            self.query_coordinator.finish();
            return Err(provider_error(
                "the Windows Update history thread is unavailable",
            ));
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        if requests.send(Request::History(sender)).is_err() {
            self.query_coordinator.finish();
            return Err(provider_error("the COM thread has stopped"));
        }
        receiver
            .recv_timeout(self.response_timeout)
            .map_err(|error| provider_error(&error.to_string()))?
            .map(Some)
            .map_err(|reason| AppError::Platform {
                operation: "query Windows Update history",
                reason,
            })
    }
}

impl UpdateHistoryProvider for WindowsUpdateProvider {
    fn history(&self) -> AppResult<Vec<UpdateHistoryRow>> {
        self.try_query_history()?.ok_or_else(|| {
            provider_error("a previous Windows Update history query is still in progress")
        })
    }

    fn try_history(&self) -> AppResult<Option<Vec<UpdateHistoryRow>>> {
        self.try_query_history()
    }
}

impl Drop for WindowsUpdateProvider {
    fn drop(&mut self) {
        if let Some(requests) = &self.requests {
            let _ = requests.send(Request::Stop);
        }
        let thread = self
            .thread
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(thread) = thread {
            // A WUA COM call is synchronous and cannot be cancelled safely. A
            // timed-out history request may therefore still own this apartment.
            // Join only when it has already exited; otherwise detach it. Once
            // the in-flight COM call returns, the queued Stop (or disconnected
            // request channel) lets `apartment_main` drop every COM pointer and
            // call CoUninitialize on the thread that initialized the apartment.
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
    }
}

fn apartment_main(receiver: mpsc::Receiver<Request>, query_coordinator: WuaQueryCoordinator) {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let searcher = if initialized.is_ok() {
        create_searcher().map_err(|error| format!("{:#010x}", error.code().0))
    } else {
        Err(format!("{:#010x}", initialized.0))
    };
    while let Ok(request) = receiver.recv() {
        match request {
            Request::History(response) => {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match &searcher {
                        Ok(searcher) => query_history(searcher),
                        Err(reason) => Err(reason.clone()),
                    }))
                    .unwrap_or_else(|_| Err("the WUA history query panicked".to_owned()));
                query_coordinator.finish();
                let _ = response.send(result);
            }
            Request::Stop => break,
        }
    }
    // Drop the last COM pointer before uninitializing its apartment.
    drop(searcher);
    if initialized.is_ok() {
        unsafe { CoUninitialize() };
    }
}

fn create_searcher() -> windows::core::Result<IUpdateSearcher> {
    unsafe {
        let session: IUpdateSession = CoCreateInstance(&UpdateSession, None, CLSCTX_INPROC_SERVER)?;
        session.CreateUpdateSearcher()
    }
}

fn query_history(searcher: &IUpdateSearcher) -> Result<Vec<UpdateHistoryRow>, String> {
    let total = unsafe { searcher.GetTotalHistoryCount() }.map_err(code)?;
    if total <= 0 {
        return Ok(Vec::new());
    }
    let capacity = usize::try_from(total).map_err(|_| "invalid history count".to_owned())?;
    let mut rows = Vec::with_capacity(capacity);
    let mut start = 0i32;
    while start < total {
        let requested = PAGE_SIZE.min(total - start);
        let page = unsafe { searcher.QueryHistory(start, requested) }.map_err(code)?;
        let count = unsafe { page.Count() }.map_err(code)?;
        for index in 0..count {
            let entry = unsafe { page.get_Item(index) }.map_err(code)?;
            let identity = unsafe { entry.UpdateIdentity() }.map_err(code)?;
            let date = unsafe { entry.Date() }.map_err(code)?;
            rows.push(UpdateHistoryRow {
                update_id: unsafe { identity.UpdateID() }.map_err(code)?.to_string(),
                revision: unsafe { identity.RevisionNumber() }.map_err(code)?,
                title: unsafe { entry.Title() }.map_err(code)?.to_string(),
                date_ms: ole_date_to_unix_ms(date)?,
                result_code: unsafe { entry.ResultCode() }.map_err(code)?.0,
                operation: unsafe { entry.Operation() }.map_err(code)?.0,
                hresult: unsafe { entry.HResult() }.map_err(code)?,
                client_application_id: unsafe { entry.ClientApplicationID() }
                    .map_err(code)?
                    .to_string(),
                support_url: unsafe { entry.SupportUrl() }.map_err(code)?.to_string(),
            });
        }
        start = start
            .checked_add(requested)
            .ok_or_else(|| "history page offset overflowed".to_owned())?;
    }
    Ok(rows)
}

fn ole_date_to_unix_ms(date: f64) -> Result<i64, String> {
    let millis = (date - OLE_AUTOMATION_UNIX_EPOCH_DAYS) * MILLIS_PER_DAY;
    if !millis.is_finite() || millis < i64::MIN as f64 || millis > i64::MAX as f64 {
        return Err("WUA returned an invalid history date".to_owned());
    }
    Ok(millis.round() as i64)
}

fn code(error: windows::core::Error) -> String {
    format!("{:#010x}", error.code().0)
}

fn provider_error(reason: &str) -> AppError {
    AppError::Platform {
        operation: "query Windows Update history",
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::app::Monitoring;
    use crate::collectors::registry::{CollectorRegistry, RegistryEntry};
    use crate::collectors::runtime::{
        BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, RuntimeClock, SystemClock,
    };
    use crate::collectors::schedule::QueryBusy;
    use crate::collectors::updates::{self, UpdateCollector};
    use crate::domain::MonitoringProfile;
    use crate::storage::Db;
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicU64;
    use std::time::Instant;

    #[test]
    fn ole_automation_epoch_conversion_is_stable() {
        assert_eq!(ole_date_to_unix_ms(25_569.0).unwrap(), 0);
        assert_eq!(ole_date_to_unix_ms(25_570.5).unwrap(), 129_600_000);
        assert!(ole_date_to_unix_ms(f64::NAN).is_err());
    }

    #[test]
    fn timed_out_request_does_not_make_provider_drop_wait_for_apartment() {
        let (requests, receiver) = mpsc::channel();
        let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let (cleaned_sender, cleaned_receiver) = mpsc::sync_channel(1);
        let query_coordinator = WuaQueryCoordinator::default();
        let apartment_coordinator = query_coordinator.clone();
        let thread = thread::spawn(move || {
            while let Ok(request) = receiver.recv() {
                match request {
                    Request::History(_response) => {
                        entered_sender.send(()).unwrap();
                        release_receiver.recv().unwrap();
                        apartment_coordinator.finish();
                    }
                    Request::Stop => break,
                }
            }
            cleaned_sender.send(()).unwrap();
        });
        let provider = WindowsUpdateProvider {
            requests: Some(requests),
            thread: Mutex::new(Some(thread)),
            startup_error: None,
            response_timeout: Duration::from_millis(20),
            query_coordinator,
        };

        assert!(provider.history().is_err());
        entered_receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap();

        let started = Instant::now();
        drop(provider);
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "provider drop waited for a stalled apartment: {:?}",
            started.elapsed()
        );

        release_sender.send(()).unwrap();
        cleaned_receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
    }

    #[derive(Default)]
    struct RecreationState {
        calls: AtomicU64,
        active: AtomicU64,
        overlaps: AtomicU64,
        release_first: AtomicBool,
        first_query_exited: AtomicBool,
    }

    fn test_provider(
        state: Arc<RecreationState>,
        query_coordinator: WuaQueryCoordinator,
    ) -> WindowsUpdateProvider {
        let (requests, receiver) = mpsc::channel();
        let apartment_coordinator = query_coordinator.clone();
        let thread = thread::spawn(move || {
            while let Ok(request) = receiver.recv() {
                match request {
                    Request::History(response) => {
                        let active = state.active.fetch_add(1, Ordering::AcqRel) + 1;
                        if active > 1 {
                            state.overlaps.fetch_add(1, Ordering::AcqRel);
                        }
                        let call = state.calls.fetch_add(1, Ordering::AcqRel);
                        if call == 0 {
                            while !state.release_first.load(Ordering::Acquire) {
                                thread::sleep(Duration::from_millis(1));
                            }
                            state.first_query_exited.store(true, Ordering::Release);
                        }
                        state.active.fetch_sub(1, Ordering::AcqRel);
                        apartment_coordinator.finish();
                        let _ = response.send(Ok(Vec::new()));
                    }
                    Request::Stop => break,
                }
            }
        });
        WindowsUpdateProvider {
            requests: Some(requests),
            thread: Mutex::new(Some(thread)),
            startup_error: None,
            response_timeout: Duration::from_millis(20),
            query_coordinator,
        }
    }

    #[derive(Debug)]
    struct ImmediateBackoff;

    impl BackoffPolicy for ImmediateBackoff {
        fn delay(&self, _consecutive_failures: u64) -> Duration {
            Duration::ZERO
        }
    }

    #[test]
    fn runtime_recreation_does_not_overlap_timed_out_wua_query_and_recovers() {
        let state = Arc::new(RecreationState::default());
        let query_coordinator = WuaQueryCoordinator::default();
        let factory_calls = Arc::new(AtomicU64::new(0));
        let descriptor = updates::catalog_descriptor().unwrap();
        let factory_descriptor = descriptor.clone();
        let factory_state = Arc::clone(&state);
        let factory_calls_for_factory = Arc::clone(&factory_calls);
        let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor, move || {
            factory_calls_for_factory.fetch_add(1, Ordering::AcqRel);
            Box::new(UpdateCollector::new(
                test_provider(factory_state.clone(), query_coordinator.clone()),
                factory_descriptor.clone(),
            ))
        })]);
        let supervisor = CollectorSupervisor::with_runtime(
            registry,
            CollectorRuntimeConfig {
                db: Arc::new(
                    Db::open_in(&crate::testing::temp_dir("wua-runtime-recreation")).unwrap(),
                ),
                monitor: Arc::new(Monitoring::running()),
                query_busy: QueryBusy::default(),
                enabled: BTreeMap::from([(updates::ID.to_owned(), true)]),
                profile: MonitoringProfile::Balanced,
                clock: Arc::new(SystemClock) as Arc<dyn RuntimeClock>,
                backoff: Arc::new(ImmediateBackoff),
            },
        );
        supervisor.spawn_all().unwrap();

        wait_until(Duration::from_secs(1), || {
            factory_calls.load(Ordering::Acquire) >= 2
        });
        assert_eq!(state.calls.load(Ordering::Acquire), 1);
        assert_eq!(state.overlaps.load(Ordering::Acquire), 0);

        state.release_first.store(true, Ordering::Release);
        wait_until(Duration::from_secs(1), || {
            state.first_query_exited.load(Ordering::Acquire)
        });
        supervisor.resume_wake_handle().request();
        wait_until(Duration::from_secs(1), || {
            state.calls.load(Ordering::Acquire) >= 2
        });
        assert_eq!(state.overlaps.load(Ordering::Acquire), 0);
    }

    fn wait_until(timeout: Duration, predicate: impl Fn() -> bool) {
        let deadline = Instant::now() + timeout;
        while !predicate() {
            assert!(Instant::now() < deadline, "timed out waiting for condition");
            thread::sleep(Duration::from_millis(2));
        }
    }
}
