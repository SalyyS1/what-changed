//! One built-in ignore policy shared by initial walks and watcher batches.

use std::path::{Path, MAIN_SEPARATOR_STR};

/// Config contents above this bound are never read.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".venv",
    "vendor",
];

const MEANINGFUL_LOCKFILES: &[&str] = &["package-lock.json", "bun.lock", "cargo.lock"];

/// Stateless built-in policy. It is deliberately not user-configurable in v1.
#[derive(Debug, Clone, Copy, Default)]
pub struct IgnoreSet;

impl IgnoreSet {
    pub const fn new() -> Self {
        Self
    }

    /// Decide before parsing or touching the database.
    pub fn ignores(
        &self,
        root: &Path,
        path: &Path,
        is_directory: bool,
        _size: Option<u64>,
        max_depth: u8,
    ) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let depth = relative.components().count();
        if depth > usize::from(max_depth).saturating_add(1) {
            return true;
        }

        let parts: Vec<String> = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();
        // The branch ref is the one intentional opening in `.git`; everything
        // else in that tree remains rejected before any file is read.
        if !is_directory && parts.as_slice() == [".git", "head"] {
            return false;
        }
        if parts
            .iter()
            .any(|part| IGNORED_DIRECTORIES.contains(&part.as_str()))
        {
            return true;
        }
        if is_directory {
            return false;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if MEANINGFUL_LOCKFILES.contains(&name.as_str()) {
            return false;
        }
        name.ends_with(".log")
            || name.ends_with(".tmp")
            || name.ends_with(".swp")
            || name.ends_with('~')
            || name.ends_with(".lock")
    }

    /// Text form useful in diagnostics without exposing platform separators.
    pub fn ignored_directories(&self) -> String {
        IGNORED_DIRECTORIES.join(MAIN_SEPARATOR_STR)
    }

    /// Content-read gate. Oversized files still produce ordinary file events;
    /// they are only excluded from config parsing.
    pub fn may_read_contents(&self, size: u64) -> bool {
        size <= MAX_FILE_BYTES
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn dependency_and_build_trees_are_rejected_before_files() {
        let root = Path::new(r"C:\work\app");
        let set = IgnoreSet::new();
        for path in [
            r"C:\work\app\node_modules\pkg\index.js",
            r"C:\work\app\.git\objects\1",
            r"C:\work\app\target\debug\app.exe",
            r"C:\work\app\.venv\Lib\site.py",
        ] {
            assert!(
                set.ignores(root, Path::new(path), false, Some(1), 6),
                "{path}"
            );
        }
    }

    #[test]
    fn meaningful_lockfiles_survive_the_lock_rule() {
        let root = Path::new(r"C:\work\app");
        let set = IgnoreSet::new();
        for name in ["package-lock.json", "bun.lock", "Cargo.lock"] {
            assert!(!set.ignores(root, &root.join(name), false, Some(10), 6));
        }
        assert!(set.ignores(root, &root.join("random.lock"), false, Some(10), 6));
    }

    #[test]
    fn only_git_head_passes_through_the_git_ignore_boundary() {
        let root = Path::new(r"C:\work\app");
        let set = IgnoreSet::new();
        assert!(!set.ignores(root, &root.join(".git/HEAD"), false, Some(20), 6));
        assert!(set.ignores(root, &root.join(".git/index"), false, Some(20), 6));
        assert!(set.ignores(root, &root.join(".git/objects/ab/cdef"), false, Some(20), 6));
    }

    #[test]
    fn depth_and_size_are_hard_bounds() {
        let root = Path::new(r"C:\work\app");
        let set = IgnoreSet::new();
        assert!(set.ignores(root, &root.join("a/b/c/d/config.json"), false, Some(1), 3));
        assert!(!set.ignores(
            root,
            &root.join("config.json"),
            false,
            Some(MAX_FILE_BYTES + 1),
            6
        ));
        assert!(!set.may_read_contents(MAX_FILE_BYTES + 1));
        assert!(set.may_read_contents(MAX_FILE_BYTES));
        assert!(!set.ignores(
            root,
            &root.join("config.json"),
            false,
            Some(MAX_FILE_BYTES),
            6
        ));
    }
}
