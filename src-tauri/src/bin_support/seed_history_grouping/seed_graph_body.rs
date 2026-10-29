//! Shared graph work; the caller alone owns transaction completion.

use std::collections::HashSet;

use rusqlite::Transaction;
use what_changed_lib::domain::{entity_type_of, Entity, Relationship};
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::entity_repository;

use super::{relation_kind, RelationSeedInput};

/// State is consumed and returned only when every selected edge succeeds.
/// The transaction owner must abandon the transaction after any body error.
pub(super) fn insert_graph_body(
    tx: &Transaction<'_>,
    relation_inputs: &[RelationSeedInput],
    relation_ids: &[i64],
    mut known_entities: HashSet<String>,
    mut owned: HashSet<String>,
) -> AppResult<(HashSet<String>, HashSet<String>)> {
    for event in relation_inputs {
        let event_id = relation_ids[event.batch_index];
        let kind = relation_kind(event.category).ok_or_else(|| AppError::InvalidInput {
            what: "synthetic relationship category",
            reason: "selected evidence has no supported relationship kind".to_owned(),
        })?;
        // A synthetic run must never rename or extend a real entity whose
        // stable id collides. Create only a missing endpoint; allocate its
        // owned entity only after the borrowed known-set lookup and SQL proof.
        if !known_entities.contains(&event.entity_id) {
            known_entities.insert(event.entity_id.clone());
            if entity_repository::by_id(tx, &event.entity_id)?.is_none() {
                owned.insert(event.entity_id.clone());
                let source = Entity {
                    id: event.entity_id.clone(),
                    entity_type: event.entity_type,
                    display_name: event.entity_display_name.clone(),
                    first_seen: event.timestamp,
                    last_seen: event.timestamp,
                    metadata: None,
                };
                let source = Sanitizer::new().sanitize(source).0;
                entity_repository::upsert_sanitized(tx, &source)?;
            }
        }
        for target_id in &event.related_entity_ids {
            let entity_type = entity_type_of(target_id).ok_or_else(|| AppError::InvalidInput {
                what: "synthetic relationship target",
                reason: "generated graph endpoint is not a valid entity id".to_owned(),
            })?;
            if !known_entities.contains(target_id) {
                known_entities.insert(target_id.clone());
                if entity_repository::by_id(tx, target_id)?.is_none() {
                    owned.insert(target_id.clone());
                    let target = Entity {
                        id: target_id.clone(),
                        entity_type,
                        display_name: target_id.clone(),
                        first_seen: event.timestamp,
                        last_seen: event.timestamp,
                        metadata: None,
                    };
                    let target = Sanitizer::new().sanitize(target).0;
                    entity_repository::upsert_sanitized(tx, &target)?;
                }
            }
            if !entity_repository::insert_qa_relationship_if_absent(
                tx,
                &Relationship {
                    source_id: event.entity_id.clone(),
                    target_id: target_id.clone(),
                    kind,
                    event_id: Some(event_id),
                    recorded_at: event.timestamp,
                },
            )? {
                return Err(AppError::InvalidInput {
                    what: "synthetic relationship quota",
                    reason: "a generated edge already exists; refusing to replace its evidence"
                        .to_owned(),
                });
            }
        }
    }
    Ok((known_entities, owned))
}
