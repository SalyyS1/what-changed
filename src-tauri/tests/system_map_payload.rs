#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeSet;

use what_changed_lib::analysis::system_map::{
    EntityRef, MAX_SYSTEM_MAP_EDGES, MAX_SYSTEM_MAP_NODES,
};
use what_changed_lib::domain::{Entity, EntityType, Relationship, RelationshipKind};
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::storage::system_map_repository::build;
use what_changed_lib::storage::{entity_repository, Db};
use what_changed_lib::testing::{synthetic_event, temp_dir};

fn entity(id: &str, entity_type: EntityType, display_name: &str) -> Sanitized<Entity> {
    Sanitizer::new()
        .sanitize(Entity {
            id: id.to_owned(),
            entity_type,
            display_name: display_name.to_owned(),
            first_seen: 100,
            last_seen: 200,
            metadata: None,
        })
        .0
}

fn relationship(source: &str, target: &str, kind: RelationshipKind, at: i64) -> Relationship {
    Relationship {
        source_id: source.to_owned(),
        target_id: target.to_owned(),
        kind,
        event_id: None,
        recorded_at: at,
    }
}

fn seed(db: &Db, entities: Vec<Sanitized<Entity>>, relationships: Vec<Relationship>) {
    db.write(move |conn| {
        for entity in &entities {
            entity_repository::upsert_sanitized(conn, entity)?;
        }
        for relationship in &relationships {
            entity_repository::insert_relationship(conn, relationship)?;
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn payload_covers_every_relationship_kind_and_is_deterministic() {
    let db = Db::open_in(&temp_dir("system-map-kinds")).unwrap();
    seed(
        &db,
        vec![
            entity("process:0123456789abcdef", EntityType::Process, "API"),
            entity("port:tcp:0.0.0.0:3000", EntityType::Port, "Port 3000"),
            entity("app:HKLM:api", EntityType::Application, "API app"),
            entity("service:api", EntityType::Service, "API service"),
            entity("file:0123456789abcdef", EntityType::File, ".env"),
            entity("container:abcdef012345", EntityType::Container, "api-dev"),
        ],
        vec![
            relationship(
                "process:0123456789abcdef",
                "port:tcp:0.0.0.0:3000",
                RelationshipKind::ListensOn,
                1,
            ),
            relationship(
                "port:tcp:0.0.0.0:3000",
                "process:0123456789abcdef",
                RelationshipKind::OpenedBy,
                2,
            ),
            relationship("app:HKLM:api", "service:api", RelationshipKind::Provides, 3),
            relationship(
                "file:0123456789abcdef",
                "app:HKLM:api",
                RelationshipKind::ConfigOf,
                4,
            ),
            relationship(
                "container:abcdef012345",
                "port:tcp:0.0.0.0:3000",
                RelationshipKind::Publishes,
                5,
            ),
        ],
    );
    let first = db.read(|conn| build(conn, None, 3, &[])).unwrap();
    let second = db.read(|conn| build(conn, None, 3, &[])).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.nodes.len(), 6);
    assert_eq!(first.edges.len(), 5);
    assert_eq!(first.dropped_edges, 0);
    assert!(!first.truncated);
    assert_eq!(
        first
            .edges
            .iter()
            .map(|edge| edge.kind)
            .collect::<BTreeSet<_>>(),
        RelationshipKind::ALL.iter().copied().collect()
    );
    assert!(first
        .kind_availability
        .iter()
        .find(|item| item.kind == EntityType::Container)
        .is_some_and(|item| item.available && item.count == 1));
    assert!(first
        .kind_availability
        .iter()
        .find(|item| item.kind == EntityType::Adapter)
        .is_some_and(|item| !item.available && item.reason.is_some()));
}

#[test]
fn focus_walks_both_directions_and_filters_without_dropping_the_focus() {
    let db = Db::open_in(&temp_dir("system-map-focus")).unwrap();
    seed(
        &db,
        vec![
            entity("file:0123456789abcdef", EntityType::File, ".env"),
            entity("app:HKLM:api", EntityType::Application, "API app"),
            entity("service:api", EntityType::Service, "API service"),
        ],
        vec![
            relationship(
                "file:0123456789abcdef",
                "app:HKLM:api",
                RelationshipKind::ConfigOf,
                10,
            ),
            relationship(
                "app:HKLM:api",
                "service:api",
                RelationshipKind::Provides,
                20,
            ),
        ],
    );
    let sanitizer = Sanitizer::new();
    let now = what_changed_lib::domain::now_ms();
    let recent = (0..7)
        .map(|index| {
            let mut event =
                synthetic_event(index, now - (6 - i64::try_from(index).unwrap()) * 1_000);
            event.entity_id = "file:0123456789abcdef".to_owned();
            event.entity_type = EntityType::File;
            event.entity_display_name = ".env".to_owned();
            event.user_friendly_summary = format!("Config change {index}");
            sanitizer.sanitize(event).0
        })
        .chain(std::iter::once({
            let mut event = synthetic_event(99, now - 8 * 24 * 60 * 60 * 1_000);
            event.entity_id = "file:0123456789abcdef".to_owned();
            event.entity_type = EntityType::File;
            event.entity_display_name = ".env".to_owned();
            event.user_friendly_summary = "Stale config change".to_owned();
            sanitizer.sanitize(event).0
        }))
        .collect();
    db.insert_events(recent).unwrap();
    let focus = EntityRef {
        entity_id: "file:0123456789abcdef".to_owned(),
        entity_type: EntityType::File,
    };
    let all = db
        .read(|conn| build(conn, Some(focus.clone()), 2, &[]))
        .unwrap();
    assert_eq!(
        all.nodes
            .iter()
            .map(|node| (node.entity_type, node.hop))
            .collect::<Vec<_>>(),
        vec![
            (EntityType::File, 0),
            (EntityType::Application, 1),
            (EntityType::Service, 2),
        ]
    );
    assert_eq!(all.focused_recent_events.len(), 5);
    assert_eq!(all.focused_recent_events[0].summary, "Config change 6");
    assert_eq!(all.nodes[0].recent_event_count, 7);
    assert!(all
        .focused_recent_events
        .iter()
        .all(|event| event.summary != "Stale config change"));
    assert!(all
        .focused_recent_events
        .windows(2)
        .all(|pair| pair[0].timestamp_ms > pair[1].timestamp_ms));

    let filtered = db
        .read(|conn| build(conn, Some(focus.clone()), 3, &[EntityType::Application]))
        .unwrap();
    assert_eq!(filtered.focus, Some(focus));
    assert_eq!(filtered.nodes.len(), 2, "focus survives its disabled kind");
    assert_eq!(filtered.edges.len(), 1);
    assert_eq!(filtered.focused_recent_events.len(), 5);
}

#[test]
fn node_and_edge_caps_are_enforced_and_disclosed() {
    let db = Db::open_in(&temp_dir("system-map-caps")).unwrap();
    let mut entities = vec![entity("service:root", EntityType::Service, "Root service")];
    let mut relationships = Vec::new();
    for index in 0..450 {
        let id = format!("app:HKLM:app-{index:04}");
        entities.push(entity(
            &id,
            EntityType::Application,
            &format!("App {index:04}"),
        ));
        relationships.push(relationship(
            &id,
            "service:root",
            RelationshipKind::Provides,
            i64::from(index),
        ));
    }
    seed(&db, entities, relationships);
    let data = db
        .read(|conn| {
            build(
                conn,
                Some(EntityRef {
                    entity_id: "service:root".to_owned(),
                    entity_type: EntityType::Service,
                }),
                9,
                &[],
            )
        })
        .unwrap();
    assert_eq!(data.total_nodes_available, 451, "exact connected count");
    assert_eq!(data.nodes.len(), MAX_SYSTEM_MAP_NODES);
    assert!(data.edges.len() <= MAX_SYSTEM_MAP_EDGES);
    assert!(data.truncated);
    assert_eq!(data.nodes[0].entity_id, "service:root");
}

#[test]
fn rejected_edges_before_the_sql_limit_cannot_crowd_out_allowed_edges() {
    let db = Db::open_in(&temp_dir("system-map-filter-before-limit")).unwrap();
    let mut entities = vec![entity("service:root", EntityType::Service, "Root")];
    let mut relationships = Vec::new();
    for index in 0..650 {
        let id = format!("adapter:disabled-{index:04}");
        entities.push(entity(
            &id,
            EntityType::Adapter,
            &format!("Disabled {index}"),
        ));
        relationships.push(relationship(
            &id,
            "service:root",
            RelationshipKind::Provides,
            i64::from(index),
        ));
    }
    for index in 0..5 {
        let id = format!("app:HKLM:allowed-{index:04}");
        entities.push(entity(
            &id,
            EntityType::Application,
            &format!("Allowed {index}"),
        ));
        relationships.push(relationship(
            &id,
            "service:root",
            RelationshipKind::Provides,
            2_000 + i64::from(index),
        ));
    }
    seed(&db, entities, relationships);
    db.write(|conn| {
        conn.execute_batch("PRAGMA foreign_keys = OFF")
            .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                operation: "disable foreign keys for stale-edge fixture",
                source,
            })?;
        for index in 0..650 {
            conn.execute(
                "INSERT INTO entity_relationships \
                 (source_id, target_id, kind, event_id, recorded_at) \
                 VALUES (?1, 'service:root', 'provides', NULL, 1)",
                [format!("aaa-missing-{index:04}")],
            )
            .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                operation: "seed stale relationship",
                source,
            })?;
        }
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                operation: "restore foreign keys after stale-edge fixture",
                source,
            })?;
        Ok(())
    })
    .unwrap();

    let data = db
        .read(|conn| {
            build(
                conn,
                Some(EntityRef {
                    entity_id: "service:root".to_owned(),
                    entity_type: EntityType::Service,
                }),
                1,
                &[EntityType::Application],
            )
        })
        .unwrap();
    assert_eq!(data.total_nodes_available, 6);
    assert_eq!(data.edges.len(), 5);
    assert_eq!(data.nodes.len(), 6);
    assert!(data
        .nodes
        .iter()
        .skip(1)
        .all(|node| node.entity_type == EntityType::Application));
}
