//! Where the application's files live.
//!
//! One folder under the per-user roaming application data directory, named
//! by the application identifier, holding the database, the logs and nothing
//! else — so "everything this app knows about you is in one folder" is
//! literally true. The path comes from the operating system's known-folder
//! API, never from a hardcoded string or an environment variable read by
//! hand.

use std::path::{Path, PathBuf};

use crate::storage::DbPaths;

/// The application identifier, which is also the data folder's name.
pub const APP_IDENTIFIER: &str = "dev.whatchanged.app";

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
