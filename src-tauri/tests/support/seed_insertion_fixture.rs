//! Exclusive diagnostic directory; never adopts or recursively removes a parent.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use what_changed_lib::storage::Db;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const PREFIX: &str = "what-changed-seed-insertion-";

pub struct SeedInsertionFixture {
    parent: PathBuf,
    root: PathBuf,
    db: Option<Db>,
}

impl SeedInsertionFixture {
    pub fn new() -> Self {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let workspace = std::fs::canonicalize(workspace).unwrap();
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
        // Resolve and validate only after exclusive creation; the RAII owner
        // already exists if resolution/validation subsequently panics.
        fixture.root = std::fs::canonicalize(&fixture.root).unwrap();
        assert!(fixture.valid_target());
        fixture
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn open(&mut self) {
        assert!(self.db.is_none());
        self.db = Some(Db::open_in(&self.root).unwrap());
    }

    pub fn db(&self) -> &Db {
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

impl Drop for SeedInsertionFixture {
    fn drop(&mut self) {
        if let Some(db) = self.db.take() {
            db.close();
            drop(db);
        }
        let cleanup = (|| -> std::io::Result<()> {
            let resolved = std::fs::canonicalize(&self.root)?;
            if !self.valid_target() || resolved != self.root {
                return Err(std::io::Error::other(
                    "owned diagnostic cleanup target changed",
                ));
            }
            std::fs::remove_dir_all(&resolved)
        })();
        if let Err(error) = cleanup {
            if std::thread::panicking() {
                eprintln!("seed insertion fixture cleanup failed while unwinding: {error}");
            } else {
                panic!("seed insertion fixture cleanup failed: {error}");
            }
        }
    }
}

#[test]
fn diagnostic_fixture_cleanup_closes_db_and_preserves_siblings_on_unwind() {
    let mut sibling = SeedInsertionFixture::new();
    sibling.open();
    let mut fixture = SeedInsertionFixture::new();
    fixture.open();
    let root = fixture.root.clone();
    drop(fixture);
    assert!(!root.exists());
    let mut unwinding = SeedInsertionFixture::new();
    unwinding.open();
    let unwind_root = unwinding.root.clone();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _owned = unwinding;
            panic!("owned insertion fixture unwind");
        }))
        .is_err()
    );
    assert!(!unwind_root.exists());
    assert!(sibling.root.exists());
    assert!(sibling.db().state().is_ready());
}
