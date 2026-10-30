//! Delete only enumerated app-owned generation namespaces.
use super::files::{exists, generation_ids, layout, pin_chain, remove_tree};
use super::*;
use std::fs;

pub(super) fn purge_owned(paths: &AppPaths) -> AppResult<()> {
    layout(paths)?;
    let _pins = pin_chain(&paths.data)?;
    let _lock = io(
        &paths.data.join(LOCK),
        "lock generation purge",
        native::operation_lock(&paths.data.join(LOCK)),
    )?;
    purge_locked(paths)
}

pub(super) fn purge_locked(paths: &AppPaths) -> AppResult<()> {
    for namespace in [BACKUPS, RESTORES] {
        let root = paths.data.join(namespace);
        for id in generation_ids(&root)? {
            let generation = root.join(id.as_str());
            if namespace == RESTORES {
                let _pin = io(
                    &generation,
                    "pin restore transaction",
                    native::pin_directory(&generation),
                )?;
                for entry in io(
                    &generation,
                    "list restore transaction",
                    fs::read_dir(&generation),
                )? {
                    let entry = io(&generation, "read restore transaction entry", entry)?;
                    if !matches!(entry.file_name().to_str(), Some("replay" | "displaced")) {
                        return Err(error("restore transaction contains an unmanaged name"));
                    }
                    remove_tree(&entry.path())?;
                }
                drop(_pin);
                io(
                    &generation,
                    "remove restore transaction",
                    fs::remove_dir(&generation),
                )?;
            } else {
                remove_tree(&generation)?;
            }
        }
        if exists(&root)? {
            io(&root, "remove generation namespace", fs::remove_dir(&root))?;
        }
    }
    for name in [JOURNAL, "data-restore.json.tmp"] {
        let path = paths.data.join(name);
        if exists(&path)? {
            let guard = io(
                &path,
                "lock restore journal deletion",
                native::for_move(&path),
            )?;
            io(&path, "remove restore journal", fs::remove_file(&path))?;
            drop(guard);
        }
    }
    Ok(())
}
