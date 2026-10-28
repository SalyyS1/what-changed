//! Bind an opened object to its expected final Windows path before moving it.
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, GetFinalPathNameByHandleW, GetLongPathNameW,
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, VOLUME_NAME_DOS,
};

fn normalized(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('/', "\\")
        .to_lowercase()
}

fn long_name(path: &Path) -> io::Result<PathBuf> {
    let input = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut buffer = vec![0_u16; 512];
    loop {
        // SAFETY: input is terminated, output is writable for its full length.
        let len = unsafe { GetLongPathNameW(PCWSTR(input.as_ptr()), Some(&mut buffer)) } as usize;
        if len == 0 {
            return Err(io::Error::last_os_error());
        }
        if len < buffer.len() {
            return Ok(PathBuf::from(OsString::from_wide(&buffer[..len])));
        }
        if len > 32768 {
            return Err(io::Error::other("managed path exceeds Windows bounds"));
        }
        buffer.resize(len + 1, 0);
    }
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
    // Expand 8.3 names without resolving junctions to their targets. The final
    // handle path must still match this expected location, including after a move.
    let expected = long_name(path)?;
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
            return if normalized(&opened) == normalized(&expected) {
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::platform::managed_file_lock::Sharing;
    use windows::Win32::Storage::FileSystem::GetShortPathNameW;

    #[test]
    fn short_names_keep_the_same_location_binding() {
        let root = crate::testing::temp_dir("generation-native-short-name");
        let input = root
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut buffer = vec![0_u16; 32768];
        // SAFETY: input is terminated and the writable output is fully bounded.
        let len = unsafe { GetShortPathNameW(PCWSTR(input.as_ptr()), Some(&mut buffer)) } as usize;
        assert!(len > 0 && len < buffer.len());
        let short = PathBuf::from(OsString::from_wide(&buffer[..len]));
        let path = root.join("held-record.txt");
        let alias = short.join("held-record.txt");
        let other = root.join("different-record.txt");
        std::fs::write(&path, b"owned fixture").unwrap();
        std::fs::write(&other, b"different fixture").unwrap();
        let guard = super::super::open(&alias, false, Sharing::Move).unwrap();
        validate_location(&guard, &path).unwrap();
        validate_location(&guard, &alias).unwrap();
        assert!(validate_location(&guard, &other).is_err());
    }
}
