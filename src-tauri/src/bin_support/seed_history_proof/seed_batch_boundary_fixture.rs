//! Private proof fixture with exclusive ownership and validated teardown.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use what_changed_lib::storage::Db;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const PREFIX: &str = "what-changed-seed-batch-proof-";

pub(super) struct OwnedFixture {
    parent: PathBuf,
    root: PathBuf,
    db: Option<Db>,
}

impl OwnedFixture {
    pub(super) fn new() -> Self {
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
        fixture.db = Some(Db::open_in(&fixture.root).unwrap());
        fixture
    }

    pub(super) fn db(&self) -> &Db {
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
        if let Some(db) = self.db.take() {
            db.close();
            drop(db);
        }
        let cleanup = (|| -> std::io::Result<()> {
            let resolved = std::fs::canonicalize(&self.root)?;
            if !self.valid_target() || resolved != self.root {
                return Err(std::io::Error::other(
                    "owned batch proof cleanup target changed",
                ));
            }
            std::fs::remove_dir_all(&resolved)
        })();
        if let Err(error) = cleanup {
            if std::thread::panicking() {
                eprintln!("batch proof fixture cleanup failed while unwinding: {error}");
            } else {
                panic!("batch proof fixture cleanup failed: {error}");
            }
        }
    }
}

#[test]
fn private_batch_proof_fixture_cleans_normal_and_unwind_paths_without_touching_siblings() {
    let sibling = OwnedFixture::new();
    let fixture = OwnedFixture::new();
    let root = fixture.root.clone();
    drop(fixture);
    assert!(!root.exists());
    let unwinding = OwnedFixture::new();
    let unwind_root = unwinding.root.clone();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _owned = unwinding;
            panic!("owned batch proof cleanup unwind");
        }))
        .is_err()
    );
    assert!(!unwind_root.exists());
    assert!(sibling.root.exists());
    assert!(sibling.db().state().is_ready());
}
