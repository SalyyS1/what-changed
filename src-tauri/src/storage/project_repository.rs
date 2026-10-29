//! Durable, explicitly registered development-project roots.
//!
//! A project registration is separate from a watched-folder registration.
//! This repository also owns the narrow evidence query that can prove a
//! process executable was observed below a registered project root.

use rusqlite::{Connection, OptionalExtension};

use super::SqlContext;
use crate::domain::ids;
use crate::error::{AppError, AppResult, DbError};

/// Maximum user-approved project roots. The bound keeps settings, collector
/// reconciliation and prefix evidence queries independent of disk contents.
pub const MAX_PROJECTS: usize = 20;

/// Maximum distinct process entities one config change can cite.
pub const MAX_PROCESS_EVIDENCE: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub path: String,
    pub name: String,
    pub added_at: i64,
    pub baseline_checkpoint_id: Option<i64>,
    pub baseline_checkpoint_name: Option<String>,
    pub baseline_checkpoint_created_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectProcessEvidence {
    pub entity_id: String,
    pub display_name: String,
    /// The bounded query had more qualifying process entities than it could
    /// return.  Callers must surface this as an uncertainty/boundary rather
    /// than retrying the same oversized relationship batch forever.
    pub truncated: bool,
}

pub fn list(conn: &Connection) -> AppResult<Vec<Project>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT p.id, p.path, p.name, p.added_at, p.baseline_checkpoint_id, \
             c.name, c.created_at FROM projects p \
             LEFT JOIN checkpoints c ON c.id = p.baseline_checkpoint_id \
             ORDER BY p.added_at, p.id",
        )
        .during("prepare projects")?;
    let projects = statement
        .query_map([], project_from_row)
        .during("list projects")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("list projects")?;
    Ok(projects)
}

pub fn count(conn: &Connection) -> AppResult<usize> {
    let count: i64 = conn
        .prepare_cached("SELECT COUNT(*) FROM projects")
        .during("prepare project count")?
        .query_row([], |row| row.get(0))
        .during("count projects")?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

pub fn by_path(conn: &Connection, path: &str) -> AppResult<Option<Project>> {
    by_id(conn, &ids::project(path))
}

pub fn by_id(conn: &Connection, project_id: &str) -> AppResult<Option<Project>> {
    conn.prepare_cached(
        "SELECT p.id, p.path, p.name, p.added_at, p.baseline_checkpoint_id, \
         c.name, c.created_at FROM projects p \
         LEFT JOIN checkpoints c ON c.id = p.baseline_checkpoint_id \
         WHERE p.id = ?1",
    )
    .during("prepare project lookup")?
    .query_row([project_id], project_from_row)
    .optional()
    .during("read project")
}

pub fn insert(conn: &Connection, path: &str, name: &str, added_at: i64) -> AppResult<Project> {
    let tx = conn
        .unchecked_transaction()
        .during("begin project registration")?;
    if let Some(existing) = by_path(&tx, path)? {
        tx.commit().during("commit existing project registration")?;
        return Ok(existing);
    }
    if count(&tx)? >= MAX_PROJECTS {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: format!("at most {MAX_PROJECTS} projects can be registered; remove one first"),
        });
    }
    let id = ids::project(path);
    tx.prepare_cached("INSERT INTO projects (id, path, name, added_at) VALUES (?1, ?2, ?3, ?4)")
        .during("prepare project insert")?
        .execute((&id, path, name, added_at))
        .during("insert project")?;
    tx.prepare_cached(
        "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) \
         VALUES (?1, 'project', ?2, ?3, ?3) \
         ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name, \
         first_seen = MIN(first_seen, excluded.first_seen), \
         last_seen = MAX(last_seen, excluded.last_seen)",
    )
    .during("prepare project entity")?
    .execute((&id, name, added_at))
    .during("store project entity")?;
    let inserted = by_path(&tx, path)?.ok_or_else(|| {
        AppError::Db(DbError::Decoding {
            reason: "inserted project could not be read".to_owned(),
        })
    })?;
    tx.commit().during("commit project registration")?;
    Ok(inserted)
}

/// Attach one immutable, successfully captured checkpoint as the project's
/// known-good baseline. The checkpoint remains global if the project is later
/// removed; this pointer is the only project-owned state.
pub fn set_baseline(conn: &Connection, project_id: &str, checkpoint_id: i64) -> AppResult<Project> {
    let tx = conn
        .unchecked_transaction()
        .during("begin project baseline update")?;
    let checkpoint_exists: bool = tx
        .prepare_cached("SELECT EXISTS(SELECT 1 FROM checkpoints WHERE id = ?1)")
        .during("prepare project baseline checkpoint lookup")?
        .query_row([checkpoint_id], |row| row.get(0))
        .during("read project baseline checkpoint")?;
    if !checkpoint_exists {
        return Err(AppError::InvalidInput {
            what: "baseline checkpoint",
            reason: "checkpoint was not found".to_owned(),
        });
    }
    let changed = tx
        .prepare_cached("UPDATE projects SET baseline_checkpoint_id = ?1 WHERE id = ?2")
        .during("prepare project baseline update")?
        .execute((checkpoint_id, project_id))
        .during("update project baseline")?;
    if changed == 0 {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: "project was not found".to_owned(),
        });
    }
    let project = by_id(&tx, project_id)?.ok_or_else(|| {
        AppError::Db(DbError::Decoding {
            reason: "updated project could not be read".to_owned(),
        })
    })?;
    tx.commit().during("commit project baseline update")?;
    Ok(project)
}

pub fn clear_baseline(conn: &Connection, project_id: &str) -> AppResult<Project> {
    let changed = conn
        .prepare_cached("UPDATE projects SET baseline_checkpoint_id = NULL WHERE id = ?1")
        .during("prepare project baseline clear")?
        .execute([project_id])
        .during("clear project baseline")?;
    if changed == 0 {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: "project was not found".to_owned(),
        });
    }
    by_id(conn, project_id)?.ok_or_else(|| {
        AppError::Db(DbError::Decoding {
            reason: "cleared project could not be read".to_owned(),
        })
    })
}

/// Remove the registration and graph facts whose only permitted producer is
/// registered-project/process evidence. Port/service `config_of` edges target
/// other entity kinds and therefore remain untouched.
pub fn remove(conn: &Connection, project_id: &str) -> AppResult<bool> {
    let tx = conn
        .unchecked_transaction()
        .during("begin project removal")?;
    let path: Option<String> = tx
        .prepare_cached("SELECT path FROM projects WHERE id = ?1")
        .during("prepare project removal")?
        .query_row([project_id], |row| row.get(0))
        .optional()
        .during("read project removal")?;
    let Some(_path) = path else {
        tx.commit().during("commit empty project removal")?;
        return Ok(false);
    };

    tx.prepare_cached("DELETE FROM projects WHERE id = ?1")
        .during("prepare project delete")?
        .execute([project_id])
        .during("delete project")?;
    tx.prepare_cached("DELETE FROM entities WHERE id = ?1 AND entity_type = 'project'")
        .during("prepare project entity cleanup")?
        .execute([project_id])
        .during("delete project entity")?;

    // Source paths come from the cited file event, or a surviving baseline if
    // retention expired that event. Indexed target lookups keep history inside
    // SQLite; both endpoints must share one of the at-most twenty remaining roots.
    tx.prepare_cached(
        r#"WITH roots(path, prefix, upper_bound) AS MATERIALIZED (
            SELECT canonical_path(path),
                   rtrim(canonical_path(path), '\') || '\',
                   rtrim(canonical_path(path), '\') || ']'
            FROM projects
        ), candidates(id, target_id, config_path) AS MATERIALIZED (
            SELECT edge.id, edge.target_id,
                   canonical_path(COALESCE(json_extract(evidence.metadata_json, '$.path'), baseline.path))
            FROM entity_relationships AS edge
            INNER JOIN entities AS target ON target.id = edge.target_id AND target.entity_type = 'process'
            INNER JOIN entities AS source ON source.id = edge.source_id AND source.entity_type = 'file'
            LEFT JOIN events AS evidence ON evidence.id = edge.event_id
                AND evidence.entity_id = edge.source_id AND evidence.source_collector = 'files'
            LEFT JOIN config_snapshots AS baseline ON baseline.path_hash = substr(edge.source_id, 6)
            WHERE edge.kind = 'config_of'
        )
        DELETE FROM entity_relationships
        WHERE id IN (
            SELECT candidate.id FROM candidates AS candidate
            WHERE NOT EXISTS (
                SELECT 1 FROM roots
                WHERE (candidate.config_path = roots.path
                    OR (candidate.config_path >= roots.prefix AND candidate.config_path < roots.upper_bound))
                AND EXISTS (
                    SELECT 1 FROM events AS observed INDEXED BY idx_events_process_started_entity_path
                    WHERE observed.entity_id = candidate.target_id
                      AND observed.source_collector = 'process' AND observed.event_type = 'started'
                      AND observed.entity_type = 'process'
                      AND json_extract(observed.metadata_json, '$.path') IS NOT NULL
                      AND (canonical_path(json_extract(observed.metadata_json, '$.path')) = roots.path
                        OR (canonical_path(json_extract(observed.metadata_json, '$.path')) >= roots.prefix
                            AND canonical_path(json_extract(observed.metadata_json, '$.path')) < roots.upper_bound))
                )
            )
        )"#,
    )
    .during("prepare project relationship cleanup")?
    .execute([])
    .during("delete project relationships")?;
    tx.commit().during("commit project removal")?;
    Ok(true)
}

/// Exact file→process evidence for a config file. A process qualifies only
/// when a persisted start observation includes an executable below a project
/// root that also contains the file. No name or directory heuristic is used.
pub fn processes_for_config_path(
    conn: &Connection,
    config_path: &str,
) -> AppResult<Vec<ProjectProcessEvidence>> {
    let canonical_config = ids::canonical_path(config_path);
    let roots = list(conn)?
        .into_iter()
        .filter(|project| is_under_or_equal(&canonical_config, &project.path))
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Ok(Vec::new());
    }

    let mut evidence = std::collections::BTreeMap::<String, String>::new();
    let mut truncated = false;
    for root in roots {
        let canonical_root = ids::canonical_path(&root.path);
        let prefix = format!("{}\\", canonical_root.trim_end_matches('\\'));
        let upper_bound = format!("{}]", canonical_root.trim_end_matches('\\'));
        let mut statement = conn
            .prepare_cached(
                "SELECT entity_id, MAX(entity_display_name) \
                 FROM events \
                 WHERE source_collector = 'process' AND event_type = 'started' \
                   AND entity_type = 'process' AND json_extract(metadata_json, '$.path') IS NOT NULL \
                   AND (canonical_path(json_extract(metadata_json, '$.path')) = ?1 \
                        OR (canonical_path(json_extract(metadata_json, '$.path')) >= ?2 \
                            AND canonical_path(json_extract(metadata_json, '$.path')) < ?3)) \
                 GROUP BY entity_id ORDER BY MAX(timestamp) DESC \
                 LIMIT ?4",
            )
            .during("prepare project process evidence")?;
        let rows = statement
            .query_map(
                (
                    &canonical_root,
                    &prefix,
                    &upper_bound,
                    i64::try_from(MAX_PROCESS_EVIDENCE + 1).unwrap_or(i64::MAX),
                ),
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .during("read project process evidence")?;
        for row in rows {
            let (entity_id, display_name) = row.during("read project process evidence")?;
            if evidence.len() >= MAX_PROCESS_EVIDENCE {
                truncated = true;
                break;
            }
            evidence.entry(entity_id).or_insert(display_name);
        }
        if truncated {
            break;
        }
    }
    Ok(evidence
        .into_iter()
        .map(|(entity_id, display_name)| ProjectProcessEvidence {
            entity_id,
            display_name,
            truncated,
        })
        .collect())
}

fn is_under_or_equal(path: &str, root: &str) -> bool {
    let path = ids::canonical_path(path);
    let root = ids::canonical_path(root);
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|suffix| root.ends_with('\\') || suffix.starts_with('\\'))
}

fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        path: row.get(1)?,
        name: row.get(2)?,
        added_at: row.get(3)?,
        baseline_checkpoint_id: row.get(4)?,
        baseline_checkpoint_name: row.get(5)?,
        baseline_checkpoint_created_at: row.get(6)?,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::storage::{connection, migrations};

    fn migrated() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        connection::register_path_functions(&conn).unwrap();
        migrations::run(
            &mut conn,
            migrations::MIGRATIONS,
            std::path::Path::new("project-baseline-backups"),
        )
        .unwrap();
        conn
    }

    #[test]
    fn baseline_pointer_round_trips_and_clear_preserves_project() {
        let conn = migrated();
        let project = insert(&conn, r#"C:\code\api"#, "api", 10).unwrap();
        conn.execute(
            "INSERT INTO checkpoints (name, kind, created_at, captured_collectors) \
             VALUES (?1, 'manual', ?2, '[\"files\"]')",
            ("Working: api", 20),
        )
        .unwrap();

        let saved = set_baseline(&conn, &project.id, 1).unwrap();
        assert_eq!(saved.baseline_checkpoint_id, Some(1));
        assert_eq!(
            saved.baseline_checkpoint_name.as_deref(),
            Some("Working: api")
        );
        assert_eq!(saved.baseline_checkpoint_created_at, Some(20));

        let cleared = clear_baseline(&conn, &project.id).unwrap();
        assert_eq!(cleared.id, project.id);
        assert_eq!(cleared.baseline_checkpoint_id, None);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn baseline_rejects_missing_project_or_checkpoint_without_mutation() {
        let conn = migrated();
        let project = insert(&conn, r#"C:\code\api"#, "api", 10).unwrap();
        assert!(set_baseline(&conn, &project.id, 99).is_err());
        assert!(set_baseline(&conn, "project:missing", 99).is_err());
        assert_eq!(list(&conn).unwrap()[0].baseline_checkpoint_id, None);
    }
}
