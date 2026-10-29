//! Strict Windows names and bounded managed artifact inventories.
use super::files::{exists, MANIFEST};
use super::*;
use std::collections::BTreeSet;
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Inventory {
    pub files: Vec<String>,
    pub directories: Vec<String>,
}

fn component(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 128
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
}

pub(super) fn relative(name: &str) -> AppResult<()> {
    let parts = name.split('/').collect::<Vec<_>>();
    if parts.len() > 16 || !parts.iter().all(|p| component(p)) || name.contains('\\') {
        return Err(error("generation inventory contains a nonportable path"));
    }
    if parts.len() == 1 && (ITEMS.contains(&name) || name == MANIFEST) {
        return Ok(());
    }
    if parts.len() > 1 && parts[0] == "owner-artifacts" {
        return Ok(());
    }
    Err(error("generation inventory names an unmanaged root"))
}

pub(super) fn scan(root: &Path) -> AppResult<Inventory> {
    scan_at(root, "")
}

pub(super) fn scan_at(root: &Path, prefix: &str) -> AppResult<Inventory> {
    fn walk(
        root: &Path,
        prefix: &str,
        out: &mut Inventory,
        seen: &mut BTreeSet<String>,
        bytes: &mut u64,
    ) -> AppResult<()> {
        let _pin = io(
            root,
            "pin generation inventory directory",
            native::pin_directory(root),
        )?;
        let mut entries = io(root, "list generation inventory", fs::read_dir(root))?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| AppError::File {
                operation: "read generation entry",
                path: root.into(),
                source: e,
            })?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| error("generation entry is not Unicode"))?;
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            self::relative(&relative)?;
            if !seen.insert(relative.to_ascii_lowercase()) {
                return Err(error("generation inventory has a Windows name collision"));
            }
            if seen.len() > MAX_FILES {
                return Err(error(
                    "generation inventory exceeds the file/directory limit",
                ));
            }
            let path = entry.path();
            let meta = io(
                &path,
                "inspect generation entry",
                fs::symlink_metadata(&path),
            )?;
            if meta.is_dir() {
                if relative.split('/').next() != Some("owner-artifacts") {
                    return Err(error("only managed artifact directories may be nested"));
                }
                out.directories.push(relative.clone());
                walk(&path, &relative, out, seen, bytes)?;
            } else {
                let opened = io(
                    &path,
                    "open generation inventory file",
                    native::for_read(&path),
                )?;
                *bytes = bytes
                    .checked_add(io(&path, "size generation file", opened.metadata())?.len())
                    .ok_or_else(|| error("generation byte count overflow"))?;
                if *bytes > MAX_BYTES {
                    return Err(error("generation exceeds the byte limit"));
                }
                out.files.push(relative);
            }
        }
        Ok(())
    }
    let mut result = Inventory {
        files: Vec::new(),
        directories: if prefix.is_empty() {
            Vec::new()
        } else {
            vec![prefix.into()]
        },
    };
    walk(root, prefix, &mut result, &mut BTreeSet::new(), &mut 0)?;
    result.files.sort();
    result.directories.sort();
    Ok(result)
}

pub(super) fn generation_ids(root: &Path) -> AppResult<Vec<BackupId>> {
    if !exists(root)? {
        return Ok(Vec::new());
    }
    let _pin = io(
        root,
        "pin generation namespace",
        native::pin_directory(root),
    )?;
    let mut ids = Vec::new();
    for entry in io(root, "list generation namespace", fs::read_dir(root))? {
        let entry = io(root, "read generation namespace entry", entry)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| error("generation ID is not Unicode"))?;
        ids.push(BackupId::parse(&name)?);
        if ids.len() > MAX_GENERATIONS {
            return Err(error("too many owned generations"));
        }
    }
    ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(ids)
}
