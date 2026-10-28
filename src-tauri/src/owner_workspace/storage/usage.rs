//! Aggregate SQLite inventory size without exposing paths or payloads to IPC.
use super::OwnerStore;

impl OwnerStore {
    pub fn owned_file_bytes(&self) -> Option<u64> {
        self.owned_files()
            .ok()?
            .into_iter()
            .try_fold(0_u64, |bytes, path| match std::fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                    bytes.checked_add(metadata.len())
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(bytes),
                _ => None,
            })
    }
}
