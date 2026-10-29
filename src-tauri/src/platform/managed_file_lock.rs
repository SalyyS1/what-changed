//! Handle ownership for managed generation files, isolated from restore policy.

use std::fs::File;
use std::io;
use std::path::Path;

/// Deny other readers/writers while permitting a same-volume managed rename.
pub(crate) fn for_move(path: &Path) -> io::Result<File> {
    implementation::open(path, false, Sharing::Move)
}

/// Pin a parent directory against rename while names below it are inspected.
pub(crate) fn pin_directory(path: &Path) -> io::Result<File> {
    implementation::open(path, true, Sharing::Pin)
}

/// Keep a source file immutable while hashing or copying its opened bytes.
pub(crate) fn for_read(path: &Path) -> io::Result<File> {
    implementation::open(path, false, Sharing::Read)
}

pub(crate) fn operation_lock(path: &Path) -> io::Result<File> {
    implementation::operation_lock(path)
}

pub(crate) fn create_new(path: &Path) -> io::Result<File> {
    implementation::create_new(path)
}

/// Reserve a new SQLite destination against replacement while SQLite opens it.
pub(crate) fn reserve_sqlite(path: &Path) -> io::Result<File> {
    implementation::reserve_sqlite(path)
}

pub(crate) fn durable_move(from: &Path, to: &Path, replace: bool) -> io::Result<()> {
    implementation::durable_move(from, to, replace)
}

pub(crate) fn rename_held(file: &File, from: &Path, to: &Path) -> io::Result<()> {
    implementation::rename_held(file, from, to, false)
}

pub(crate) fn validate_location(file: &File, path: &Path) -> io::Result<()> {
    implementation::validate_location(file, path)
}

#[derive(Clone, Copy)]
pub(crate) enum Sharing {
    Move,
    Pin,
    Read,
}

#[cfg(windows)]
use crate::platform::windows::managed_file_lock as implementation;

#[cfg(not(windows))]
mod implementation {
    use super::*;

    // This product ships on Windows. Other hosts must not silently provide a
    // weaker closed-store/rename guarantee.
    pub(crate) fn open(_: &Path, _: bool, _: Sharing) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
    pub(crate) fn operation_lock(_: &Path) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
    pub(crate) fn create_new(_: &Path) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
    pub(crate) fn reserve_sqlite(_: &Path) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
    pub(crate) fn durable_move(_: &Path, _: &Path, _: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
    pub(crate) fn rename_held(_: &File, _: &Path, _: &Path, _: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
    pub(crate) fn validate_location(_: &File, _: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "managed generation locks require Windows",
        ))
    }
}
