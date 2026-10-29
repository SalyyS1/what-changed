//! Read-only uninstall-registry enumeration and change notification.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::thread::{self, JoinHandle};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HANDLE,
    WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegGetValueW, RegNotifyChangeKeyValue,
    RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_DWORD, RRF_RT_REG_EXPAND_SZ,
    RRF_RT_REG_SZ, RRF_ZEROONFAILURE,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};

use crate::collectors::applications::{
    RegistryNotificationGuard, RegistryUninstallProvider, UninstallEntry, UninstallHive,
};
use crate::collectors::dev_env::{
    DevEnvironmentNotificationGuard, DevEnvironmentProvider, ToolScan, ToolState,
};
use crate::collectors::{CancelToken, WakeHandle};
use crate::error::{AppError, AppResult};

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_WOW6432: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
const KEY_NAME_CHARS: usize = 512;
const MAX_VALUE_BYTES: u32 = 1024 * 1024;
const MAX_ENV_VALUE_BYTES: usize = 64 * 1024;
const MAX_EXPANDED_PATH_BYTES: usize = MAX_ENV_VALUE_BYTES * 2 + 1;
const MAX_ENV_EXPANSION_DEPTH: usize = 8;
const FALLBACK_MS: u32 = 300_000;
const USER_ENVIRONMENT: &str = r"Environment";
const MACHINE_ENVIRONMENT: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowsRegistryUninstallProvider;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowsDevEnvironmentProvider;

impl DevEnvironmentProvider for WindowsDevEnvironmentProvider {
    fn environment_variables(&self) -> AppResult<BTreeMap<String, String>> {
        let machine = enumerate_environment(HKEY_LOCAL_MACHINE, MACHINE_ENVIRONMENT)?;
        let user = enumerate_environment(HKEY_CURRENT_USER, USER_ENVIRONMENT)?;
        let mut merged = merge_environment(machine, user);
        let expansion_context = merged
            .iter()
            .map(|(name, value)| (name.clone(), value.value.clone()))
            .collect::<BTreeMap<_, _>>();
        // REG_EXPAND_SZ is not limited to PATH (SDK installers commonly use
        // it for SDK_HOME, JAVA_HOME and tool-specific roots). Expand every
        // marked value using the merged, case-insensitive registry view. The
        // expansion helper is bounded and leaves unknown/cyclic references
        // literal, so this never invokes a shell or consults inherited state.
        for (name, value) in &mut merged {
            if value.expandable {
                if let Some(expanded) =
                    expand_environment_value(&value.value, &expansion_context, name)
                {
                    value.value = expanded;
                }
            }
        }
        Ok(merged
            .into_iter()
            .map(|(name, value)| (name, value.value))
            .collect())
    }

    fn probe_tools(&self, path: &str) -> ToolScan {
        let result = crate::platform::windows::tool_probe::ToolProbe.scan(path);
        ToolScan {
            complete: result.complete,
            tools: result
                .tools
                .into_iter()
                .map(|(name, state)| {
                    (
                        name,
                        ToolState {
                            path: state.path,
                            version: state.version,
                        },
                    )
                })
                .collect(),
        }
    }

    fn register_notifications(
        &self,
        wake: WakeHandle,
        cancel: CancelToken,
    ) -> AppResult<Box<dyn DevEnvironmentNotificationGuard>> {
        EnvironmentRegistryWatch::start(wake, cancel)
            .map(|guard| Box::new(guard) as Box<dyn DevEnvironmentNotificationGuard>)
    }
}

fn enumerate_environment(
    root: HKEY,
    path: &str,
) -> AppResult<BTreeMap<String, RegistryEnvironmentValue>> {
    let Some(key) = open_key_optional(root, path, "open an environment registry hive")? else {
        return Ok(BTreeMap::new());
    };
    let mut values = std::collections::BTreeMap::new();
    let mut index = 0u32;
    let mut name = vec![0u16; 16_384];
    let mut data = vec![0u8; MAX_ENV_VALUE_BYTES];
    loop {
        let mut name_len = u32::try_from(name.len()).unwrap_or(u32::MAX);
        let mut data_len = u32::try_from(data.len()).unwrap_or(u32::MAX);
        let mut value_type = 0u32;
        let status = unsafe {
            RegEnumValueW(
                key.get(),
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut value_type),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        require_success("enumerate environment registry values", status.0)?;
        index = index.saturating_add(1);
        if value_type != windows::Win32::System::Registry::REG_SZ.0
            && value_type != windows::Win32::System::Registry::REG_EXPAND_SZ.0
        {
            continue;
        }
        let name_units = usize::try_from(name_len).unwrap_or(0).min(name.len());
        let key_name = String::from_utf16_lossy(&name[..name_units]);
        let byte_len = usize::try_from(data_len).unwrap_or(0).min(data.len());
        if byte_len % 2 != 0 {
            continue;
        }
        let units = data[..byte_len]
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .take_while(|unit| *unit != 0)
            .collect::<Vec<_>>();
        values.insert(
            key_name,
            RegistryEnvironmentValue {
                value: String::from_utf16_lossy(&units),
                expandable: value_type == windows::Win32::System::Registry::REG_EXPAND_SZ.0,
            },
        );
        data.fill(0);
        name.fill(0);
    }
    Ok(values)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryEnvironmentValue {
    value: String,
    expandable: bool,
}

/// Merge the two Windows environment scopes using Windows' case-insensitive
/// variable identity. User values replace machine values, except PATH, whose
/// effective search order is machine first and user second.
fn merge_environment(
    machine: BTreeMap<String, RegistryEnvironmentValue>,
    user: BTreeMap<String, RegistryEnvironmentValue>,
) -> BTreeMap<String, RegistryEnvironmentValue> {
    let mut merged = machine
        .into_iter()
        .map(|(name, value)| (name.to_ascii_uppercase(), value))
        .collect::<BTreeMap<_, _>>();
    for (name, user_value) in user {
        let name = name.to_ascii_uppercase();
        if name != "PATH" {
            merged.insert(name, user_value);
            continue;
        }
        let Some(machine_value) = merged.remove("PATH") else {
            merged.insert(name, user_value);
            continue;
        };
        let value = match (machine_value.value.is_empty(), user_value.value.is_empty()) {
            (true, _) => user_value.value,
            (_, true) => machine_value.value,
            (false, false) => format!("{};{}", machine_value.value, user_value.value),
        };
        merged.insert(
            name,
            RegistryEnvironmentValue {
                value,
                expandable: machine_value.expandable || user_value.expandable,
            },
        );
    }
    merged
}

/// Expand `%NAME%` references without invoking a shell or consulting this
/// process' inherited environment. Unknown variables and cycles remain
/// literal. Returning `None` on the hard output bound lets the caller retain
/// the original registry value rather than use a partial search path.
fn expand_environment_value(
    value: &str,
    variables: &BTreeMap<String, String>,
    root_name: &str,
) -> Option<String> {
    let mut active = BTreeSet::from([root_name.to_ascii_uppercase()]);
    expand_environment_value_inner(value, variables, &mut active, 0, MAX_EXPANDED_PATH_BYTES)
}

fn expand_environment_value_inner(
    value: &str,
    variables: &BTreeMap<String, String>,
    active: &mut BTreeSet<String>,
    depth: usize,
    limit: usize,
) -> Option<String> {
    if depth >= MAX_ENV_EXPANSION_DEPTH {
        return bounded_string(value, limit);
    }
    let mut output = String::with_capacity(value.len().min(limit));
    let mut remaining = value;
    while let Some(open) = remaining.find('%') {
        push_bounded(&mut output, &remaining[..open], limit)?;
        let after_open = &remaining[open + 1..];
        let Some(close) = after_open.find('%') else {
            push_bounded(&mut output, &remaining[open..], limit)?;
            return Some(output);
        };
        let name = &after_open[..close];
        let literal = &remaining[open..open + close + 2];
        let canonical = name.to_ascii_uppercase();
        if name.is_empty() || active.contains(&canonical) {
            push_bounded(&mut output, literal, limit)?;
        } else if let Some(replacement) = variables.get(&canonical) {
            active.insert(canonical.clone());
            let expanded = expand_environment_value_inner(
                replacement,
                variables,
                active,
                depth + 1,
                limit.saturating_sub(output.len()),
            );
            active.remove(&canonical);
            push_bounded(&mut output, &expanded?, limit)?;
        } else {
            push_bounded(&mut output, literal, limit)?;
        }
        remaining = &after_open[close + 1..];
    }
    push_bounded(&mut output, remaining, limit)?;
    Some(output)
}

fn bounded_string(value: &str, limit: usize) -> Option<String> {
    (value.len() <= limit).then(|| value.to_owned())
}

fn push_bounded(output: &mut String, value: &str, limit: usize) -> Option<()> {
    (output.len().saturating_add(value.len()) <= limit).then(|| output.push_str(value))
}

impl RegistryUninstallProvider for WindowsRegistryUninstallProvider {
    fn enumerate(&self) -> AppResult<Vec<UninstallEntry>> {
        let mut entries = Vec::with_capacity(192);
        enumerate_hive(
            HKEY_LOCAL_MACHINE,
            UNINSTALL,
            UninstallHive::LocalMachine64,
            &mut entries,
        )?;
        enumerate_hive(
            HKEY_LOCAL_MACHINE,
            UNINSTALL_WOW6432,
            UninstallHive::LocalMachine32,
            &mut entries,
        )?;
        enumerate_hive(
            HKEY_CURRENT_USER,
            UNINSTALL,
            UninstallHive::CurrentUser,
            &mut entries,
        )?;
        Ok(entries)
    }

    fn register_notifications(
        &self,
        wake: WakeHandle,
        cancel: CancelToken,
    ) -> AppResult<Box<dyn RegistryNotificationGuard>> {
        RegistryWatch::start(wake, cancel)
            .map(|guard| Box::new(guard) as Box<dyn RegistryNotificationGuard>)
    }
}

fn enumerate_hive(
    root: HKEY,
    path: &str,
    hive: UninstallHive,
    output: &mut Vec<UninstallEntry>,
) -> AppResult<()> {
    let Some(key) = open_key_optional(root, path, "open an uninstall registry hive")? else {
        return Ok(());
    };
    let mut name = vec![0u16; KEY_NAME_CHARS];
    let mut index = 0u32;
    loop {
        let mut length = u32::try_from(name.len()).unwrap_or(u32::MAX);
        let status = unsafe {
            RegEnumKeyExW(
                key.get(),
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut length,
                None,
                None,
                None,
                None,
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            return Ok(());
        }
        if status == ERROR_MORE_DATA {
            return Err(platform_status(
                "enumerate uninstall registry entries",
                status.0,
            ));
        }
        require_success("enumerate uninstall registry entries", status.0)?;
        let written = usize::try_from(length).unwrap_or(0).min(name.len());
        let subkey_name = String::from_utf16_lossy(&name[..written]);
        if let Some(entry) = read_entry(key.get(), hive, subkey_name)? {
            output.push(entry);
        }
        name[..written].fill(0);
        index = index.saturating_add(1);
    }
}

fn read_entry(
    parent: HKEY,
    hive: UninstallHive,
    subkey_name: String,
) -> AppResult<Option<UninstallEntry>> {
    // Installers commonly delete a key between enumeration and open. That one
    // identity is genuinely absent from the later point-in-time view. Every
    // other read/type/ACL failure aborts the complete snapshot so the diff
    // cannot fabricate remove/install churn from partial evidence.
    let Some(subkey) = open_key_optional(parent, &subkey_name, "open an uninstall registry entry")?
    else {
        return Ok(None);
    };
    let display_name = read_string(subkey.get(), "DisplayName")?.unwrap_or_default();
    let system_component = read_dword(subkey.get(), "SystemComponent")?.unwrap_or(0) == 1;
    if system_component || display_name.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(UninstallEntry {
        hive,
        subkey_name,
        display_name,
        display_version: read_string(subkey.get(), "DisplayVersion")?,
        publisher: read_string(subkey.get(), "Publisher")?,
        install_date: read_string(subkey.get(), "InstallDate")?,
        install_location: read_string(subkey.get(), "InstallLocation")?,
        system_component,
    }))
}

fn read_string(key: HKEY, value_name: &str) -> AppResult<Option<String>> {
    let name = wide(value_name);
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_ZEROONFAILURE;
    let mut bytes = 0u32;
    let status = unsafe {
        RegGetValueW(
            key,
            None,
            PCWSTR(name.as_ptr()),
            flags,
            None,
            None,
            Some(&mut bytes),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    require_success("measure an uninstall registry string", status.0)?;
    if bytes == 0 {
        return Ok(None);
    }
    if bytes > MAX_VALUE_BYTES || bytes % 2 != 0 {
        return Err(AppError::Platform {
            operation: "read an uninstall registry string",
            reason: "registry value size is invalid".to_owned(),
        });
    }
    let mut buffer = vec![0u16; usize::try_from(bytes / 2).unwrap_or(0)];
    let status = unsafe {
        RegGetValueW(
            key,
            None,
            PCWSTR(name.as_ptr()),
            flags,
            None,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            Some(&mut bytes),
        )
    };
    require_success("read an uninstall registry string", status.0)?;
    let units = usize::try_from(bytes / 2).unwrap_or(0).min(buffer.len());
    let end = buffer[..units]
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units);
    let value = String::from_utf16_lossy(&buffer[..end]);
    buffer.fill(0);
    Ok((!value.is_empty()).then_some(value))
}

fn read_dword(key: HKEY, value_name: &str) -> AppResult<Option<u32>> {
    let name = wide(value_name);
    let mut value = 0u32;
    let mut bytes = u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4);
    let status = unsafe {
        RegGetValueW(
            key,
            None,
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_DWORD | RRF_ZEROONFAILURE,
            None,
            Some((&mut value as *mut u32).cast::<c_void>()),
            Some(&mut bytes),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    require_success("read an uninstall registry number", status.0)?;
    Ok(Some(value))
}

fn open_key(root: HKEY, path: &str, operation: &'static str) -> AppResult<OwnedKey> {
    open_key_optional(root, path, operation)?
        .ok_or_else(|| platform_status(operation, ERROR_FILE_NOT_FOUND.0))
}

fn open_key_optional(
    root: HKEY,
    path: &str,
    operation: &'static str,
) -> AppResult<Option<OwnedKey>> {
    let path = wide(path);
    let mut key = HKEY::default();
    let status = unsafe { RegOpenKeyExW(root, PCWSTR(path.as_ptr()), None, KEY_READ, &mut key) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    require_success(operation, status.0)?;
    Ok(Some(OwnedKey(key)))
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn require_success(operation: &'static str, status: u32) -> AppResult<()> {
    if status == ERROR_SUCCESS.0 {
        Ok(())
    } else {
        Err(platform_status(operation, status))
    }
}

fn platform_status(operation: &'static str, status: u32) -> AppError {
    AppError::Platform {
        operation,
        reason: format!("0x{status:08x}"),
    }
}

struct OwnedKey(HKEY);

impl OwnedKey {
    const fn get(&self) -> HKEY {
        self.0
    }
}

impl Drop for OwnedKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

struct PreparedWatch {
    key: OwnedKey,
    notify: OwnedHandle,
    stop: OwnedHandle,
}

// Registry keys and kernel event handles may be waited on from the thread they
// are moved into. The wrapper owns each close, and no handle is concurrently
// mutated through Rust memory.
unsafe impl Send for PreparedWatch {}

struct ActiveWatch {
    key: OwnedKey,
    notify: OwnedHandle,
    stop_raw: usize,
}

unsafe impl Send for ActiveWatch {}

struct RegistryWatch {
    stops: Vec<OwnedHandle>,
    threads: Vec<JoinHandle<()>>,
}

// Kernel event handles may be signalled from the runtime thread while the
// watcher threads wait on them. RegistryWatch owns every close and joins all
// waiters before those handles are dropped.
unsafe impl Send for RegistryWatch {}

impl RegistryWatch {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        let specifications = [
            (HKEY_LOCAL_MACHINE, UNINSTALL),
            (HKEY_LOCAL_MACHINE, UNINSTALL_WOW6432),
            (HKEY_CURRENT_USER, UNINSTALL),
        ];
        Self::start_for(&specifications, wake, cancel, "registry-uninstall-watch")
    }

    fn start_for(
        specifications: &[(HKEY, &str)],
        wake: WakeHandle,
        cancel: CancelToken,
        thread_name: &'static str,
    ) -> AppResult<Self> {
        let mut prepared = Vec::with_capacity(specifications.len());
        for &(root, path) in specifications {
            prepared.push(prepare_watch(root, path)?);
        }
        let mut stops = Vec::with_capacity(prepared.len());
        let mut threads = Vec::with_capacity(prepared.len());
        for watch in prepared {
            let stop_raw = watch.stop.raw();
            let PreparedWatch { key, notify, stop } = watch;
            let active = ActiveWatch {
                key,
                notify,
                stop_raw,
            };
            let wake = wake.clone();
            let cancel = cancel.clone();
            match thread::Builder::new()
                .name(thread_name.to_owned())
                .spawn(move || run_watch(active, wake, cancel))
            {
                Ok(thread) => {
                    stops.push(stop);
                    threads.push(thread);
                }
                Err(error) => {
                    stop_and_join(&stops, &mut threads);
                    return Err(AppError::Platform {
                        operation: "start an uninstall registry watcher",
                        reason: error.kind().to_string(),
                    });
                }
            }
        }
        Ok(Self { stops, threads })
    }
}

impl RegistryNotificationGuard for RegistryWatch {}

struct EnvironmentRegistryWatch {
    _inner: RegistryWatch,
}

impl EnvironmentRegistryWatch {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        let specifications = [
            (HKEY_LOCAL_MACHINE, MACHINE_ENVIRONMENT),
            (HKEY_CURRENT_USER, USER_ENVIRONMENT),
        ];
        RegistryWatch::start_for(&specifications, wake, cancel, "registry-environment-watch")
            .map(|inner| Self { _inner: inner })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::items_after_test_module)]
mod environment_tests {
    use super::*;

    fn value(value: &str, expandable: bool) -> RegistryEnvironmentValue {
        RegistryEnvironmentValue {
            value: value.to_owned(),
            expandable,
        }
    }

    #[test]
    fn environment_scopes_merge_case_insensitively_with_user_precedence() {
        let machine = BTreeMap::from([
            ("Path".to_owned(), value(r"C:\Windows\System32", false)),
            ("TEMP".to_owned(), value(r"C:\machine-temp", false)),
            ("OnlyMachine".to_owned(), value("machine", false)),
        ]);
        let user = BTreeMap::from([
            ("PATH".to_owned(), value(r"C:\Users\me\bin", true)),
            ("temp".to_owned(), value(r"C:\user-temp", false)),
        ]);

        let merged = merge_environment(machine, user);

        assert_eq!(merged.len(), 3);
        assert_eq!(
            merged["PATH"],
            value(r"C:\Windows\System32;C:\Users\me\bin", true)
        );
        assert_eq!(merged["TEMP"], value(r"C:\user-temp", false));
        assert_eq!(merged["ONLYMACHINE"], value("machine", false));
    }

    #[test]
    fn path_expansion_is_case_insensitive_recursive_and_cycle_safe() {
        let variables = BTreeMap::from([
            ("SYSTEMROOT".to_owned(), r"C:\Windows".to_owned()),
            ("TOOLS".to_owned(), r"%SystemRoot%\tools".to_owned()),
            ("LOOP_A".to_owned(), "%LOOP_B%".to_owned()),
            ("LOOP_B".to_owned(), "%LOOP_A%".to_owned()),
            ("PATH".to_owned(), r"C:\effective".to_owned()),
        ]);

        assert_eq!(
            expand_environment_value(
                r"%TOOLS%;%missing%;%LOOP_A%;%PATH%;C:\literal",
                &variables,
                "PATH",
            )
            .unwrap(),
            r"C:\Windows\tools;%missing%;%LOOP_A%;%PATH%;C:\literal"
        );
    }

    #[test]
    fn path_expansion_rejects_output_beyond_the_hard_bound() {
        let variables = BTreeMap::from([("LARGE".to_owned(), "x".repeat(MAX_EXPANDED_PATH_BYTES))]);
        assert!(expand_environment_value("%LARGE%;tail", &variables, "PATH").is_none());
    }

    #[test]
    fn expandable_non_path_values_are_expanded_with_the_same_bound() {
        let variables = BTreeMap::from([
            ("SYSTEMROOT".to_owned(), r"C:\Windows".to_owned()),
            ("SDK_HOME".to_owned(), r"%SystemRoot%\SDK".to_owned()),
        ]);
        assert_eq!(
            expand_environment_value(r"%SDK_HOME%\bin", &variables, "TOOLS").unwrap(),
            r"C:\Windows\SDK\bin"
        );
    }
}

impl DevEnvironmentNotificationGuard for EnvironmentRegistryWatch {}

impl Drop for RegistryWatch {
    fn drop(&mut self) {
        stop_and_join(&self.stops, &mut self.threads);
    }
}

fn stop_and_join(stops: &[OwnedHandle], threads: &mut Vec<JoinHandle<()>>) {
    for stop in stops {
        unsafe {
            let _ = SetEvent(stop.get());
        }
    }
    for thread in threads.drain(..) {
        let _ = thread.join();
    }
}

fn prepare_watch(root: HKEY, path: &str) -> AppResult<PreparedWatch> {
    let key = open_key(root, path, "open an uninstall registry notification hive")?;
    let notify = OwnedHandle(unsafe { CreateEventW(None, false, false, None) }.map_err(
        |source| AppError::Platform {
            operation: "create an uninstall registry notification event",
            reason: format!("{:#010x}", source.code().0),
        },
    )?);
    let stop = OwnedHandle(unsafe { CreateEventW(None, false, false, None) }.map_err(
        |source| AppError::Platform {
            operation: "create an uninstall registry stop event",
            reason: format!("{:#010x}", source.code().0),
        },
    )?);
    arm_notification(key.get(), notify.get())?;
    Ok(PreparedWatch { key, notify, stop })
}

fn run_watch(watch: ActiveWatch, wake: WakeHandle, cancel: CancelToken) {
    let handles = [watch.notify.get(), handle(watch.stop_raw)];
    loop {
        let result = unsafe { WaitForMultipleObjects(&handles, false, FALLBACK_MS) };
        if result == WAIT_OBJECT_0 {
            if !cancel.is_cancelled() {
                let _ = wake.request_snapshot();
            }
            if let Err(error) = arm_notification(watch.key.get(), watch.notify.get()) {
                tracing::debug!(reason = %error, "registry notification could not be re-armed");
                break;
            }
        } else if result == WAIT_TIMEOUT {
            if !cancel.is_cancelled() {
                let _ = wake.request_snapshot();
            }
        } else if result == windows::Win32::Foundation::WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
            break;
        } else {
            if result == WAIT_FAILED {
                tracing::debug!("waiting for a registry notification failed");
            }
            break;
        }
    }
}

fn arm_notification(key: HKEY, event: HANDLE) -> AppResult<()> {
    let status = unsafe {
        RegNotifyChangeKeyValue(
            key,
            true,
            REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
            Some(event),
            true,
        )
    };
    require_success("register an uninstall registry notification", status.0)
}

const fn handle(raw: usize) -> HANDLE {
    HANDLE(raw as *mut c_void)
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    const fn get(&self) -> HANDLE {
        self.0
    }

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
