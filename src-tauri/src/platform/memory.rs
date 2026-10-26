//! Process memory readings used by Diagnostics.

#[cfg(windows)]
pub fn current_working_set_bytes() -> u64 {
    crate::platform::windows::memory::current_working_set_bytes()
}

#[cfg(windows)]
pub fn peak_working_set_bytes() -> u64 {
    crate::platform::windows::memory::peak_working_set_bytes()
}

#[cfg(not(windows))]
pub const fn current_working_set_bytes() -> u64 {
    0
}

#[cfg(not(windows))]
pub const fn peak_working_set_bytes() -> u64 {
    0
}
