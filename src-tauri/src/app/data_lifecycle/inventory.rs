//! Exact app-owned paths. External source projects and chosen exports are absent.

use std::fs::{self, Metadata, OpenOptions};
use std::path::{Component, Path, PathBuf};

use super::{recovery_error, AppPaths};
use crate::error::{AppError, AppResult};

pub(super) fn companion(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn metadata(path: &Path) -> AppResult<Option<Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if is_redirect(&metadata) {
                return Err(recovery_error(
                    "an owned purge path is a symlink or reparse point",
                ));
            }
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(AppError::File {
            operation: "inspect owned purge path",
            path: path.into(),
            source,
        }),
    }
}

fn is_redirect(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

pub(super) fn directory_chain(path: &Path) -> AppResult<()> {
    let mut ancestors = path.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    for ancestor in ancestors {
        if let Some(metadata) = metadata(ancestor)? {
            if !metadata.is_dir() {
                return Err(recovery_error("an owned purge parent is not a directory"));
            }
        }
    }
    Ok(())
}

pub(super) fn regular_file(path: &Path) -> AppResult<Option<Metadata>> {
    if let Some(parent) = path.parent() {
        directory_chain(parent)?;
    }
    let found = metadata(path)?;
    if found.as_ref().is_some_and(|value| !value.is_file()) {
        return Err(recovery_error("an owned purge file is not a regular file"));
    }
    Ok(found)
}

pub(super) fn validate_layout(paths: &AppPaths) -> AppResult<()> {
    if !paths.data.is_absolute()
        || paths.data.file_name().is_none()
        || paths
            .data
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        || *paths != AppPaths::under(&paths.data)
    {
        return Err(recovery_error(
            "purge paths do not match the resolved application layout",
        ));
    }
    directory_chain(&paths.data)
}

fn sqlite_base(name: &str) -> &str {
    ["-wal", "-shm", "-journal"]
        .into_iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}

fn digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn legacy_copy(name: &str) -> bool {
    let base = sqlite_base(name);
    if base == "events.repair.db" {
        return true;
    }
    [
        "events.corrupt-",
        "events.purge-old-",
        "events.purge-fresh-",
    ]
    .into_iter()
    .any(|prefix| {
        base.strip_prefix(prefix)
            .and_then(|tail| tail.strip_suffix(".db"))
            .is_some_and(digits)
    })
}

pub(super) fn legacy_backup(name: &str) -> bool {
    sqlite_base(name)
        .strip_prefix("events-pre-v")
        .and_then(|tail| tail.strip_suffix(".db"))
        .is_some_and(digits)
}

pub(super) fn owner_copy(name: &str) -> bool {
    sqlite_base(name)
        .strip_prefix("owner-workspace.pre-migration-")
        .and_then(|tail| tail.strip_suffix(".sqlite"))
        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
}

fn diagnostics_copy(name: &str) -> bool {
    name.strip_prefix("diagnostics-")
        .and_then(|tail| tail.strip_suffix(".json"))
        .is_some_and(digits)
}

fn entries(root: &Path) -> AppResult<Vec<PathBuf>> {
    directory_chain(root)?;
    if metadata(root)?.is_none() {
        return Ok(Vec::new());
    }
    fs::read_dir(root)
        .map_err(|source| AppError::Directory {
            path: root.into(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|value| value.path())
                .map_err(|source| AppError::Directory {
                    path: root.into(),
                    source,
                })
        })
        .collect()
}

fn files(paths: &AppPaths, live: bool) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    if live {
        for store in [&paths.db.database, &paths.owner_workspace()] {
            files.push(store.clone());
            for suffix in ["-wal", "-shm", "-journal"] {
                files.push(companion(store, suffix));
            }
        }
    }
    for path in entries(&paths.data)? {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if legacy_copy(name) || owner_copy(name) || diagnostics_copy(name) {
            files.push(path);
        }
    }
    for path in entries(&paths.db.backups)? {
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(legacy_backup)
        {
            files.push(path);
        }
    }
    Ok(files)
}

fn artifact_roots(paths: &AppPaths) -> [PathBuf; 3] {
    [
        "owner-artifacts",
        "owner-temp-profiles",
        "owner-cached-exports",
    ]
    .map(|name| paths.data.join(name))
}

fn validate_tree(root: &Path) -> AppResult<()> {
    directory_chain(root)?;
    for path in entries(root)? {
        let Some(metadata) = metadata(&path)? else {
            continue;
        };
        if metadata.is_dir() {
            validate_tree(&path)?;
        } else {
            regular_file(&path)?;
        }
    }
    Ok(())
}

pub(super) fn validate_owned(paths: &AppPaths, live: bool) -> AppResult<()> {
    validate_layout(paths)?;
    for path in files(paths, live)? {
        regular_file(&path)?;
    }
    for root in artifact_roots(paths) {
        validate_tree(&root)?;
    }
    validate_tree(&paths.logs)
}

fn remove_file(path: &Path) -> AppResult<()> {
    if regular_file(path)?.is_none() {
        return Ok(());
    }
    fs::remove_file(path).map_err(|source| AppError::File {
        operation: "remove owned purge file",
        path: path.into(),
        source,
    })
}

pub(super) fn remove_tree(root: &Path) -> AppResult<()> {
    directory_chain(root)?;
    if metadata(root)?.is_none() {
        return Ok(());
    }
    for path in entries(root)? {
        let Some(metadata) = metadata(&path)? else {
            continue;
        };
        if metadata.is_dir() {
            remove_tree(&path)?;
        } else {
            remove_file(&path)?;
        }
    }
    fs::remove_dir(root).map_err(|source| AppError::File {
        operation: "remove owned artifact directory",
        path: root.into(),
        source,
    })
}

fn truncate_logs(root: &Path) -> AppResult<()> {
    for path in entries(root)? {
        let Some(metadata) = metadata(&path)? else {
            continue;
        };
        if metadata.is_dir() {
            truncate_logs(&path)?;
            continue;
        }
        regular_file(&path)?;
        // The logger may hold this file on Windows. Truncation failure is
        // visible and retriable; it never silently completes privacy deletion.
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .and_then(|file| file.sync_all())
            .map_err(|source| AppError::File {
                operation: "truncate owned privacy log",
                path,
                source,
            })?;
    }
    Ok(())
}

pub(super) fn remove_owned(paths: &AppPaths, live: bool) -> AppResult<()> {
    validate_owned(paths, live)?;
    super::super::data_restore::purge_owned_generations(paths)?;
    for path in files(paths, live)? {
        remove_file(&path)?;
    }
    for root in artifact_roots(paths) {
        remove_tree(&root)?;
    }
    truncate_logs(&paths.logs)
}
