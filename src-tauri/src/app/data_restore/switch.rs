//! Deterministic crash recovery: prepared restores roll back, committed ones finish.

use super::*;
use journal::{Journal, Phase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Boundary {
    Prepared,
    Displaced(usize),
    Promoted(usize),
    ArtifactDisplaced(usize),
    ArtifactPromoted(usize),
    DisplacedArtifactsBeforeCleanup,
    PromotedArtifactsBeforeCleanup,
    Committed,
}

pub(super) fn restore(
    closed: &mut ClosedStores<'_>,
    source: &BackupId,
) -> AppResult<RestoreReceipt> {
    restore_with_hook(closed, source, |_| Ok(()))
}

pub(super) fn restore_with_hook(
    closed: &mut ClosedStores<'_>,
    source: &BackupId,
    mut hook: impl FnMut(Boundary) -> AppResult<()>,
) -> AppResult<RestoreReceipt> {
    closed.validate_epoch()?;
    if journal::load(&closed.paths)?.is_some() {
        return Err(error("recover the active restore first"));
    }
    let (_, target_manifest) = manifest::validate(closed, source)?;
    let (rollback, preimage) = manifest::backup_with_fingerprint(closed)?;
    let (_, rollback_manifest) = manifest::validate(closed, &rollback)?;
    let record = Journal {
        version: 1,
        transaction: BackupId::new(),
        epoch: closed.epoch,
        phase: Phase::Prepared,
        target: source.clone(),
        rollback: rollback.clone(),
        target_manifest,
        rollback_manifest,
    };
    let (root, replay, manifest, digest) = prepare_replay(closed, &record, source)?;
    let mut incoming = locking::bundle(&replay, &manifest, &digest)?;
    let mut live = locking::live(&closed.paths)?;
    if live.fingerprint != preimage {
        return Err(error("live generation changed after its rollback snapshot"));
    }
    closed.validate_epoch()?;
    journal::save(&closed.paths, &record)?;
    hook(Boundary::Prepared)?;
    install(closed, &root, &replay, &mut live, &mut incoming, &mut hook)?;
    locking::verify_installed(&incoming, &manifest, &closed.paths)?;
    closed.validate_epoch()?;
    journal::save(
        &closed.paths,
        &Journal {
            phase: Phase::Committed,
            ..record.clone()
        },
    )?;
    hook(Boundary::Committed)?;
    // These same handles protected incoming hashes and all live/sidecar names
    // throughout the switch. Release before enumerating obsolete files.
    drop(live);
    drop(incoming);
    cleanup(&root)?;
    journal::clear(&closed.paths)?;
    Ok(RestoreReceipt {
        restored: source.clone(),
        rollback,
    })
}

fn transaction_root(closed: &ClosedStores<'_>, record: &Journal) -> AppResult<PathBuf> {
    BackupId::parse(record.transaction.as_str())?;
    let namespace = closed.paths.data.join(RESTORES);
    files::directory(&namespace)?;
    if files::generation_ids(&namespace)?.len() >= MAX_GENERATIONS
        && !files::exists(&namespace.join(record.transaction.as_str()))?
    {
        return Err(error("restore staging capacity reached"));
    }
    let root = namespace.join(record.transaction.as_str());
    files::directory(&root)?;
    Ok(root)
}

fn prepare_replay(
    closed: &ClosedStores<'_>,
    record: &Journal,
    selected: &BackupId,
) -> AppResult<(PathBuf, PathBuf, manifest::Manifest, String)> {
    let (manifest, digest) = manifest::validate(closed, selected)?;
    let expected = if *selected == record.rollback {
        &record.rollback_manifest
    } else {
        &record.target_manifest
    };
    if &digest != expected {
        return Err(error("journal snapshot manifest checksum differs"));
    }
    let root = transaction_root(closed, record)?;
    let replay = root.join("replay");
    files::remove_tree(&replay)?;
    let _parents = files::pin_chain(&root)?;
    files::clone_tree(&closed.backup_path(selected)?, &replay)?;
    let (copy, copy_digest) = manifest::validate_at(&replay, selected, closed.epoch)?;
    if copy != manifest || copy_digest != digest {
        return Err(error("staged generation differs from selected snapshot"));
    }
    Ok((root, replay, manifest, digest))
}

fn install(
    closed: &ClosedStores<'_>,
    root: &Path,
    replay: &Path,
    live: &mut locking::LockedFiles,
    incoming: &mut locking::LockedFiles,
    hook: &mut impl FnMut(Boundary) -> AppResult<()>,
) -> AppResult<()> {
    let displaced = root.join("displaced");
    files::remove_tree(&displaced)?;
    files::directory(&displaced)?;
    let _parents = files::pin_chain(root)?;
    for (index, name) in ITEMS.into_iter().enumerate() {
        closed.validate_epoch()?;
        let from = closed.paths.data.join(name);
        let to = displaced.join(name);
        if files::exists(&from)? {
            live.rename_root(name, &from, &to, &mut |point| {
                hook(match point {
                    locking::ArtifactBoundary::FileMoved(index) => {
                        Boundary::ArtifactDisplaced(index)
                    }
                    locking::ArtifactBoundary::BeforeCleanup => {
                        Boundary::DisplacedArtifactsBeforeCleanup
                    }
                })
            })?;
        }
        hook(Boundary::Displaced(index))?;
    }
    for (index, name) in ["events.db", "owner-workspace.sqlite", "owner-artifacts"]
        .into_iter()
        .enumerate()
    {
        closed.validate_epoch()?;
        let from = replay.join(name);
        let to = closed.paths.data.join(name);
        incoming.rename_root(name, &from, &to, &mut |point| {
            hook(match point {
                locking::ArtifactBoundary::FileMoved(index) => Boundary::ArtifactPromoted(index),
                locking::ArtifactBoundary::BeforeCleanup => {
                    Boundary::PromotedArtifactsBeforeCleanup
                }
            })
        })?;
        hook(Boundary::Promoted(index))?;
    }
    Ok(())
}

pub(super) fn recover(closed: &mut ClosedStores<'_>) -> AppResult<RecoveryOutcome> {
    let Some(record) = journal::load(&closed.paths)? else {
        return Ok(RecoveryOutcome::Clean);
    };
    if record.epoch != closed.epoch {
        // A completed privacy tombstone also invalidates an old restore copied
        // back into app-owned storage. Never open or apply its snapshots.
        files::purge_locked(&closed.paths)?;
        return Ok(RecoveryOutcome::PurgeRequired);
    }
    closed.validate_epoch()?;
    let selected = match record.phase {
        Phase::Prepared => &record.rollback,
        Phase::Committed => &record.target,
    };
    let (root, replay, manifest, digest) = prepare_replay(closed, &record, selected)?;
    let mut incoming = locking::bundle(&replay, &manifest, &digest)?;
    let mut live = locking::live(&closed.paths)?;
    install(
        closed,
        &root,
        &replay,
        &mut live,
        &mut incoming,
        &mut |_| Ok(()),
    )?;
    locking::verify_installed(&incoming, &manifest, &closed.paths)?;
    closed.validate_epoch()?;
    drop(live);
    drop(incoming);
    cleanup(&root)?;
    journal::clear(&closed.paths)?;
    Ok(match record.phase {
        Phase::Prepared => RecoveryOutcome::RolledBack,
        Phase::Committed => RecoveryOutcome::Completed,
    })
}

fn cleanup(root: &Path) -> AppResult<()> {
    for name in ["replay", "displaced"] {
        files::remove_tree(&root.join(name))?;
    }
    io(
        root,
        "remove finished restore transaction",
        std::fs::remove_dir(root),
    )
}
