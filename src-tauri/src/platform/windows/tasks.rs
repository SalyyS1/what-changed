//! Read-only Task Scheduler and Windows startup-surface provider.
//!
//! All Task Scheduler COM interfaces stay on one STA thread. The collector's
//! outer deadline may abandon a request, so provider teardown never waits for
//! an apartment that is still inside a synchronous COM call.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use windows::core::{Interface, BSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HANDLE,
    WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_NOTIFY_CHANGE_LAST_SET,
    REG_NOTIFY_CHANGE_NAME, REG_SZ,
};
use windows::Win32::System::TaskScheduler::{
    IExecAction, IRegisteredTask, ITaskFolder, ITaskService, TaskScheduler, TASK_ENUM_HIDDEN,
    TASK_TRIGGER_BOOT, TASK_TRIGGER_EVENT, TASK_TRIGGER_IDLE, TASK_TRIGGER_LOGON,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};

use crate::collectors::startup::{
    RunEntry, ScheduledTask, StartupFile, StartupInventory, StartupLocation,
    StartupNotificationGuard, StartupProvider, TaskTriggerKind,
};
use crate::collectors::{CancelToken, WakeHandle};
use crate::error::{AppError, AppResult};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_ONCE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\RunOnce";
const STARTUP_APPROVED_RUN: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const STARTUP_APPROVED_FOLDER: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder";
const MAX_STARTUP_REGISTRY_ENTRIES: usize = 1_024;
const MAX_STARTUP_TEXT_BYTES: usize = 16 * 1024;
const MAX_SCHEDULED_TASKS: usize = 16_384;
const MAX_TASK_FOLDERS: usize = 4_096;
const MAX_TASK_FOLDER_DEPTH: usize = 64;
const OLE_AUTOMATION_UNIX_EPOCH_DAYS: f64 = 25_569.0;
const MILLIS_PER_DAY: f64 = 86_400_000.0;

// A timed-out synchronous COM call may outlive both its request and collector
// instance. This process-wide gate prevents a replacement provider (which owns
// a different apartment) from starting a second Task Scheduler walk until the
// abandoned call actually returns.
static TASK_SCHEDULER_BUSY: AtomicBool = AtomicBool::new(false);

enum Request {
    Enumerate {
        deadline: Instant,
        response: mpsc::SyncSender<Result<TaskEnumeration, String>>,
    },
    Stop,
}

struct TaskEnumeration {
    tasks: Vec<ScheduledTask>,
    complete: bool,
}

pub struct WindowsStartupProvider {
    requests: Option<mpsc::Sender<Request>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    startup_error: Option<String>,
}

impl WindowsStartupProvider {
    pub fn new() -> AppResult<Self> {
        let (requests, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("what-changed-task-scheduler".to_owned())
            .spawn(move || apartment_main(receiver))
            .map_err(|source| AppError::Platform {
                operation: "start the Task Scheduler thread",
                reason: source.kind().to_string(),
            })?;
        Ok(Self {
            requests: Some(requests),
            thread: Mutex::new(Some(thread)),
            startup_error: None,
        })
    }

    /// Inactive provider used when the apartment thread could not be created.
    /// Production composition can still start the other collectors and this
    /// collector reports a normal provider failure through runtime status.
    pub fn failed(reason: String) -> Self {
        Self {
            requests: None,
            thread: Mutex::new(None),
            startup_error: Some(reason),
        }
    }

    fn scheduled_tasks(&self, deadline: Instant) -> AppResult<TaskEnumeration> {
        if let Some(reason) = &self.startup_error {
            return Err(platform("enumerate scheduled tasks", reason));
        }
        let Some(requests) = &self.requests else {
            return Err(platform(
                "enumerate scheduled tasks",
                "the COM thread is unavailable",
            ));
        };
        if TASK_SCHEDULER_BUSY
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(platform(
                "enumerate scheduled tasks",
                "a previous Task Scheduler query is still in progress",
            ));
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        if requests
            .send(Request::Enumerate {
                deadline,
                response: sender,
            })
            .is_err()
        {
            TASK_SCHEDULER_BUSY.store(false, Ordering::Release);
            return Err(platform(
                "enumerate scheduled tasks",
                "the COM thread stopped",
            ));
        }
        receiver
            .recv()
            .map_err(|error| {
                TASK_SCHEDULER_BUSY.store(false, Ordering::Release);
                platform("enumerate scheduled tasks", &error.to_string())
            })?
            .map_err(|reason| platform("enumerate scheduled tasks", &reason))
    }
}

impl StartupProvider for WindowsStartupProvider {
    fn inventory(&self, cancel: &CancelToken, deadline: Instant) -> AppResult<StartupInventory> {
        let (tasks, task_store_available) = match self.scheduled_tasks(deadline) {
            Ok(enumeration) => (enumeration.tasks, enumeration.complete),
            Err(error) => {
                // Task Scheduler is one partition of this inventory. COM,
                // access, service, or root-folder failures must not discard
                // readable Run keys and Startup-folder state.
                tracing::warn!(reason = %error, "Task Scheduler inventory is incomplete; preserving known tasks and continuing other startup sources");
                (Vec::new(), false)
            }
        };
        if cancel.is_cancelled() {
            return Ok(StartupInventory::default());
        }
        let approvals = read_startup_approvals()?;
        let mut run_entries = Vec::new();
        read_run_key(
            HKEY_CURRENT_USER,
            RUN_KEY,
            StartupLocation::CurrentUserRun,
            Some(&approvals.run),
            &mut run_entries,
        )?;
        read_run_key(
            HKEY_CURRENT_USER,
            RUN_ONCE_KEY,
            StartupLocation::CurrentUserRunOnce,
            None,
            &mut run_entries,
        )?;
        read_run_key(
            HKEY_LOCAL_MACHINE,
            RUN_KEY,
            StartupLocation::LocalMachineRun,
            None,
            &mut run_entries,
        )?;
        read_run_key(
            HKEY_LOCAL_MACHINE,
            RUN_ONCE_KEY,
            StartupLocation::LocalMachineRunOnce,
            None,
            &mut run_entries,
        )?;
        let startup_files = read_startup_folders(&approvals.folder)?;
        Ok(StartupInventory {
            tasks,
            run_entries,
            startup_files,
            task_store_available,
        })
    }

    fn register_notifications(
        &self,
        wake: WakeHandle,
        cancel: CancelToken,
    ) -> AppResult<Box<dyn StartupNotificationGuard>> {
        StartupWatch::start(wake, cancel)
            .map(|guard| Box::new(guard) as Box<dyn StartupNotificationGuard>)
    }
}

impl Drop for WindowsStartupProvider {
    fn drop(&mut self) {
        if let Some(requests) = &self.requests {
            let _ = requests.send(Request::Stop);
        }
        let thread = self
            .thread
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(thread) = thread.filter(|thread| thread.is_finished()) {
            let _ = thread.join();
        }
    }
}

struct StartupWatch {
    _registry: RegistryWatches,
    _folders: RecommendedWatcher,
}

impl StartupNotificationGuard for StartupWatch {}

impl StartupWatch {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        // Run and RunOnce are the only startup registry locations in scope.
        // StartupApproved is read-only metadata and changes are caught by the
        // same HKCU Explorer branch activity or the hourly correctness sweep.
        let registry = RegistryWatches::start(
            &[
                (HKEY_CURRENT_USER, RUN_KEY),
                (HKEY_CURRENT_USER, RUN_ONCE_KEY),
                (HKEY_LOCAL_MACHINE, RUN_KEY),
                (HKEY_LOCAL_MACHINE, RUN_ONCE_KEY),
            ],
            wake.clone(),
            cancel.clone(),
        )?;
        let callback_wake = wake;
        let callback_cancel = cancel;
        let mut folders = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                if result.is_ok() {
                    request_if_live(&callback_wake, &callback_cancel);
                }
            },
            Config::default(),
        )
        .map_err(|error| platform("start Startup-folder notifications", &error.to_string()))?;
        for path in startup_paths().into_iter().flatten() {
            if path.is_dir() {
                folders
                    .watch(&path, RecursiveMode::NonRecursive)
                    .map_err(|error| {
                        platform(
                            "watch a Startup folder",
                            &format!("{}: {error}", path.display()),
                        )
                    })?;
            }
        }
        Ok(Self {
            _registry: registry,
            _folders: folders,
        })
    }
}

fn request_if_live(wake: &WakeHandle, cancel: &CancelToken) {
    if !cancel.is_cancelled() {
        let _ = wake.request_snapshot();
    }
}

struct RegistryWatches {
    stops: Vec<OwnedHandle>,
    threads: Vec<JoinHandle<()>>,
}

// Kernel handles are explicitly signalled and joined before close.
unsafe impl Send for RegistryWatches {}

impl RegistryWatches {
    fn start(
        specifications: &[(HKEY, &'static str)],
        wake: WakeHandle,
        cancel: CancelToken,
    ) -> AppResult<Self> {
        let mut prepared = Vec::new();
        for (root, path) in specifications {
            if let Some(watch) = prepare_registry_watch(*root, path)? {
                prepared.push(watch);
            }
        }
        let mut stops = Vec::with_capacity(prepared.len());
        let mut threads = Vec::with_capacity(prepared.len());
        for prepared in prepared {
            let stop_raw = prepared.stop.raw();
            let active = ActiveRegistryWatch {
                key: prepared.key,
                notify: prepared.notify,
                stop_raw,
            };
            let thread_wake = wake.clone();
            let thread_cancel = cancel.clone();
            match thread::Builder::new()
                .name("startup-registry-watch".to_owned())
                .spawn(move || run_registry_watch(active, thread_wake, thread_cancel))
            {
                Ok(thread) => {
                    stops.push(prepared.stop);
                    threads.push(thread);
                }
                Err(error) => {
                    stop_and_join(&stops, &mut threads);
                    return Err(platform(
                        "start a startup registry watcher",
                        &error.kind().to_string(),
                    ));
                }
            }
        }
        Ok(Self { stops, threads })
    }

    fn stop(&mut self) {
        stop_and_join(&self.stops, &mut self.threads);
    }
}

impl Drop for RegistryWatches {
    fn drop(&mut self) {
        self.stop();
    }
}

struct PreparedRegistryWatch {
    key: OwnedKey,
    notify: OwnedHandle,
    stop: OwnedHandle,
}

unsafe impl Send for PreparedRegistryWatch {}

struct ActiveRegistryWatch {
    key: OwnedKey,
    notify: OwnedHandle,
    stop_raw: usize,
}

unsafe impl Send for ActiveRegistryWatch {}

fn prepare_registry_watch(
    root: HKEY,
    path: &'static str,
) -> AppResult<Option<PreparedRegistryWatch>> {
    let Some(key) = open_key(root, path)? else {
        return Ok(None);
    };
    let notify = create_event("create a startup registry notification event")?;
    let stop = create_event("create a startup registry stop event")?;
    arm_registry_notification(key.0, notify.0)?;
    Ok(Some(PreparedRegistryWatch { key, notify, stop }))
}

fn create_event(operation: &'static str) -> AppResult<OwnedHandle> {
    unsafe { CreateEventW(None, false, false, None) }
        .map(OwnedHandle)
        .map_err(|error| platform(operation, &format!("{:#010x}", error.code().0)))
}

fn run_registry_watch(watch: ActiveRegistryWatch, wake: WakeHandle, cancel: CancelToken) {
    let handles = [watch.notify.0, raw_handle(watch.stop_raw)];
    loop {
        let result = unsafe { WaitForMultipleObjects(&handles, false, 60 * 60 * 1000) };
        if result == WAIT_OBJECT_0 {
            request_if_live(&wake, &cancel);
            if arm_registry_notification(watch.key.0, watch.notify.0).is_err() {
                break;
            }
        } else if result == WAIT_TIMEOUT {
            request_if_live(&wake, &cancel);
        } else if result == windows::Win32::Foundation::WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
            break;
        } else {
            if result == WAIT_FAILED {
                tracing::debug!("waiting for a startup registry notification failed");
            }
            break;
        }
    }
}

fn arm_registry_notification(key: HKEY, event: HANDLE) -> AppResult<()> {
    let status = unsafe {
        RegNotifyChangeKeyValue(
            key,
            true,
            REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
            Some(event),
            true,
        )
    };
    require_success("register a startup registry notification", status.0)
}

fn stop_and_join(stops: &[OwnedHandle], threads: &mut Vec<JoinHandle<()>>) {
    for stop in stops {
        unsafe {
            let _ = SetEvent(stop.0);
        }
    }
    for thread in threads.drain(..) {
        let _ = thread.join();
    }
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn raw(&self) -> usize {
        self.0 .0 as usize
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

const fn raw_handle(raw: usize) -> HANDLE {
    HANDLE(raw as *mut c_void)
}

fn apartment_main(receiver: mpsc::Receiver<Request>) {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let service = if initialized.is_ok() {
        create_service().map_err(code)
    } else {
        Err(format!("{:#010x}", initialized.0))
    };
    while let Ok(request) = receiver.recv() {
        match request {
            Request::Enumerate { deadline, response } => {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    service
                        .as_ref()
                        .map_err(Clone::clone)
                        .and_then(|service| enumerate_tasks(service, deadline))
                }))
                .unwrap_or_else(|_| Err("the Task Scheduler query panicked".to_owned()));
                TASK_SCHEDULER_BUSY.store(false, Ordering::Release);
                let _ = response.send(result);
            }
            Request::Stop => break,
        }
    }
    drop(service);
    if initialized.is_ok() {
        unsafe { CoUninitialize() };
    }
}

fn create_service() -> windows::core::Result<ITaskService> {
    unsafe {
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)?;
        let empty = VARIANT::default();
        service.Connect(&empty, &empty, &empty, &empty)?;
        Ok(service)
    }
}

fn enumerate_tasks(service: &ITaskService, deadline: Instant) -> Result<TaskEnumeration, String> {
    let root = unsafe { service.GetFolder(&BSTR::from(r"\")) }
        .map_err(|error| format!("open task root: {}", code(error)))?;
    let mut tasks = Vec::with_capacity(256);
    let mut complete = true;
    let mut folders_seen = 0usize;
    walk_folder(
        &root,
        &mut tasks,
        &mut complete,
        deadline,
        0,
        &mut folders_seen,
    )?;
    Ok(TaskEnumeration { tasks, complete })
}

fn walk_folder(
    folder: &ITaskFolder,
    output: &mut Vec<ScheduledTask>,
    complete: &mut bool,
    deadline: Instant,
    depth: usize,
    folders_seen: &mut usize,
) -> Result<(), String> {
    if Instant::now() >= deadline
        || depth > MAX_TASK_FOLDER_DEPTH
        || *folders_seen >= MAX_TASK_FOLDERS
    {
        *complete = false;
        return Ok(());
    }
    *folders_seen = folders_seen.saturating_add(1);
    let folder_path = unsafe { folder.Path() }
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "<unreadable>".to_owned());
    let tasks = match unsafe { folder.GetTasks(TASK_ENUM_HIDDEN.0) } {
        Ok(tasks) => tasks,
        Err(error) => {
            *complete = false;
            tracing::debug!(folder = %folder_path, reason = %code(error), "skipping an unreadable Task Scheduler folder");
            return Ok(());
        }
    };
    let count = match unsafe { tasks.Count() } {
        Ok(count) => count,
        Err(error) => {
            *complete = false;
            tracing::debug!(folder = %folder_path, reason = %code(error), "cannot count scheduled tasks in folder");
            0
        }
    };
    for index in 1..=count {
        if Instant::now() >= deadline || output.len() >= MAX_SCHEDULED_TASKS {
            *complete = false;
            break;
        }
        match unsafe { tasks.get_Item(&index_variant(index)) }
            .map_err(code)
            .and_then(|task| read_task(&task))
        {
            Ok(task) => output.push(task),
            Err(reason) => {
                *complete = false;
                tracing::debug!(folder = %folder_path, %reason, "skipping an unreadable scheduled task");
            }
        }
    }
    let folders = match unsafe { folder.GetFolders(0) } {
        Ok(folders) => folders,
        Err(error) => {
            *complete = false;
            tracing::debug!(folder = %folder_path, reason = %code(error), "cannot enumerate child task folders");
            return Ok(());
        }
    };
    let count = match unsafe { folders.Count() } {
        Ok(count) => count,
        Err(error) => {
            *complete = false;
            tracing::debug!(folder = %folder_path, reason = %code(error), "cannot count child task folders");
            return Ok(());
        }
    };
    for index in 1..=count {
        if Instant::now() >= deadline || *folders_seen >= MAX_TASK_FOLDERS {
            *complete = false;
            break;
        }
        match unsafe { folders.get_Item(&index_variant(index)) } {
            Ok(child) => {
                walk_folder(
                    &child,
                    output,
                    complete,
                    deadline,
                    depth.saturating_add(1),
                    folders_seen,
                )?;
            }
            Err(error) => {
                *complete = false;
                tracing::debug!(folder = %folder_path, reason = %code(error), "skipping an unreadable child task folder");
            }
        }
    }
    Ok(())
}

fn read_task(task: &IRegisteredTask) -> Result<ScheduledTask, String> {
    let definition = unsafe { task.Definition() }.map_err(code)?;
    let trigger_collection = unsafe { definition.Triggers() }.map_err(code)?;
    let mut trigger_count = 0i32;
    unsafe { trigger_collection.Count(&mut trigger_count) }.map_err(code)?;
    let mut triggers = Vec::with_capacity(usize::try_from(trigger_count).unwrap_or(0));
    for index in 1..=trigger_count {
        let trigger = unsafe { trigger_collection.get_Item(index) }.map_err(code)?;
        let mut kind = Default::default();
        unsafe { trigger.Type(&mut kind) }.map_err(code)?;
        triggers.push(match kind {
            TASK_TRIGGER_BOOT => TaskTriggerKind::Boot,
            TASK_TRIGGER_LOGON => TaskTriggerKind::Logon,
            TASK_TRIGGER_IDLE => TaskTriggerKind::Idle,
            TASK_TRIGGER_EVENT => TaskTriggerKind::Event,
            value if matches!(value.0, 1..=5 | 7 | 11) => TaskTriggerKind::Schedule,
            _ => TaskTriggerKind::Other,
        });
    }
    let action_path = first_exec_action(&definition)?;
    let last_run = unsafe { task.LastRunTime() }.map_err(code)?;
    Ok(ScheduledTask {
        name: unsafe { task.Name() }.map_err(code)?.to_string(),
        path: unsafe { task.Path() }.map_err(code)?.to_string(),
        enabled: unsafe { task.Enabled() }.map_err(code)?.0 != 0,
        triggers,
        action_path,
        last_run_ms: ole_date_to_unix_ms(last_run),
        last_result: unsafe { task.LastTaskResult() }.map_err(code)?,
    })
}

fn first_exec_action(
    definition: &windows::Win32::System::TaskScheduler::ITaskDefinition,
) -> Result<Option<String>, String> {
    let actions = unsafe { definition.Actions() }.map_err(code)?;
    let mut count = 0i32;
    unsafe { actions.Count(&mut count) }.map_err(code)?;
    for index in 1..=count {
        let action = unsafe { actions.get_Item(index) }.map_err(code)?;
        if let Ok(exec) = action.cast::<IExecAction>() {
            let mut path = BSTR::new();
            unsafe { exec.Path(&mut path) }.map_err(code)?;
            return Ok(Some(path.to_string()).filter(|path| !path.trim().is_empty()));
        }
    }
    Ok(None)
}

fn index_variant(index: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: index },
            }),
        },
    }
}

fn read_run_key(
    root: HKEY,
    path: &str,
    location: StartupLocation,
    approvals: Option<&BTreeMap<String, bool>>,
    output: &mut Vec<RunEntry>,
) -> AppResult<()> {
    let Some(key) = open_key(root, path)? else {
        return Ok(());
    };
    for_each_registry_value(key.0, |name, value, kind| {
        if kind == REG_SZ.0 || kind == REG_EXPAND_SZ.0 {
            let mut command = decode_utf16(value);
            truncate_utf8(&mut command, MAX_STARTUP_TEXT_BYTES);
            output.push(RunEntry {
                approved: approvals
                    .and_then(|states| states.get(&name.to_ascii_lowercase()).copied()),
                location,
                name: name.to_owned(),
                command,
            });
        }
        Ok(())
    })?;
    Ok(())
}

struct StartupApprovals {
    run: BTreeMap<String, bool>,
    folder: BTreeMap<String, bool>,
}

fn read_startup_approvals() -> AppResult<StartupApprovals> {
    Ok(StartupApprovals {
        run: read_approval_key(STARTUP_APPROVED_RUN)?,
        folder: read_approval_key(STARTUP_APPROVED_FOLDER)?,
    })
}

fn read_approval_key(path: &str) -> AppResult<BTreeMap<String, bool>> {
    let mut states = BTreeMap::new();
    let Some(key) = open_key(HKEY_CURRENT_USER, path)? else {
        return Ok(states);
    };
    for_each_registry_value(key.0, |name, value, _| {
        if let Some(status) = value.first() {
            states.insert(name.to_ascii_lowercase(), *status == 0x02);
        }
        Ok(())
    })?;
    Ok(states)
}

fn read_startup_folders(approvals: &BTreeMap<String, bool>) -> AppResult<Vec<StartupFile>> {
    let [current, common] = startup_paths();
    let mut files = Vec::new();
    for (location, path) in [
        (StartupLocation::CurrentUserStartupFolder, current),
        (StartupLocation::CommonStartupFolder, common),
    ] {
        let Some(path) = path else { continue };
        read_folder(location, &path, approvals, &mut files)?;
    }
    Ok(files)
}

fn startup_paths() -> [Option<PathBuf>; 2] {
    let current =
        dirs::data_dir().map(|path| path.join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    let common = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .map(|path| path.join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    [current, common]
}

fn read_folder(
    location: StartupLocation,
    path: &Path,
    approvals: &BTreeMap<String, bool>,
    output: &mut Vec<StartupFile>,
) -> AppResult<()> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(platform("read a Startup folder", &error.to_string())),
    };
    for entry in entries {
        let entry = entry.map_err(|error| platform("read a Startup folder", &error.to_string()))?;
        let kind = entry
            .file_type()
            .map_err(|error| platform("read a Startup entry", &error.to_string()))?;
        if kind.is_file() || kind.is_symlink() {
            let name = entry.file_name().to_string_lossy().into_owned();
            output.push(StartupFile {
                location,
                path: entry.path().to_string_lossy().into_owned(),
                approved: (location == StartupLocation::CurrentUserStartupFolder)
                    .then(|| approvals.get(&name.to_ascii_lowercase()).copied())
                    .flatten(),
                name,
            });
        }
    }
    Ok(())
}

fn for_each_registry_value(
    key: HKEY,
    mut visit: impl FnMut(&str, &[u8], u32) -> AppResult<()>,
) -> AppResult<()> {
    let mut index = 0u32;
    loop {
        if usize::try_from(index).unwrap_or(usize::MAX) >= MAX_STARTUP_REGISTRY_ENTRIES {
            return Err(platform(
                "enumerate startup registry values",
                "the registry entry count exceeded the safe bound",
            ));
        }
        let mut name = vec![0u16; 16_384];
        let mut data = vec![0u8; 1024 * 1024];
        let mut name_len = u32::try_from(name.len()).unwrap_or(u32::MAX);
        let mut data_len = u32::try_from(data.len()).unwrap_or(u32::MAX);
        let mut kind = 0u32;
        let status = unsafe {
            RegEnumValueW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut kind),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        if status == ERROR_MORE_DATA {
            return Err(platform(
                "enumerate startup registry values",
                "a registry value exceeded the read bound",
            ));
        }
        require_success("enumerate startup registry values", status.0)?;
        name.truncate(usize::try_from(name_len).unwrap_or(0));
        data.truncate(usize::try_from(data_len).unwrap_or(0));
        let name = String::from_utf16_lossy(&name);
        visit(&name, &data, kind)?;
        index = index.saturating_add(1);
    }
    Ok(())
}

fn truncate_utf8(value: &mut String, max_bytes: usize) {
    if value.len() <= max_bytes {
        return;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    value.truncate(boundary);
}

fn decode_utf16(bytes: &[u8]) -> String {
    let mut units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    if units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16_lossy(&units)
}

fn open_key(root: HKEY, path: &str) -> AppResult<Option<OwnedKey>> {
    let path = path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut key = HKEY::default();
    let status = unsafe { RegOpenKeyExW(root, PCWSTR(path.as_ptr()), None, KEY_READ, &mut key) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    require_success("open a startup registry key", status.0)?;
    Ok(Some(OwnedKey(key)))
}

struct OwnedKey(HKEY);

impl Drop for OwnedKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn ole_date_to_unix_ms(date: f64) -> Option<i64> {
    let millis = (date - OLE_AUTOMATION_UNIX_EPOCH_DAYS) * MILLIS_PER_DAY;
    (date > 0.0 && millis.is_finite() && millis >= i64::MIN as f64 && millis <= i64::MAX as f64)
        .then_some(millis.round() as i64)
}

fn require_success(operation: &'static str, status: u32) -> AppResult<()> {
    if status == ERROR_SUCCESS.0 {
        Ok(())
    } else {
        Err(platform(operation, &format!("0x{status:08x}")))
    }
}

fn code(error: windows::core::Error) -> String {
    format!("{:#010x}", error.code().0)
}

fn platform(operation: &'static str, reason: &str) -> AppError {
    AppError::Platform {
        operation,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn notification_callback_wakes_only_while_live() {
        let calls = Arc::new(AtomicU64::new(0));
        let callback_calls = calls.clone();
        let wake = WakeHandle::new(move || {
            callback_calls.fetch_add(1, Ordering::AcqRel);
            Ok(())
        });
        let cancel = CancelToken::default();
        request_if_live(&wake, &cancel);
        assert_eq!(calls.load(Ordering::Acquire), 1);
        cancel.cancel();
        request_if_live(&wake, &cancel);
        assert_eq!(calls.load(Ordering::Acquire), 1);
    }

    #[test]
    fn failed_task_partition_does_not_hide_other_startup_sources() {
        let provider = WindowsStartupProvider::failed("fixture apartment failure".to_owned());
        let inventory = provider
            .inventory(
                &CancelToken::default(),
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();
        assert!(!inventory.task_store_available);
        assert!(inventory.tasks.is_empty());
    }

    #[test]
    fn ole_date_conversion_rejects_never_run_and_converts_epoch() {
        assert_eq!(ole_date_to_unix_ms(0.0), None);
        assert_eq!(ole_date_to_unix_ms(OLE_AUTOMATION_UNIX_EPOCH_DAYS), Some(0));
    }
}
