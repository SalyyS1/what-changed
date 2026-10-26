use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use notify::event::{ModifyKind, RenameMode};
use notify::EventKind;
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent};

use crate::collectors::ignore_set::IgnoreSet;
use crate::collectors::WakeHandle;

const MAX_PENDING_PATHS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LogicalChange {
    Created(PathBuf),
    Modified(PathBuf),
    Swept(PathBuf),
    SweptDeleted(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
    RenamedThenDeleted { from: PathBuf, to: PathBuf },
}

impl LogicalChange {
    fn key(&self) -> &Path {
        match self {
            Self::Created(path)
            | Self::Modified(path)
            | Self::Swept(path)
            | Self::SweptDeleted(path)
            | Self::Deleted(path) => path,
            Self::Renamed { to, .. } | Self::RenamedThenDeleted { to, .. } => to,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RootSpec {
    pub path: PathBuf,
    pub max_depth: u8,
}

#[derive(Default)]
struct Pending {
    changes: BTreeMap<PathBuf, LogicalChange>,
    sweep_needed: bool,
    watcher_reset_needed: bool,
}

#[derive(Clone, Default)]
pub(super) struct ChangeQueue {
    pending: Arc<Mutex<Pending>>,
}

impl ChangeQueue {
    pub fn push(&self, change: LogicalChange) -> bool {
        let mut pending = lock(&self.pending);
        let was_empty =
            pending.changes.is_empty() && !pending.sweep_needed && !pending.watcher_reset_needed;
        let key = change.key().to_path_buf();
        if pending.changes.len() >= MAX_PENDING_PATHS && !pending.changes.contains_key(&key) {
            pending.changes.clear();
            pending.sweep_needed = true;
            return was_empty;
        }
        match (pending.changes.remove(&key), change) {
            (Some(LogicalChange::Created(_)), LogicalChange::Modified(path)) => {
                pending.changes.insert(key, LogicalChange::Created(path));
            }
            (Some(LogicalChange::Created(_)), LogicalChange::Deleted(_)) => {}
            (Some(LogicalChange::Deleted(_)), LogicalChange::Created(path)) => {
                pending.changes.insert(key, LogicalChange::Modified(path));
            }
            (Some(LogicalChange::Renamed { from, .. }), LogicalChange::Modified(to)) => {
                pending
                    .changes
                    .insert(key, LogicalChange::Renamed { from, to });
            }
            (Some(LogicalChange::Renamed { from, .. }), LogicalChange::Created(to)) => {
                pending
                    .changes
                    .insert(key, LogicalChange::Renamed { from, to });
            }
            (Some(LogicalChange::Renamed { from, .. }), LogicalChange::Deleted(to)) => {
                pending
                    .changes
                    .insert(key, LogicalChange::RenamedThenDeleted { from, to });
            }
            (_, next) => {
                pending.changes.insert(key, next);
            }
        }
        was_empty
    }

    pub fn request_sweep(&self) -> bool {
        let mut pending = lock(&self.pending);
        let was_empty =
            pending.changes.is_empty() && !pending.sweep_needed && !pending.watcher_reset_needed;
        pending.sweep_needed = true;
        was_empty
    }

    pub fn request_watcher_reset(&self) -> bool {
        let mut pending = lock(&self.pending);
        let was_empty =
            pending.changes.is_empty() && !pending.sweep_needed && !pending.watcher_reset_needed;
        pending.sweep_needed = true;
        pending.watcher_reset_needed = true;
        was_empty
    }

    pub fn take_watcher_reset(&self) -> bool {
        std::mem::take(&mut lock(&self.pending).watcher_reset_needed)
    }

    pub fn drain(&self, limit: usize) -> (Vec<LogicalChange>, bool) {
        let mut pending = lock(&self.pending);
        let keys = pending
            .changes
            .keys()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let changes = keys
            .into_iter()
            .filter_map(|key| pending.changes.remove(&key))
            .collect();
        let sweep = std::mem::take(&mut pending.sweep_needed);
        (changes, sweep)
    }

    pub fn len(&self) -> usize {
        lock(&self.pending).changes.len()
    }
}

pub(super) type SharedRoots = Arc<RwLock<Vec<RootSpec>>>;

pub(super) fn callback(
    queue: ChangeQueue,
    roots: SharedRoots,
    wake: WakeHandle,
) -> impl FnMut(DebounceEventResult) + Send + 'static {
    move |result| {
        let should_wake = match result {
            Ok(events) => {
                let mut wake_needed = false;
                for change in events
                    .into_iter()
                    .filter_map(|event| classify(&roots, event))
                {
                    wake_needed = queue.push(change) || wake_needed;
                }
                wake_needed
            }
            Err(errors) => {
                tracing::debug!(
                    count = errors.len(),
                    "watched-folder notification errors; scheduling a sweep"
                );
                queue.request_watcher_reset()
            }
        };
        if should_wake {
            let _ = wake.request_snapshot();
        }
    }
}

fn classify(roots: &SharedRoots, event: DebouncedEvent) -> Option<LogicalChange> {
    let paths = &event.paths;
    match event.kind {
        EventKind::Create(_) => accepted(roots, paths.last()?).map(LogicalChange::Created),
        EventKind::Remove(_) => accepted(roots, paths.last()?).map(LogicalChange::Deleted),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if paths.len() >= 2 => {
            let from = paths.first()?.clone();
            let to = accepted(roots, paths.last()?)?;
            Some(LogicalChange::Renamed { from, to })
        }
        EventKind::Modify(ModifyKind::Name(_))
        | EventKind::Modify(_)
        | EventKind::Any
        | EventKind::Other => accepted(roots, paths.last()?).map(LogicalChange::Modified),
        EventKind::Access(_) => None,
    }
}

fn accepted(roots: &SharedRoots, path: &Path) -> Option<PathBuf> {
    let roots = read(roots);
    let root = roots
        .iter()
        .filter(|root| path.starts_with(&root.path))
        .max_by_key(|root| root.path.components().count())?;
    if !crate::app::validate::watched_path_still_safe(&root.path, path) {
        return None;
    }
    (!IgnoreSet::new().ignores(&root.path, path, false, None, root.max_depth))
        .then(|| path.to_path_buf())
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use notify::{event::CreateKind, Event};

    #[test]
    fn editor_temp_lifecycle_collapses_to_one_logical_modification() {
        let queue = ChangeQueue::default();
        let path = PathBuf::from("config.json");
        queue.push(LogicalChange::Deleted(path.clone()));
        queue.push(LogicalChange::Created(path.clone()));
        queue.push(LogicalChange::Modified(path.clone()));
        let (changes, _) = queue.drain(10);
        assert_eq!(changes, vec![LogicalChange::Modified(path)]);
    }

    #[test]
    fn ignored_dependency_storm_never_enters_the_queue() {
        let queue = ChangeQueue::default();
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: PathBuf::from("project"),
            max_depth: 6,
        }]));
        for index in 0..5_000 {
            let event = DebouncedEvent::new(
                Event::new(EventKind::Create(CreateKind::File)).add_path(
                    PathBuf::from("project/node_modules/pkg").join(format!("file-{index}.js")),
                ),
                std::time::Instant::now(),
            );
            assert!(classify(&roots, event).is_none());
        }
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn temp_rename_keeps_the_meaningful_destination() {
        let project = crate::testing::temp_dir("queue-rename");
        let destination = project.join("config.json");
        std::fs::write(&destination, "{}").unwrap();
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: project.clone(),
            max_depth: 6,
        }]));
        let event = DebouncedEvent::new(
            Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
                .add_path(project.join("config.json.tmp"))
                .add_path(destination.clone()),
            std::time::Instant::now(),
        );
        assert_eq!(
            classify(&roots, event),
            Some(LogicalChange::Renamed {
                from: project.join("config.json.tmp"),
                to: destination,
            })
        );
    }

    #[test]
    fn follow_up_create_keeps_the_rename_source() {
        let queue = ChangeQueue::default();
        queue.push(LogicalChange::Renamed {
            from: PathBuf::from("before.json"),
            to: PathBuf::from("after.json"),
        });
        queue.push(LogicalChange::Created(PathBuf::from("after.json")));
        let (changes, _) = queue.drain(10);
        assert_eq!(
            changes,
            vec![LogicalChange::Renamed {
                from: PathBuf::from("before.json"),
                to: PathBuf::from("after.json"),
            }]
        );
    }

    #[test]
    fn follow_up_delete_keeps_the_source_and_terminal_deletion() {
        let queue = ChangeQueue::default();
        queue.push(LogicalChange::Renamed {
            from: PathBuf::from("before.json"),
            to: PathBuf::from("after.json"),
        });
        queue.push(LogicalChange::Deleted(PathBuf::from("after.json")));
        let (changes, _) = queue.drain(10);
        assert_eq!(
            changes,
            vec![LogicalChange::RenamedThenDeleted {
                from: PathBuf::from("before.json"),
                to: PathBuf::from("after.json"),
            }]
        );
    }
}
