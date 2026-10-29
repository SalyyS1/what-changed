//! Exclude existing/new SQLite handles and verify the bytes behind held handles.

use super::*;
use std::collections::BTreeMap;

mod artifact_move;
pub(super) use artifact_move::ArtifactBoundary;

pub(super) type Fingerprint = BTreeMap<String, (u64, String)>;

pub(super) struct LockedFiles {
    pub fingerprint: Fingerprint,
    files: BTreeMap<String, File>,
    directories: BTreeMap<String, File>,
}

impl LockedFiles {
    pub(super) fn rename_root(
        &mut self,
        name: &str,
        from: &Path,
        to: &Path,
        hook: &mut impl FnMut(ArtifactBoundary) -> AppResult<()>,
    ) -> AppResult<()> {
        if self.directories.contains_key(name) {
            return self.move_artifacts(name, from, to, hook);
        }
        let guard = self
            .files
            .get(name)
            .ok_or_else(|| error("managed root appeared after its handles were locked"))?;
        for (key, file) in &self.files {
            if key == name || key.starts_with(&format!("{name}/")) {
                if let Some(expected) = self.fingerprint.get(key) {
                    if &files::hash(file)? != expected {
                        return Err(error("held generation bytes changed before promotion"));
                    }
                }
            }
        }
        io(
            from,
            "move exact managed object",
            native::rename_held(guard, from, to),
        )
    }
}

pub(super) fn live_inventory(paths: &AppPaths) -> AppResult<files::Inventory> {
    let mut inventory = files::Inventory {
        files: Vec::new(),
        directories: Vec::new(),
    };
    for name in ITEMS {
        let path = paths.data.join(name);
        if !files::exists(&path)? {
            continue;
        }
        if name == "owner-artifacts" {
            let artifacts = files::scan_at(&path, "owner-artifacts")?;
            inventory.files.extend(artifacts.files);
            inventory.directories.extend(artifacts.directories);
        } else {
            inventory.files.push(name.into());
        }
    }
    inventory.files.sort();
    inventory.directories.sort();
    Ok(inventory)
}

fn lock(root: &Path, inventory: &files::Inventory) -> AppResult<LockedFiles> {
    let mut guards = BTreeMap::new();
    let mut directories = BTreeMap::new();
    let mut fingerprint = Fingerprint::new();
    for name in &inventory.directories {
        let path = root.join(name);
        directories.insert(
            name.clone(),
            io(
                &path,
                "pin managed artifact directory",
                native::pin_directory(&path),
            )?,
        );
    }
    for name in &inventory.files {
        let path = root.join(name);
        let guard = io(
            &path,
            "exclude external generation handles",
            native::for_move(&path),
        )?;
        let digest = files::hash(&guard)?;
        // Read-only SQLite inspection can create an empty WAL and a shared
        // memory index. Neither holds committed pages. Every sidecar remains
        // locked; any nonempty WAL, journal, database or artifact is hashed.
        let shared_memory = matches!(
            name.as_str(),
            "events.db-shm" | "owner-workspace.sqlite-shm"
        );
        let empty_wal = matches!(
            name.as_str(),
            "events.db-wal" | "owner-workspace.sqlite-wal"
        ) && digest.0 == 0;
        if !shared_memory && !empty_wal {
            fingerprint.insert(name.clone(), digest);
        }
        guards.insert(name.clone(), guard);
    }
    Ok(LockedFiles {
        fingerprint,
        files: guards,
        directories,
    })
}

pub(super) fn live(paths: &AppPaths) -> AppResult<LockedFiles> {
    let inventory = live_inventory(paths)?;
    let locked = lock(&paths.data, &inventory)?;
    // Recheck the full name inventory while files are excluded. Newly opened
    // SQLite handles cannot introduce a sidecar beneath a locked database.
    if live_inventory_names(paths)? != inventory {
        return Err(error("managed live inventory changed while locking"));
    }
    Ok(locked)
}

fn live_inventory_names(paths: &AppPaths) -> AppResult<files::Inventory> {
    // Do not reopen locked files to scan. Re-enumerate only managed names and
    // filesystem types; initial scans already rejected every opened redirect.
    let mut inventory = files::Inventory {
        files: Vec::new(),
        directories: Vec::new(),
    };
    fn artifacts(path: &Path, prefix: &str, inventory: &mut files::Inventory) -> AppResult<()> {
        inventory.directories.push(prefix.into());
        for entry in io(
            path,
            "recheck locked artifact names",
            std::fs::read_dir(path),
        )? {
            let entry = io(path, "recheck locked artifact entry", entry)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| error("invalid artifact name"))?;
            let name = format!("{prefix}/{name}");
            files::relative(&name)?;
            let meta = io(
                &entry.path(),
                "recheck locked artifact type",
                entry.file_type(),
            )?;
            if meta.is_symlink() {
                return Err(error("artifact redirect appeared while locking"));
            }
            if meta.is_dir() {
                artifacts(&entry.path(), &name, inventory)?;
            } else {
                inventory.files.push(name);
            }
            if inventory.files.len() + inventory.directories.len() > MAX_FILES {
                return Err(error("artifact inventory changed beyond bounds"));
            }
        }
        Ok(())
    }
    for name in ITEMS {
        let path = paths.data.join(name);
        if !files::exists(&path)? {
            continue;
        }
        if name == "owner-artifacts" {
            artifacts(&path, name, &mut inventory)?;
        } else {
            inventory.files.push(name.into());
        }
    }
    inventory.files.sort();
    inventory.directories.sort();
    Ok(inventory)
}

pub(super) fn verify_installed(
    held: &LockedFiles,
    manifest: &manifest::Manifest,
    paths: &AppPaths,
) -> AppResult<()> {
    let actual = live_inventory_names(paths)?;
    let expected = files::Inventory {
        files: manifest.files.iter().map(|e| e.path.clone()).collect(),
        directories: manifest.directories.clone(),
    };
    if actual != expected {
        return Err(error("promoted generation inventory differs before commit"));
    }
    for entry in &manifest.files {
        let guard = held
            .files
            .get(&entry.path)
            .ok_or_else(|| error("a promoted file has no validated handle"))?;
        io(
            &paths.data.join(&entry.path),
            "verify promoted handle containment",
            native::validate_location(guard, &paths.data.join(&entry.path)),
        )?;
        if files::hash(guard)? != (entry.bytes, entry.sha256.clone()) {
            return Err(error("promoted bytes differ before commit"));
        }
    }
    for name in &manifest.directories {
        let guard = held
            .directories
            .get(name)
            .ok_or_else(|| error("a promoted directory has no validated handle"))?;
        io(
            &paths.data.join(name),
            "verify promoted directory containment",
            native::validate_location(guard, &paths.data.join(name)),
        )?;
    }
    Ok(())
}

pub(super) fn bundle(
    root: &Path,
    manifest: &manifest::Manifest,
    manifest_digest: &str,
) -> AppResult<LockedFiles> {
    let inventory = files::scan(root)?;
    let locked = lock(root, &inventory)?;
    let expected = manifest
        .files
        .iter()
        .map(|f| (f.path.clone(), (f.bytes, f.sha256.clone())))
        .collect::<Fingerprint>();
    let mut actual = locked.fingerprint.clone();
    let held_manifest = actual
        .remove(files::MANIFEST)
        .ok_or_else(|| error("staged manifest disappeared"))?;
    if held_manifest.1 != manifest_digest {
        return Err(error("staged manifest changed before exclusive locks"));
    }
    if actual != expected || inventory.directories != manifest.directories {
        return Err(error(
            "staged bytes changed before exclusive generation locks",
        ));
    }
    Ok(locked)
}
