//! Exclusive workspace-local databases with checked normal/unwind cleanup.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use what_changed_lib::storage::Db;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const PREFIX: &str = "what-changed-seed-grouping-";

pub(super) struct OwnedFixture {
    parent: PathBuf,
    root: PathBuf,
    db: Option<Arc<Db>>,
}

impl OwnedFixture {
    pub fn new() -> Self {
        let workspace =
            std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap();
        let parent = workspace.join(".perf-temp");
        std::fs::create_dir_all(&parent).unwrap();
        let parent = std::fs::canonicalize(parent).unwrap();
        assert!(parent.starts_with(&workspace));
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = parent.join(format!("{PREFIX}{}-{nonce}-{serial}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let mut fixture = Self {
            parent,
            root,
            db: None,
        };
        fixture.root = std::fs::canonicalize(&fixture.root).unwrap();
        assert!(fixture.valid_target());
        fixture.db = Some(Arc::new(Db::open_in(&fixture.root).unwrap()));
        fixture
    }

    pub fn db(&self) -> &Arc<Db> {
        self.db.as_ref().unwrap()
    }

    fn valid_target(&self) -> bool {
        self.root.parent() == Some(self.parent.as_path())
            && self
                .root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(PREFIX))
    }
}

impl Drop for OwnedFixture {
    fn drop(&mut self) {
        let cleanup = (|| -> std::io::Result<()> {
            if let Some(db) = self.db.take() {
                let exclusive = Arc::strong_count(&db) == 1;
                db.close(); // Drains and joins the owned writer before deletion.
                drop(db);
                if !exclusive {
                    return Err(std::io::Error::other(
                        "group fixture database ownership escaped",
                    ));
                }
            }
            let resolved_parent = std::fs::canonicalize(&self.parent)?;
            let resolved = std::fs::canonicalize(&self.root)?;
            if !self.valid_target() || resolved != self.root || resolved_parent != self.parent {
                return Err(std::io::Error::other(
                    "owned group fixture cleanup target changed",
                ));
            }
            std::fs::remove_dir_all(&resolved)
        })();
        if let Err(error) = cleanup {
            if std::thread::panicking() {
                eprintln!("group fixture cleanup failed while unwinding: {error}");
            } else {
                panic!("group fixture cleanup failed: {error}");
            }
        }
    }
}

#[test]
fn grouped_fixture_cleans_normal_and_unwind_paths_without_touching_siblings() {
    let sibling = OwnedFixture::new();
    let fixture = OwnedFixture::new();
    let root = fixture.root.clone();
    drop(fixture);
    assert!(!root.exists());
    let fixture = OwnedFixture::new();
    let root = fixture.root.clone();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _owned = fixture;
            panic!("owned grouping fixture unwind");
        }))
        .is_err()
    );
    assert!(!root.exists());
    assert!(sibling.root.exists());
    assert!(sibling.db().state().is_ready());
}
