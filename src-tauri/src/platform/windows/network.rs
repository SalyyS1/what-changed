//! Windows network configuration provider and best-effort wake registration.

use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_BUFFER_OVERFLOW, ERROR_FILE_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER,
    ERROR_SUCCESS, HANDLE, WAIT_FAILED, WAIT_OBJECT_0,
};
use windows::Win32::NetworkManagement::IpHelper::{
    CancelMibChangeNotify2, FreeMibTable, GetAdaptersAddresses, GetIpForwardTable2,
    NotifyIpInterfaceChange, NotifyRouteChange2, GAA_FLAG_INCLUDE_ALL_INTERFACES,
    GAA_FLAG_INCLUDE_GATEWAYS, IF_TYPE_TUNNEL, IP_ADAPTER_ADDRESSES_LH,
    IP_ADAPTER_DNS_SERVER_ADDRESS_XP, IP_ADAPTER_GATEWAY_ADDRESS_LH, IP_ADAPTER_UNICAST_ADDRESS_LH,
    MIB_IPFORWARD_ROW2, MIB_IPFORWARD_TABLE2, MIB_IPINTERFACE_ROW, MIB_NOTIFICATION_TYPE,
};
use windows::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_INET, SOCKET_ADDRESS,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    KEY_READ, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_DWORD,
    RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RRF_ZEROONFAILURE,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};

use crate::collectors::network::{
    NetworkAdapter, NetworkProvider, NetworkState, NetworkWakeTier, ProxyConfiguration,
};
use crate::collectors::{CancelToken, WakeHandle};
use crate::error::{AppError, AppResult};

const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
const MAX_ADAPTER_BYTES: usize = 16 * 1024 * 1024;
const MAX_LINKED_ROWS: usize = 16_384;
const MAX_REGISTRY_BYTES: u32 = 1024 * 1024;
const ADAPTER_ATTEMPTS: usize = 3;

#[derive(Default)]
pub struct WindowsNetworkProvider {
    registration: Mutex<Option<WakeRegistration>>,
}

impl NetworkProvider for WindowsNetworkProvider {
    fn network_state(&self, cancel: &CancelToken, deadline: Instant) -> AppResult<NetworkState> {
        let default_routes = default_routes(cancel, deadline)?;
        let adapters = adapters(&default_routes, cancel, deadline)?;
        let proxy = proxy_configuration()?;
        Ok(NetworkState::from_parts(adapters, proxy))
    }

    fn register_wake(&self, wake: WakeHandle, cancel: CancelToken) -> NetworkWakeTier {
        self.unregister_wake();
        let ip = IpHelperRegistration::start(wake.clone(), cancel.clone())
            .map_err(|error| tracing::debug!(reason = %error, "IP Helper network wake unavailable"))
            .ok();
        let registry = RegistryRegistration::start(wake, cancel)
            .map_err(|error| tracing::debug!(reason = %error, "proxy registry wake unavailable"))
            .ok();
        let tier = match (ip.is_some(), registry.is_some()) {
            (true, true) => NetworkWakeTier::IpHelperAndRegistry,
            (true, false) => NetworkWakeTier::IpHelper,
            (false, true) => NetworkWakeTier::Registry,
            (false, false) => NetworkWakeTier::TimerOnly,
        };
        *lock(&self.registration) = Some(WakeRegistration { ip, registry });
        tier
    }

    fn unregister_wake(&self) {
        *lock(&self.registration) = None;
    }
}

fn adapters(
    default_routes: &[(u32, String)],
    cancel: &CancelToken,
    deadline: Instant,
) -> AppResult<Vec<NetworkAdapter>> {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_INCLUDE_ALL_INTERFACES;
    let mut bytes = 15_000u32;
    let mut storage = aligned_storage(bytes)?;
    for _ in 0..ADAPTER_ATTEMPTS {
        if cancelled_or_expired(cancel, deadline) {
            return interrupted();
        }
        let status = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC.0),
                flags,
                None,
                Some(storage.as_mut_ptr().cast()),
                &mut bytes,
            )
        };
        if status == ERROR_SUCCESS.0 {
            return unsafe {
                parse_adapters(storage.as_ptr().cast(), default_routes, cancel, deadline)
            };
        }
        if status != ERROR_BUFFER_OVERFLOW.0 && status != ERROR_INSUFFICIENT_BUFFER.0 {
            return Err(platform_status("enumerate network adapters", status));
        }
        storage = aligned_storage(bytes)?;
    }
    Err(AppError::Platform {
        operation: "enumerate network adapters",
        reason: "the adapter table kept growing across three attempts".to_owned(),
    })
}

fn aligned_storage(bytes: u32) -> AppResult<Vec<usize>> {
    let bytes = usize::try_from(bytes).unwrap_or(MAX_ADAPTER_BYTES + 1);
    if bytes == 0 || bytes > MAX_ADAPTER_BYTES {
        return Err(AppError::Platform {
            operation: "enumerate network adapters",
            reason: format!("the platform requested an invalid {bytes}-byte buffer"),
        });
    }
    let words = bytes.div_ceil(std::mem::size_of::<usize>());
    Ok(vec![0usize; words])
}

unsafe fn parse_adapters(
    mut current: *const IP_ADAPTER_ADDRESSES_LH,
    default_routes: &[(u32, String)],
    cancel: &CancelToken,
    deadline: Instant,
) -> AppResult<Vec<NetworkAdapter>> {
    let mut rows = Vec::new();
    for _ in 0..MAX_LINKED_ROWS {
        if current.is_null() {
            return Ok(rows);
        }
        if cancelled_or_expired(cancel, deadline) {
            return interrupted();
        }
        let row = unsafe { &*current };
        let guid = unsafe { narrow(row.AdapterName.0.cast_const()) };
        let friendly_name = unsafe { wide(row.FriendlyName) };
        let description = unsafe { wide(row.Description) };
        let addresses = unsafe { unicast_addresses(row.FirstUnicastAddress)? };
        let dns_servers = unsafe { dns_addresses(row.FirstDnsServerAddress)? };
        let gateways = unsafe { gateway_addresses(row.FirstGatewayAddress)? };
        let route_gateways = default_routes
            .iter()
            .filter(|(index, _)| *index == unsafe { row.Anonymous1.Anonymous.IfIndex })
            .map(|(_, gateway)| gateway.clone())
            .collect();
        let vpn = is_vpn(row.IfType, &friendly_name, &description);
        rows.push(NetworkAdapter {
            guid,
            friendly_name,
            description,
            if_type: row.IfType,
            interface_index: unsafe { row.Anonymous1.Anonymous.IfIndex },
            addresses,
            dns_servers,
            gateways,
            default_routes: route_gateways,
            vpn,
        });
        current = row.Next;
    }
    Err(AppError::Platform {
        operation: "enumerate network adapters",
        reason: "the adapter list exceeded its safety bound".to_owned(),
    })
}

unsafe fn unicast_addresses(
    mut current: *const IP_ADAPTER_UNICAST_ADDRESS_LH,
) -> AppResult<Vec<String>> {
    let mut values = Vec::new();
    for _ in 0..MAX_LINKED_ROWS {
        if current.is_null() {
            return Ok(values);
        }
        let row = unsafe { &*current };
        if let Some(value) = unsafe { socket_address(&row.Address) } {
            values.push(format!("{value}/{}", row.OnLinkPrefixLength));
        }
        current = row.Next;
    }
    linked_list_too_long("read adapter IP addresses")
}

unsafe fn dns_addresses(
    mut current: *const IP_ADAPTER_DNS_SERVER_ADDRESS_XP,
) -> AppResult<Vec<String>> {
    let mut values = Vec::new();
    for _ in 0..MAX_LINKED_ROWS {
        if current.is_null() {
            return Ok(values);
        }
        let row = unsafe { &*current };
        if let Some(value) = unsafe { socket_address(&row.Address) } {
            values.push(value);
        }
        current = row.Next;
    }
    linked_list_too_long("read adapter DNS servers")
}

unsafe fn gateway_addresses(
    mut current: *const IP_ADAPTER_GATEWAY_ADDRESS_LH,
) -> AppResult<Vec<String>> {
    let mut values = Vec::new();
    for _ in 0..MAX_LINKED_ROWS {
        if current.is_null() {
            return Ok(values);
        }
        let row = unsafe { &*current };
        if let Some(value) = unsafe { socket_address(&row.Address) } {
            values.push(value);
        }
        current = row.Next;
    }
    linked_list_too_long("read adapter gateways")
}

unsafe fn socket_address(address: &SOCKET_ADDRESS) -> Option<String> {
    if address.lpSockaddr.is_null() || address.iSockaddrLength < 2 {
        return None;
    }
    let family = unsafe { (*address.lpSockaddr).sa_family };
    if family == AF_INET
        && usize::try_from(address.iSockaddrLength).ok()? >= std::mem::size_of::<SOCKADDR_IN>()
    {
        let row = unsafe { &*address.lpSockaddr.cast::<SOCKADDR_IN>() };
        let octets = unsafe { row.sin_addr.S_un.S_addr.to_ne_bytes() };
        Some(Ipv4Addr::from(octets).to_string())
    } else if family == AF_INET6
        && usize::try_from(address.iSockaddrLength).ok()? >= std::mem::size_of::<SOCKADDR_IN6>()
    {
        let row = unsafe { &*address.lpSockaddr.cast::<SOCKADDR_IN6>() };
        let octets = unsafe { row.sin6_addr.u.Byte };
        let value = Ipv6Addr::from(octets).to_string();
        let scope = unsafe { row.Anonymous.sin6_scope_id };
        Some(if scope == 0 {
            value
        } else {
            format!("{value}%{scope}")
        })
    } else {
        None
    }
}

fn default_routes(cancel: &CancelToken, deadline: Instant) -> AppResult<Vec<(u32, String)>> {
    let mut table = std::ptr::null_mut::<MIB_IPFORWARD_TABLE2>();
    let status = unsafe { GetIpForwardTable2(AF_UNSPEC, &mut table) };
    if status != ERROR_SUCCESS {
        return Err(platform_status("read the IP forward table", status.0));
    }
    let table = OwnedMibTable(table);
    if table.0.is_null() {
        return Ok(Vec::new());
    }
    let count = usize::try_from(unsafe { (*table.0).NumEntries }).unwrap_or(usize::MAX);
    if count > MAX_LINKED_ROWS {
        return Err(AppError::Platform {
            operation: "read the IP forward table",
            reason: format!("the route table reported {count} rows"),
        });
    }
    let rows = unsafe { std::slice::from_raw_parts((*table.0).Table.as_ptr(), count) };
    let mut routes = Vec::new();
    for row in rows {
        if cancelled_or_expired(cancel, deadline) {
            return interrupted();
        }
        if row.DestinationPrefix.PrefixLength == 0 {
            if let Some(next_hop) = unsafe { sockaddr_inet(&row.NextHop) } {
                routes.push((row.InterfaceIndex, next_hop));
            }
        }
    }
    Ok(routes)
}

unsafe fn sockaddr_inet(address: &SOCKADDR_INET) -> Option<String> {
    let family = unsafe { address.si_family };
    if family == AF_INET {
        let row = unsafe { address.Ipv4 };
        let octets = unsafe { row.sin_addr.S_un.S_addr.to_ne_bytes() };
        Some(Ipv4Addr::from(octets).to_string())
    } else if family == AF_INET6 {
        let row = unsafe { address.Ipv6 };
        let value = Ipv6Addr::from(unsafe { row.sin6_addr.u.Byte }).to_string();
        let scope = unsafe { row.Anonymous.sin6_scope_id };
        Some(if scope == 0 {
            value
        } else {
            format!("{value}%{scope}")
        })
    } else {
        None
    }
}

fn proxy_configuration() -> AppResult<ProxyConfiguration> {
    let key = open_internet_settings()?;
    Ok(ProxyConfiguration {
        enabled: registry_dword(key.get(), "ProxyEnable")?.unwrap_or(0) != 0,
        server: registry_string(key.get(), "ProxyServer")?,
        bypass: registry_string(key.get(), "ProxyOverride")?,
        auto_config_url: registry_string(key.get(), "AutoConfigURL")?,
    })
}

fn registry_dword(key: HKEY, name: &str) -> AppResult<Option<u32>> {
    let name = wide_string(name);
    let mut value = 0u32;
    let mut bytes = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            key,
            None,
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_DWORD | RRF_ZEROONFAILURE,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut bytes),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    require_success("read a proxy registry value", status.0)?;
    Ok(Some(value))
}

fn registry_string(key: HKEY, name: &str) -> AppResult<Option<String>> {
    let name = wide_string(name);
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
    require_success("measure a proxy registry value", status.0)?;
    if bytes == 0 {
        return Ok(None);
    }
    if bytes > MAX_REGISTRY_BYTES || bytes % 2 != 0 {
        return Err(AppError::Platform {
            operation: "read a proxy registry value",
            reason: format!("the value reported an invalid {bytes}-byte size"),
        });
    }
    let mut value = vec![0u16; usize::try_from(bytes / 2).unwrap_or(0)];
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
    require_success("read a proxy registry value", status.0)?;
    let decoded = decode_wide(&value);
    Ok((!decoded.is_empty()).then_some(decoded))
}

fn is_vpn(if_type: u32, friendly_name: &str, description: &str) -> bool {
    if if_type == IF_TYPE_TUNNEL {
        return true;
    }
    let haystack = format!("{friendly_name} {description}").to_ascii_lowercase();
    const TOKENS: &[&str] = &[
        "wireguard",
        "openvpn",
        "tailscale",
        "nordvpn",
        "protonvpn",
        "mullvad",
        "anyconnect",
        "globalprotect",
        "fortinet",
        "zerotier",
    ];
    TOKENS.iter().any(|token| haystack.contains(token))
}

struct OwnedMibTable(*mut MIB_IPFORWARD_TABLE2);

impl Drop for OwnedMibTable {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { FreeMibTable(self.0.cast()) };
        }
    }
}

struct WakeRegistration {
    ip: Option<IpHelperRegistration>,
    registry: Option<RegistryRegistration>,
}

impl Drop for WakeRegistration {
    fn drop(&mut self) {
        // Fields own unregister/join. Reading them here documents that both
        // registrations stay live for exactly the provider registration.
        let _ = (&self.ip, &self.registry);
    }
}

struct WakeContext {
    wake: WakeHandle,
    cancel: CancelToken,
}

static WAKE_CONTEXTS: OnceLock<Mutex<HashMap<usize, Arc<WakeContext>>>> = OnceLock::new();
static NEXT_WAKE_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn wake_contexts() -> &'static Mutex<HashMap<usize, Arc<WakeContext>>> {
    WAKE_CONTEXTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_wake_context(context: WakeContext) -> usize {
    loop {
        let context_id = NEXT_WAKE_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
        if context_id == 0 {
            continue;
        }
        wake_contexts()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(context_id, Arc::new(context));
        return context_id;
    }
}

fn remove_wake_context(context_id: usize) {
    wake_contexts()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&context_id);
}

struct IpHelperRegistration {
    handles: Vec<HANDLE>,
    context_id: usize,
}

unsafe impl Send for IpHelperRegistration {}

impl IpHelperRegistration {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        let context_id = register_wake_context(WakeContext { wake, cancel });
        let context_ptr = context_id as *const c_void;
        let mut handles = Vec::with_capacity(2);

        let mut interface = HANDLE::default();
        let interface_status = unsafe {
            NotifyIpInterfaceChange(
                AF_UNSPEC,
                Some(interface_changed),
                Some(context_ptr),
                false,
                &mut interface,
            )
        };
        if interface_status == ERROR_SUCCESS {
            handles.push(interface);
        }

        let mut route = HANDLE::default();
        let route_status = unsafe {
            NotifyRouteChange2(
                AF_UNSPEC,
                Some(route_changed),
                context_ptr,
                false,
                &mut route,
            )
        };
        if route_status == ERROR_SUCCESS {
            handles.push(route);
        }
        if handles.is_empty() {
            remove_wake_context(context_id);
            return Err(AppError::Platform {
                operation: "register IP Helper network notifications",
                reason: format!(
                    "interface=0x{:08x}, route=0x{:08x}",
                    interface_status.0, route_status.0
                ),
            });
        }
        Ok(Self {
            handles,
            context_id,
        })
    }
}

impl Drop for IpHelperRegistration {
    fn drop(&mut self) {
        for handle in self.handles.drain(..) {
            unsafe {
                let _ = CancelMibChangeNotify2(handle);
            }
        }
        // Native callbacks receive only this opaque id. A callback already in
        // flight owns an Arc; a late callback after any failed cancel sees no
        // registry entry, so context memory is never dereferenced after free.
        remove_wake_context(self.context_id);
    }
}

unsafe extern "system" fn interface_changed(
    context: *const c_void,
    _row: *const MIB_IPINTERFACE_ROW,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        request_wake(context);
    }));
}

unsafe extern "system" fn route_changed(
    context: *const c_void,
    _row: *const MIB_IPFORWARD_ROW2,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        request_wake(context);
    }));
}

unsafe fn request_wake(context: *const c_void) {
    let context_id = context as usize;
    let context = wake_contexts()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&context_id)
        .cloned();
    if let Some(context) = context.filter(|context| !context.cancel.is_cancelled()) {
        let _ = context.wake.request_snapshot();
    }
}

struct RegistryRegistration {
    stop: OwnedHandle,
    thread: Option<JoinHandle<()>>,
}

impl RegistryRegistration {
    fn start(wake: WakeHandle, cancel: CancelToken) -> AppResult<Self> {
        let key = open_internet_settings()?;
        let notify = OwnedHandle(
            unsafe { CreateEventW(None, false, false, None) }
                .map_err(|source| platform_error("create a proxy notification event", source))?,
        );
        let stop = OwnedHandle(
            unsafe { CreateEventW(None, false, false, None) }
                .map_err(|source| platform_error("create a proxy stop event", source))?,
        );
        arm_registry_notification(key.get(), notify.get())?;
        let stop_raw = stop.raw();
        let thread = thread::Builder::new()
            .name("network-proxy-wake".to_owned())
            .spawn(move || run_registry_watch(key, notify, stop_raw, wake, cancel))
            .map_err(|source| AppError::Platform {
                operation: "start a proxy notification thread",
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
                tracing::debug!("waiting for a proxy registry notification failed");
            }
            break;
        }
    }
}

fn open_internet_settings() -> AppResult<OwnedKey> {
    let path = wide_string(INTERNET_SETTINGS);
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            KEY_READ,
            &mut key,
        )
    };
    require_success("open current-user Internet Settings", status.0)?;
    Ok(OwnedKey(key))
}

fn arm_registry_notification(key: HKEY, event: HANDLE) -> AppResult<()> {
    let status = unsafe {
        RegNotifyChangeKeyValue(
            key,
            false,
            REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
            Some(event),
            true,
        )
    };
    require_success("register an Internet Settings notification", status.0)
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

unsafe fn narrow(pointer: *const u8) -> String {
    if pointer.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(pointer.cast()) }
        .to_string_lossy()
        .trim()
        .to_owned()
}

unsafe fn wide(pointer: PWSTR) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0usize;
    while length < 32_768 && unsafe { *pointer.0.add(length) } != 0 {
        length += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(pointer.0, length) };
    decode_wide(slice)
}

fn decode_wide(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|word| *word == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end]).trim().to_owned()
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn cancelled_or_expired(cancel: &CancelToken, deadline: Instant) -> bool {
    cancel.is_cancelled() || Instant::now() >= deadline
}

fn interrupted<T>() -> AppResult<T> {
    Err(AppError::Platform {
        operation: "enumerate network configuration",
        reason: "the network sweep was cancelled or exceeded its deadline".to_owned(),
    })
}

fn linked_list_too_long<T>(operation: &'static str) -> AppResult<T> {
    Err(AppError::Platform {
        operation,
        reason: "the platform list exceeded its safety bound".to_owned(),
    })
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
mod tests {
    use super::*;

    #[test]
    fn vpn_detection_does_not_treat_hyper_v_as_a_vpn() {
        assert!(!is_vpn(
            6,
            "vEthernet (Default Switch)",
            "Hyper-V Virtual Ethernet Adapter"
        ));
        assert!(is_vpn(IF_TYPE_TUNNEL, "Tunnel", "Microsoft tunnel"));
        assert!(is_vpn(6, "Tailscale", "virtual adapter"));
    }

    #[test]
    #[allow(clippy::panic)]
    fn native_wake_callback_contains_a_panicking_runtime_closure() {
        let context_id = register_wake_context(WakeContext {
            wake: WakeHandle::new(|| -> AppResult<()> { panic!("fixture wake panic") }),
            cancel: CancelToken::default(),
        });
        unsafe {
            interface_changed(
                context_id as *const c_void,
                std::ptr::null(),
                MIB_NOTIFICATION_TYPE::default(),
            );
        }
        remove_wake_context(context_id);
    }
}
