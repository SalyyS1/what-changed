//! Where the application's files live.
//!
//! One folder under the per-user roaming application data directory, named
//! by the application identifier, holding the databases, logs and app-owned
//! lifecycle metadata. External source projects and user-chosen exports stay
//! outside this inventory. The path comes from the operating system's known-folder
//! API, never from a hardcoded string or an environment variable read by
//! hand.

use std::path::{Path, PathBuf};

use crate::storage::DbPaths;

/// The application identifier, which is also the data folder's name.
pub const APP_IDENTIFIER: &str = "dev.whatchanged.app";

/// The independent owner-workspace database. It deliberately does not reuse
/// the legacy event filename so upgrades and privacy operations can keep the
/// two stores' identities visible.
pub const OWNER_WORKSPACE_FILE: &str = "owner-workspace.sqlite";

/// The durable cross-store deletion journal. A journal is app-owned state, not
/// an event or owner-workspace record, and is checked before either store opens.
pub const DATA_PURGE_JOURNAL_FILE: &str = "data-purge.json";

/// The application's directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// The data folder.
    pub data: PathBuf,
    /// Rolling log files.
    pub logs: PathBuf,
    /// The database and its companions.
    pub db: DbPaths,
    /// The crash-loop counter.
    pub startup_marker: PathBuf,
}

impl AppPaths {
    /// The layout under a data folder.
    pub fn under(data: &Path) -> Self {
        Self {
            data: data.to_path_buf(),
            logs: data.join("logs"),
            db: DbPaths::under(data),
            startup_marker: data.join("startup.json"),
        }
    }

    /// The owner-workspace store under the same resolved application root.
    pub fn owner_workspace(&self) -> PathBuf {
        self.db.root.join(OWNER_WORKSPACE_FILE)
    }

    /// The global deletion journal under the application root.
    pub fn purge_journal(&self) -> PathBuf {
        self.data.join(DATA_PURGE_JOURNAL_FILE)
    }

    /// The per-user data folder for this application.
    ///
    /// Falls back to the temporary directory only if the platform has no
    /// application data folder at all, which on Windows means a broken
    /// profile; a warning is logged so the fallback is never silent.
    pub fn resolve() -> Self {
        let base = dirs::data_dir().unwrap_or_else(|| {
            tracing::warn!("no application data directory; using the temporary directory");
            std::env::temp_dir()
        });
        Self::under(&base.join(APP_IDENTIFIER))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_sits_under_the_one_data_folder() {
        let paths = AppPaths::under(Path::new(
            r"C:\Users\dev\AppData\Roaming\dev.whatchanged.app",
        ));
        assert!(paths.logs.starts_with(&paths.data));
        assert!(paths.db.database.starts_with(&paths.data));
        assert!(paths.db.backups.starts_with(&paths.data));
        assert!(paths.startup_marker.starts_with(&paths.data));
        assert_eq!(
            paths.owner_workspace(),
            paths.data.join(OWNER_WORKSPACE_FILE)
        );
        assert_eq!(
            paths.purge_journal(),
            paths.data.join(DATA_PURGE_JOURNAL_FILE)
        );
    }

    #[test]
    fn the_resolved_folder_is_named_by_the_identifier() {
        let paths = AppPaths::resolve();
        assert_eq!(
            paths.data.file_name().and_then(|n| n.to_str()),
            Some(APP_IDENTIFIER)
        );
    }
}
