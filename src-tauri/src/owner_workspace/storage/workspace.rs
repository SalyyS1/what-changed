//! Mutable app/capability records with optimistic versions.

use super::repository::{expected, require_app, row};
use crate::error::AppResult;
use crate::owner_workspace::{validation, *};
use crate::storage::{to_json, SqlContext};
use rusqlite::{params, OptionalExtension};

impl OwnerStore {
    pub fn save_app(&self, input: SaveOwnerAppInput) -> AppResult<OwnerAppDto> {
        validation::uuid(&input.id, "app id")?;
        validation::required(&input.name, "app name")?;
        if let Some(link) = &input.legacy_project_id {
            if !link
                .strip_prefix("project:")
                .is_some_and(crate::domain::ids::is_short_hash)
            {
                return Err(validation::invalid(
                    "legacy_project_id",
                    "must be a project identity with sixteen lower-case hexadecimal digits",
                ));
            }
        }
        self.mutate(input.operation_id.clone(), "save_app", input, |tx, clean| {
            let input = clean.get();
            let previous: Option<OwnerAppDto> = row(tx, "SELECT payload FROM owner_apps WHERE id=?1", [&input.id])?;
            let version = expected(previous.as_ref().map(|r| r.version), input.expected_version)?;
            if previous.is_none() {
                let raw: i64 = tx.query_row("SELECT count(*) FROM owner_apps", [], |r| r.get(0)).during("count owner apps")?;
                let count = usize::try_from(raw).map_err(|_| validation::invalid("apps", "stored app count is invalid"))?;
                if count >= MAX_APPS { return Err(validation::invalid("apps", "at most 20 apps are supported, including archived apps")); }
            }
            let now = crate::domain::now_ms();
            let result = OwnerAppDto { id: input.id.clone(), name: input.name.clone(), description: input.description.clone(),
                legacy_project_id: input.legacy_project_id.clone(), current_candidate_id: previous.as_ref().and_then(|r| r.current_candidate_id.clone()),
                version, archived_at: input.archived.then_some(previous.as_ref().and_then(|r| r.archived_at).unwrap_or(now)),
                created_at: previous.as_ref().map_or(now, |r| r.created_at), updated_at: now };
            tx.execute("INSERT INTO owner_apps(id,version,archived,payload) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET version=excluded.version,archived=excluded.archived,payload=excluded.payload",
                params![result.id,result.version,input.archived,to_json(&result)?]).during("save owner app")?;
            Ok(result)
        })
    }

    pub fn get_app(&self, id: &str) -> AppResult<Option<OwnerAppDto>> {
        validation::uuid(id, "app id")?;
        self.read(|conn| row(conn, "SELECT payload FROM owner_apps WHERE id=?1", [id]))
    }

    pub fn list_apps(&self, include_archived: bool) -> AppResult<Vec<OwnerAppDto>> {
        self.read(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT payload FROM owner_apps WHERE (?1 OR archived=0) ORDER BY id LIMIT 21",
                )
                .during("list owner apps")?;
            let texts = statement
                .query_map([include_archived], |r| r.get::<_, String>(0))
                .during("list owner apps")?
                .collect::<rusqlite::Result<Vec<_>>>()
                .during("decode owner apps")?;
            if texts.len() > MAX_APPS {
                return Err(validation::invalid(
                    "apps",
                    "stored app count exceeds the supported limit",
                ));
            }
            texts
                .iter()
                .map(|text| crate::storage::from_json("owner app", text))
                .collect()
        })
    }

    pub fn save_capability(
        &self,
        input: SaveOwnerCapabilityInput,
    ) -> AppResult<OwnerCapabilityDto> {
        validation::uuid(&input.id, "capability id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::required(&input.label, "capability label")?;
        self.mutate(input.operation_id.clone(), "save_capability", input, |tx, clean| {
            let input = clean.get();
            require_app(tx, &input.app_id)?;
            let previous: Option<(String,u32)> = tx.query_row("SELECT app_id,version FROM owner_capabilities WHERE id=?1", [&input.id], |r| Ok((r.get(0)?,r.get(1)?)))
                .optional().during("check capability ownership")?;
            if previous.as_ref().is_some_and(|r| r.0 != input.app_id) { return Err(validation::invalid("capability", "does not belong to this app")); }
            let version = expected(previous.as_ref().map(|r| r.1), input.expected_version)?;
            if previous.is_none() {
                super::repository::capacity(tx, "SELECT count(*) FROM owner_capabilities WHERE app_id=?1", &input.app_id, MAX_CAPABILITIES_PER_APP, "capabilities")?;
            }
            let old: Option<OwnerCapabilityDto> = row(tx,"SELECT payload FROM owner_capabilities WHERE id=?1",[&input.id])?;
            let result = OwnerCapabilityDto { id: input.id.clone(), app_id: input.app_id.clone(), label: input.label.clone(), description: input.description.clone(), sort_order: input.sort_order, version,
                archived_at: input.archived.then_some(old.as_ref().and_then(|r| r.archived_at).unwrap_or_else(crate::domain::now_ms)) };
            tx.execute("INSERT INTO owner_capabilities(id,app_id,version,archived,payload) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET version=excluded.version,archived=excluded.archived,payload=excluded.payload",
                params![result.id,result.app_id,result.version,input.archived,to_json(&result)?]).during("save owner capability")?;
            Ok(result)
        })
    }

    pub fn list_capabilities(
        &self,
        app_id: &str,
        include_archived: bool,
    ) -> AppResult<Vec<OwnerCapabilityDto>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn| {
            let mut statement = conn.prepare("SELECT payload FROM owner_capabilities WHERE app_id=?1 AND (?2 OR archived=0) ORDER BY json_extract(payload,'$.sort_order'),id LIMIT 201").during("list owner capabilities")?;
            let texts = statement.query_map(params![app_id,include_archived], |r| r.get::<_,String>(0)).during("list owner capabilities")?
                .collect::<rusqlite::Result<Vec<_>>>().during("decode owner capabilities")?;
            if texts.len() > MAX_CAPABILITIES_PER_APP { return Err(validation::invalid("capabilities", "stored capability count exceeds the supported limit")); }
            texts.iter().map(|text| crate::storage::from_json("owner capability", text)).collect()
        })
    }
}
