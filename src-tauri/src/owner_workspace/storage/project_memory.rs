//! Immutable owner-authored project decisions with explicit supersession.
use super::repository::{expected, require_app, row};
use crate::dto::owner_memory_dto::*;
use crate::error::AppResult;
use crate::owner_workspace::{validation, OwnerPageDto, OwnerStore};
use crate::privacy::{Redaction, Sanitizer, Scrub};
use crate::storage::{from_json, to_json, SqlContext};
use rusqlite::{params, OptionalExtension};

impl Scrub for SaveProjectDecisionInput {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        self.text
            .scrub_in_place(sanitizer, &format!("{field}.text"), report);
        self.rationale
            .scrub_in_place(sanitizer, &format!("{field}.rationale"), report);
        self.recorded_by
            .scrub_in_place(sanitizer, &format!("{field}.recorded_by"), report);
    }
}

impl OwnerStore {
    pub fn save_project_decision(
        &self,
        input: SaveProjectDecisionInput,
    ) -> AppResult<ProjectDecisionRevisionDto> {
        validation::uuid(&input.id, "project decision id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::required(&input.text, "decision")?;
        validation::required(&input.rationale, "decision rationale")?;
        validation::required(&input.recorded_by, "decision author")?;
        self.mutate(input.operation_id.clone(), "project_decision", input, |tx, clean| {
            let input = clean.get();
            require_app(tx, &input.app_id)?;
            for (id, sql) in [
                (&input.capability_id, "SELECT id FROM owner_capabilities WHERE app_id=?1 AND id=?2"),
                (&input.change_id, "SELECT id FROM owner_change_drafts WHERE app_id=?1 AND id=?2"),
            ] {
                if let Some(id) = id {
                    validation::uuid(id, "decision reference")?;
                    let found: Option<String> = tx.query_row(sql, params![input.app_id, id], |r| r.get(0)).optional().during("check project decision reference")?;
                    if found.is_none() { return Err(validation::invalid("decision reference", "does not belong to this app")); }
                }
            }
            let prior: Option<ProjectDecisionRevisionDto> = row(tx, "SELECT payload FROM owner_project_decisions WHERE id=?1 ORDER BY revision DESC LIMIT 1", [&input.id])?;
            if prior.as_ref().is_some_and(|p| p.app_id != input.app_id) { return Err(validation::invalid("decision", "does not belong to this app")); }
            let revision = expected(prior.as_ref().map(|p| p.revision), input.expected_revision)?;
            super::repository::capacity(tx, "SELECT count(*) FROM owner_project_decisions WHERE app_id=?1", &input.app_id, 1000, "project decision revisions")?;
            let result = ProjectDecisionRevisionDto {
                id: input.id.clone(), app_id: input.app_id.clone(), revision,
                capability_id: input.capability_id.clone(), change_id: input.change_id.clone(),
                text: input.text.clone(), rationale: input.rationale.clone(), provenance: input.provenance,
                recorded_by: input.recorded_by.clone(), supersedes: prior.map(|p| p.revision), recorded_at: crate::domain::now_ms(),
            };
            tx.execute("INSERT INTO owner_project_decisions(id,app_id,revision,capability_id,change_id,payload) VALUES(?1,?2,?3,?4,?5,?6)",
                params![result.id, result.app_id, result.revision, result.capability_id, result.change_id, to_json(&result)?]).during("record project decision")?;
            Ok(result)
        })
    }

    pub fn list_project_decisions(
        &self,
        app_id: &str,
        input: ProjectMemoryFilterInput,
    ) -> AppResult<OwnerPageDto<ProjectDecisionRevisionDto>> {
        validation::uuid(app_id, "app id")?;
        validation::page(&input.page)?;
        for id in [&input.capability_id, &input.change_id]
            .into_iter()
            .flatten()
        {
            validation::uuid(id, "decision filter")?;
        }
        self.read(|conn| {
            let mut statement = conn.prepare("SELECT id || ':' || printf('%010d',revision),payload FROM owner_project_decisions WHERE app_id=?1 AND (?2 IS NULL OR capability_id=?2) AND (?3 IS NULL OR change_id=?3) AND (id || ':' || printf('%010d',revision))>?4 ORDER BY id,revision LIMIT ?5").during("prepare project memory")?;
            let rows = statement.query_map(params![app_id,input.capability_id,input.change_id,input.page.after.unwrap_or_default(),input.page.limit+1], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).during("read project memory")?.collect::<rusqlite::Result<Vec<_>>>().during("decode project memory")?;
            let more = rows.len() > input.page.limit as usize;
            let mut items = Vec::new(); let mut last = None;
            for (cursor,text) in rows.into_iter().take(input.page.limit as usize) { items.push(from_json("project decision",&text)?); last=Some(cursor); }
            Ok(OwnerPageDto {items,next_cursor: if more {last} else {None}})
        })
    }
}
