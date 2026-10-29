//! Rule revisions are append-only; change drafts bind exact app-owned revisions.

use super::repository::{capacity, expected, page, require_app, row};
use crate::error::AppResult;
use crate::owner_workspace::{validation, *};
use crate::storage::{to_json, SqlContext};
use rusqlite::{params, OptionalExtension};

pub(super) fn rule_refs(
    conn: &rusqlite::Connection,
    app_id: &str,
    rules: &[RuleRefDto],
) -> AppResult<Vec<OwnerRuleRevisionDto>> {
    if rules.len() > MAX_RULES_PER_APP {
        return Err(validation::invalid(
            "preserved rules",
            "at most 500 rules are supported",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    for rule in rules {
        validation::uuid(&rule.rule_id, "rule id")?;
        if !seen.insert((&rule.rule_id, rule.revision)) {
            return Err(validation::invalid(
                "rule references",
                "duplicates are not allowed",
            ));
        }
        result.push(row(conn,"SELECT payload FROM owner_rule_revisions WHERE app_id=?1 AND rule_id=?2 AND revision=?3",
            params![app_id,rule.rule_id,rule.revision])?.ok_or_else(|| validation::invalid("rule reference", "does not belong to this app or revision is missing"))?);
    }
    Ok(result)
}

impl OwnerStore {
    pub fn confirm_rule(&self, input: ConfirmOwnerRuleInput) -> AppResult<OwnerRuleRevisionDto> {
        validation::uuid(&input.rule_id, "rule id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::uuid(&input.capability_id, "capability id")?;
        validation::required(&input.expected_behavior, "expected behavior")?;
        validation::required(&input.confirmed_by, "confirmed by")?;
        self.mutate(input.operation_id.clone(),"confirm_rule",input,|tx,clean| {
            let input = clean.get();
            require_app(tx,&input.app_id)?;
            let capability: Option<bool> = tx.query_row("SELECT archived FROM owner_capabilities WHERE app_id=?1 AND id=?2",params![input.app_id,input.capability_id],|r|r.get(0))
                .optional().during("check rule capability")?;
            if capability != Some(false) { return Err(validation::invalid("capability", "is missing, archived or belongs to another app")); }
            let prior: Option<OwnerRuleRevisionDto> = row(tx,"SELECT payload FROM owner_rule_revisions WHERE rule_id=?1 ORDER BY revision DESC LIMIT 1",[&input.rule_id])?;
            if prior.as_ref().is_some_and(|r|r.app_id != input.app_id || r.capability_id != input.capability_id) {
                return Err(validation::invalid("rule", "cannot move a rule between apps or capabilities"));
            }
            let revision = expected(prior.as_ref().map(|r|r.revision),input.expected_version)?;
            if prior.is_none() { capacity(tx,"SELECT count(DISTINCT rule_id) FROM owner_rule_revisions WHERE app_id=?1",&input.app_id,MAX_RULES_PER_APP,"rules")?; }
            let result = OwnerRuleRevisionDto { rule_id:input.rule_id.clone(),app_id:input.app_id.clone(),capability_id:input.capability_id.clone(),revision,
                expected_behavior:input.expected_behavior.clone(),provenance:input.provenance,confirmed_by:input.confirmed_by.clone(),confirmed_at:crate::domain::now_ms(),
                supersedes:prior.map(|r|r.revision) };
            tx.execute("INSERT INTO owner_rule_revisions(rule_id,app_id,capability_id,revision,payload) VALUES(?1,?2,?3,?4,?5)",params![result.rule_id,result.app_id,result.capability_id,result.revision,to_json(&result)?]).during("confirm owner rule revision")?;
            Ok(result)
        })
    }

    pub fn list_rule_revisions(
        &self,
        app_id: &str,
        input: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<OwnerRuleRevisionDto>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn| page(conn,"SELECT rule_id || ':' || printf('%010d',revision) AS cursor,payload FROM owner_rule_revisions WHERE app_id=?1 AND cursor>?2 ORDER BY rule_id,revision LIMIT ?3",app_id,input))
    }

    pub fn save_change_draft(&self, input: SaveOwnerChangeInput) -> AppResult<OwnerChangeDraftDto> {
        validation::uuid(&input.id, "change id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::required(&input.intent, "change intent")?;
        if let Some(id) = &input.candidate_id {
            validation::uuid(id, "candidate id")?;
        }
        self.mutate(input.operation_id.clone(),"save_change",input,|tx,clean| {
            let input = clean.get();
            require_app(tx,&input.app_id)?;
            rule_refs(tx,&input.app_id,&input.preserved_rules)?;
            if let Some(id) = &input.candidate_id {
                let candidate: Option<OwnerCandidateRevisionDto> = row(tx,"SELECT payload FROM owner_candidates WHERE app_id=?1 AND id=?2",params![input.app_id,id])?;
                if candidate.is_none() { return Err(validation::invalid("candidate", "does not belong to this app")); }
            }
            let prior:Option<OwnerChangeDraftDto> = row(tx,"SELECT payload FROM owner_change_drafts WHERE id=?1",[&input.id])?;
            if prior.as_ref().is_some_and(|r|r.app_id!=input.app_id) { return Err(validation::invalid("change","does not belong to this app")); }
            let version=expected(prior.as_ref().map(|r|r.version),input.expected_version)?;
            if prior.is_none() { capacity(tx,"SELECT count(*) FROM owner_change_drafts WHERE app_id=?1",&input.app_id,500,"change drafts")?; }
            let result=OwnerChangeDraftDto { id:input.id.clone(),app_id:input.app_id.clone(),version,intent:input.intent.clone(),example:input.example.clone(),scope:input.scope.clone(),
                preserved_rules:input.preserved_rules.clone(),candidate_id:input.candidate_id.clone(),state:input.state,updated_at:crate::domain::now_ms() };
            tx.execute("INSERT INTO owner_change_drafts(id,app_id,version,candidate_id,payload) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET version=excluded.version,candidate_id=excluded.candidate_id,payload=excluded.payload",
                params![result.id,result.app_id,result.version,result.candidate_id,to_json(&result)?]).during("save owner change draft")?;
            tx.execute("DELETE FROM owner_change_rules WHERE change_id=?1",[&result.id]).during("replace change rule references")?;
            for rule in &result.preserved_rules { tx.execute("INSERT INTO owner_change_rules(change_id,app_id,rule_id,revision) VALUES(?1,?2,?3,?4)",params![result.id,result.app_id,rule.rule_id,rule.revision]).during("bind change rule revision")?; }
            Ok(result)
        })
    }

    pub fn get_change_draft(
        &self,
        app_id: &str,
        id: &str,
    ) -> AppResult<Option<OwnerChangeDraftDto>> {
        validation::uuid(app_id, "app id")?;
        validation::uuid(id, "change id")?;
        self.read(|conn| {
            row(
                conn,
                "SELECT payload FROM owner_change_drafts WHERE app_id=?1 AND id=?2",
                params![app_id, id],
            )
        })
    }

    pub fn list_change_drafts(
        &self,
        app_id: &str,
        input: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<OwnerChangeDraftDto>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn|page(conn,"SELECT id,payload FROM owner_change_drafts WHERE app_id=?1 AND id>?2 ORDER BY id LIMIT ?3",app_id,input))
    }

    pub fn get_change_review(&self, app_id: &str, id: &str) -> AppResult<OwnerChangeReviewDto> {
        validation::uuid(app_id, "app id")?;
        validation::uuid(id, "change id")?;
        self.read(|conn| {
            // One read transaction freezes the joined review against concurrent edits.
            let tx=conn.unchecked_transaction().during("begin owner review snapshot")?;
            let app=row(&tx,"SELECT payload FROM owner_apps WHERE id=?1",[app_id])?.ok_or_else(||validation::invalid("app","does not exist"))?;
            let change:OwnerChangeDraftDto=row(&tx,"SELECT payload FROM owner_change_drafts WHERE app_id=?1 AND id=?2",params![app_id,id])?.ok_or_else(||validation::invalid("change","does not belong to this app"))?;
            let preserved_rules=rule_refs(&tx,app_id,&change.preserved_rules)?;
            let candidate:Option<OwnerCandidateRevisionDto>=match &change.candidate_id { Some(id)=>row(&tx,"SELECT payload FROM owner_candidates WHERE app_id=?1 AND id=?2",params![app_id,id])?,None=>None };
            let environment=match &candidate { Some(c)=>row(&tx,"SELECT payload FROM owner_environment_revisions WHERE app_id=?1 AND id=?2 AND revision=?3",params![app_id,c.environment_id,c.environment_revision])?,None=>None };
            Ok(OwnerChangeReviewDto { app,change,preserved_rules,candidate,environment })
        })
    }
}
