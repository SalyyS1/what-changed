//! Bounded exact-root inventory; names never originate in a restore journal.

pub(super) use super::cleanup::{purge_locked, purge_owned};
pub(super) use super::inventory::{generation_ids, relative, scan, scan_at, Inventory};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};

use sha2::{Digest, Sha256};

use super::*;

pub(super) const MANIFEST: &str = "manifest.json";

pub(super) fn layout(paths: &AppPaths) -> AppResult<()> {
    if !paths.data.is_absolute()
        || paths.data.file_name().is_none()
        || paths
            .data
            .components()
            .any(|p| matches!(p, Component::ParentDir | Component::CurDir))
        || *paths != AppPaths::under(&paths.data)
    {
        return Err(error(
            "generation paths differ from the resolved app layout",
        ));
    }
    // A fresh installation has no data directory yet. Validate/pin every
    // existing ancestor; recovery must not manufacture a generation there.
    let mut ancestors = paths.data.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    let mut pins = Vec::new();
    for ancestor in ancestors {
        if exists(ancestor)? {
            pins.push(io(
                ancestor,
                "validate existing managed ancestor",
                native::pin_directory(ancestor),
            )?);
        }
    }
    Ok(())
}

pub(super) fn exists(path: &Path) -> AppResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(error("generation path redirects"))
        }
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => io(path, "inspect generation path", Err(e)),
    }
}

pub(super) fn pin_chain(path: &Path) -> AppResult<Vec<File>> {
    let mut ancestors = path.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    ancestors
        .into_iter()
        .map(|p| io(p, "pin managed parent", native::pin_directory(p)))
        .collect()
}

pub(super) fn directory(path: &Path) -> AppResult<()> {
    if !exists(path)? {
        io(
            path,
            "create managed generation directory",
            fs::create_dir(path),
        )?;
    }
    let _pin = io(
        path,
        "validate managed directory",
        native::pin_directory(path),
    )?;
    Ok(())
}

pub(super) fn hash(file: &File) -> AppResult<(u64, String)> {
    let mut input = file;
    input
        .seek(SeekFrom::Start(0))
        .map_err(|_| error("cannot seek an opened generation file"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    let mut bytes = 0_u64;
    loop {
        let len = input
            .read(&mut buffer)
            .map_err(|_| error("cannot hash an opened generation file"))?;
        if len == 0 {
            break;
        }
        bytes = bytes
            .checked_add(len as u64)
            .ok_or_else(|| error("generation file size overflow"))?;
        if bytes > MAX_BYTES {
            return Err(error("generation file exceeds byte limit"));
        }
        digest.update(&buffer[..len]);
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}

pub(super) fn read(path: &Path, limit: u64) -> AppResult<Vec<u8>> {
    let file = io(
        path,
        "open bounded generation record",
        native::for_read(path),
    )?;
    let mut bytes = Vec::new();
    io(
        path,
        "read bounded generation record",
        file.take(limit + 1).read_to_end(&mut bytes),
    )?;
    if bytes.len() as u64 > limit {
        return Err(error("generation record exceeds limit"));
    }
    Ok(bytes)
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file = io(path, "reserve generation file", native::create_new(path))?;
    io(path, "write generation file", file.write_all(bytes))?;
    io(path, "flush generation file", file.sync_all())
}

pub(super) fn clone_tree(source: &Path, destination: &Path) -> AppResult<()> {
    let inventory = scan(source)?;
    clone_inventory(source, destination, &inventory)
}

pub(super) fn clone_inventory(
    source: &Path,
    destination: &Path,
    inventory: &Inventory,
) -> AppResult<()> {
    directory(destination)?;
    for dir in &inventory.directories {
        directory(&destination.join(dir))?;
    }
    for name in &inventory.files {
        let from = source.join(name);
        let to = destination.join(name);
        let mut input = io(
            &from,
            "open immutable generation source",
            native::for_read(&from),
        )?;
        let mut output = io(&to, "reserve generation copy", native::create_new(&to))?;
        io(
            &to,
            "copy generation bytes",
            std::io::copy(&mut input, &mut output),
        )?;
        io(&to, "flush generation copy", output.sync_all())?;
    }
    Ok(())
}

/// Every child was enumerated and validated; no recursive-delete primitive is used.
pub(super) fn remove_tree(root: &Path) -> AppResult<()> {
    if !exists(root)? {
        return Ok(());
    }
    let inventory = scan(root)?;
    for name in inventory.files {
        let path = root.join(name);
        io(
            &path,
            "remove owned generation file",
            fs::remove_file(&path),
        )?;
    }
    for name in inventory.directories.into_iter().rev() {
        let path = root.join(name);
        io(
            &path,
            "remove owned generation directory",
            fs::remove_dir(&path),
        )?;
    }
    io(root, "remove owned generation root", fs::remove_dir(root))
}
