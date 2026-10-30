//! Complete snapshots have a bounded SHA inventory written only after SQLite backup.

use super::*;
use crate::storage::data_generation::{self, StoreSchemas};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    version: u32,
    pub id: BackupId,
    pub epoch: u64,
    pub schemas: StoreSchemas,
    pub files: Vec<Entry>,
    pub directories: Vec<String>,
}

pub(super) fn create_backup(closed: &ClosedStores<'_>) -> AppResult<BackupId> {
    Ok(backup_with_fingerprint(closed)?.0)
}

pub(super) fn backup_with_fingerprint(
    closed: &ClosedStores<'_>,
) -> AppResult<(BackupId, locking::Fingerprint)> {
    closed.validate_epoch()?;
    if journal::load(&closed.paths)?.is_some() {
        return Err(error("recover the active restore before creating a backup"));
    }
    let root = closed.paths.data.join(BACKUPS);
    files::directory(&root)?;
    if files::generation_ids(&root)?.len() >= MAX_GENERATIONS {
        return Err(error("backup generation capacity reached"));
    }
    // Probe all existing SQLite handles before temporarily opening read-only
    // SQLite sources ourselves. Recheck stable DB/WAL/artifact bytes afterward.
    let before = locking::live(&closed.paths)?.fingerprint;
    let id = BackupId::new();
    let target = closed.backup_path(&id)?;
    files::directory(&target)?;
    let _target_parents = files::pin_chain(&target)?;
    let mut reserved = Vec::new();
    for name in ["events.db", "owner-workspace.sqlite"] {
        let path = target.join(name);
        reserved.push((
            name.to_owned(),
            io(
                &path,
                "reserve SQLite generation object",
                native::reserve_sqlite(&path),
            )?,
        ));
    }
    let schemas = data_generation::snapshot_pair(&closed.paths.data, &target)?;
    let mut sqlite_hashes = std::collections::BTreeMap::new();
    for (name, file) in &reserved {
        io(
            &target.join(name),
            "flush reserved SQLite generation",
            file.sync_all(),
        )?;
        sqlite_hashes.insert(name.clone(), files::hash(file)?);
    }
    drop(reserved);
    let artifacts = closed.paths.data.join("owner-artifacts");
    if files::exists(&artifacts)? {
        let inventory = files::scan_at(&artifacts, "owner-artifacts")?;
        files::clone_inventory(&closed.paths.data, &target, &inventory)?;
    } else {
        files::directory(&target.join("owner-artifacts"))?;
    }
    let after = locking::live(&closed.paths)?.fingerprint;
    if before != after {
        return Err(error(
            "live committed files changed while SQLite backup was taken",
        ));
    }
    closed.validate_epoch()?;
    let inventory = files::scan(&target)?;
    let mut entries = Vec::new();
    for name in inventory.files {
        let path = target.join(&name);
        let file = io(&path, "hash complete generation", native::for_read(&path))?;
        let (bytes, sha256) = files::hash(&file)?;
        if sqlite_hashes
            .get(&name)
            .is_some_and(|expected| expected != &(bytes, sha256.clone()))
        {
            return Err(error(
                "SQLite snapshot object changed after its reservation was released",
            ));
        }
        // Flushing read-only handles is not supported on Windows. Snapshot
        // files were synced when written/closed by SQLite or our copy routine.
        entries.push(Entry {
            path: name,
            bytes,
            sha256,
        });
    }
    let manifest = Manifest {
        version: 1,
        id: id.clone(),
        epoch: closed.epoch,
        schemas,
        files: entries,
        directories: inventory.directories,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| error("cannot encode generation manifest"))?;
    files::write_new(&target.join(files::MANIFEST), &bytes)?;
    validate(closed, &id)?;
    Ok((id, after))
}

pub(super) fn validate(closed: &ClosedStores<'_>, id: &BackupId) -> AppResult<(Manifest, String)> {
    let root = closed.backup_path(id)?;
    validate_at(&root, id, closed.epoch)
}

pub(super) fn validate_at(root: &Path, id: &BackupId, epoch: u64) -> AppResult<(Manifest, String)> {
    let _parents = files::pin_chain(root)?;
    let bytes = files::read(&root.join(files::MANIFEST), 2 * 1024 * 1024)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|_| error("generation manifest is incomplete or unsupported"))?;
    BackupId::parse(manifest.id.as_str())?;
    if manifest.version != 1
        || manifest.id != *id
        || manifest.epoch != epoch
        || manifest.files.len() > MAX_FILES
    {
        return Err(error("generation version, identity or purge epoch differs"));
    }
    let inventory = files::scan(root)?;
    let expected = manifest
        .files
        .iter()
        .map(|e| e.path.clone())
        .collect::<Vec<_>>();
    let mut actual = inventory.files.clone();
    actual.retain(|s| s != files::MANIFEST);
    if actual != expected
        || inventory.directories != manifest.directories
        || !expected.iter().any(|s| s == "events.db")
        || !expected.iter().any(|s| s == "owner-workspace.sqlite")
        || !manifest.directories.iter().any(|s| s == "owner-artifacts")
        || expected.iter().any(|s| {
            s != "events.db" && s != "owner-workspace.sqlite" && !s.starts_with("owner-artifacts/")
        })
    {
        return Err(error(
            "generation inventory is partial or contains unsupported files",
        ));
    }
    for entry in &manifest.files {
        let path = root.join(&entry.path);
        let file = io(&path, "validate generation digest", native::for_read(&path))?;
        if files::hash(&file)? != (entry.bytes, entry.sha256.clone()) {
            return Err(error("generation checksum differs"));
        }
    }
    if data_generation::validate_pair(root)? != manifest.schemas {
        return Err(error("generation schema manifest differs"));
    }
    use sha2::{Digest, Sha256};
    Ok((manifest, format!("{:x}", Sha256::digest(&bytes))))
}
