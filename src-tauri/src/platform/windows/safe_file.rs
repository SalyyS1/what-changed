//! Race-resistant watched-file opening for Windows.
//!
//! Both containment and reparse checks describe the object behind the handle,
//! not a path inspected before `CreateFileW`. Bytes are then read through that
//! same handle, closing the name-to-object race at the parser boundary.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetFileInformationByHandle, GetFinalPathNameByHandleW, BY_HANDLE_FILE_INFORMATION,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_GENERIC_READ, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    VOLUME_NAME_DOS,
};

pub(crate) fn read_under(
    root: &Path,
    path: &Path,
    limit: u64,
    destination: &mut Vec<u8>,
) -> io::Result<()> {
    let root_handle = open(root, true)?;
    reject_reparse(&root_handle)?;
    let approved_root = final_path(&root_handle)?;
    if normalize(&approved_root) != normalize(root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "opened root no longer resolves to the approved directory",
        ));
    }

    let file_handle = open(path, false)?;
    reject_reparse(&file_handle)?;
    let opened_path = final_path(&file_handle)?;
    if !is_strictly_under(&opened_path, &approved_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "opened file is outside the approved root",
        ));
    }

    let mut file = File::from(file_handle);
    file.by_ref()
        .take(limit.saturating_add(1))
        .read_to_end(destination)?;
    Ok(())
}

fn open(path: &Path, directory: bool) -> io::Result<OwnedHandle> {
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    wide.push(0);
    let mut flags = FILE_FLAG_OPEN_REPARSE_POINT;
    if directory {
        flags |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    // SAFETY: `wide` is NUL-terminated and remains alive for the call. The
    // returned owned handle is closed on every path.
    let raw = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            flags,
            None,
        )
    }
    .map_err(io::Error::other)?;
    // SAFETY: `CreateFileW` returned a fresh valid handle whose ownership is
    // transferred exactly once to `OwnedHandle`.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw.0) })
}

fn reject_reparse(handle: &OwnedHandle) -> io::Result<()> {
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the pointer is valid for writes and the handle remains owned for
    // the duration of the call.
    unsafe { GetFileInformationByHandle(as_handle(handle), &mut information) }
        .map_err(io::Error::other)?;
    if information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "reparse points are not readable watched files",
        ));
    }
    Ok(())
}

fn final_path(handle: &OwnedHandle) -> io::Result<PathBuf> {
    let mut buffer = vec![0_u16; 512];
    loop {
        // SAFETY: the slice is writable and the handle remains valid.
        let length =
            unsafe { GetFinalPathNameByHandleW(as_handle(handle), &mut buffer, VOLUME_NAME_DOS) };
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        let length = usize::try_from(length).map_err(io::Error::other)?;
        if length < buffer.len() {
            return Ok(PathBuf::from(OsString::from_wide(&buffer[..length])));
        }
        buffer.resize(length.saturating_add(1), 0);
    }
}

fn as_handle(handle: &OwnedHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}

fn is_strictly_under(path: &Path, root: &Path) -> bool {
    let path = normalize(path);
    let root = normalize(root);
    path != root
        && path
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn normalize(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('/', "\\")
        .to_lowercase()
}
