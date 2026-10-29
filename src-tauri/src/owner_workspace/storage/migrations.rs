//! An independent checksummed ledger; invalid files are never renamed or reset.

use crate::error::{AppResult, DbError};
use crate::storage::{connection, SqlContext};
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const SCHEMA_VERSION: usize = 2;
pub const SCHEMA_SQL: &str = include_str!("migrations/001_owner_workspace.sql");
const MIGRATIONS: &[&str] = &[
    SCHEMA_SQL,
    include_str!("migrations/002_project_memory.sql"),
];

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn inspect(conn: &Connection) -> AppResult<usize> {
    if connection::quick_check(conn)?.is_some() {
        return Err(DbError::NotReady {
            state: "owner store integrity check failed".into(),
        }
        .into());
    }
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .during("read owner schema version")?;
    let version = usize::try_from(version).map_err(|_| DbError::NotReady {
        state: "owner schema version is invalid".into(),
    })?;
    if version > SCHEMA_VERSION {
        return Err(DbError::Ahead {
            found: version,
            known: SCHEMA_VERSION,
        }
        .into());
    }
    if version == 0 {
        let tables: u32 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get(0))
            .during("inspect unversioned owner schema")?;
        if tables != 0 {
            return Err(DbError::NotReady {
                state: "unversioned owner schema requires recovery".into(),
            }
            .into());
        }
    } else {
        for (index, sql) in MIGRATIONS.iter().enumerate().take(version) {
            let migration_version = index + 1;
            let checksum: Option<String> = conn
                .query_row(
                    "SELECT checksum FROM owner_schema_migrations WHERE version=?1",
                    [migration_version as i64],
                    |r| r.get(0),
                )
                .optional()
                .during("verify owner schema checksum")?;
            if checksum.as_deref() != Some(digest(sql.as_bytes()).as_str()) {
                return Err(DbError::Drift {
                    version: migration_version,
                }
                .into());
            }
        }
        let count: i64 = conn
            .query_row("SELECT count(*) FROM owner_schema_migrations", [], |r| {
                r.get(0)
            })
            .during("verify owner schema ledger")?;
        if usize::try_from(count).ok() != Some(version) {
            return Err(DbError::Drift { version }.into());
        }
        let violation: Option<String> = conn
            .query_row("PRAGMA foreign_key_check", [], |r| r.get(0))
            .optional()
            .during("check owner references")?;
        if violation.is_some() {
            return Err(DbError::NotReady {
                state: "owner store contains invalid references".into(),
            }
            .into());
        }
    }
    Ok(version)
}

pub fn initialize(conn: &mut Connection, path: &Path, existed: bool) -> AppResult<()> {
    let version = inspect(conn)?;
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    if existed {
        // Back up an existing SQLite file through SQLite, including committed WAL pages.
        let backup = path.with_file_name(format!(
            "owner-workspace.pre-migration-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        conn.backup(rusqlite::MAIN_DB, &backup, None)
            .during("back up owner schema")?;
    }
    let transaction = conn.transaction().during("begin owner migration")?;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let next = index + 1;
        transaction
            .execute_batch(sql)
            .during("create owner schema")?;
        transaction
            .execute(
                "INSERT INTO owner_schema_migrations(version,checksum) VALUES(?1,?2)",
                rusqlite::params![next as i64, digest(sql.as_bytes())],
            )
            .during("record owner migration checksum")?;
        transaction
            .pragma_update(None, "user_version", next as i64)
            .during("set owner schema version")?;
    }
    transaction.commit().during("commit owner migration")
}
