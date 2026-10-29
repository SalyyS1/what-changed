//! Read-only access to the Windows Service Control Manager.
//!
//! The manager is opened with enumeration rights and each service with
//! `SERVICE_QUERY_CONFIG`; this module has no symbol capable of starting,
//! stopping or reconfiguring a service.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA};
use windows::Win32::System::Services::{
    CloseServiceHandle, EnumServicesStatusExW, OpenSCManagerW, OpenServiceW, QueryServiceConfigW,
    ENUM_SERVICE_STATUS_PROCESSW, QUERY_SERVICE_CONFIGW, SC_ENUM_PROCESS_INFO, SC_HANDLE,
    SC_MANAGER_ENUMERATE_SERVICE, SERVICE_AUTO_START, SERVICE_BOOT_START, SERVICE_CONTINUE_PENDING,
    SERVICE_DEMAND_START, SERVICE_DISABLED, SERVICE_PAUSED, SERVICE_PAUSE_PENDING,
    SERVICE_QUERY_CONFIG, SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STATE_ALL,
    SERVICE_STOPPED, SERVICE_STOP_PENDING, SERVICE_SYSTEM_START, SERVICE_WIN32,
};

use crate::collectors::services::{
    ServiceProvider, ServiceRecord, ServiceRunState, ServiceStartType,
};
use crate::error::{AppError, AppResult};

const ENUM_ATTEMPTS: usize = 3;
const MAX_WIDE_CHARS: usize = 32_767;
const FULL_CONFIG_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Default)]
pub struct WindowsServiceProvider {
    cache: Arc<Mutex<ServiceConfigCache>>,
    status_storage: Arc<Mutex<Vec<usize>>>,
}

#[derive(Debug, Default)]
struct ServiceConfigCache {
    rows: BTreeMap<String, CachedServiceConfig>,
    last_full_config_refresh: Option<Instant>,
}

#[derive(Debug, Clone)]
struct CachedServiceConfig {
    status: u32,
    process_id: u32,
    config: ServiceConfig,
    available: bool,
}

impl ServiceProvider for WindowsServiceProvider {
    fn services(&self) -> AppResult<Vec<ServiceRecord>> {
        enumerate_services(self)
    }

    // Service notifications are an optimization only. The callback API has no
    // cancellation primitive independent of closing its SCM handle; until its
    // lifetime can be tied to the runtime fence without a dangling callback,
    // decline registration and retain the complete five-second diff sweep.
    fn try_register_notify(&self, _wake: crate::collectors::WakeHandle) -> bool {
        false
    }
}

fn enumerate_services(provider: &WindowsServiceProvider) -> AppResult<Vec<ServiceRecord>> {
    let manager = OwnedServiceHandle(
        unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE) }
            .map_err(|source| provider_error("open the service control manager", source))?,
    );
    let mut storage = lock(&provider.status_storage);
    let count = enumerate_status_rows(manager.get(), &mut storage)?;
    let rows = unsafe {
        std::slice::from_raw_parts(
            storage.as_ptr().cast::<ENUM_SERVICE_STATUS_PROCESSW>(),
            count,
        )
    };
    let mut cache = lock(&provider.cache);
    let initial_fill = cache.rows.is_empty();
    let now = Instant::now();
    let full_refresh_due = cache.last_full_config_refresh.is_none_or(|last| {
        now.saturating_duration_since(last) >= FULL_CONFIG_RECONCILIATION_INTERVAL
    });
    let mut seen = BTreeSet::new();
    let mut services = Vec::with_capacity(rows.len());
    for row in rows {
        let name = unsafe { wide_string(row.lpServiceName.0) }?;
        let display_name =
            unsafe { wide_string(row.lpDisplayName.0) }.unwrap_or_else(|_| name.clone());
        let key = name.to_ascii_lowercase();
        seen.insert(key.clone());
        let status = row.ServiceStatusProcess.dwCurrentState.0;
        let process_id = row.ServiceStatusProcess.dwProcessId;
        let status_changed = cache
            .rows
            .get(&key)
            .is_some_and(|cached| cached.status != status || cached.process_id != process_id);
        let query_due =
            initial_fill || full_refresh_due || !cache.rows.contains_key(&key) || status_changed;
        let queried = query_due.then(|| query_config(manager.get(), &name));
        if let Some(Ok(config)) = queried.as_ref() {
            cache.rows.insert(
                key.clone(),
                CachedServiceConfig {
                    status,
                    process_id,
                    config: config.clone(),
                    available: true,
                },
            );
        } else if matches!(queried, Some(Err(_))) && !cache.rows.contains_key(&key) {
            cache.rows.insert(
                key.clone(),
                CachedServiceConfig {
                    status,
                    process_id,
                    config: ServiceConfig::default(),
                    available: false,
                },
            );
        } else if let Some(cached) = cache.rows.get_mut(&key) {
            cached.status = status;
            cached.process_id = process_id;
            if matches!(queried, Some(Err(_))) {
                cached.available = false;
            }
        }
        let config = cache
            .rows
            .get(&key)
            .map(|cached| cached.config.clone())
            .unwrap_or_default();
        let config_available = cache.rows.get(&key).is_some_and(|cached| cached.available);
        services.push(ServiceRecord {
            name,
            display_name,
            state: run_state(row.ServiceStatusProcess.dwCurrentState.0),
            start_type: start_type(config.start_type),
            config_available,
            binary_path: config
                .binary_path
                .as_deref()
                .and_then(executable_from_image_path),
            process_id: (process_id != 0).then_some(process_id),
        });
    }
    cache.rows.retain(|name, _| seen.contains(name));
    if full_refresh_due {
        cache.last_full_config_refresh = Some(now);
    }
    let denied = services
        .iter()
        .filter(|service| !service.config_available)
        .count();
    if denied != 0 {
        tracing::debug!(
            denied,
            services = services.len(),
            "service configuration was unavailable to the non-elevated collector"
        );
    }
    Ok(services)
}

fn enumerate_status_rows(manager: SC_HANDLE, storage: &mut Vec<usize>) -> AppResult<usize> {
    if storage.is_empty() {
        let mut requested = 0u32;
        let mut returned = 0u32;
        let probe = unsafe {
            EnumServicesStatusExW(
                manager,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                None,
                &mut requested,
                &mut returned,
                None,
                PCWSTR::null(),
            )
        };
        if let Err(source) = probe {
            if source.code() != ERROR_MORE_DATA.to_hresult() {
                return Err(provider_error("measure the service table", source));
            }
        }
        if requested == 0 {
            return Ok(0);
        }
        let bytes = usize::try_from(requested).map_err(|_| table_too_large())?;
        storage.resize(words_for(bytes), 0);
    }

    for _ in 0..ENUM_ATTEMPTS {
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                storage.as_mut_ptr().cast::<u8>(),
                storage.len() * std::mem::size_of::<usize>(),
            )
        };
        let mut requested = 0u32;
        let mut returned = 0u32;
        let result = unsafe {
            EnumServicesStatusExW(
                manager,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                Some(buffer),
                &mut requested,
                &mut returned,
                None,
                PCWSTR::null(),
            )
        };
        match result {
            Ok(()) => {
                let count = usize::try_from(returned).map_err(|_| table_too_large())?;
                let row_bytes = count
                    .checked_mul(std::mem::size_of::<ENUM_SERVICE_STATUS_PROCESSW>())
                    .ok_or_else(table_too_large)?;
                if row_bytes > storage.len() * std::mem::size_of::<usize>() {
                    return Err(AppError::Platform {
                        operation: "enumerate services",
                        reason: "the service control manager returned an invalid row count"
                            .to_owned(),
                    });
                }
                return Ok(count);
            }
            Err(source) if source.code() == ERROR_MORE_DATA.to_hresult() => {
                let bytes = usize::try_from(requested).map_err(|_| table_too_large())?;
                storage.resize(words_for(bytes), 0);
            }
            Err(source) => return Err(provider_error("enumerate services", source)),
        }
    }
    Err(AppError::Platform {
        operation: "enumerate services",
        reason: "the service table kept changing during the bounded read".to_owned(),
    })
}

#[derive(Debug, Clone)]
struct ServiceConfig {
    start_type: u32,
    binary_path: Option<String>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            start_type: u32::MAX,
            binary_path: None,
        }
    }
}

/// Extract an executable from the SCM's ImagePath command line without
/// guessing beyond a quoted path or an executable/driver suffix.
fn executable_from_image_path(command: &str) -> Option<String> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if let Some(quoted) = command.strip_prefix('"') {
        return quoted
            .find('"')
            .map(|end| quoted[..end].trim().to_owned())
            .filter(|path| !path.is_empty());
    }
    let first = command.split_whitespace().next()?;
    let first_lower = first.to_ascii_lowercase();
    if first_lower.ends_with(".exe") || first_lower.ends_with(".sys") {
        return Some(first.to_owned());
    }
    // An unquoted path containing whitespace is ambiguous under CreateProcess
    // parsing. Refuse to attribute it to an installation directory instead of
    // manufacturing a plausible executable from the first `.exe` substring.
    (!command.chars().any(char::is_whitespace)).then(|| command.to_owned())
}

fn query_config(manager: SC_HANDLE, name: &str) -> AppResult<ServiceConfig> {
    let wide_name = HSTRING::from(name);
    let service = OwnedServiceHandle(
        unsafe { OpenServiceW(manager, &wide_name, SERVICE_QUERY_CONFIG) }
            .map_err(|source| provider_error("open a service for configuration query", source))?,
    );
    let mut requested = 0u32;
    let probe = unsafe { QueryServiceConfigW(service.get(), None, 0, &mut requested) };
    if let Err(source) = probe {
        if source.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult() {
            return Err(provider_error("measure service configuration", source));
        }
    }
    let bytes = usize::try_from(requested).map_err(|_| table_too_large())?;
    if bytes < std::mem::size_of::<QUERY_SERVICE_CONFIGW>() {
        return Err(AppError::Platform {
            operation: "measure service configuration",
            reason: "the reported service configuration was too small".to_owned(),
        });
    }
    let mut storage = vec![0usize; words_for(bytes)];
    let config = storage.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>();
    unsafe {
        QueryServiceConfigW(service.get(), Some(config), requested, &mut requested)
            .map_err(|source| provider_error("read service configuration", source))?;
    }
    let config = unsafe { &*config };
    let binary_path = unsafe { optional_wide_string(config.lpBinaryPathName.0) };
    Ok(ServiceConfig {
        start_type: config.dwStartType.0,
        binary_path,
    })
}

const fn words_for(bytes: usize) -> usize {
    bytes.saturating_add(std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>()
}

fn run_state(raw: u32) -> ServiceRunState {
    match raw {
        value if value == SERVICE_STOPPED.0 => ServiceRunState::Stopped,
        value if value == SERVICE_START_PENDING.0 => ServiceRunState::StartPending,
        value if value == SERVICE_STOP_PENDING.0 => ServiceRunState::StopPending,
        value if value == SERVICE_RUNNING.0 => ServiceRunState::Running,
        value if value == SERVICE_CONTINUE_PENDING.0 => ServiceRunState::ContinuePending,
        value if value == SERVICE_PAUSE_PENDING.0 => ServiceRunState::PausePending,
        value if value == SERVICE_PAUSED.0 => ServiceRunState::Paused,
        value => ServiceRunState::Unknown(value),
    }
}

fn start_type(raw: u32) -> ServiceStartType {
    match raw {
        value if value == SERVICE_BOOT_START.0 => ServiceStartType::Boot,
        value if value == SERVICE_SYSTEM_START.0 => ServiceStartType::System,
        value if value == SERVICE_AUTO_START.0 => ServiceStartType::Automatic,
        value if value == SERVICE_DEMAND_START.0 => ServiceStartType::Manual,
        value if value == SERVICE_DISABLED.0 => ServiceStartType::Disabled,
        value => ServiceStartType::Unknown(value),
    }
}

unsafe fn wide_string(pointer: *const u16) -> AppResult<String> {
    if pointer.is_null() {
        return Err(AppError::Platform {
            operation: "decode a service name",
            reason: "the service control manager returned a null name".to_owned(),
        });
    }
    let mut length = 0usize;
    while length < MAX_WIDE_CHARS && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    if length == MAX_WIDE_CHARS {
        return Err(AppError::Platform {
            operation: "decode a service name",
            reason: "the service control manager returned an unterminated name".to_owned(),
        });
    }
    Ok(String::from_utf16_lossy(unsafe {
        std::slice::from_raw_parts(pointer, length)
    }))
}

unsafe fn optional_wide_string(pointer: *const u16) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    let mut length = 0usize;
    while length < MAX_WIDE_CHARS && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    (length < MAX_WIDE_CHARS && length != 0)
        .then(|| String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) }))
}

fn table_too_large() -> AppError {
    AppError::Platform {
        operation: "read the service table",
        reason: "the service table was too large for this process".to_owned(),
    }
}

fn provider_error(operation: &'static str, source: windows::core::Error) -> AppError {
    AppError::Platform {
        operation,
        reason: format!("{:#010x}", source.code().0),
    }
}

struct OwnedServiceHandle(SC_HANDLE);

impl OwnedServiceHandle {
    const fn get(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for OwnedServiceHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn image_path_extraction_handles_quotes_arguments_and_drivers() {
        assert_eq!(
            executable_from_image_path(r#""C:\Program Files\Acme\api.exe" --service"#).unwrap(),
            r"C:\Program Files\Acme\api.exe"
        );
        assert_eq!(
            executable_from_image_path(r"C:\Windows\System32\svchost.exe -k netsvcs").unwrap(),
            r"C:\Windows\System32\svchost.exe"
        );
        assert_eq!(
            executable_from_image_path(r"C:\Windows\System32\drivers\disk.sys").unwrap(),
            r"C:\Windows\System32\drivers\disk.sys"
        );
        assert!(executable_from_image_path("ambiguous command line").is_none());
        assert!(executable_from_image_path(r"C:\Program Files\Acme\api.exe --service").is_none());
    }

    #[test]
    fn a_non_elevated_real_snapshot_is_complete_and_named() {
        let rows = WindowsServiceProvider::default().services().unwrap();
        assert!(rows.len() > 20, "Windows exposes more than twenty services");
        assert!(rows
            .iter()
            .all(|row| !row.name.is_empty() && !row.display_name.is_empty()));
        let mut names: Vec<_> = rows
            .iter()
            .map(|row| row.name.to_ascii_lowercase())
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "one SCM snapshot has unique names");
    }
}
