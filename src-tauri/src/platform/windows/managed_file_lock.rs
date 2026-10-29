//! Native exclusion and durable renames for a closed managed data generation.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Component, Path};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    FileRenameInfo, GetFileInformationByHandle, SetFileInformationByHandle,
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_WRITE_THROUGH, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_RENAME_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

mod identity;
pub(crate) use identity::validate_location;

use crate::platform::managed_file_lock::Sharing;

fn validate(file: File, directory: bool) -> io::Result<File> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the handle remains owned by file and info is valid output memory.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
        .map_err(io::Error::other)?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        || (!directory && info.nNumberOfLinks != 1)
        || file.metadata()?.is_dir() != directory
        || (!directory && !file.metadata()?.is_file())
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "managed generations reject redirects, hard links and nonregular files",
        ));
    }
    Ok(file)
}

pub(crate) fn open(path: &Path, directory: bool, sharing: Sharing) -> io::Result<File> {
    let moving = matches!(sharing, Sharing::Move);
    let sharing = match sharing {
        Sharing::Move => FILE_SHARE_DELETE.0,
        Sharing::Pin => (FILE_SHARE_READ | FILE_SHARE_WRITE).0,
        Sharing::Read => FILE_SHARE_READ.0,
    };
    let mut flags = FILE_FLAG_OPEN_REPARSE_POINT.0;
    if directory {
        flags |= FILE_FLAG_BACKUP_SEMANTICS.0;
    }
    let mut options = OpenOptions::new();
    options.read(true);
    if moving {
        options.access_mode(FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0 | 0x00010000);
        flags |= FILE_FLAG_WRITE_THROUGH.0;
    }
    let file = validate(
        options.share_mode(sharing).custom_flags(flags).open(path)?,
        directory,
    )?;
    validate_location(&file, path)?;
    Ok(file)
}

pub(crate) fn operation_lock(path: &Path) -> io::Result<File> {
    validate(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)?,
        false,
    )
}

pub(crate) fn create_new(path: &Path) -> io::Result<File> {
    validate(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)?,
        false,
    )
}

pub(crate) fn reserve_sqlite(path: &Path) -> io::Result<File> {
    let file = validate(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE).0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path)?,
        false,
    )?;
    validate_location(&file, path)?;
    Ok(file)
}

pub(crate) fn durable_move(from: &Path, to: &Path, replace: bool) -> io::Result<()> {
    let file = open(from, false, Sharing::Move)?;
    rename_held(&file, from, to, replace)
}

pub(crate) fn rename_held(file: &File, from: &Path, to: &Path, replace: bool) -> io::Result<()> {
    let source_parent = from
        .parent()
        .ok_or_else(|| io::Error::other("source has no managed parent"))?;
    let target_parent = to
        .parent()
        .ok_or_else(|| io::Error::other("destination has no managed parent"))?;
    // Some Windows versions reject RootDirectory + a relative leaf with
    // ERROR_INVALID_PARAMETER. Keep the exact source handle, and bind the
    // absolute destination by pinning every ancestor against replacement.
    let _source_parents = pin_ancestors(source_parent)?;
    let _target_parents = pin_ancestors(target_parent)?;
    validate_location(file, from)?;
    if to.file_name().is_none() {
        return Err(io::Error::other("destination has no filename"));
    }
    let wide = to.as_os_str().encode_wide().collect::<Vec<_>>();
    let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    // The Win32 absolute-path conversion also requires FileName's trailing
    // UTF-16 zero. FileNameLength describes only the name, not that terminator.
    let byte_len = offset + (wide.len() + 1) * 2;
    let mut aligned = vec![0_u64; byte_len.div_ceil(8)];
    let info = aligned.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: u64 storage has the native header alignment and enough trailing
    // space for the complete UTF-16 filename. Source and all parent handles
    // remain owned through SetFileInformationByHandle.
    unsafe {
        (*info).Anonymous.ReplaceIfExists = replace;
        (*info).RootDirectory = HANDLE::default();
        (*info).FileNameLength = (wide.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            aligned.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>(),
            wide.len(),
        );
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileRenameInfo,
            info.cast(),
            byte_len as u32,
        )
    }
    .map_err(io::Error::other)?;
    validate_location(file, to)?;
    // Flush the exact renamed regular-file handle. Directory contents are
    // already sealed; the journal remains authoritative until the full switch.
    if file.metadata()?.is_file() {
        file.sync_all()?;
    }
    Ok(())
}

fn pin_ancestors(path: &Path) -> io::Result<Vec<File>> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(io::Error::other("managed rename requires an absolute path"));
    }
    let mut ancestors = path.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    ancestors
        .into_iter()
        .map(|parent| open(parent, true, Sharing::Pin))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
