//! Entities and the proven relationships between them.

use rusqlite::params_from_iter;
use rusqlite::Connection;

use super::{count_column, from_json, parse_word, to_json, SqlContext};
use crate::domain::{Entity, EntityType, NormalizedEvent, Relationship, RelationshipKind};
use crate::error::AppResult;
use crate::privacy::Sanitized;

#[cfg(feature = "devtools")]
static DENSE_RELATIONSHIP_FALLBACKS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// QA telemetry for the adaptive relationship read. This is intentionally
/// absent from normal builds and does not expose any entity identifiers.
#[cfg(feature = "devtools")]
pub fn dense_relationship_fallback_count() -> u64 {
    DENSE_RELATIONSHIP_FALLBACKS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Indexed aggregates needed to render one System Map node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemMapNodeMetrics {
    pub degree: u32,
    pub last_changed_at: Option<i64>,
    pub recent_event_count: u32,
}

/// A deterministic relationship page plus proof that at least one more row
/// existed. The System Map uses the sentinel to stop expansion before a dense
/// graph can allocate in proportion to the database.
pub struct RelationshipPage {
    pub relationships: Vec<Relationship>,
    pub has_more: bool,
}

/// Record that an event mentioned its entity: insert it, or refresh its name
/// and last-seen time. Called inside the event insert transaction.
pub(crate) fn touch(conn: &Connection, event: &NormalizedEvent) -> AppResult<()> {
    let mut statement = conn
        .prepare_cached(
            "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) \
             VALUES (?1, ?2, ?3, ?4, ?4) \
             ON CONFLICT(id) DO UPDATE SET \
                display_name = excluded.display_name, \
                last_seen = MAX(last_seen, excluded.last_seen), \
                first_seen = MIN(first_seen, excluded.first_seen)",
        )
        .during("prepare an entity upsert")?;
    statement
        .execute((
            &event.entity_id,
            event.entity_type.as_str(),
            &event.entity_display_name,
            event.timestamp,
        ))
        .during("record an entity")?;
    Ok(())
}

/// Store a redacted entity with metadata, keeping the earliest first-seen
/// time. The public repository boundary requires proof that display name and
/// metadata passed through the sanitizer.
pub fn upsert_sanitized(conn: &Connection, entity: &Sanitized<Entity>) -> AppResult<()> {
    upsert(conn, entity.get())
}

/// Store an entity that a crate-internal producer already derived from a
/// sanitized event. Keeping this path crate-private prevents an external
/// collector from bypassing [`upsert_sanitized`].
pub(crate) fn upsert(conn: &Connection, entity: &Entity) -> AppResult<()> {
    let metadata = entity.metadata.as_ref().map(to_json).transpose()?;
    conn.prepare_cached(
        "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen, metadata_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT(id) DO UPDATE SET \
            display_name = excluded.display_name, \
            last_seen = MAX(last_seen, excluded.last_seen), \
            first_seen = MIN(first_seen, excluded.first_seen), \
            metadata_json = COALESCE(excluded.metadata_json, metadata_json)",
    )
    .during("prepare an entity upsert")?
    .execute((
        &entity.id,
        entity.entity_type.as_str(),
        &entity.display_name,
        entity.first_seen,
        entity.last_seen,
        metadata,
    ))
    .during("store an entity")?;
    Ok(())
}

/// One entity by id.
pub fn by_id(conn: &Connection, id: &str) -> AppResult<Option<Entity>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, entity_type, display_name, first_seen, last_seen, metadata_json \
             FROM entities WHERE id = ?1",
        )
        .during("prepare an entity lookup")?;
    let mut rows = statement.query([id]).during("look up an entity")?;
    let Some(row) = rows.next().during("look up an entity")? else {
        return Ok(None);
    };
    let kind: String = row.get(1).during("read an entity")?;
    let metadata: Option<String> = row.get(5).during("read an entity")?;
    Ok(Some(Entity {
        id: row.get(0).during("read an entity")?,
        entity_type: parse_word::<EntityType>("entity_type", &kind)?,
        display_name: row.get(2).during("read an entity")?,
        first_seen: row.get(3).during("read an entity")?,
        last_seen: row.get(4).during("read an entity")?,
        metadata: metadata
            .as_deref()
            .map(|t| from_json("metadata_json", t))
            .transpose()?,
    }))
}

/// Entities whose display name contains `needle`, up to `limit`.
pub fn search_by_name(conn: &Connection, needle: &str, limit: u16) -> AppResult<Vec<Entity>> {
    let pattern = format!("%{}%", needle.replace(['%', '_'], ""));
    let mut statement = conn
        .prepare_cached(
            "SELECT id, entity_type, display_name, first_seen, last_seen, metadata_json \
             FROM entities WHERE display_name LIKE ?1 ORDER BY last_seen DESC LIMIT ?2",
        )
        .during("prepare an entity search")?;
    let rows = statement
        .query_map((pattern, i64::from(limit)), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .during("search entities")?;
    let mut entities = Vec::new();
    for row in rows {
        let (id, kind, display_name, first_seen, last_seen, metadata) =
            row.during("search entities")?;
        entities.push(Entity {
            id,
            entity_type: parse_word("entity_type", &kind)?,
            display_name,
            first_seen,
            last_seen,
            metadata: metadata
                .as_deref()
                .map(|t| from_json("metadata_json", t))
                .transpose()?,
        });
    }
    Ok(entities)
}

/// Record an edge. The same edge recorded twice keeps evidence from the
/// newest observed time. Equal timestamps preserve the first evidence id:
/// row ids reflect commit order, not a stronger observation.
pub fn insert_relationship(conn: &Connection, relationship: &Relationship) -> AppResult<()> {
    conn.prepare_cached(
        "INSERT INTO entity_relationships (source_id, target_id, kind, event_id, recorded_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(source_id, target_id, kind) DO UPDATE SET \
            event_id = excluded.event_id, recorded_at = excluded.recorded_at \
         WHERE entity_relationships.recorded_at < excluded.recorded_at",
    )
    .during("prepare a relationship insert")?
    .execute((
        &relationship.source_id,
        &relationship.target_id,
        relationship.kind.as_str(),
        relationship.event_id,
        relationship.recorded_at,
    ))
    .during("record a relationship")?;
    Ok(())
}

/// Seed a QA-only graph edge without ever replacing existing evidence. The
/// caller checks the return value: a conflict means its corpus is not exact.
#[cfg(feature = "devtools")]
pub fn insert_qa_relationship_if_absent(
    conn: &Connection,
    relationship: &Relationship,
) -> AppResult<bool> {
    let inserted = conn
        .prepare_cached(
            "INSERT INTO entity_relationships (source_id, target_id, kind, event_id, recorded_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(source_id, target_id, kind) DO NOTHING",
        )
        .during("prepare a QA relationship insert")?
        .execute((
            &relationship.source_id,
            &relationship.target_id,
            relationship.kind.as_str(),
            relationship.event_id,
            relationship.recorded_at,
        ))
        .during("record a QA relationship")?;
    Ok(inserted == 1)
}

/// Edges leaving `source_id`.
pub fn relationships_from(conn: &Connection, source_id: &str) -> AppResult<Vec<Relationship>> {
    relationships(
        conn,
        "SELECT source_id, target_id, kind, event_id, recorded_at \
         FROM entity_relationships WHERE source_id = ?1",
        source_id,
    )
}

/// Edges arriving at `target_id`.
pub fn relationships_to(conn: &Connection, target_id: &str) -> AppResult<Vec<Relationship>> {
    relationships(
        conn,
        "SELECT source_id, target_id, kind, event_id, recorded_at \
         FROM entity_relationships WHERE target_id = ?1",
        target_id,
    )
}

/// Highest-degree entities of the requested kinds. These are stable natural
/// entry points when the system map is opened without an explicit focus.
pub fn highest_degree_entities(
    conn: &Connection,
    kinds: &[EntityType],
    limit: usize,
) -> AppResult<Vec<Entity>> {
    if kinds.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let placeholders = (1..=kinds.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let limit_parameter = kinds.len() + 1;
    let sql = format!(
        "WITH endpoint_degrees(id, degree) AS (\
           SELECT source_id, COUNT(*) FROM entity_relationships GROUP BY source_id \
           UNION ALL \
           SELECT target_id, COUNT(*) FROM entity_relationships GROUP BY target_id\
         ) \
         SELECT e.id, e.entity_type, e.display_name, e.first_seen, e.last_seen, e.metadata_json \
         FROM (SELECT id, SUM(degree) AS degree FROM endpoint_degrees GROUP BY id) d \
         JOIN entities e ON e.id = d.id \
         WHERE e.entity_type IN ({placeholders}) \
         ORDER BY d.degree DESC, e.entity_type, e.display_name, e.id \
         LIMIT ?{limit_parameter}"
    );
    let mut values = kinds
        .iter()
        .map(|kind| kind.as_str().to_owned())
        .collect::<Vec<_>>();
    values.push(limit.to_string());
    query_entities(
        conn,
        &sql,
        params_from_iter(values.iter()),
        "load system-map roots",
    )
}

/// Entities matching the ids, returned in id order. Missing rows are omitted
/// so callers can drop stale/unresolvable relationship endpoints explicitly.
pub fn entities_by_ids(conn: &Connection, ids: &[String]) -> AppResult<Vec<Entity>> {
    const IDS_PER_BATCH: usize = 800;
    let mut entities = Vec::new();
    for chunk in ids.chunks(IDS_PER_BATCH) {
        if chunk.is_empty() {
            continue;
        }
        let placeholders = (1..=chunk.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT id, entity_type, display_name, first_seen, last_seen, metadata_json \
             FROM entities WHERE id IN ({placeholders}) ORDER BY id"
        );
        entities.extend(query_entities(
            conn,
            &sql,
            params_from_iter(chunk.iter()),
            "load system-map entities",
        )?);
    }
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    entities.dedup_by(|left, right| left.id == right.id);
    Ok(entities)
}

/// Proven relationships touching a bounded frontier. The two statements stay
/// separate so SQLite can use the source and target indexes independently.
/// Variant with an explicit cap, used as traversal consumes its shared edge
/// budget. At most `limit + 1` rows survive the merge.
pub fn relationships_touching_with_limit(
    conn: &Connection,
    entity_ids: &[String],
    allowed_kinds: &[EntityType],
    focus_id: Option<&str>,
    limit: usize,
) -> AppResult<RelationshipPage> {
    const IDS_PER_BATCH: usize = 400;
    let mut found = Vec::new();
    if entity_ids.is_empty() || limit == 0 {
        return Ok(RelationshipPage {
            relationships: Vec::new(),
            has_more: false,
        });
    }
    let query_limit = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
    let all_kinds = all_entity_kinds_allowed(allowed_kinds);
    for chunk in entity_ids.chunks(IDS_PER_BATCH) {
        if chunk.is_empty() {
            continue;
        }
        let placeholders = (1..=chunk.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let kind_start = chunk.len() + 1;
        let kind_placeholders = (0..allowed_kinds.len())
            .map(|offset| format!("?{}", kind_start + offset))
            .collect::<Vec<_>>()
            .join(", ");
        let focus_parameter = kind_start + allowed_kinds.len();
        let limit_parameter = focus_parameter + 1;
        for (column, operation) in [
            ("r.source_id", "load outgoing system-map relationships"),
            ("r.target_id", "load incoming system-map relationships"),
        ] {
            let mut owned: Vec<Box<dyn rusqlite::ToSql>> = chunk
                .iter()
                .cloned()
                .map(|id| Box::new(id) as Box<dyn rusqlite::ToSql>)
                .collect();
            let sql =
                if all_kinds {
                    let limit_parameter = chunk.len() + 1;
                    let resolvable_endpoint = if column == "r.source_id" {
                        "r.target_id"
                    } else {
                        "r.source_id"
                    };
                    format!(
                        "SELECT r.source_id, r.target_id, r.kind, r.event_id, r.recorded_at \
                     FROM entity_relationships r \
                     JOIN entities endpoint_entity ON endpoint_entity.id = {resolvable_endpoint} \
                     WHERE {column} IN ({placeholders}) \
                     ORDER BY r.source_id, r.target_id, r.kind LIMIT ?{limit_parameter}"
                    )
                } else {
                    owned.extend(allowed_kinds.iter().map(|kind| {
                        Box::new(kind.as_str().to_owned()) as Box<dyn rusqlite::ToSql>
                    }));
                    owned.push(Box::new(focus_id.unwrap_or("").to_owned()));
                    format!(
                        "SELECT r.source_id, r.target_id, r.kind, r.event_id, r.recorded_at \
                     FROM entity_relationships r \
                     JOIN entities source_entity ON source_entity.id = r.source_id \
                     JOIN entities target_entity ON target_entity.id = r.target_id \
                     WHERE {column} IN ({placeholders}) \
                       AND (source_entity.entity_type IN ({kind_placeholders}) \
                            OR source_entity.id = ?{focus_parameter}) \
                       AND (target_entity.entity_type IN ({kind_placeholders}) \
                            OR target_entity.id = ?{focus_parameter}) \
                     ORDER BY r.source_id, r.target_id, r.kind LIMIT ?{limit_parameter}"
                    )
                };
            owned.push(Box::new(query_limit));
            let mut statement = conn.prepare(&sql).during(operation)?;
            let parameters = owned.iter().map(|value| value.as_ref()).collect::<Vec<_>>();
            let rows = statement
                .query_map(parameters.as_slice(), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                })
                .during(operation)?;
            for row in rows {
                let raw = row.during(operation)?;
                found.push(Relationship {
                    source_id: raw.0,
                    target_id: raw.1,
                    kind: parse_word("kind", &raw.2)?,
                    event_id: raw.3,
                    recorded_at: raw.4,
                });
            }
        }
    }
    found.sort_by(|left, right| {
        (&left.source_id, &left.target_id, left.kind.as_str()).cmp(&(
            &right.source_id,
            &right.target_id,
            right.kind.as_str(),
        ))
    });
    found.dedup_by(|left, right| {
        left.source_id == right.source_id
            && left.target_id == right.target_id
            && left.kind == right.kind
    });
    let has_more = found.len() > limit;
    found.truncate(limit);
    Ok(RelationshipPage {
        relationships: found,
        has_more,
    })
}

/// Global entity counts power disabled-with-reason filter chips without
/// pretending a filtered empty graph means the collector recorded nothing.
pub fn counts_by_entity_type(conn: &Connection) -> AppResult<Vec<(EntityType, u32)>> {
    let mut statement = conn
        .prepare_cached("SELECT entity_type, COUNT(*) FROM entities GROUP BY entity_type")
        .during("prepare system-map kind counts")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .during("load system-map kind counts")?;
    let mut counts: Vec<(EntityType, u32)> = Vec::new();
    for row in rows {
        let (kind, count) = row.during("load a system-map kind count")?;
        counts.push((
            parse_word("entity_type", &kind)?,
            u32::try_from(count).unwrap_or(u32::MAX),
        ));
    }
    counts.sort_by_key(|(kind, _)| kind.as_str());
    Ok(counts)
}

/// Exact number of resolvable, kind-allowed entities reachable from `roots`
/// within the bounded depth. SQLite owns the working set; the payload walk
/// can remain capped at one sentinel without weakening the disclosure count.
pub fn reachable_node_count(
    conn: &Connection,
    roots: &[String],
    allowed_kinds: &[EntityType],
    focus_id: Option<&str>,
    depth: u8,
) -> AppResult<u32> {
    if roots.is_empty() {
        return Ok(0);
    }
    let global = allowed_entity_count(conn, allowed_kinds, focus_id)?;
    let direct = directly_reachable_count(conn, roots, allowed_kinds, focus_id)?;
    if depth <= 1 || direct == global {
        return Ok(direct);
    }
    conn.execute_batch(
        "DROP TABLE IF EXISTS temp_system_map_seen; \
         CREATE TEMP TABLE temp_system_map_seen (\
           id TEXT PRIMARY KEY, depth INTEGER NOT NULL\
         ) WITHOUT ROWID;",
    )
    .during("prepare exact system-map count")?;
    let result = (|| -> AppResult<u32> {
        let mut insert_root = conn
            .prepare_cached("INSERT OR IGNORE INTO temp_system_map_seen (id, depth) VALUES (?1, 0)")
            .during("prepare system-map count roots")?;
        for root in roots {
            insert_root
                .execute([root])
                .during("seed system-map count root")?;
        }
        drop(insert_root);

        for hop in 0..depth.clamp(1, 3) {
            let outgoing = expand_reachable_count(conn, allowed_kinds, focus_id, hop, true)?;
            let incoming = expand_reachable_count(conn, allowed_kinds, focus_id, hop, false)?;
            if outgoing == 0 && incoming == 0 {
                break;
            }
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM temp_system_map_seen", [], |row| {
                row.get(0)
            })
            .during("count reachable system-map entities")?;
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    })();
    let cleanup = conn
        .execute_batch("DROP TABLE IF EXISTS temp_system_map_seen")
        .during("clean up exact system-map count");
    match (result, cleanup) {
        (Ok(count), Ok(())) => Ok(count),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn allowed_entity_count(
    conn: &Connection,
    allowed_kinds: &[EntityType],
    focus_id: Option<&str>,
) -> AppResult<u32> {
    if all_entity_kinds_allowed(allowed_kinds) {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .during("count all system-map entities")?;
        return Ok(u32::try_from(count).unwrap_or(u32::MAX));
    }
    let placeholders = (1..=allowed_kinds.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let focus_parameter = allowed_kinds.len() + 1;
    let sql = format!(
        "SELECT COUNT(*) FROM entities WHERE entity_type IN ({placeholders}) \
         OR id = ?{focus_parameter}"
    );
    let mut owned: Vec<Box<dyn rusqlite::ToSql>> = allowed_kinds
        .iter()
        .map(|kind| Box::new(kind.as_str().to_owned()) as Box<dyn rusqlite::ToSql>)
        .collect();
    owned.push(Box::new(focus_id.unwrap_or("").to_owned()));
    let parameters = owned.iter().map(|value| value.as_ref()).collect::<Vec<_>>();
    let count: i64 = conn
        .query_row(&sql, parameters.as_slice(), |row| row.get(0))
        .during("count allowed system-map entities")?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

fn directly_reachable_count(
    conn: &Connection,
    roots: &[String],
    allowed_kinds: &[EntityType],
    focus_id: Option<&str>,
) -> AppResult<u32> {
    let root_values = (0..roots.len())
        .map(|index| format!("(?{})", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    if all_entity_kinds_allowed(allowed_kinds) {
        let sql = format!(
            "WITH requested(id) AS (VALUES {root_values}) \
             SELECT COUNT(DISTINCT id) FROM (\
               SELECT id FROM requested \
               UNION ALL \
               SELECT r.target_id FROM entity_relationships r \
               JOIN requested ON requested.id = r.source_id \
               JOIN entities endpoint_entity ON endpoint_entity.id = r.target_id \
               UNION ALL \
               SELECT r.source_id FROM entity_relationships r \
               JOIN requested ON requested.id = r.target_id \
               JOIN entities endpoint_entity ON endpoint_entity.id = r.source_id\
             ) reachable"
        );
        let count: i64 = conn
            .query_row(&sql, params_from_iter(roots.iter()), |row| row.get(0))
            .during("count directly reachable system-map entities")?;
        return Ok(u32::try_from(count).unwrap_or(u32::MAX));
    }

    let kind_start = roots.len() + 1;
    let kind_placeholders = (0..allowed_kinds.len())
        .map(|offset| format!("?{}", kind_start + offset))
        .collect::<Vec<_>>()
        .join(", ");
    let focus_parameter = kind_start + allowed_kinds.len();
    let sql = format!(
        "WITH requested(id) AS (VALUES {root_values}) \
         SELECT COUNT(DISTINCT id) FROM (\
           SELECT id FROM requested \
           UNION ALL \
           SELECT r.target_id FROM entity_relationships r \
           JOIN requested ON requested.id = r.source_id \
           JOIN entities source_entity ON source_entity.id = r.source_id \
           JOIN entities target_entity ON target_entity.id = r.target_id \
           WHERE (source_entity.entity_type IN ({kind_placeholders}) \
                  OR source_entity.id = ?{focus_parameter}) \
             AND (target_entity.entity_type IN ({kind_placeholders}) \
                  OR target_entity.id = ?{focus_parameter}) \
           UNION ALL \
           SELECT r.source_id FROM entity_relationships r \
           JOIN requested ON requested.id = r.target_id \
           JOIN entities source_entity ON source_entity.id = r.source_id \
           JOIN entities target_entity ON target_entity.id = r.target_id \
           WHERE (source_entity.entity_type IN ({kind_placeholders}) \
                  OR source_entity.id = ?{focus_parameter}) \
             AND (target_entity.entity_type IN ({kind_placeholders}) \
                  OR target_entity.id = ?{focus_parameter})\
         ) reachable"
    );
    let mut owned: Vec<Box<dyn rusqlite::ToSql>> = roots
        .iter()
        .cloned()
        .map(|id| Box::new(id) as Box<dyn rusqlite::ToSql>)
        .collect();
    owned.extend(
        allowed_kinds
            .iter()
            .map(|kind| Box::new(kind.as_str().to_owned()) as Box<dyn rusqlite::ToSql>),
    );
    owned.push(Box::new(focus_id.unwrap_or("").to_owned()));
    let parameters = owned.iter().map(|value| value.as_ref()).collect::<Vec<_>>();
    let count: i64 = conn
        .query_row(&sql, parameters.as_slice(), |row| row.get(0))
        .during("count directly reachable system-map entities")?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

fn all_entity_kinds_allowed(allowed_kinds: &[EntityType]) -> bool {
    allowed_kinds.len() >= EntityType::ALL.len()
        && EntityType::ALL
            .iter()
            .all(|kind| allowed_kinds.contains(kind))
}

fn expand_reachable_count(
    conn: &Connection,
    allowed_kinds: &[EntityType],
    focus_id: Option<&str>,
    hop: u8,
    outgoing: bool,
) -> AppResult<usize> {
    let kind_start = 1;
    let kind_placeholders = (0..allowed_kinds.len())
        .map(|offset| format!("?{}", kind_start + offset))
        .collect::<Vec<_>>()
        .join(", ");
    let focus_parameter = kind_start + allowed_kinds.len();
    let hop_parameter = focus_parameter + 1;
    let next_hop_parameter = hop_parameter + 1;
    let (endpoint, frontier) = if outgoing {
        ("r.target_id", "r.source_id")
    } else {
        ("r.source_id", "r.target_id")
    };
    if all_entity_kinds_allowed(allowed_kinds) {
        let sql = format!(
            "INSERT OR IGNORE INTO temp_system_map_seen (id, depth) \
             SELECT {endpoint}, ?2 FROM entity_relationships r \
             JOIN temp_system_map_seen frontier \
               ON frontier.id = {frontier} AND frontier.depth = ?1 \
             JOIN entities endpoint_entity ON endpoint_entity.id = {endpoint}"
        );
        return conn
            .execute(&sql, (hop, hop.saturating_add(1)))
            .during("expand exact system-map count");
    }
    let sql = format!(
        "INSERT OR IGNORE INTO temp_system_map_seen (id, depth) \
         SELECT {endpoint}, ?{next_hop_parameter} FROM entity_relationships r \
         JOIN temp_system_map_seen frontier \
           ON frontier.id = {frontier} AND frontier.depth = ?{hop_parameter} \
         JOIN entities source_entity ON source_entity.id = r.source_id \
         JOIN entities target_entity ON target_entity.id = r.target_id \
         WHERE (source_entity.entity_type IN ({kind_placeholders}) \
                OR source_entity.id = ?{focus_parameter}) \
           AND (target_entity.entity_type IN ({kind_placeholders}) \
                OR target_entity.id = ?{focus_parameter})"
    );
    let mut owned: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    owned.extend(
        allowed_kinds
            .iter()
            .map(|kind| Box::new(kind.as_str().to_owned()) as Box<dyn rusqlite::ToSql>),
    );
    owned.push(Box::new(focus_id.unwrap_or("").to_owned()));
    owned.push(Box::new(hop));
    owned.push(Box::new(hop.saturating_add(1)));
    let parameters = owned.iter().map(|value| value.as_ref()).collect::<Vec<_>>();
    conn.execute(&sql, parameters.as_slice())
        .during("expand exact system-map count")
}

/// Degree and change metrics for the small, already-capped set of rendered
/// nodes. Every correlated lookup is served by a named schema index.
pub fn system_map_metrics(
    conn: &Connection,
    ids: &[String],
    recent_since: i64,
) -> AppResult<std::collections::BTreeMap<String, SystemMapNodeMetrics>> {
    if ids.is_empty() {
        return Ok(std::collections::BTreeMap::new());
    }
    let values = (0..ids.len())
        .map(|index| format!("(?{})", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let recent_parameter = ids.len() + 1;
    let sql = format!(
        "WITH requested(id) AS (VALUES {values}) \
         SELECT requested.id, \
           (SELECT COUNT(*) FROM entity_relationships WHERE source_id = requested.id) + \
           (SELECT COUNT(*) FROM entity_relationships WHERE target_id = requested.id), \
           (SELECT MAX(timestamp) FROM events WHERE entity_id = requested.id), \
           (SELECT COUNT(*) FROM events WHERE entity_id = requested.id \
              AND timestamp >= ?{recent_parameter}) \
         FROM requested ORDER BY requested.id"
    );
    let mut parameters: Vec<&dyn rusqlite::ToSql> =
        ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
    parameters.push(&recent_since);
    let mut statement = conn
        .prepare(&sql)
        .during("prepare system-map node metrics")?;
    let rows = statement
        .query_map(parameters.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .during("load system-map node metrics")?;
    let mut metrics = std::collections::BTreeMap::new();
    for row in rows {
        let (id, degree, last_changed_at, recent_event_count) =
            row.during("read system-map node metrics")?;
        metrics.insert(
            id,
            SystemMapNodeMetrics {
                degree: u32::try_from(degree).unwrap_or(u32::MAX),
                last_changed_at,
                recent_event_count: u32::try_from(recent_event_count).unwrap_or(u32::MAX),
            },
        );
    }
    Ok(metrics)
}

fn query_entities<P>(
    conn: &Connection,
    sql: &str,
    params: P,
    operation: &'static str,
) -> AppResult<Vec<Entity>>
where
    P: rusqlite::Params,
{
    let mut statement = conn.prepare(sql).during(operation)?;
    let rows = statement
        .query_map(params, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .during(operation)?;
    let mut entities = Vec::new();
    for row in rows {
        let (id, kind, display_name, first_seen, last_seen, metadata) = row.during(operation)?;
        entities.push(Entity {
            id,
            entity_type: parse_word("entity_type", &kind)?,
            display_name,
            first_seen,
            last_seen,
            metadata: metadata
                .as_deref()
                .map(|text| from_json("metadata_json", text))
                .transpose()?,
        });
    }
    Ok(entities)
}

/// Structural edges touching any requested entity, as they were known by the
/// end of an analysis window. The keyed branch binds ids in chunks; neither
/// branch interpolates caller ids into SQL. Duplicate edges are removed
/// deterministically.
/// When the requested set covers most entities, one ordered edge scan avoids
/// repeatedly hydrating the same dense graph through the keyed batches.
pub fn relationships_for_entities_at(
    conn: &Connection,
    entity_ids: &[String],
    recorded_at_or_before: i64,
) -> AppResult<Vec<Relationship>> {
    const IDS_PER_BATCH: usize = 800;
    const MIN_REQUESTED_FOR_SCAN: usize = 256;
    let mut relationships = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let requested = entity_ids
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    if requested.is_empty() {
        return Ok(relationships);
    }
    let scan_all = if requested.len() < MIN_REQUESTED_FOR_SCAN {
        false
    } else {
        let total_entities = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| {
                count_column(row, 0)
            })
            .during("count entities for relationship lookup")?;
        // Only scan when at least 75% of known entities are requested. A
        // small selection in a much larger production graph keeps keyed IO.
        let requested_count = requested.len() as u64;
        requested_count <= total_entities
            && requested_count.saturating_mul(4) >= total_entities.saturating_mul(3)
    };
    let mut use_keyed = !scan_all;
    if scan_all {
        let mut scanned = 0_usize;
        let mut matched = 0_usize;
        {
            let mut statement = conn
                .prepare(
                    "SELECT source_id, target_id, kind, event_id, recorded_at \
                     FROM entity_relationships WHERE recorded_at <= ?1 \
                     ORDER BY recorded_at, source_id, target_id",
                )
                .during("prepare dense relationship scan")?;
            let rows = statement
                .query_map([recorded_at_or_before], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                })
                .during("read dense relationships")?;
            for row in rows {
                let raw = row.during("read a dense relationship")?;
                scanned = scanned.saturating_add(1);
                if requested.contains(raw.0.as_str()) || requested.contains(raw.1.as_str()) {
                    matched = matched.saturating_add(1);
                    // This branch reads one table once. The schema's UNIQUE
                    // (source_id, target_id, kind) makes a per-row BTreeSet
                    // redundant; the keyed/chunked branch still deduplicates.
                    relationships.push(relationship_from_raw(raw)?);
                }
                // High entity coverage can still hide edge skew: the excluded
                // quarter might own almost every edge. Bound wasted hydration
                // and retry with the keyed predicate without changing rows.
                if scanned > 4096_usize.saturating_add(matched.saturating_mul(4)) {
                    use_keyed = true;
                    break;
                }
            }
        }
        if use_keyed {
            relationships.clear();
            seen.clear();
            #[cfg(feature = "devtools")]
            DENSE_RELATIONSHIP_FALLBACKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    if use_keyed {
        for ids in entity_ids.chunks(IDS_PER_BATCH) {
            let values = (0..ids.len())
                .map(|index| format!("(?{})", index + 2))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "WITH requested(id) AS (VALUES {values}) \
                 SELECT source_id, target_id, kind, event_id, recorded_at \
                 FROM entity_relationships WHERE recorded_at <= ?1 \
                   AND (source_id IN (SELECT id FROM requested) \
                        OR target_id IN (SELECT id FROM requested)) \
                 ORDER BY recorded_at, source_id, target_id, kind"
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> =
                Vec::with_capacity(ids.len().saturating_add(1));
            params.push(Box::new(recorded_at_or_before));
            params.extend(
                ids.iter()
                    .cloned()
                    .map(|id| Box::new(id) as Box<dyn rusqlite::ToSql>),
            );
            let refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|value| value.as_ref()).collect();
            let mut statement = conn
                .prepare(&sql)
                .during("prepare batched relationship query")?;
            let rows = statement
                .query_map(refs.as_slice(), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                })
                .during("read batched relationships")?;
            for row in rows {
                add_relationship_row(
                    &mut relationships,
                    &mut seen,
                    row.during("read a batched relationship")?,
                )?;
            }
        }
    }
    relationships.sort_by(|left, right| {
        (
            left.recorded_at,
            &left.source_id,
            &left.target_id,
            left.kind.as_str(),
        )
            .cmp(&(
                right.recorded_at,
                &right.source_id,
                &right.target_id,
                right.kind.as_str(),
            ))
    });
    Ok(relationships)
}

fn add_relationship_row(
    relationships: &mut Vec<Relationship>,
    seen: &mut std::collections::BTreeSet<(String, String, String)>,
    raw: (String, String, String, Option<i64>, i64),
) -> AppResult<()> {
    if seen.insert((raw.0.clone(), raw.1.clone(), raw.2.clone())) {
        relationships.push(relationship_from_raw(raw)?);
    }
    Ok(())
}

fn relationship_from_raw(
    raw: (String, String, String, Option<i64>, i64),
) -> AppResult<Relationship> {
    Ok(Relationship {
        source_id: raw.0,
        target_id: raw.1,
        kind: parse_word::<RelationshipKind>("kind", &raw.2)?,
        event_id: raw.3,
        recorded_at: raw.4,
    })
}

/// Entity-name candidates for symptom tokens. The query is bounded and all
/// LIKE patterns are parameters; wildcard characters in user text are escaped.
pub fn name_candidates(
    conn: &Connection,
    needles: &[String],
    limit: u16,
) -> AppResult<Vec<Entity>> {
    const MAX_NEEDLES: usize = 32;
    let patterns: Vec<String> = needles
        .iter()
        .filter_map(|needle| {
            let needle = needle.trim().to_ascii_lowercase();
            (needle.chars().count() >= 4).then(|| {
                format!(
                    "%{}%",
                    needle
                        .replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_")
                )
            })
        })
        .take(MAX_NEEDLES)
        .collect();
    if patterns.is_empty() {
        return Ok(Vec::new());
    }
    let filters = (0..patterns.len())
        .map(|index| format!("LOWER(display_name) LIKE ?{} ESCAPE '\\'", index + 1))
        .collect::<Vec<_>>()
        .join(" OR ");
    let limit_index = patterns.len() + 1;
    let sql = format!(
        "SELECT id, entity_type, display_name, first_seen, last_seen, metadata_json \
         FROM entities WHERE {filters} ORDER BY last_seen DESC, id LIMIT ?{limit_index}"
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = patterns
        .into_iter()
        .map(|pattern| Box::new(pattern) as Box<dyn rusqlite::ToSql>)
        .collect();
    params.push(Box::new(i64::from(limit.clamp(1, 500))));
    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|value| value.as_ref()).collect();
    let mut statement = conn
        .prepare(&sql)
        .during("prepare entity-name candidates")?;
    let rows = statement
        .query_map(refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .during("read entity-name candidates")?;
    let mut entities = Vec::new();
    for row in rows {
        let raw = row.during("read an entity-name candidate")?;
        entities.push(Entity {
            id: raw.0,
            entity_type: parse_word("entity_type", &raw.1)?,
            display_name: raw.2,
            first_seen: raw.3,
            last_seen: raw.4,
            metadata: raw
                .5
                .as_deref()
                .map(|text| from_json("metadata_json", text))
                .transpose()?,
        });
    }
    Ok(entities)
}

fn relationships(conn: &Connection, sql: &str, id: &str) -> AppResult<Vec<Relationship>> {
    let mut statement = conn
        .prepare_cached(sql)
        .during("prepare a relationship query")?;
    let rows = statement
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .during("read relationships")?;
    let mut out = Vec::new();
    for row in rows {
        let (source_id, target_id, kind, event_id, recorded_at) =
            row.during("read relationships")?;
        out.push(Relationship {
            source_id,
            target_id,
            kind: parse_word::<RelationshipKind>("kind", &kind)?,
            event_id,
            recorded_at,
        });
    }
    Ok(out)
}

/// Total entities.
pub fn count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM entities")
        .during("prepare a count")?
        .query_row([], |row| count_column(row, 0))
        .during("count entities")
}
