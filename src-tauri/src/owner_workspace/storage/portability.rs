//! Snapshot all portable rows atomically; OS paths and consent are not portable.
mod apply;
use super::repository::row;
use crate::error::AppResult;
use crate::owner_workspace::{portability::*, validation, *};
use crate::storage::{from_json, SqlContext};
use rusqlite::Connection;
use serde::de::DeserializeOwned;

fn rows<T: DeserializeOwned>(conn: &Connection, sql: &str, app_id: &str) -> AppResult<Vec<T>> {
    let mut query = conn
        .prepare(sql)
        .during("prepare portable memory snapshot")?;
    let values = query
        .query_map([app_id], |r| r.get::<_, String>(0))
        .during("read portable memory snapshot")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("decode portable memory snapshot")?;
    if values.len() > MAX_RECORDS {
        return Err(validation::invalid(
            "export",
            "too many records; archive a smaller workspace",
        ));
    }
    values
        .iter()
        .map(|s| from_json("portable record", s))
        .collect()
}

impl OwnerStore {
    pub fn portable_snapshot(&self, app_id: &str) -> AppResult<PortableWorkspace> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn| {
            let tx=conn.unchecked_transaction().during("begin portable snapshot")?;
            let app=row(&tx,"SELECT payload FROM owner_apps WHERE id=?1",[app_id])?.ok_or_else(||validation::invalid("app","does not exist"))?;
            let mut records=PortableRecords {
                app,
                capabilities:rows(&tx,"SELECT payload FROM owner_capabilities WHERE app_id=?1 ORDER BY id LIMIT 2001",app_id)?,
                rules:rows(&tx,"SELECT payload FROM owner_rule_revisions WHERE app_id=?1 ORDER BY rule_id,revision LIMIT 2001",app_id)?,
                changes:rows(&tx,"SELECT payload FROM owner_change_drafts WHERE app_id=?1 ORDER BY id LIMIT 2001",app_id)?,
                environments:rows(&tx,"SELECT payload FROM owner_environment_revisions WHERE app_id=?1 ORDER BY id,revision LIMIT 2001",app_id)?,
                candidates:rows(&tx,"SELECT payload FROM owner_candidates WHERE app_id=?1 ORDER BY id LIMIT 2001",app_id)?,
                scenarios:rows(&tx,"SELECT payload FROM owner_scenario_revisions WHERE app_id=?1 ORDER BY id,revision LIMIT 2001",app_id)?,
                project_decisions:rows(&tx,"SELECT payload FROM owner_project_decisions WHERE app_id=?1 ORDER BY id,revision LIMIT 2001",app_id)?,
                runs:rows(&tx,"SELECT payload FROM owner_runs WHERE app_id=?1 ORDER BY id LIMIT 2001",app_id)?,
                results:rows(&tx,"SELECT payload FROM owner_results WHERE app_id=?1 ORDER BY id LIMIT 2001",app_id)?,
                decisions:rows(&tx,"SELECT payload FROM owner_decisions WHERE app_id=?1 ORDER BY id LIMIT 2001",app_id)?,
            };
            // Evaluate against this same SQLite snapshot. A previously saved
            // acceptance is history; its decision-time freshness is preserved
            // and its current freshness is recomputed for the export.
            records.decisions.clear();
            for run in &records.runs {
                records.decisions.extend(super::evidence::review_in_snapshot(&tx,app_id,&run.id)?.decisions);
            }
            records.decisions.sort_by(|a,b|a.id.cmp(&b.id));
            let history=rows(&tx,"SELECT payload FROM owner_imported_history WHERE app_id=?1 ORDER BY source_digest LIMIT 21",app_id)?;
            let result=PortableWorkspace{schema_version:PORTABLE_SCHEMA,records,history}; result.validate()?; Ok(result)
        })
    }
    pub fn imported_owner_history(&self, app_id: &str) -> AppResult<Vec<PortableHistory>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn|rows(conn,"SELECT payload FROM owner_imported_history WHERE app_id=?1 ORDER BY source_digest LIMIT 21",app_id))
    }
}
