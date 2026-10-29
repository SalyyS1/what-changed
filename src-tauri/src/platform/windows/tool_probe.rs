//! Bounded, shell-free version probing for the fixed developer-tool allowlist.

use std::collections::BTreeMap;
use std::io::{Read, Result as IoResult};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::domain::tool::{definition, TOOL_ALLOWLIST};

const PER_TOOL_TIMEOUT: Duration = Duration::from_secs(2);
// Leave scheduler and process-termination margin for the public 1.5 second
// wall-clock contract, including on a contended Windows host.
const TOTAL_SCAN_BUDGET: Duration = Duration::from_millis(1_200);
const MAX_OUTPUT_BYTES: usize = 256;
const PROCESS_EXCLUSION_GRACE: Duration = Duration::from_secs(5);

static HAS_PROBE_PROCESSES: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
struct ProbeProcess {
    executable: Option<String>,
    expires_at: Instant,
}

fn probe_processes() -> &'static Mutex<BTreeMap<u32, ProbeProcess>> {
    static PROCESSES: OnceLock<Mutex<BTreeMap<u32, ProbeProcess>>> = OnceLock::new();
    PROCESSES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn register_probe_process(pid: u32, path: &Path) {
    let executable = path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| extension.eq_ignore_ascii_case("exe"))
        .map(|_| path.to_string_lossy().into_owned());
    let mut processes = probe_processes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    processes.insert(
        pid,
        ProbeProcess {
            executable,
            expires_at: Instant::now() + PROCESS_EXCLUSION_GRACE,
        },
    );
    HAS_PROBE_PROCESSES.store(true, Ordering::Release);
}

/// Immutable probe-process view acquired once per process-table walk.
///
/// Process enumeration is on a one-second hot path. Taking the registry lock
/// once here avoids one mutex acquisition and full-map expiry sweep per row.
pub(crate) struct ProcessExclusions {
    processes: BTreeMap<u32, Option<String>>,
}

impl ProcessExclusions {
    pub(crate) fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }

    pub(crate) fn excludes(&self, pid: u32, executable_path: Option<&str>) -> bool {
        self.processes.get(&pid).is_some_and(|expected| {
            expected.as_deref().is_none_or(|expected| {
                executable_path.is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
            })
        })
    }
}

pub(crate) fn process_exclusions() -> ProcessExclusions {
    if !HAS_PROBE_PROCESSES.load(Ordering::Acquire) {
        return ProcessExclusions {
            processes: BTreeMap::new(),
        };
    }
    let now = Instant::now();
    let mut processes = probe_processes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    processes.retain(|_, process| process.expires_at > now);
    HAS_PROBE_PROCESSES.store(!processes.is_empty(), Ordering::Release);
    ProcessExclusions {
        processes: processes
            .iter()
            .map(|(pid, process)| (*pid, process.executable.clone()))
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolSnapshot {
    pub path: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ToolProbe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProbeResult {
    pub tools: BTreeMap<String, ToolSnapshot>,
    pub complete: bool,
}

impl ToolProbe {
    pub fn scan(&self, path_value: &str) -> ToolProbeResult {
        let started = Instant::now();
        let deadline = started + TOTAL_SCAN_BUDGET;
        let directories = parse_path(path_value);
        let mut found = BTreeMap::new();
        let mut complete = true;
        for tool in TOOL_ALLOWLIST {
            let Some(path) = resolve_before(&directories, tool.name, Some(deadline)) else {
                if Instant::now() >= deadline {
                    complete = false;
                    break;
                }
                continue;
            };
            let version = TOTAL_SCAN_BUDGET
                .checked_sub(started.elapsed())
                .and_then(|remaining| {
                    self.probe_resolved(tool.name, &path, remaining.min(PER_TOOL_TIMEOUT))
                });
            found.insert(
                tool.name.to_owned(),
                ToolSnapshot {
                    path: path.to_string_lossy().into_owned(),
                    version,
                },
            );
        }
        ToolProbeResult {
            tools: found,
            complete,
        }
    }

    pub fn probe_resolved(&self, name: &str, path: &Path, timeout: Duration) -> Option<String> {
        let tool = definition(name)?;
        if !spawnable_resolved_name(path, name) {
            return None;
        }
        let output = run_bounded(path, timeout).ok()?;
        tool.extract_version(&output)
    }
}

pub fn parse_path(value: &str) -> Vec<PathBuf> {
    crate::domain::tool::path_entries(value)
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

pub fn resolve(directories: &[PathBuf], name: &str) -> Option<PathBuf> {
    resolve_before(directories, name, None)
}

fn resolve_before(
    directories: &[PathBuf],
    name: &str,
    deadline: Option<Instant>,
) -> Option<PathBuf> {
    definition(name)?;
    for directory in directories {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return None;
        }
        if is_remote_or_reparse(directory) {
            continue;
        }
        for extension in ["exe", "cmd"] {
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return None;
            }
            let candidate = directory.join(format!("{name}.{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn spawnable_resolved_name(path: &Path, name: &str) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    // CreateProcessW cannot execute `.cmd` without a command interpreter.
    // Shell execution would make PATH contents executable syntax, so scripts
    // are deliberately reported with an Unknown version and never spawned.
    file_name.eq_ignore_ascii_case(&format!("{name}.exe"))
}

fn is_unc(path: &Path) -> bool {
    path.to_string_lossy().starts_with(r"\\")
}

fn is_remote_or_reparse(path: &Path) -> bool {
    if is_unc(path) || is_remote_drive(path) {
        return true;
    }
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes()
            & windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT.0
            != 0
    })
}

fn is_remote_drive(path: &Path) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;

    let text = path.to_string_lossy().as_bytes().to_vec();
    if text.len() < 2 || text[1] != b':' || !text[0].is_ascii_alphabetic() {
        return false;
    }
    let root = [
        u16::from(text[0].to_ascii_uppercase()),
        u16::from(b':'),
        u16::from(b'\\'),
        0,
    ];
    // Win32's `DRIVE_REMOTE` value. The typed constant lives in the broad
    // WindowsProgramming feature; keeping the numeric ABI value here avoids
    // enabling an unrelated API family for one comparison.
    const DRIVE_REMOTE_VALUE: u32 = 4;
    unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE_VALUE }
}

fn run_bounded(path: &Path, timeout: Duration) -> IoResult<String> {
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    register_probe_process(child.id(), path);
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let mut bytes = Vec::with_capacity(MAX_OUTPUT_BYTES);
    let mut error = Vec::with_capacity(MAX_OUTPUT_BYTES);
    let process_handle = windows::Win32::Foundation::HANDLE(child.as_raw_handle());
    loop {
        // Anonymous child pipes have one reader. Peek first, then read only
        // bytes already available: inherited write handles cannot block us.
        read_available(&mut stdout, &mut bytes);
        read_available(&mut stderr, &mut error);
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            let _ = child.kill();
            return Ok(String::new());
        };
        let timeout_ms = u32::try_from(remaining.as_millis().min(5)).unwrap_or(5);
        let wait = unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(process_handle, timeout_ms)
        };
        if wait == windows::Win32::Foundation::WAIT_OBJECT_0 {
            // The parent can leave final output immediately before exit.
            read_available(&mut stdout, &mut bytes);
            read_available(&mut stderr, &mut error);
            break;
        }
        if wait != windows::Win32::Foundation::WAIT_TIMEOUT {
            let source = std::io::Error::last_os_error();
            let _ = child.kill();
            return Err(source);
        }
    }
    let _ = child.wait()?;
    let remaining = MAX_OUTPUT_BYTES.saturating_sub(bytes.len());
    error.truncate(remaining);
    bytes.extend(error);
    bytes.truncate(MAX_OUTPUT_BYTES);
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_available<R: Read + AsRawHandle>(reader: &mut Option<R>, output: &mut Vec<u8>) {
    let Some(pipe) = reader.as_mut() else {
        return;
    };
    let mut available = 0u32;
    // The ChildStdout/ChildStderr owns a valid synchronous pipe handle and
    // this function is its only reader. No read can race away peeked bytes.
    let peek = unsafe {
        windows::Win32::System::Pipes::PeekNamedPipe(
            windows::Win32::Foundation::HANDLE(pipe.as_raw_handle()),
            None,
            0,
            None,
            Some(&mut available),
            None,
        )
    };
    if peek.is_err() {
        *reader = None;
        return;
    }
    let mut buffer = [0u8; MAX_OUTPUT_BYTES];
    let wanted = (available as usize).min(buffer.len());
    if wanted == 0 {
        return;
    }
    match pipe.read(&mut buffer[..wanted]) {
        Ok(0) | Err(_) => *reader = None,
        Ok(read) => {
            let kept = read.min(MAX_OUTPUT_BYTES.saturating_sub(output.len()));
            output.extend_from_slice(&buffer[..kept]);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn path_parser_respects_quotes_and_ignores_empty_entries() {
        assert_eq!(
            parse_path(r#""C:\tools;stable";C:\bin;;C:\BIN;"#),
            vec![PathBuf::from(r"C:\tools;stable"), PathBuf::from(r"C:\bin")]
        );
    }

    #[test]
    fn resolution_refuses_names_outside_the_allowlist() {
        let directory = crate::testing::temp_dir("tool-probe-refusal");
        std::fs::write(directory.join("powershell.exe"), b"").unwrap();
        assert!(resolve(&[directory], "powershell").is_none());
    }

    #[test]
    fn registered_probe_pid_is_excluded_only_for_its_executable() {
        let pid = u32::MAX - 91;
        register_probe_process(pid, Path::new(r"C:\tools\node.exe"));
        let snapshot = process_exclusions();
        assert!(snapshot.excludes(pid, Some(r"c:\TOOLS\NODE.EXE")));
        assert!(!snapshot.excludes(pid, Some(r"C:\tools\unrelated.exe")));
        assert!(!snapshot.excludes(pid - 1, Some(r"C:\tools\node.exe")));
    }
}
