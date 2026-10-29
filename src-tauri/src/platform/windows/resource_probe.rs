//! Process/resource sampling used only by the Phase 20 devtools binary.
//!
//! This is kept under the Windows platform boundary so the binary itself does
//! not become a second Win32 call site. Paths and command lines are never read.

use std::collections::BTreeMap;
use std::mem::size_of;
use std::time::Instant;

use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessHandleCount, GetProcessTimes, OpenProcess,
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

/// One privacy-safe process-tree sample.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceSample {
    pub process_count: usize,
    pub total_working_set_bytes: u64,
    pub private_working_set_bytes: Option<u64>,
    pub cpu_percent: Option<f64>,
    pub handle_count: Option<u32>,
    pub cpu_time_100ns: u64,
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn process_times(handle: HANDLE) -> Option<u64> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
        .ok()
        .map(|_| filetime_value(kernel).saturating_add(filetime_value(user)))
}

fn filetime_value(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

fn memory(handle: HANDLE) -> Option<u64> {
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?,
        ..Default::default()
    };
    unsafe {
        GetProcessMemoryInfo(
            handle,
            &mut counters,
            u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?,
        )
    }
    .ok()
    .map(|_| counters.WorkingSetSize as u64)
}

fn process_tree() -> BTreeMap<u32, Vec<u32>> {
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return children;
    };
    let snapshot = OwnedHandle(snapshot);
    let mut entry = PROCESSENTRY32W {
        dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(u32::MAX),
        ..Default::default()
    };
    let Ok(()) = (unsafe { Process32FirstW(snapshot.0, &mut entry) }) else {
        return children;
    };
    loop {
        children
            .entry(entry.th32ParentProcessID)
            .or_default()
            .push(entry.th32ProcessID);
        entry = PROCESSENTRY32W {
            dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(u32::MAX),
            ..Default::default()
        };
        if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
            break;
        }
    }
    children
}

fn descendants(children: &BTreeMap<u32, Vec<u32>>, root: u32) -> Vec<u32> {
    let mut result = vec![root];
    let mut cursor = 0;
    while let Some(pid) = result.get(cursor).copied() {
        cursor += 1;
        if let Some(entries) = children.get(&pid) {
            result.extend(entries.iter().copied());
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}

fn open_process(pid: u32) -> Option<OwnedHandle> {
    let access = PROCESS_QUERY_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ;
    unsafe { OpenProcess(access, false, pid) }
        .ok()
        .map(OwnedHandle)
}

/// Sample the current process and every descendant visible to the token.
pub fn sample(previous: Option<(Instant, u64)>) -> ResourceSample {
    let root = std::process::id();
    let pids = descendants(&process_tree(), root);
    let mut total_memory = 0u64;
    let mut total_cpu = 0u64;
    let mut readable = 0usize;
    for pid in pids.iter().copied() {
        let Some(handle) = open_process(pid) else {
            continue;
        };
        total_memory = total_memory.saturating_add(memory(handle.0).unwrap_or(0));
        total_cpu = total_cpu.saturating_add(process_times(handle.0).unwrap_or(0));
        readable += 1;
    }
    let cpu_percent = previous.and_then(|(started, old_cpu)| {
        // FILETIME uses 100-nanosecond ticks.  Convert wall-clock nanoseconds
        // with division (not multiplication) before calculating the process
        // tree's CPU share.
        let elapsed_100ns = started.elapsed().as_nanos() as f64 / 100.0;
        if elapsed_100ns <= 0.0 {
            return None;
        }
        Some((total_cpu.saturating_sub(old_cpu) as f64 / elapsed_100ns) * 100.0)
    });
    let handle_count = unsafe {
        let mut count = 0u32;
        GetProcessHandleCount(GetCurrentProcess(), &mut count)
            .ok()
            .map(|_| count)
    };
    ResourceSample {
        process_count: readable,
        total_working_set_bytes: total_memory,
        // The low-level counter exposes total working set only. Keep this
        // absent rather than substituting commit size for private working set.
        private_working_set_bytes: None,
        cpu_percent,
        handle_count,
        cpu_time_100ns: total_cpu,
    }
}

pub fn initial_cpu() -> u64 {
    descendants(&process_tree(), std::process::id())
        .into_iter()
        .filter_map(open_process)
        .filter_map(|handle| process_times(handle.0))
        .sum()
}
