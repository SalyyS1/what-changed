//! Assign the suspended child before it can create a single descendant.
use std::fs::File;
use std::io::{self, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::sync::Mutex;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{SetHandleInformation, HANDLE, HANDLE_FLAGS, HANDLE_FLAG_INHERIT};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::JobObjects::*;
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::*;

fn handle(value: &OwnedHandle) -> HANDLE {
    HANDLE(value.as_raw_handle())
}
fn owned(value: HANDLE) -> OwnedHandle {
    unsafe { OwnedHandle::from_raw_handle(value.0) }
}
fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(Some(0)).collect()
}
fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let mut read = HANDLE::default();
    let mut write = HANDLE::default();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        bInheritHandle: true.into(),
        ..Default::default()
    };
    unsafe {
        CreatePipe(&mut read, &mut write, Some(&attributes), 0)?;
    }
    Ok((owned(read), owned(write)))
}

struct Attributes {
    _storage: Vec<usize>,
    _jobs: Vec<HANDLE>,
    list: LPPROC_THREAD_ATTRIBUTE_LIST,
}
impl Attributes {
    fn new(handles: &mut [HANDLE], job: HANDLE) -> io::Result<Self> {
        let mut bytes = 0;
        unsafe {
            let _ = InitializeProcThreadAttributeList(None, 2, None, &mut bytes);
        }
        let mut storage = vec![0usize; bytes.div_ceil(std::mem::size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        unsafe {
            InitializeProcThreadAttributeList(Some(list), 2, None, &mut bytes)?;
        }
        let result = Self {
            _storage: storage,
            _jobs: vec![job],
            list,
        };
        unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                Some(handles.as_ptr().cast()),
                std::mem::size_of_val(handles),
                None,
                None,
            )?;
        }
        // Atomic job membership closes the parent-crash gap before suspended
        // assignment. The explicit assignment remains checked before resume.
        unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                Some(result._jobs.as_ptr().cast()),
                std::mem::size_of::<HANDLE>(),
                None,
                None,
            )?;
        }
        Ok(result)
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.list) }
    }
}

pub struct WindowsJob {
    job: OwnedHandle,
    process: OwnedHandle,
    input: Mutex<File>,
}
impl WindowsJob {
    pub fn spawn(
        executable: &Path,
        script: &Path,
        cwd: &Path,
        environment: &[(&str, String)],
    ) -> io::Result<(Self, File, File)> {
        let executable_text = executable.as_os_str().to_string_lossy();
        let script_text = script.as_os_str().to_string_lossy();
        if [executable_text.as_ref(), script_text.as_ref()]
            .iter()
            .any(|value| value.contains(['"', '\0', '\r', '\n']))
        {
            return Err(io::Error::other("Invalid packaged image path"));
        }
        let app = wide(executable.as_os_str());
        let directory = wide(cwd.as_os_str());
        let mut command = wide(std::ffi::OsStr::new(&format!(
            "\"{executable_text}\" \"{script_text}\""
        )));
        let mut variables = environment
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>();
        variables.sort_by_key(|value| value.to_uppercase());
        if variables.iter().any(|value| value.contains('\0')) {
            return Err(io::Error::other("Invalid environment"));
        }
        let environment = variables.join("\0") + "\0\0";
        let environment = wide(std::ffi::OsStr::new(&environment));
        let job = owned(unsafe { CreateJobObjectW(None, PCWSTR::null())? });
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(
                handle(&job),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )?;
        }
        let (input_read, input_write) = pipe()?;
        let (output_read, output_write) = pipe()?;
        let (error_read, error_write) = pipe()?;
        for parent in [&input_write, &output_read, &error_read] {
            unsafe {
                SetHandleInformation(handle(parent), HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))?;
            }
        }
        let mut inherited = [
            handle(&input_read),
            handle(&output_write),
            handle(&error_write),
        ];
        let attributes = Attributes::new(&mut inherited, handle(&job))?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = inherited[0];
        startup.StartupInfo.hStdOutput = inherited[1];
        startup.StartupInfo.hStdError = inherited[2];
        startup.lpAttributeList = attributes.list;
        let mut info = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                PCWSTR(app.as_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                true,
                CREATE_SUSPENDED
                    | CREATE_UNICODE_ENVIRONMENT
                    | CREATE_NO_WINDOW
                    | EXTENDED_STARTUPINFO_PRESENT,
                Some(environment.as_ptr().cast()),
                PCWSTR(directory.as_ptr()),
                &startup.StartupInfo,
                &mut info,
            )?;
        }
        let process = owned(info.hProcess);
        let thread = owned(info.hThread);
        // Any failure drops the kill-on-close job and explicitly ends a child
        // which may not yet belong to it. It is never resumed without assignment.
        let mut belongs = windows::core::BOOL::default();
        let assigned =
            unsafe { IsProcessInJob(handle(&process), Some(handle(&job)), &mut belongs) }.and_then(
                |_| {
                    if belongs.as_bool() {
                        Ok(())
                    } else {
                        unsafe { AssignProcessToJobObject(handle(&job), handle(&process)) }
                    }
                },
            );
        if let Err(error) = assigned {
            unsafe {
                let _ = TerminateProcess(handle(&process), 1);
                let _ = WaitForSingleObject(handle(&process), 5000);
            }
            return Err(error.into());
        }
        if unsafe { ResumeThread(handle(&thread)) } == u32::MAX {
            unsafe {
                let _ = TerminateJobObject(handle(&job), 1);
            }
            return Err(io::Error::last_os_error());
        }
        drop((input_read, output_write, error_write, thread));
        Ok((
            Self {
                job,
                process,
                input: Mutex::new(File::from(input_write)),
            },
            File::from(output_read),
            File::from(error_read),
        ))
    }
    pub fn send(&self, frame: &str) -> io::Result<()> {
        if frame.len() > 1024 * 1024 || frame.contains('\n') {
            return Err(io::Error::other("Invalid control frame"));
        }
        let mut input = self
            .input
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        input.write_all(frame.as_bytes())?;
        input.write_all(b"\n")?;
        input.flush()
    }
    pub fn terminate(&self) {
        unsafe {
            let _ = TerminateJobObject(handle(&self.job), 1);
        }
    }
    pub fn wait(&self, milliseconds: u32) -> Option<u32> {
        if unsafe { WaitForSingleObject(handle(&self.process), milliseconds) }.0 != 0 {
            return None;
        }
        let mut exit = 0;
        unsafe {
            GetExitCodeProcess(handle(&self.process), &mut exit).ok()?;
        }
        Some(exit)
    }
}
