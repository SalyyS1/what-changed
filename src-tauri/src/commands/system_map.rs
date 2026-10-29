//! The validated IPC boundary for the relationship System Map.

use tauri::State;

use crate::analysis::system_map::{EntityRef, SystemMapData, MAX_SYSTEM_MAP_DEPTH};
use crate::app::{validate, AppState};
use crate::domain::{ids, EntityType};
use crate::dto::CommandResult;
use crate::error::AppError;

#[tauri::command]
pub async fn get_system_map(
    state: State<'_, AppState>,
    mut focus: Option<EntityRef>,
    depth: u8,
    mut kinds: Vec<EntityType>,
) -> CommandResult<SystemMapData> {
    let _query = state.query_busy.enter();
    if let Some(entity) = &mut focus {
        entity.entity_id = validate_entity_ref(entity)?;
    }
    if kinds.len() > EntityType::ALL.len() {
        return Err(invalid("kinds", "too many entity kinds").into());
    }
    kinds.sort_by_key(|kind| kind.as_str());
    kinds.dedup();
    let depth = depth.clamp(1, MAX_SYSTEM_MAP_DEPTH);
    Ok(state
        .db
        .read(|conn| crate::storage::system_map_repository::build(conn, focus, depth, &kinds))?)
}

fn validate_entity_ref(entity: &EntityRef) -> Result<String, AppError> {
    let id = validate::text("entity id", &entity.entity_id, validate::MAX_TEXT)?;
    if !ids::is_valid_entity_id_for(&id, entity.entity_type) {
        return Err(invalid(
            "entity id",
            "its prefix does not match the entity type",
        ));
    }
    Ok(id)
}

fn invalid(what: &'static str, reason: &'static str) -> AppError {
    AppError::InvalidInput {
        what,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn entity_reference_prefix_must_match_its_kind() {
        assert!(validate_entity_ref(&EntityRef {
            entity_id: "service:api".to_owned(),
            entity_type: EntityType::Service,
        })
        .is_ok());
        assert!(validate_entity_ref(&EntityRef {
            entity_id: "port:tcp:0.0.0.0:3000".to_owned(),
            entity_type: EntityType::Service,
        })
        .is_err());
        assert_eq!(
            validate_entity_ref(&EntityRef {
                entity_id: "  service:api  ".to_owned(),
                entity_type: EntityType::Service,
            })
            .unwrap(),
            "service:api"
        );
        assert!(validate_entity_ref(&EntityRef {
            entity_id: "service:api\nspoof".to_owned(),
            entity_type: EntityType::Service,
        })
        .is_err());
        assert!(validate_entity_ref(&EntityRef {
            entity_id: "service:".to_owned(),
            entity_type: EntityType::Service,
        })
        .is_err());
        assert!(validate_entity_ref(&EntityRef {
            entity_id: "unknown:value".to_owned(),
            entity_type: EntityType::Service,
        })
        .is_err());
    }
}
