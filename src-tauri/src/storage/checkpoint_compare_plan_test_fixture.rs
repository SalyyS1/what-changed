//! Exclusive migrated SQLite service fixtures; no collectors or user paths.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crate::{
    app::Monitoring,
    checkpoint::{prepare_capture, CheckpointService},
    collectors::{registry::CollectorRegistry, runtime::CollectorSupervisor, schedule::QueryBusy},
    domain::{CheckpointItem, CheckpointKind, MonitoringProfile},
    privacy::Sanitizer,
    storage::{checkpoint_repository, Db},
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
const PREFIX: &str = "what-changed-checkpoint-compare-";

struct Directory {
    root: PathBuf,
    parent: PathBuf,
}
impl Directory {
    fn new() -> Self {
        let workspace = std::fs::canonicalize(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap(),
        )
        .unwrap();
        let parent = workspace.join(".perf-temp");
        std::fs::create_dir_all(&parent).unwrap();
        let parent = std::fs::canonicalize(parent).unwrap();
        assert!(parent.starts_with(workspace));
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = parent.join(format!(
            "{PREFIX}{}-{nonce}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self { root, parent }
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let cleanup = (|| -> std::io::Result<()> {
            let resolved = std::fs::canonicalize(&self.root)?;
            if resolved != self.root
                || resolved.parent() != Some(self.parent.as_path())
                || !resolved
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(PREFIX))
            {
                return Err(std::io::Error::other(
                    "owned comparison fixture target changed",
                ));
            }
            std::fs::remove_dir_all(&resolved)
        })();
        if let Err(error) = cleanup {
            if std::thread::panicking() {
                eprintln!("comparison fixture cleanup failed while unwinding: {error}");
            } else {
                panic!("comparison fixture cleanup failed: {error}");
            }
        }
    }
}

pub(super) struct Fixture {
    pub(super) service: Option<CheckpointService>,
    supervisor: Option<CollectorSupervisor>,
    db: Option<Arc<Db>>,
    directory: Directory,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let directory = Directory::new();
        let db = Arc::new(Db::open_in(&directory.root).unwrap());
        let supervisor = CollectorSupervisor::new(
            CollectorRegistry::default(),
            Arc::clone(&db),
            Arc::new(Monitoring::running()),
            QueryBusy::default(),
            &BTreeMap::new(),
            MonitoringProfile::Balanced,
        );
        let service = CheckpointService::new(Arc::clone(&db), supervisor.capture_handle());
        Self {
            service: Some(service),
            supervisor: Some(supervisor),
            db: Some(db),
            directory,
        }
    }
    pub(super) fn db(&self) -> &Db {
        self.db.as_ref().unwrap()
    }
    pub(super) fn service(&self) -> &CheckpointService {
        self.service.as_ref().unwrap()
    }
    pub(super) fn checkpoint(&self, timestamp: i64, items: Vec<CheckpointItem>) -> i64 {
        let collectors = items.iter().map(|item| item.collector_id.clone()).collect();
        let (capture, _) = prepare_capture(
            Some("Owned comparison fixture".to_owned()),
            CheckpointKind::Manual,
            None,
            timestamp,
            0,
            collectors,
            Vec::new(),
            items,
            &Sanitizer::new(),
        );
        self.db()
            .write(move |conn| checkpoint_repository::create(conn, &capture))
            .unwrap()
            .id
    }
    pub(super) fn events(&self, timestamps: &[i64]) -> Vec<i64> {
        self.db()
            .insert_events(
                timestamps
                    .iter()
                    .enumerate()
                    .map(|(index, timestamp)| {
                        Sanitizer::new()
                            .sanitize(crate::testing::synthetic_event(index as u64, *timestamp))
                            .0
                    })
                    .collect(),
            )
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.service.take());
        drop(self.supervisor.take()); // No workers started; still joins anything owned.
        if let Some(db) = self.db.take() {
            db.close();
            drop(db);
        }
        // Directory drops only after every SQLite handle above has closed.
    }
}

#[test]
fn owned_comparison_fixture_closes_on_return_and_unwind_without_touching_sibling() {
    let sibling = Fixture::new();
    let fixture = Fixture::new();
    let root = fixture.directory.root.clone();
    drop(fixture);
    assert!(!root.exists() && sibling.directory.root.exists());
    let fixture = Fixture::new();
    let root = fixture.directory.root.clone();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _fixture = fixture;
            panic!("owned comparison fixture unwind");
        }))
        .is_err()
    );
    assert!(!root.exists() && sibling.directory.root.exists());
    assert!(sibling.db().state().is_ready());
}
