//! Windows process memory provider. All Win32 calls stay behind this boundary.

fn counters() -> Option<windows::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS> {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    // SAFETY: the structure is initialized with its documented size and the
    // current-process pseudo handle is valid for this read-only query.
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .ok()?;
    }
    Some(counters)
}

pub fn current_working_set_bytes() -> u64 {
    counters()
        .map(|value| value.WorkingSetSize as u64)
        .unwrap_or(0)
}

pub fn peak_working_set_bytes() -> u64 {
    counters()
        .map(|value| value.PeakWorkingSetSize as u64)
        .unwrap_or(0)
}
