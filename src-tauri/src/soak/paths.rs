//! Fresh, exclusively owned QA roots and non-overwriting protocol files.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::app::AppPaths;
use crate::error::{AppError, AppResult};

pub(super) const READY: &str = "soak-ready.json";
pub(super) const REPORT: &str = "soak-result.json";

pub(super) struct OwnedRoot {
    pub root: PathBuf,
    pub trigger: PathBuf,
    pub ack: PathBuf,
    // On Windows, deny-delete directory handles prevent ancestor replacement
    // while SQLite and the file protocol are using this explicitly owned tree.
    _pins: Vec<File>,
}

fn invalid(reason: &str) -> AppError {
    AppError::InvalidInput {
        what: "soak paths",
        reason: reason.to_owned(),
    }
}

fn io_error(path: &Path, source: std::io::Error) -> AppError {
    AppError::File {
        operation: "soak protocol",
        path: path.to_owned(),
        source,
    }
}

fn is_alias(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn validate_absolute(path: &Path) -> AppResult<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(invalid(
            "paths must be absolute and contain no parent traversal",
        ));
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_)))
    {
        return Err(invalid("only ordinary local drive paths are accepted"));
    }
    // Win32 trims trailing spaces/dots and interprets colons as alternate
    // streams. Accept only ordinary components, including for nonexistent roots.
    for part in path.components() {
        if let Component::Normal(name) = part {
            let name = name.to_string_lossy();
            if name.ends_with(['.', ' ']) || name.contains(':') {
                return Err(invalid("noncanonical path component"));
            }
        }
    }
    Ok(())
}

#[allow(unused_mut)] // Only Windows needs live directory handles.
fn pin_existing_directories(path: &Path) -> AppResult<Vec<File>> {
    let mut pins = Vec::new();
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|e| io_error(ancestor, e))?;
        if is_alias(&metadata) || !metadata.is_dir() {
            return Err(invalid(
                "directory aliases and reparse points are forbidden",
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let file = OpenOptions::new()
                .read(true)
                .share_mode(3)
                .custom_flags(0x0200_0000 | 0x0020_0000)
                .open(ancestor)
                .map_err(|e| io_error(ancestor, e))?;
            if is_alias(&file.metadata().map_err(|e| io_error(ancestor, e))?) {
                return Err(invalid("directory changed to a reparse point"));
            }
            pins.push(file);
        }
    }
    Ok(pins)
}

fn protocol_child(root: &Path, child: &Path) -> AppResult<PathBuf> {
    let path = if child.is_absolute() {
        child.to_owned()
    } else {
        root.join(child)
    };
    validate_absolute(&path)?;
    if path.parent() != Some(root) {
        return Err(invalid(
            "protocol files must be direct children of the QA root",
        ));
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| invalid("invalid protocol filename"))?;
    // A separate namespace prevents collisions with SQLite, logs and startup
    // settings. Existing callers may keep trigger.txt / ack.txt names.
    if !name
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        || !name.to_ascii_lowercase().ends_with(".txt")
    {
        return Err(invalid(
            "trigger and acknowledgement names must be ordinary .txt files",
        ));
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        return Err(invalid("reserved device filename"));
    }
    Ok(path)
}

impl OwnedRoot {
    pub fn create(root: &Path, trigger: &Path, ack: &Path) -> AppResult<Self> {
        validate_absolute(root)?;
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if !name.starts_with("what-changed-perf-soak-")
            || name.len() <= "what-changed-perf-soak-".len()
        {
            return Err(invalid(
                "expected a fresh reserved what-changed-perf-soak-* directory",
            ));
        }
        let trigger = protocol_child(root, trigger)?;
        let ack = protocol_child(root, ack)?;
        // Both paths already have the same direct parent. Comparing filenames
        // also handles equivalent separator and current-directory spellings.
        let trigger_name = trigger
            .file_name()
            .ok_or_else(|| invalid("missing trigger filename"))?;
        let ack_name = ack
            .file_name()
            .ok_or_else(|| invalid("missing acknowledgement filename"))?;
        if trigger_name
            .to_string_lossy()
            .eq_ignore_ascii_case(&ack_name.to_string_lossy())
        {
            return Err(invalid("trigger and acknowledgement files must differ"));
        }
        let parent = root
            .parent()
            .ok_or_else(|| invalid("QA root requires an existing parent"))?;
        let mut pins = pin_existing_directories(parent)?;
        // Canonicalize before creation: Windows short (8.3) names are aliases
        // without reparse attributes. Resolve the nearest existing ancestor of
        // normal app data too, so a missing normal directory remains excluded.
        let root_text = resolved_key(root)?;
        let normal_text = resolved_key(&AppPaths::resolve().data)?;
        if root_text == normal_text || root_text.starts_with(&format!("{normal_text}/")) {
            return Err(invalid("normal application data cannot be used by soak"));
        }
        // create_dir is exclusive; even an empty preexisting root is rejected.
        fs::create_dir(root).map_err(|e| io_error(root, e))?;
        pins.extend(pin_existing_directories(root)?);
        Ok(Self {
            root: root.to_owned(),
            trigger,
            ack,
            _pins: pins,
        })
    }
}

fn resolved_key(path: &Path) -> AppResult<String> {
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| invalid("unresolvable path"))?
                        .to_owned(),
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| invalid("unresolvable path"))?;
            }
            Err(error) => return Err(io_error(ancestor, error)),
        }
    }
    let mut canonical = fs::canonicalize(ancestor).map_err(|e| io_error(ancestor, e))?;
    for name in missing.into_iter().rev() {
        canonical.push(name);
    }
    Ok(canonical
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_ascii_lowercase())
}

pub(super) fn write_new(path: &Path, text: &str) -> AppResult<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| io_error(&temporary, e))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| io_error(&temporary, e))?;
    drop(file);
    // Hard-link publication is atomic and refuses any existing destination,
    // including a dangling symlink. rename may replace files on some platforms.
    let result = fs::hard_link(&temporary, path).map_err(|e| io_error(path, e));
    let cleanup = fs::remove_file(&temporary).map_err(|e| io_error(&temporary, e));
    result.and(cleanup)
}

pub(super) fn read_trigger(path: &Path) -> AppResult<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(path, error)),
    };
    if is_alias(&metadata) || !metadata.is_file() || metadata.len() > 128 {
        return Err(invalid(
            "trigger must be a regular file of at most 128 bytes",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000).share_mode(1);
    }
    let file = options.open(path).map_err(|e| io_error(path, e))?;
    if is_alias(&file.metadata().map_err(|e| io_error(path, e))?) {
        return Err(invalid("trigger became an alias"));
    }
    let mut payload = String::new();
    file.take(129)
        .read_to_string(&mut payload)
        .map_err(|e| io_error(path, e))?;
    if payload.len() > 128 {
        return Err(invalid("trigger exceeded the byte limit"));
    }
    Ok(Some(payload))
}
