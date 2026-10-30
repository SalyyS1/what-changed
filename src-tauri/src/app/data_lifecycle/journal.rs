//! Small, typed durable intent. No paths, credentials or user text are stored.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{inventory, recovery_error};
use crate::domain::{Theme, VisualizationMode};
use crate::error::{AppError, AppResult};

const VERSION: u32 = 1;
const MAX_BYTES: u64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PurgeJournal {
    pub version: u32,
    pub epoch: u64,
    pub pending: bool,
    pub theme: Theme,
    pub visualization: VisualizationMode,
}

impl Default for PurgeJournal {
    fn default() -> Self {
        Self {
            version: VERSION,
            epoch: 0,
            pending: false,
            theme: Theme::System,
            visualization: VisualizationMode::Auto,
        }
    }
}

fn validate(record: &PurgeJournal) -> AppResult<()> {
    if record.version != VERSION
        || record.epoch > i64::MAX as u64
        || (record.pending && record.epoch == 0)
    {
        return Err(recovery_error(
            "the purge journal version or epoch is invalid",
        ));
    }
    Ok(())
}

/// Accept only idempotence, completion, or the next deletion generation.
fn transition(prior: &PurgeJournal, next: &PurgeJournal) -> AppResult<()> {
    let completion = prior.pending
        && next.epoch == prior.epoch
        && prior.theme == next.theme
        && prior.visualization == next.visualization;
    let advance = !prior.pending && next.pending && prior.epoch.checked_add(1) == Some(next.epoch);
    if prior != next && !completion && !advance {
        return Err(recovery_error(
            "the purge journal transition is inconsistent",
        ));
    }
    Ok(())
}

fn read(path: &Path) -> AppResult<Option<PurgeJournal>> {
    let Some(metadata) = inventory::regular_file(path)? else {
        return Ok(None);
    };
    if metadata.len() > MAX_BYTES {
        return Err(recovery_error("the purge journal is too large"));
    }
    let bytes = fs::read(path).map_err(|source| AppError::File {
        operation: "read purge journal",
        path: path.into(),
        source,
    })?;
    let record = serde_json::from_slice(&bytes)
        .map_err(|_| recovery_error("the purge journal is invalid; deletion remains closed"))?;
    validate(&record)?;
    Ok(Some(record))
}

/// The flushed temporary record is authoritative if a process exited between
/// removing the old record and renaming the new one. A partial temp fails closed.
pub(super) fn load(path: &Path) -> AppResult<Option<PurgeJournal>> {
    let current = read(path)?;
    let temp = inventory::companion(path, ".tmp");
    let Some(staged) = read(&temp)? else {
        return Ok(current);
    };
    if let Some(prior) = current.as_ref() {
        transition(prior, &staged)?;
    }
    install(&temp, path)?;
    Ok(Some(staged))
}

pub(super) fn save(path: &Path, record: &PurgeJournal) -> AppResult<()> {
    validate(record)?;
    if let Some(current) = load(path)? {
        transition(&current, record)?;
        if current == *record {
            return Ok(());
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| recovery_error("the purge journal has no owned parent"))?;
    inventory::directory_chain(parent)?;
    fs::create_dir_all(parent).map_err(|source| AppError::Directory {
        path: parent.into(),
        source,
    })?;
    let temp = inventory::companion(path, ".tmp");
    let bytes = serde_json::to_vec(record)
        .map_err(|_| recovery_error("the purge journal could not be encoded"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|source| AppError::File {
            operation: "create staged purge journal",
            path: temp.clone(),
            source,
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|source| AppError::File {
            operation: "flush staged purge journal",
            path: temp.clone(),
            source,
        })?;
    drop(file);
    install(&temp, path)
}

fn install(temp: &Path, path: &Path) -> AppResult<()> {
    inventory::regular_file(temp)?;
    if inventory::regular_file(path)?.is_some() {
        fs::remove_file(path).map_err(|source| AppError::File {
            operation: "replace purge journal",
            path: path.into(),
            source,
        })?;
    }
    fs::rename(temp, path).map_err(|source| AppError::File {
        operation: "install purge journal",
        path: path.into(),
        source,
    })?;
    // Flush the installed handle as well as the staged bytes. The staged/main
    // recovery pair never interprets a missing main file as completed deletion.
    OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| AppError::File {
            operation: "flush installed purge journal",
            path: path.into(),
            source,
        })
}

pub(super) fn appearance_json(value: &impl Serialize) -> AppResult<String> {
    serde_json::to_string(value).map_err(|_| recovery_error("appearance could not be encoded"))
}
