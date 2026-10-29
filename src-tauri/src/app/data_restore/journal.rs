//! A flushed intent binds complete snapshots; no journal supplies arbitrary paths.

use super::*;
use crate::domain::{Theme, VisualizationMode};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PurgeFence {
    version: u32,
    pub epoch: u64,
    pub pending: bool,
    theme: Theme,
    visualization: VisualizationMode,
}

pub(super) fn purge_fence(paths: &AppPaths) -> AppResult<PurgeFence> {
    let mut found = Vec::new();
    for name in ["data-purge.json", "data-purge.json.tmp"] {
        let path = paths.data.join(name);
        if files::exists(&path)? {
            let record: PurgeFence = serde_json::from_slice(&files::read(&path, 4096)?)
                .map_err(|_| error("purge journal is invalid; restore remains closed"))?;
            if record.version != 1
                || record.epoch > i64::MAX as u64
                || (record.pending && record.epoch == 0)
            {
                return Err(error("purge journal epoch/version is invalid"));
            }
            found.push(record);
        }
    }
    let mut fence = found.first().cloned().unwrap_or(PurgeFence {
        version: 1,
        epoch: 0,
        pending: false,
        theme: Theme::System,
        visualization: VisualizationMode::Auto,
    });
    for next in found.into_iter().skip(1) {
        if next.epoch < fence.epoch || next.epoch > fence.epoch.saturating_add(1) {
            return Err(error("purge journal transition is inconsistent"));
        }
        fence.pending |= next.pending;
        fence.epoch = next.epoch;
    }
    Ok(fence)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Prepared,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    pub version: u32,
    pub transaction: BackupId,
    pub epoch: u64,
    pub phase: Phase,
    pub target: BackupId,
    pub rollback: BackupId,
    pub target_manifest: String,
    pub rollback_manifest: String,
}

fn validate(record: &Journal) -> AppResult<()> {
    for id in [&record.transaction, &record.target, &record.rollback] {
        BackupId::parse(id.as_str())?;
    }
    if record.version != 1
        || record.epoch > i64::MAX as u64
        || [&record.target_manifest, &record.rollback_manifest]
            .iter()
            .any(|s| {
                s.len() != 64
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            })
    {
        return Err(error("restore journal format is invalid"));
    }
    Ok(())
}

fn read(path: &Path) -> AppResult<Option<Journal>> {
    if !files::exists(path)? {
        return Ok(None);
    }
    let record = serde_json::from_slice(&files::read(path, 4096)?)
        .map_err(|_| error("restore journal is incomplete or invalid"))?;
    validate(&record)?;
    Ok(Some(record))
}

pub(super) fn load(paths: &AppPaths) -> AppResult<Option<Journal>> {
    let path = paths.data.join(JOURNAL);
    let temp = paths.data.join("data-restore.json.tmp");
    let prior = read(&path)?;
    let Some(staged) = read(&temp)? else {
        return Ok(prior);
    };
    if let Some(prior) = prior {
        let mut expected = prior.clone();
        expected.phase = staged.phase;
        if expected != staged
            || (prior.phase == Phase::Committed && staged.phase != Phase::Committed)
        {
            return Err(error("restore journal transition is inconsistent"));
        }
    }
    io(
        &path,
        "install staged restore journal",
        native::durable_move(&temp, &path, true),
    )?;
    Ok(Some(staged))
}

pub(super) fn save(paths: &AppPaths, record: &Journal) -> AppResult<()> {
    validate(record)?;
    if let Some(prior) = load(paths)? {
        let mut expected = prior.clone();
        expected.phase = record.phase;
        if expected != *record
            || (prior.phase == Phase::Committed && record.phase != Phase::Committed)
        {
            return Err(error("cannot replace an active restore transaction"));
        }
    }
    let path = paths.data.join(JOURNAL);
    let temp = paths.data.join("data-restore.json.tmp");
    let bytes = serde_json::to_vec(record).map_err(|_| error("cannot encode restore intent"))?;
    files::write_new(&temp, &bytes)?;
    io(
        &path,
        "commit restore journal",
        native::durable_move(&temp, &path, true),
    )
}

pub(super) fn clear(paths: &AppPaths) -> AppResult<()> {
    for name in ["data-restore.json.tmp", JOURNAL] {
        let path = paths.data.join(name);
        if files::exists(&path)? {
            let _guard = io(
                &path,
                "lock completed restore journal",
                native::for_move(&path),
            )?;
            io(
                &path,
                "remove completed restore journal",
                std::fs::remove_file(&path),
            )?;
        }
    }
    Ok(())
}
