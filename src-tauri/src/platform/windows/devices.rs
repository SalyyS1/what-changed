//! Read-only SetupAPI provider and best-effort device-change wake sources.

use std::ffi::c_void;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Register_Notification, CM_Unregister_Notification, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
    SetupDiGetDeviceRegistryPropertyW, CM_NOTIFY_ACTION, CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER,
    CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES, CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE, CONFIGRET,
    CR_SUCCESS, DIGCF_ALLCLASSES, DIGCF_PRESENT, HCMNOTIFICATION, HDEVINFO, SPDRP_CLASS,
    SPDRP_DEVICEDESC, SPDRP_DRIVER, SPDRP_MFG, SP_DEVINFO_DATA,
};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_DATA,
    ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HANDLE, WAIT_FAILED, WAIT_OBJECT_0,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_READ, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_EXPAND_SZ,
    RRF_RT_REG_SZ, RRF_ZEROONFAILURE,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};

use crate::collectors::drivers::{DriverProvider, DriverRecord, DriverWakeTier};
use crate::collectors::{CancelToken, WakeHandle};
use crate::error::{AppError, AppResult};

const DRIVER_CLASS_ROOT: &str = r"SYSTEM\CurrentControlSet\Control\Class";
const MAX_PROPERTY_BYTES: u32 = 1024 * 1024;

/// SetupAPI provider. Notification ownership stays here so callbacks cannot
/// outlive the collector's wake handle.
#[derive(Default)]
pub struct WindowsDriverProvider {
    notification: Mutex<Option<WakeRegistration>>,
}

impl DriverProvider for WindowsDriverProvider {
    fn drivers(&self, cancel: &CancelToken, deadline: Instant) -> AppResult<Vec<DriverRecord>> {
        enumerate_present_drivers(cancel, deadline)
    }

    fn register_wake(&self, wake: WakeHandle, cancel: CancelToken) -> DriverWakeTier {
        self.unregister_wake();
        let registration = CmRegistration::start(wake.clone(), cancel.clone())
            .map(|guard| WakeRegistration::ConfigurationManager { _guard: guard })
            .or_else(|cm_error| {
                tracing::debug!(reason = %cm_error, "CM driver notification unavailable; trying registry wake");
                RegistryRegistration::start(wake, cancel)
                    .map(|guard| WakeRegistration::Registry { _guard: guard })
            });
        match registration {
            Ok(registration) => {
                let tier = registration.tier();
                *lock(&self.notification) = Some(registration);
                tier
            }
            Err(error) => {
                tracing::debug!(reason = %error, "driver registry notification unavailable; using six-hour timer");
                DriverWakeTier::TimerOnly
            }
        }
    }

    fn unregister_wake(&self) {
        *lock(&self.notification) = None;
    }
}

fn enumerate_present_drivers(
    cancel: &CancelToken,
    deadline: Instant,
) -> AppResult<Vec<DriverRecord>> {
    let set = OwnedDeviceInfoSet(unsafe {
        SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT)
            .map_err(|source| platform_error("open the present-device table", source))?
    });
    let mut rows = Vec::with_capacity(256);
    let mut index = 0u32;
    loop {
        if cancel.is_cancelled() {
            return Ok(Vec::new());
        }
        if Instant::now() > deadline {
            return Err(AppError::Platform {
                operation: "enumerate present drivers",
                reason: "the ten-second sweep deadline elapsed".to_owned(),
            });
        }
        let mut device = SP_DEVINFO_DATA {
            cbSize: u32::try_from(std::mem::size_of::<SP_DEVINFO_DATA>()).unwrap_or(u32::MAX),
            ..SP_DEVINFO_DATA::default()
        };
        let result = unsafe { SetupDiEnumDeviceInfo(set.get(), index, &mut device) };
        if let Err(source) = result {
            if source.code() == ERROR_NO_MORE_ITEMS.to_hresult() {
                break;
            }
            return Err(platform_error("enumerate a present device", source));
        }
        index = index.saturating_add(1);
        let device_instance_id = instance_id(set.get(), &device)?;
        let device_description = property_string(set.get(), &device, SPDRP_DEVICEDESC)?
            .unwrap_or_else(|| device_instance_id.clone());
        let class_name = property_string(set.get(), &device, SPDRP_CLASS)?.unwrap_or_default();
        let manufacturer = property_string(set.get(), &device, SPDRP_MFG)?.unwrap_or_default();
        let driver_key = property_string(set.get(), &device, SPDRP_DRIVER)?;
        let (driver_version, driver_date, provider_name) =
            read_driver_values(driver_key.as_deref())?;
        rows.push(DriverRecord {
            device_instance_id,
            device_description,
            class_name,
            manufacturer,
            driver_version,
            driver_date,
            provider_name,
        });
    }
    Ok(rows)
}

fn instance_id(set: HDEVINFO, device: &SP_DEVINFO_DATA) -> AppResult<String> {
    let mut required = 0u32;
    let probe = unsafe { SetupDiGetDeviceInstanceIdW(set, device, None, Some(&mut required)) };
    if let Err(source) = probe {
        if source.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult() {
            return Err(platform_error("measure a device instance id", source));
        }
    }
    let size = usize::try_from(required).map_err(|_| invalid_device_value())?;
    let mut value = vec![0u16; size.max(1)];
    unsafe {
        SetupDiGetDeviceInstanceIdW(set, device, Some(&mut value), Some(&mut required))
            .map_err(|source| platform_error("read a device instance id", source))?;
    }
    Ok(decode_wide(&value))
}

fn property_string(
    set: HDEVINFO,
    device: &SP_DEVINFO_DATA,
    property: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> AppResult<Option<String>> {
    let mut bytes = 0u32;
    let probe = unsafe {
        SetupDiGetDeviceRegistryPropertyW(set, device, property, None, None, Some(&mut bytes))
    };
    if let Err(source) = probe {
        if source.code() == ERROR_FILE_NOT_FOUND.to_hresult()
            || source.code() == ERROR_INVALID_DATA.to_hresult()
        {
            return Ok(None);
        }
        if source.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult() {
            return Err(platform_error("measure a SetupAPI device property", source));
        }
    }
    if bytes == 0 {
        return Ok(None);
    }
    if bytes > MAX_PROPERTY_BYTES || bytes % 2 != 0 {
        return Err(invalid_device_value());
    }
    let words_len = usize::try_from(bytes / 2).map_err(|_| invalid_device_value())?;
    let mut value = vec![0u16; words_len];
    let property_bytes =
        unsafe { std::slice::from_raw_parts_mut(value.as_mut_ptr().cast::<u8>(), value.len() * 2) };
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            device,
            property,
            None,
            Some(property_bytes),
            Some(&mut bytes),
        )
        .map_err(|source| platform_error("read a SetupAPI device property", source))?;
    }
    let decoded = decode_wide(&value);
    Ok((!decoded.is_empty()).then_some(decoded))
}

fn read_driver_values(
    relative_key: Option<&str>,
) -> AppResult<(Option<String>, Option<String>, Option<String>)> {
    let Some(relative_key) = relative_key.filter(|value| !value.trim().is_empty()) else {
        return Ok((None, None, None));
    };
    let path = format!(r"{DRIVER_CLASS_ROOT}\{relative_key}");
    let path = wide(&path);
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            None,
            KEY_READ,
            &mut key,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok((None, None, None));
    }
    require_success("open a driver class registry key", status.0)?;
    let key = OwnedKey(key);
    Ok((
        registry_string(key.get(), "DriverVersion")?,
        registry_string(key.get(), "DriverDate")?,
        registry_string(key.get(), "ProviderName")?,
    ))
}

fn registry_string(key: HKEY, name: &str) -> AppResult<Option<String>> {
    let name = wide(name);
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
    require_success("measure a driver registry value", status.0)?;
    if bytes == 0 || bytes > MAX_PROPERTY_BYTES || bytes % 2 != 0 {
        return Ok(None);
    }
    let words_len = usize::try_from(bytes / 2).map_err(|_| invalid_device_value())?;
    let mut value = vec![0u16; words_len];
    let status = unsafe {
        RegGetValueW(
            key,
            None,
            PCWSTR(name.as_ptr()),
            flags,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    require_success("read a driver registry value", status.0)?;
    let decoded = decode_wide(&value);
    Ok((!decoded.is_empty()).then_some(decoded))
}

fn decode_wide(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|word| *word == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end]).trim().to_owned()
}

enum WakeRegistration {
    ConfigurationManager { _guard: CmRegistration },
    Registry { _guard: RegistryRegistration },
}

impl WakeRegistration {
    const fn tier(&self) -> DriverWakeTier {
        match self {
            Self::ConfigurationManager { .. } => DriverWakeTier::ConfigurationManager,
            Self::Registry { .. } => DriverWakeTier::Registry,
        }
    }
}

struct WakeContext {
    wake: WakeHandle,
    cancel: CancelToken,
}

struct CmRegistration {
    handle: HCMNOTIFICATION,
    context: Box<WakeContext>,
}

unsafe impl Send for CmRegistration {}

impl CmRegistration {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        let mut context = Box::new(WakeContext { wake, cancel });
        let mut handle = HCMNOTIFICATION::default();
        let filter = CM_NOTIFY_FILTER {
            cbSize: u32::try_from(std::mem::size_of::<CM_NOTIFY_FILTER>()).unwrap_or(u32::MAX),
            Flags: CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES,
            FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            ..CM_NOTIFY_FILTER::default()
        };
        let status = unsafe {
            CM_Register_Notification(
                &filter,
                Some((&mut *context as *mut WakeContext).cast()),
                Some(device_notification),
                &mut handle,
            )
        };
        require_config_success("register Configuration Manager notification", status)?;
        Ok(Self { handle, context })
    }
}

impl Drop for CmRegistration {
    fn drop(&mut self) {
        unsafe {
            let _ = CM_Unregister_Notification(self.handle);
        }
        // Keep context live until unregistration has returned.
        let _ = &self.context;
    }
}

unsafe extern "system" fn device_notification(
    _handle: HCMNOTIFICATION,
    context: *const c_void,
    _action: CM_NOTIFY_ACTION,
    _data: *const CM_NOTIFY_EVENT_DATA,
    _size: u32,
) -> u32 {
    if let Some(context) = unsafe { context.cast::<WakeContext>().as_ref() } {
        if !context.cancel.is_cancelled() {
            let _ = context.wake.request_snapshot();
        }
    }
    CR_SUCCESS.0
}

struct RegistryRegistration {
    stop: OwnedHandle,
    thread: Option<JoinHandle<()>>,
}

impl RegistryRegistration {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        let key = open_class_key()?;
        let notify = OwnedHandle(unsafe { CreateEventW(None, false, false, None) }.map_err(
            |source| platform_error("create a driver registry notification event", source),
        )?);
        let stop = OwnedHandle(
            unsafe { CreateEventW(None, false, false, None) }
                .map_err(|source| platform_error("create a driver registry stop event", source))?,
        );
        arm_registry_notification(key.get(), notify.get())?;
        let stop_raw = stop.raw();
        let thread = thread::Builder::new()
            .name("driver-registry-wake".to_owned())
            .spawn(move || run_registry_watch(key, notify, stop_raw, wake, cancel))
            .map_err(|source| AppError::Platform {
                operation: "start a driver registry notification thread",
                reason: source.kind().to_string(),
            })?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for RegistryRegistration {
    fn drop(&mut self) {
        unsafe {
            let _ = SetEvent(self.stop.get());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_registry_watch(
    key: OwnedKey,
    notify: OwnedHandle,
    stop_raw: usize,
    wake: WakeHandle,
    cancel: CancelToken,
) {
    let handles = [notify.get(), raw_handle(stop_raw)];
    loop {
        let result = unsafe { WaitForMultipleObjects(&handles, false, u32::MAX) };
        if result == WAIT_OBJECT_0 {
            if !cancel.is_cancelled() {
                let _ = wake.request_snapshot();
            }
            if arm_registry_notification(key.get(), notify.get()).is_err() {
                break;
            }
        } else if result == windows::Win32::Foundation::WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
            break;
        } else {
            if result == WAIT_FAILED {
                tracing::debug!("waiting for a driver registry notification failed");
            }
            break;
        }
    }
}

fn open_class_key() -> AppResult<OwnedKey> {
    let path = wide(DRIVER_CLASS_ROOT);
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            None,
            KEY_READ,
            &mut key,
        )
    };
    require_success("open the driver Class registry tree", status.0)?;
    Ok(OwnedKey(key))
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
    require_success("register a driver Class registry notification", status.0)
}

struct OwnedDeviceInfoSet(HDEVINFO);

impl OwnedDeviceInfoSet {
    const fn get(&self) -> HDEVINFO {
        self.0
    }
}

impl Drop for OwnedDeviceInfoSet {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

struct OwnedKey(HKEY);

impl OwnedKey {
    const fn get(&self) -> HKEY {
        self.0
    }
}

unsafe impl Send for OwnedKey {}

impl Drop for OwnedKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
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

unsafe impl Send for OwnedHandle {}

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

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn require_success(operation: &'static str, status: u32) -> AppResult<()> {
    if status == ERROR_SUCCESS.0 {
        Ok(())
    } else {
        Err(AppError::Platform {
            operation,
            reason: format!("0x{status:08x}"),
        })
    }
}

fn require_config_success(operation: &'static str, status: CONFIGRET) -> AppResult<()> {
    if status == CR_SUCCESS {
        Ok(())
    } else {
        Err(AppError::Platform {
            operation,
            reason: format!("0x{:08x}", status.0),
        })
    }
}

fn invalid_device_value() -> AppError {
    AppError::Platform {
        operation: "read a SetupAPI device property",
        reason: "the device property size was invalid".to_owned(),
    }
}

fn platform_error(operation: &'static str, source: windows::core::Error) -> AppError {
    AppError::Platform {
        operation,
        reason: format!("{:#010x}", source.code().0),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::Duration;

    use super::*;

    #[test]
    #[ignore = "real SetupAPI performance gate"]
    fn five_real_sweeps_stay_inside_the_phase_budget() {
        let cancel = CancelToken::default();
        let mut elapsed = Vec::new();
        let mut row_count = 0usize;
        for _ in 0..5 {
            let started = Instant::now();
            let rows =
                enumerate_present_drivers(&cancel, started + Duration::from_secs(10)).unwrap();
            elapsed.push(started.elapsed());
            row_count = rows.len();
            assert!(rows.len() > 20, "Windows exposes more than twenty devices");
            let identities: BTreeSet<_> = rows
                .iter()
                .map(|row| row.device_instance_id.to_ascii_uppercase())
                .collect();
            assert_eq!(identities.len(), rows.len());
        }
        elapsed.sort_unstable();
        assert!(
            elapsed[4] <= Duration::from_millis(3_500),
            "driver sweep p95 {:?} exceeded 3500 ms",
            elapsed[4]
        );
        eprintln!("driver rows={row_count}, sweep p95={:?}", elapsed[4]);
    }
}
