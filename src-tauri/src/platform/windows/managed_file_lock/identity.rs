//! Bind an opened object to its expected final Windows path before moving it.
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, GetFinalPathNameByHandleW, BY_HANDLE_FILE_INFORMATION,
    FILE_ATTRIBUTE_REPARSE_POINT, VOLUME_NAME_DOS,
};

fn normalized(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('/', "\\")
        .to_lowercase()
}

pub(crate) fn validate_location(file: &File, path: &Path) -> io::Result<()> {
    let handle = HANDLE(file.as_raw_handle());
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: file owns the valid handle and info is writable output memory.
    unsafe { GetFileInformationByHandle(handle, &mut info) }.map_err(io::Error::other)?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        || (file.metadata()?.is_file() && info.nNumberOfLinks != 1)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "managed handle became a redirect or hard link",
        ));
    }
    let mut buffer = vec![0_u16; 512];
    loop {
        // SAFETY: buffer is writable and the file handle remains live.
        let len =
            unsafe { GetFinalPathNameByHandleW(handle, &mut buffer, VOLUME_NAME_DOS) } as usize;
        if len == 0 {
            return Err(io::Error::last_os_error());
        }
        if len < buffer.len() {
            let opened = PathBuf::from(OsString::from_wide(&buffer[..len]));
            return if normalized(&opened) == normalized(path) {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "opened object no longer matches its managed path",
                ))
            };
        }
        if len > 32768 {
            return Err(io::Error::other("managed path exceeds Windows bounds"));
        }
        buffer.resize(len + 1, 0);
    }
}
