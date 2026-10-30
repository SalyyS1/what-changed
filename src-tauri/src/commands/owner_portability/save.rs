//! Write a complete bundle before replacing a user-selected destination.
use crate::error::{AppError, AppResult};
use crate::platform::managed_file_lock as native;
use std::io::{self, Write};
use std::path::Path;

pub(crate) fn bundle(destination: &Path, bytes: &[u8], replace: bool) -> AppResult<()> {
    let perform = || -> io::Result<()> {
        let parent = destination
            .parent()
            .ok_or_else(|| io::Error::other("missing parent"))?;
        let _parent = native::pin_directory(parent)?;
        match std::fs::symlink_metadata(destination) {
            Ok(_) if !replace => return Err(io::ErrorKind::AlreadyExists.into()),
            Ok(_) => {
                // Reject redirects, links and directories without following them.
                // The final replacement changes a directory entry, never its target.
                drop(native::for_read(destination)?);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let temporary = parent.join(format!(".project-memory-{}.tmp", uuid::Uuid::new_v4()));
        let mut output = native::create_new(&temporary)?;
        let write = output.write_all(bytes).and_then(|_| output.sync_all());
        drop(output);
        let result = write.and_then(|_| native::durable_move(&temporary, destination, replace));
        finish(result, &temporary)
    };
    perform().map_err(|source| AppError::File {
        operation: "save reviewed bundle",
        path: destination.into(),
        source,
    })
}

fn finish(result: io::Result<()>, temporary: &Path) -> io::Result<()> {
    let Err(source) = result else {
        return Ok(());
    };
    // A rename may have succeeded before a final flush/location check failed.
    // Report uncertainty and any remaining reviewed bytes instead of promising
    // that every error preserves the old destination.
    match std::fs::remove_file(temporary) {
        Ok(()) => Err(source),
        Err(cleanup) if cleanup.kind() == io::ErrorKind::NotFound => Err(io::Error::other(
            format!("{source}; inspect the selected file: replacement may already have completed"),
        )),
        Err(cleanup) => Err(io::Error::other(format!(
            "{source}; temporary export {} could not be removed ({cleanup}). It contains reviewed project data; inspect the selected folder and file before retrying",
            temporary.file_name().unwrap_or_default().to_string_lossy(),
        ))),
    }
}

#[cfg(test)]
#[cfg(windows)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn replacing_a_selected_file_requires_confirmation_and_preserves_other_files() {
        let root = crate::testing::temp_dir("bundle-save");
        let selected = root.join("memory.zip");
        let other = root.join("keep.txt");
        std::fs::write(&other, b"unrelated").unwrap();
        bundle(&selected, b"first", false).unwrap();
        assert!(bundle(&selected, b"unconfirmed", false).is_err());
        assert_eq!(std::fs::read(&selected).unwrap(), b"first");
        bundle(&selected, b"reviewed replacement", true).unwrap();
        assert_eq!(std::fs::read(&selected).unwrap(), b"reviewed replacement");
        assert_eq!(std::fs::read(&other).unwrap(), b"unrelated");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    }

    #[test]
    fn directories_and_linked_targets_are_preserved() {
        let root = crate::testing::temp_dir("bundle-save-target");
        let directory = root.join("directory.zip");
        std::fs::create_dir(&directory).unwrap();
        assert!(bundle(&directory, b"review", true).is_err());
        let original = root.join("original.zip");
        let linked = root.join("linked.zip");
        std::fs::write(&original, b"keep").unwrap();
        std::fs::hard_link(&original, &linked).unwrap();
        assert!(bundle(&linked, b"review", true).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"keep");
        assert!(directory.is_dir());
    }

    #[test]
    fn failed_cleanup_discloses_the_reviewed_bytes_left_in_the_selected_folder() {
        let root = crate::testing::temp_dir("bundle-save-cleanup");
        let temporary = root.join(".project-memory-fixture.tmp");
        let mut held = native::create_new(&temporary).unwrap();
        held.write_all(b"reviewed project data").unwrap();
        let error = finish(Err(io::Error::other("rename failed")), &temporary).unwrap_err();
        assert!(error.to_string().contains(".project-memory-fixture.tmp"));
        assert!(error.to_string().contains("contains reviewed project data"));
        drop(held);
        assert_eq!(std::fs::read(&temporary).unwrap(), b"reviewed project data");
        std::fs::remove_file(&temporary).unwrap();
    }
}
