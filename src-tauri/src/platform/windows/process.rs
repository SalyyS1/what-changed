//! Process enumeration through the ToolHelp snapshot API.
//!
//! Two rules shape this module. Every handle goes through a guard that closes it
//! on drop, because the enumeration runs once per second for as long as the app
//! is open and one leaked handle per process would be visible in Task Manager
//! within an hour. And a partial table is an error, never a short `Vec`: the
//! collector diffs whatever it is given, so a truncated walk would forge a stop
//! for every process past the point where reading failed.

use std::sync::{Mutex, MutexGuard, OnceLock};

use windows::core::PWSTR;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_FILES, FILETIME, HANDLE,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessHandleCount, GetProcessTimes, OpenProcess,
    QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::error::{AppError, AppResult};
use crate::platform::traits::{clear_process_rows, ProcessProvider, ProcessRow};

/// The widest path `QueryFullProcessImageNameW` can return, in UTF-16 units.
///
/// The provider grows its reusable buffer to this limit only when the operating
/// system reports that a real path needs more space.
const MAX_PATH_BUFFER_CHARS: usize = 32_767;

/// Most Windows executable paths fit comfortably in this reusable buffer. The
/// long-path limit remains available through the bounded retry in `read_path`.
const INITIAL_PATH_BUFFER_CHARS: usize = 4_096;

/// Rows to reserve before the first entry is read. The development machine
/// reports about 370 processes, so this avoids a handful of reallocations per
/// second without guessing high.
const RESERVE_ROWS: usize = 512;

/// `CreateToolhelp32Snapshot` documents `ERROR_BAD_LENGTH` as "try again", so
/// it gets a bounded retry rather than failing a cycle on a transient.
const SNAPSHOT_ATTEMPTS: usize = 3;

/// Reads the process table through a ToolHelp snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowsProcessProvider;

/// Heap workspace shared by the process provider's serial polling worker.
///
/// The row allocation belongs to the caller and is reused by the collector.
/// The UTF-16 path workspace is also reused, starting at a typical path size
/// and growing only for a process that actually needs the extended limit.
struct ProcessScratch {
    path: Vec<u16>,
}

impl Default for ProcessScratch {
    fn default() -> Self {
        Self {
            path: vec![0; INITIAL_PATH_BUFFER_CHARS],
        }
    }
}

fn process_scratch() -> &'static Mutex<ProcessScratch> {
    static SCRATCH: OnceLock<Mutex<ProcessScratch>> = OnceLock::new();
    SCRATCH.get_or_init(|| Mutex::new(ProcessScratch::default()))
}

fn lock_scratch() -> MutexGuard<'static, ProcessScratch> {
    process_scratch()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ProcessProvider for WindowsProcessProvider {
    fn enumerate(&self) -> AppResult<Vec<ProcessRow>> {
        let mut rows = Vec::with_capacity(RESERVE_ROWS);
        self.enumerate_into(&mut rows)?;
        Ok(rows)
    }

    fn enumerate_into(&self, output: &mut Vec<ProcessRow>) -> AppResult<()> {
        clear_process_rows(output);
        let result = enumerate_process_rows(output);
        if result.is_err() {
            // A failed ToolHelp walk may already have appended rows. Keeping
            // that prefix would retain paths and makes it too easy for a later
            // caller to mistake a partial snapshot for a complete one.
            clear_process_rows(output);
        }
        result
    }
}

/// Append one complete ToolHelp walk to caller-owned storage.
fn enumerate_process_rows(rows: &mut Vec<ProcessRow>) -> AppResult<()> {
    let snapshot = OwnedHandle(take_snapshot()?);
    let mut scratch = lock_scratch();
    let probe_processes = crate::platform::windows::tool_probe::process_exclusions();
    let has_probe_processes = !probe_processes.is_empty();

    let mut entry = new_entry();
    if let Err(source) = unsafe { Process32FirstW(snapshot.get(), &mut entry) } {
        if is_no_more_files(&source) {
            return Ok(());
        }
        return Err(provider_error("read the first process entry", source));
    }
    loop {
        rows.push(row_from(&entry, &mut scratch.path));
        entry = new_entry();
        match unsafe { Process32NextW(snapshot.get(), &mut entry) } {
            Ok(()) => {}
            Err(source) => {
                if is_no_more_files(&source) {
                    break;
                }
                return Err(provider_error("walk the process table", source));
            }
        }
    }
    if has_probe_processes {
        rows.retain(|row| !probe_processes.excludes(row.pid, row.executable_path.as_deref()));
    }
    Ok(())
}

/// Open a process snapshot, retrying the one error the API documents as
/// transient.
fn take_snapshot() -> AppResult<HANDLE> {
    let mut last = None;
    for _ in 0..SNAPSHOT_ATTEMPTS {
        match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
            Ok(handle) => return Ok(handle),
            Err(source) => last = Some(source),
        }
    }
    Err(last.map_or_else(
        || AppError::Platform {
            operation: "take a process snapshot",
            reason: "no attempt reported a reason".to_owned(),
        },
        |source| provider_error("take a process snapshot", source),
    ))
}

/// A zeroed entry with its size field set, which is what the walk functions
/// require before they will write into it.
fn new_entry() -> PROCESSENTRY32W {
    PROCESSENTRY32W {
        dwSize: u32::try_from(std::mem::size_of::<PROCESSENTRY32W>()).unwrap_or(u32::MAX),
        ..Default::default()
    }
}

/// One snapshot entry, enriched with what a limited handle can tell us.
fn row_from(entry: &PROCESSENTRY32W, path_buffer: &mut Vec<u16>) -> ProcessRow {
    let details = open_details(entry.th32ProcessID, path_buffer);
    ProcessRow {
        pid: entry.th32ProcessID,
        parent_pid: entry.th32ParentProcessID,
        image_name: wide_to_string(&entry.szExeFile),
        executable_path: details.path,
        creation_time: details.creation,
    }
}

/// What a `PROCESS_QUERY_LIMITED_INFORMATION` handle can tell us about one pid.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProcessDetails {
    path: Option<String>,
    creation: Option<u64>,
}

/// Open one process for the least access that answers both questions.
///
/// Access denied here is the common case rather than a failure: a non-elevated
/// reader cannot see into roughly half the processes on a normal machine, and
/// the pids it can never open (0 and 4) are denied to everyone. Nothing is
/// logged per process. The collector keeps one aggregate count and reports it
/// once an hour at DEBUG, because a line per denial would be forty lines a
/// second saying nothing new.
fn open_details(pid: u32, buffer: &mut Vec<u16>) -> ProcessDetails {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return ProcessDetails::default();
    };
    let handle = OwnedHandle(handle);
    ProcessDetails {
        path: read_path(handle.get(), buffer),
        creation: read_creation(handle.get()),
    }
}

/// The full image path, or `None` when the read is denied or returns nothing.
fn read_path(handle: HANDLE, buffer: &mut Vec<u16>) -> Option<String> {
    loop {
        let mut length = u32::try_from(buffer.len()).unwrap_or(u32::MAX);
        let result = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        };
        if let Err(source) = result {
            if source.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult()
                && buffer.len() < MAX_PATH_BUFFER_CHARS
            {
                // The API reports the required size when the initial buffer is
                // too small. Clear any partial text before reusing the memory,
                // then grow by the larger of the reported size and a doubling
                // step. The cap keeps a malformed requirement from looping.
                let required = usize::try_from(length).unwrap_or(0);
                let next = required
                    .max(buffer.len().saturating_mul(2))
                    .min(MAX_PATH_BUFFER_CHARS);
                buffer.fill(0);
                if next > buffer.len() {
                    buffer.resize(next, 0);
                    continue;
                }
            }
            // Windows does not promise the destination is untouched on a
            // non-size failure, including access denial.
            buffer.fill(0);
            return None;
        }
        let written = usize::try_from(length).unwrap_or(0).min(buffer.len());
        if written == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..written]);
        // The reusable allocation must not become an accidental long-lived copy
        // of the last executable path. Clearing only the written range retains
        // the allocation win without retaining path text between calls.
        buffer[..written].fill(0);
        return Some(path);
    }
}

/// Creation time as the raw `FILETIME`, which is what makes a pid reusable
/// without becoming ambiguous.
fn read_creation(handle: HANDLE) -> Option<u64> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }.ok()?;
    Some((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}

/// A NUL-terminated UTF-16 buffer as text, tolerating a missing terminator.
fn wide_to_string(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// Whether the walk ended because the table ran out, which is success.
fn is_no_more_files(source: &windows::core::Error) -> bool {
    source.code() == ERROR_NO_MORE_FILES.to_hresult()
}

/// Report the platform's own code and nothing else.
///
/// An error message is a log line, and the text Windows attaches to a failed
/// process call can name the executable it was about.
fn provider_error(operation: &'static str, source: windows::core::Error) -> AppError {
    AppError::Platform {
        operation,
        reason: format!("{:#010x}", source.code().0),
    }
}

/// A handle that closes itself.
struct OwnedHandle(HANDLE);

impl OwnedHandle {
    const fn get(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // Closing is best-effort. There is nowhere useful to report a failure
        // from a destructor, and a log line per process per second would cost
        // more than the failure it describes.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Handles this process holds, for the leak regression that asserts a stable
/// count across many enumerations.
pub fn current_handle_count() -> Option<u32> {
    let mut count = 0u32;
    unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.ok()?;
    Some(count)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Enumerations to run in the leak regression.
    ///
    /// Five hundred walks of a ~370-process machine open and close well over
    /// 100,000 handles, which is the quantity the guard has to survive; running
    /// ten thousand walks would prove the same thing over a minute of wall
    /// clock.
    const LEAK_WALKS: usize = 500;

    #[test]
    fn a_snapshot_of_this_machine_names_the_process_reading_it() {
        let provider = WindowsProcessProvider;
        let rows = provider.enumerate().unwrap();
        assert!(
            rows.len() > 10,
            "a real machine runs more than ten processes"
        );
        let self_pid = std::process::id();
        let myself = rows
            .iter()
            .find(|row| row.pid == self_pid)
            .expect("this process is in its own snapshot");
        assert!(
            myself.executable_path.is_some(),
            "a process can always read its own path"
        );
        assert!(myself.creation_time.is_some());
        assert!(!myself.image_name.is_empty());
    }

    #[test]
    fn no_pid_appears_twice_in_one_snapshot() {
        let provider = WindowsProcessProvider;
        let mut pids: Vec<u32> = provider
            .enumerate()
            .unwrap()
            .into_iter()
            .map(|row| row.pid)
            .collect();
        let total = pids.len();
        pids.sort_unstable();
        pids.dedup();
        assert_eq!(pids.len(), total, "a pid identifies one live process");
    }

    #[test]
    fn a_path_denial_is_reported_as_absence_rather_than_as_a_failure() {
        // Roughly 46% of a normal machine denies the path read. The assertion
        // is that both outcomes occur and that neither is an error, which is
        // the whole degradation contract.
        let provider = WindowsProcessProvider;
        let rows = provider.enumerate().unwrap();
        assert!(rows.iter().any(|row| row.executable_path.is_some()));
        assert!(
            rows.iter().any(|row| row.executable_path.is_none()),
            "system processes deny a non-elevated path read"
        );
        for row in rows.iter().filter(|row| row.executable_path.is_none()) {
            assert!(!row.image_name.is_empty());
        }
    }

    #[test]
    fn a_wide_buffer_becomes_text_at_its_terminator() {
        let mut buffer = [0u16; 8];
        buffer[..3].copy_from_slice(&[0x61, 0x62, 0x63]);
        assert_eq!(wide_to_string(&buffer), "abc");
        let unterminated = [0x78u16; 4];
        assert_eq!(wide_to_string(&unterminated), "xxxx");
    }

    #[test]
    fn repeated_snapshots_reuse_the_large_path_workspace() {
        let provider = WindowsProcessProvider;
        let before = {
            let scratch = lock_scratch();
            (scratch.path.as_ptr(), scratch.path.capacity())
        };
        provider.enumerate().unwrap();
        provider.enumerate().unwrap();
        let after = {
            let scratch = lock_scratch();
            (scratch.path.as_ptr(), scratch.path.capacity())
        };
        assert_eq!(before, after);
        assert_eq!(after.1, INITIAL_PATH_BUFFER_CHARS);
    }

    #[test]
    fn caller_owned_row_storage_is_reused_across_snapshots() {
        let provider = WindowsProcessProvider;
        let mut rows = Vec::with_capacity(RESERVE_ROWS);
        provider.enumerate_into(&mut rows).unwrap();
        let first = (rows.as_ptr(), rows.capacity());
        provider.enumerate_into(&mut rows).unwrap();
        assert_eq!(
            (rows.as_ptr(), rows.capacity()),
            first,
            "a steady process table stays in the caller's original allocation"
        );
    }

    #[test]
    fn enumerating_repeatedly_does_not_accumulate_handles() {
        let provider = WindowsProcessProvider;
        let before = current_handle_count().expect("our own handle count is readable");
        for _ in 0..LEAK_WALKS {
            assert!(provider.enumerate().is_ok());
        }
        let after = current_handle_count().expect("our own handle count is readable");
        // A guard that missed one path would add thousands here. A small drift
        // is the test harness's own bookkeeping, not this module's.
        assert!(
            after <= before + 32,
            "handle count grew from {before} to {after} across {LEAK_WALKS} enumerations"
        );
    }
}
