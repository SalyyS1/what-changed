//! Frozen scenario revisions and recipes. Authoring may arrive later, but its schema is stable.

use super::repository::{capacity, expected, page, require_app, row};
use crate::error::AppResult;
use crate::owner_workspace::{validation, *};
use crate::storage::{to_json, SqlContext};
use rusqlite::{params, OptionalExtension};

impl OwnerStore {
    pub fn confirm_scenario(
        &self,
        input: ConfirmOwnerScenarioInput,
    ) -> AppResult<ScenarioRevisionDto> {
        validation::uuid(&input.id, "scenario id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::uuid(&input.capability_id, "capability id")?;
        validation::required(&input.title, "scenario title")?;
        validation::required(&input.expected_behavior, "scenario expected behavior")?;
        validation::required(&input.confirmed_by, "scenario confirmed by")?;
        validation::recipe(&input.recipe)?;
        self.mutate(input.operation_id.clone(),"confirm_scenario",input,|tx,clean| {
            let input=clean.get(); require_app(tx,&input.app_id)?;
            validation::recipe(&input.recipe)?;
            let cap:Option<bool>=tx.query_row("SELECT archived FROM owner_capabilities WHERE app_id=?1 AND id=?2",params![input.app_id,input.capability_id],|r|r.get(0)).optional().during("check scenario capability")?;
            if cap!=Some(false) { return Err(validation::invalid("capability","is missing, archived or belongs to another app")); }
            super::rules::rule_refs(tx,&input.app_id,&input.rules)?;
            let prior:Option<ScenarioRevisionDto>=row(tx,"SELECT payload FROM owner_scenario_revisions WHERE id=?1 ORDER BY revision DESC LIMIT 1",[&input.id])?;
            if prior.as_ref().is_some_and(|r|r.app_id!=input.app_id || r.capability_id!=input.capability_id) { return Err(validation::invalid("scenario","cannot move it between apps or capabilities")); }
            let revision=expected(prior.as_ref().map(|r|r.revision),input.expected_version)?;
            if prior.is_none(){capacity(tx,"SELECT count(DISTINCT id) FROM owner_scenario_revisions WHERE app_id=?1",&input.app_id,MAX_SCENARIOS_PER_APP,"scenarios")?;}
            let result=ScenarioRevisionDto{id:input.id.clone(),app_id:input.app_id.clone(),capability_id:input.capability_id.clone(),revision,title:input.title.clone(),expected_behavior:input.expected_behavior.clone(),confirmed_by:input.confirmed_by.clone(),method:input.method,recipe:input.recipe.clone(),rules:input.rules.clone(),confirmed_at:crate::domain::now_ms()};
            tx.execute("INSERT INTO owner_scenario_revisions(id,app_id,capability_id,revision,payload) VALUES(?1,?2,?3,?4,?5)",params![result.id,result.app_id,result.capability_id,result.revision,to_json(&result)?]).during("confirm scenario revision")?;
            for rule in &result.rules {tx.execute("INSERT INTO owner_scenario_rules(scenario_id,scenario_revision,app_id,rule_id,rule_revision) VALUES(?1,?2,?3,?4,?5)",params![result.id,result.revision,result.app_id,rule.rule_id,rule.revision]).during("bind scenario rule revision")?;}
            Ok(result)
        })
    }

    pub fn list_scenarios(
        &self,
        app_id: &str,
        input: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<ScenarioRevisionDto>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn|page(conn,"SELECT id || ':' || printf('%010d',revision) AS cursor,payload FROM owner_scenario_revisions WHERE app_id=?1 AND cursor>?2 ORDER BY id,revision LIMIT ?3",app_id,input))
    }
}
