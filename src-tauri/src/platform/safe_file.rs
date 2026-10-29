//! Bounded reads that retain the validated file handle through EOF.

use std::io;
use std::path::Path;

/// Open `path`, prove the opened object remains below `root`, and read no more
/// than `limit + 1` bytes through that same handle. The extra byte is the
/// caller's oversize sentinel.
pub fn read_under(
    root: &Path,
    path: &Path,
    limit: u64,
    destination: &mut Vec<u8>,
) -> io::Result<()> {
    #[cfg(windows)]
    {
        crate::platform::windows::safe_file::read_under(root, path, limit, destination)
    }
    #[cfg(not(windows))]
    {
        fallback::read_under(root, path, limit, destination)
    }
}

#[cfg(not(windows))]
mod fallback {
    use std::fs::File;
    use std::io::{self, Read};
    use std::path::Path;

    pub(super) fn read_under(
        root: &Path,
        path: &Path,
        limit: u64,
        destination: &mut Vec<u8>,
    ) -> io::Result<()> {
        let root = std::fs::canonicalize(root)?;
        let resolved = std::fs::canonicalize(path)?;
        if !resolved.starts_with(&root) || resolved == root {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "opened file is outside the approved root",
            ));
        }
        let mut file = File::open(resolved)?;
        file.by_ref()
            .take(limit.saturating_add(1))
            .read_to_end(destination)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn bounded_read_keeps_one_sentinel_byte() {
        let root = crate::testing::temp_dir("safe-file-bounded");
        let path = root.join("config.json");
        std::fs::write(&path, b"123456").unwrap();
        let mut bytes = Vec::new();
        read_under(&root, &path, 5, &mut bytes).unwrap();
        assert_eq!(bytes, b"123456");
    }

    #[test]
    fn path_outside_the_root_is_rejected() {
        let root = crate::testing::temp_dir("safe-file-root");
        let outside = crate::testing::temp_dir("safe-file-outside").join("config.json");
        std::fs::write(&outside, b"{}").unwrap();
        let mut bytes = Vec::new();
        assert!(read_under(&root, &outside, 10, &mut bytes).is_err());
        assert!(bytes.is_empty());
    }

    #[test]
    fn redirected_ancestor_cannot_escape_the_root() {
        let root = crate::testing::temp_dir("safe-file-link-root");
        let outside = crate::testing::temp_dir("safe-file-link-outside");
        let secret = outside.join("config.json");
        std::fs::write(&secret, b"{\"secret\":true}").unwrap();
        let redirect = root.join("redirect");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&outside, &redirect).is_ok();
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&outside, &redirect).is_ok();
        if !linked {
            return;
        }

        let mut bytes = Vec::new();
        assert!(read_under(&root, &redirect.join("config.json"), 100, &mut bytes).is_err());
        assert!(bytes.is_empty());
    }
}
