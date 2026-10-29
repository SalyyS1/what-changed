//! Registered projects are bounded, independent opt-ins with exact cleanup.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::domain::ids;
use what_changed_lib::storage::{
    config_snapshot_repository, project_repository, watched_location_repository, Db,
};
use what_changed_lib::testing::temp_dir;

#[test]
fn one_path_can_be_both_watched_and_a_project_and_case_duplicates_are_idempotent() {
    let db = Db::open_in(&temp_dir("project-independent-opt-in")).unwrap();
    let path = r"C:\Users\dev\my-api".to_owned();
    let upper = path.to_uppercase();
    let row = db
        .write(move |connection| {
            watched_location_repository::insert(connection, &path, 6, 1)?;
            let first = project_repository::insert(connection, &path, "my-api", 2)?;
            let second = project_repository::insert(connection, &upper, "ignored", 3)?;
            assert_eq!(first, second);
            Ok(first)
        })
        .unwrap();
    assert_eq!(row.id, ids::project(r"C:\Users\dev\my-api"));
    assert_eq!(db.read(project_repository::count).unwrap(), 1);
}

#[test]
fn project_capacity_is_enforced_inside_the_serial_writer() {
    let db = Db::open_in(&temp_dir("project-capacity")).unwrap();
    db.write(move |connection| {
        for index in 0..project_repository::MAX_PROJECTS {
            project_repository::insert(
                connection,
                &format!(r"C:\Users\dev\project-{index}"),
                &format!("project-{index}"),
                index as i64,
            )
            .unwrap();
        }
        Ok(())
    })
    .unwrap();
    let error = db
        .write(|connection| {
            project_repository::insert(connection, r"C:\Users\dev\overflow", "overflow", 99)
        })
        .unwrap_err();
    assert!(error.to_string().contains("at most 20 projects"));
}

#[test]
fn failed_entity_write_rolls_back_the_project_registration() {
    let db = Db::open_in(&temp_dir("project-registration-rollback")).unwrap();
    db.write(|connection| {
        connection
            .execute_batch(
                "CREATE TRIGGER reject_project_entity \
                 BEFORE INSERT ON entities WHEN NEW.entity_type = 'project' \
                 BEGIN SELECT RAISE(ABORT, 'injected project entity failure'); END;",
            )
            .unwrap();
        let failure = project_repository::insert(
            connection,
            r"C:\Users\dev\transactional",
            "transactional",
            1,
        )
        .unwrap_err();
        assert!(failure
            .to_string()
            .contains("injected project entity failure"));
        assert_eq!(project_repository::count(connection)?, 0);

        connection
            .execute_batch("DROP TRIGGER reject_project_entity")
            .unwrap();
        let project = project_repository::insert(
            connection,
            r"C:\Users\dev\transactional",
            "transactional",
            2,
        )?;
        let entities: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = ?1 AND entity_type = 'project'",
                [&project.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(entities, 1);
        Ok(())
    })
    .unwrap();
}

#[test]
fn unicode_case_variants_use_the_same_project_path_identity() {
    let db = Db::open_in(&temp_dir("project-unicode-path-identity")).unwrap();
    let project_path = r"C:\Users\dev\Équipe";
    let executable = r"c:\users\DEV\éQUIPE\bin\api.exe";
    let process_id = ids::process(executable);
    let process_id_for_write = process_id.clone();
    db.write(move |connection| {
        project_repository::insert(connection, project_path, "Équipe", 1)?;
        connection
            .execute(
                "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
                 entity_display_name, severity, importance, source_collector, metadata_json, \
                 user_friendly_summary, technical_summary, confidence) \
                 VALUES (2, 'process', 'started', 'process', ?1, 'api.exe', 'info', \
                 'informational', 'process', ?2, 'started', 'started', 'high')",
                (
                    &process_id_for_write,
                    serde_json::json!({ "path": executable }).to_string(),
                ),
            )
            .unwrap();
        Ok(())
    })
    .unwrap();

    let evidence = db
        .read(|connection| {
            project_repository::processes_for_config_path(connection, r"C:\USERS\DEV\ÉQUIPE\.env")
        })
        .unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].entity_id, process_id);
}

#[test]
fn project_process_evidence_is_bounded_without_rejecting_the_config_cycle() {
    let db = Db::open_in(&temp_dir("project-process-evidence-bound")).unwrap();
    let root = r"C:\Users\dev\large-api";
    db.write(move |connection| {
        project_repository::insert(connection, root, "large-api", 1)?;
        for index in 0..(project_repository::MAX_PROCESS_EVIDENCE + 5) {
            let executable = format!(r"{root}\bin\worker-{index}.exe");
            let process_id = ids::process(&executable);
            connection
                .execute(
                    "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id,\
                 entity_display_name, severity, importance, source_collector, metadata_json,\
                 user_friendly_summary, technical_summary, confidence)\
                 VALUES (?1, 'process', 'started', 'process', ?2, 'worker', 'info',\
                 'informational', 'process', ?3, 'started', 'started', 'high')",
                    (
                        index as i64 + 2,
                        &process_id,
                        serde_json::json!({ "path": executable }).to_string(),
                    ),
                )
                .unwrap();
        }
        Ok(())
    })
    .unwrap();

    let evidence = db
        .read(|connection| {
            project_repository::processes_for_config_path(
                connection,
                r"C:\Users\dev\large-api\.env",
            )
        })
        .unwrap();
    assert_eq!(evidence.len(), project_repository::MAX_PROCESS_EVIDENCE);
    assert!(evidence.iter().all(|row| row.truncated));
}

#[test]
fn removing_a_project_deletes_its_entity_and_project_process_edges() {
    let db = Db::open_in(&temp_dir("project-cleanup")).unwrap();
    let project_path = r"C:\Users\dev\my-api";
    let executable = r"C:\Users\dev\my-api\bin\api.exe";
    let process_id = ids::process(executable);
    let project_id = ids::project(project_path);
    let process_id_for_write = process_id.clone();
    db.write(move |connection| {
        project_repository::insert(connection, project_path, "my-api", 1)?;
        connection
            .execute(
                "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
             entity_display_name, severity, importance, source_collector, metadata_json, \
             user_friendly_summary, technical_summary, confidence) \
             VALUES (2, 'process', 'started', 'process', ?1, 'api.exe', 'info', \
             'informational', 'process', ?2, 'started', 'started', 'high')",
                (
                    &process_id_for_write,
                    serde_json::json!({ "path": executable }).to_string(),
                ),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) \
             VALUES ('file:0123456789abcdef', 'file', '.env', 2, 2), \
                    (?1, 'process', 'api.exe', 2, 2)",
                [&process_id_for_write],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO entity_relationships (source_id, target_id, kind, recorded_at) \
             VALUES ('file:0123456789abcdef', ?1, 'config_of', 2)",
                [&process_id_for_write],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();

    db.write(move |connection| project_repository::remove(connection, &project_id))
        .unwrap();
    let (projects, project_entities, process_edges): (i64, i64, i64) = db
        .read(move |connection| {
            Ok((
                connection
                    .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
                    .unwrap(),
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM entities WHERE entity_type = 'project'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap(),
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM entity_relationships WHERE target_id = ?1",
                        [&process_id],
                        |row| row.get(0),
                    )
                    .unwrap(),
            ))
        })
        .unwrap();
    assert_eq!((projects, project_entities, process_edges), (0, 0, 0));
}

#[test]
fn removal_is_set_based_and_preserves_unicode_evidence_covered_by_an_overlapping_root() {
    let db = Db::open_in(&temp_dir("project-overlapping-cleanup")).unwrap();
    let parent_path = r"C:\Users\dev\Éspace";
    let child_path = r"C:\Users\dev\Éspace\my-api";
    let child_executable = r"\\?\C:/Users/DEV/éSPACE/my-api/bin/api.exe/";
    let sibling_executable = r"C:\Users\dev\éspace\other\worker.exe";
    let child_config = r"C:\Users\DEV\éSPACE\my-api\.env";
    let sibling_config = r"C:\Users\dev\éspace\other\.env";
    let child_process = ids::process(child_executable);
    let sibling_process = ids::process(sibling_executable);
    let child_file = ids::file(child_config);
    let sibling_file = ids::file(sibling_config);
    let parent_id = ids::project(parent_path);
    let child_id = ids::project(child_path);
    let child_process_for_write = child_process.clone();
    let sibling_process_for_write = sibling_process.clone();
    let child_file_for_write = child_file.clone();
    let sibling_file_for_write = sibling_file.clone();

    db.write(move |connection| {
        project_repository::insert(connection, parent_path, "workspace", 1)?;
        project_repository::insert(connection, child_path, "my-api", 2)?;

        // A large unrelated history must not be materialised in Rust during
        // removal. The cleanup statement is driven by relationship targets.
        connection
            .execute_batch(
                "WITH RECURSIVE sequence(value) AS (\
                 SELECT 1 UNION ALL SELECT value + 1 FROM sequence WHERE value < 25000\
             )\
             INSERT INTO events (timestamp, category, event_type, entity_type, entity_id,\
                 entity_display_name, severity, importance, source_collector, metadata_json,\
                 user_friendly_summary, technical_summary, confidence)\
             SELECT value + 100, 'network', 'changed', 'adapter',\
                 'adapter:' || value, 'adapter', 'info', 'informational', 'network', NULL,\
                 'changed', 'changed', 'high' FROM sequence;",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id,\
                 entity_display_name, severity, importance, source_collector, metadata_json,\
                 user_friendly_summary, technical_summary, confidence) VALUES\
             (3, 'process', 'started', 'process', ?1, 'api.exe', 'info', 'informational',\
                 'process', ?2, 'started', 'started', 'high'),\
             (4, 'process', 'started', 'process', ?3, 'worker.exe', 'info', 'informational',\
                 'process', ?4, 'started', 'started', 'high')",
                rusqlite::params![
                    &child_process_for_write,
                    serde_json::json!({ "path": child_executable }).to_string(),
                    &sibling_process_for_write,
                    serde_json::json!({ "path": sibling_executable }).to_string(),
                ],
            )
            .unwrap();
        connection.execute(
            "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) VALUES\
             (?1, 'file', 'child.env', 3, 3),\
             (?2, 'file', 'sibling.env', 4, 4),\
             (?3, 'process', 'api.exe', 3, 3),\
             (?4, 'process', 'worker.exe', 4, 4)",
            rusqlite::params![
                &child_file_for_write,
                &sibling_file_for_write,
                &child_process_for_write,
                &sibling_process_for_write,
            ],
        )
        .unwrap();
        connection
            .execute(
                "INSERT INTO config_snapshots (path_hash, path, keys_json, captured_at, format) VALUES\
                 (?1, ?2, '{}', 3, 'env'), (?3, ?4, '{}', 4, 'env')",
                rusqlite::params![
                    config_snapshot_repository::path_hash(child_config),
                    child_config,
                    config_snapshot_repository::path_hash(sibling_config),
                    sibling_config,
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO entity_relationships (source_id, target_id, kind, recorded_at) VALUES\
             (?1, ?2, 'config_of', 3), (?3, ?4, 'config_of', 4)",
                rusqlite::params![
                    &child_file_for_write,
                    &child_process_for_write,
                    &sibling_file_for_write,
                    &sibling_process_for_write,
                ],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();

    db.write(move |connection| project_repository::remove(connection, &parent_id))
        .unwrap();
    let (child_edges, sibling_edges): (i64, i64) = db
        .read({
            let child_process = child_process.clone();
            let sibling_process = sibling_process.clone();
            move |connection| {
                Ok((
                    connection
                        .query_row(
                            "SELECT COUNT(*) FROM entity_relationships WHERE target_id = ?1",
                            [&child_process],
                            |row| row.get(0),
                        )
                        .unwrap(),
                    connection
                        .query_row(
                            "SELECT COUNT(*) FROM entity_relationships WHERE target_id = ?1",
                            [&sibling_process],
                            |row| row.get(0),
                        )
                        .unwrap(),
                ))
            }
        })
        .unwrap();
    assert_eq!((child_edges, sibling_edges), (1, 0));

    db.write(move |connection| project_repository::remove(connection, &child_id))
        .unwrap();
    let child_edges: i64 = db
        .read(move |connection| {
            Ok(connection
                .query_row(
                    "SELECT COUNT(*) FROM entity_relationships WHERE target_id = ?1",
                    [&child_process],
                    |row| row.get(0),
                )
                .unwrap())
        })
        .unwrap();
    assert_eq!(child_edges, 0);
}

#[test]
fn disjoint_surviving_roots_cannot_jointly_preserve_one_config_process_edge() {
    let db = Db::open_in(&temp_dir("project-disjoint-cleanup")).unwrap();
    let parent_path = r"C:\workspace";
    let source_root = r"C:\workspace\frontend";
    let target_root = r"C:\workspace\backend";
    let config_path = r"C:\workspace\frontend\.env";
    let executable = r"C:\workspace\backend\api.exe";
    let parent_id = ids::project(parent_path);
    let file_id = ids::file(config_path);
    let process_id = ids::process(executable);
    let file_id_for_write = file_id.clone();
    let process_id_for_write = process_id.clone();

    db.write(move |connection| {
        project_repository::insert(connection, parent_path, "workspace", 1)?;
        project_repository::insert(connection, source_root, "frontend", 2)?;
        project_repository::insert(connection, target_root, "backend", 3)?;
        connection
            .execute(
                "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
                 entity_display_name, severity, importance, source_collector, metadata_json, \
                 user_friendly_summary, technical_summary, confidence) \
                 VALUES (4, 'process', 'started', 'process', ?1, 'api.exe', 'info', \
                 'informational', 'process', ?2, 'started', 'started', 'high')",
                (
                    &process_id_for_write,
                    serde_json::json!({ "path": executable }).to_string(),
                ),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) VALUES \
                 (?1, 'file', '.env', 4, 4), (?2, 'process', 'api.exe', 4, 4)",
                [&file_id_for_write, &process_id_for_write],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO config_snapshots (path_hash, path, keys_json, captured_at, format) \
                 VALUES (?1, ?2, '{}', 4, 'env')",
                (config_snapshot_repository::path_hash(config_path), config_path),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO entity_relationships (source_id, target_id, kind, recorded_at) \
                 VALUES (?1, ?2, 'config_of', 4)",
                [&file_id_for_write, &process_id_for_write],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();

    db.write(move |connection| project_repository::remove(connection, &parent_id))
        .unwrap();
    let edges: i64 = db
        .read(move |connection| {
            Ok(connection
                .query_row(
                    "SELECT COUNT(*) FROM entity_relationships WHERE source_id = ?1 AND target_id = ?2",
                    [&file_id, &process_id],
                    |row| row.get(0),
                )
                .unwrap())
        })
        .unwrap();
    assert_eq!(edges, 0);
    assert_eq!(db.read(project_repository::count).unwrap(), 2);
}
