//! Pure configuration and candidate selection. Saving a URL never connects to it.

use super::repository::{capacity, expected, page, require_app, row};
use crate::error::AppResult;
use crate::owner_workspace::{validation, *};
use crate::storage::{to_json, SqlContext};
use rusqlite::params;

impl OwnerStore {
    pub fn save_environment(
        &self,
        input: SaveOwnerEnvironmentInput,
    ) -> AppResult<OwnerEnvironmentRevisionDto> {
        validation::uuid(&input.id, "environment id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::payload(&input)?;
        validation::environment(&input)?;
        self.mutate(input.operation_id.clone(),"save_environment",input,|tx,clean| {
            let input=clean.get();
            validation::environment(input)?;
            require_app(tx,&input.app_id)?;
            let prior:Option<OwnerEnvironmentRevisionDto>=row(tx,"SELECT payload FROM owner_environment_revisions WHERE id=?1 ORDER BY revision DESC LIMIT 1",[&input.id])?;
            if prior.as_ref().is_some_and(|r|r.app_id!=input.app_id) { return Err(validation::invalid("environment","does not belong to this app")); }
            let revision=expected(prior.as_ref().map(|r|r.revision),input.expected_version)?;
            if prior.is_none() { capacity(tx,"SELECT count(DISTINCT id) FROM owner_environment_revisions WHERE app_id=?1",&input.app_id,20,"environments")?; }
            let result=OwnerEnvironmentRevisionDto { id:input.id.clone(),app_id:input.app_id.clone(),revision,label:input.label.clone(),preview_url:input.preview_url.clone(),origins:input.origins.clone(),kind:input.kind,revoked:input.revoked,created_at:crate::domain::now_ms() };
            tx.execute("INSERT INTO owner_environment_revisions(id,app_id,revision,revoked,payload) VALUES(?1,?2,?3,?4,?5)",params![result.id,result.app_id,result.revision,result.revoked,to_json(&result)?]).during("append owner environment revision")?;
            Ok(result)
        })
    }

    pub fn list_environment_revisions(
        &self,
        app_id: &str,
        input: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<OwnerEnvironmentRevisionDto>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn|page(conn,"SELECT id || ':' || printf('%010d',revision) AS cursor,payload FROM owner_environment_revisions WHERE app_id=?1 AND cursor>?2 ORDER BY id,revision LIMIT ?3",app_id,input))
    }

    pub fn save_candidate(
        &self,
        input: SaveOwnerCandidateInput,
    ) -> AppResult<OwnerCandidateRevisionDto> {
        validation::uuid(&input.id, "candidate id")?;
        validation::uuid(&input.app_id, "app id")?;
        validation::uuid(&input.environment_id, "environment id")?;
        validation::required(&input.identity, "candidate identity")?;
        if input.observed_at < 0 {
            return Err(validation::invalid(
                "observed_at",
                "must be epoch milliseconds at or after zero",
            ));
        }
        if input.trust == CandidateTrust::EnvironmentReported {
            return Err(validation::invalid("candidate trust","environment-reported identity requires a verified runner observation; configuration remains self-reported"));
        }
        if input.trust == CandidateTrust::SourceOnly && input.source != CandidateSource::GitCommit {
            return Err(validation::invalid(
                "candidate trust",
                "source-only trust requires a Git source identity",
            ));
        }
        self.mutate(input.operation_id.clone(),"save_candidate",input,|tx,clean| {
            let input=clean.get(); require_app(tx,&input.app_id)?;
            let prior:Option<OwnerCandidateRevisionDto>=row(tx,"SELECT payload FROM owner_candidates WHERE id=?1",[&input.id])?;
            if prior.is_some() { return Err(validation::invalid("candidate id","already exists; create a new immutable candidate")); }
            let environment:Option<OwnerEnvironmentRevisionDto>=row(tx,"SELECT payload FROM owner_environment_revisions WHERE app_id=?1 AND id=?2 ORDER BY revision DESC LIMIT 1",params![input.app_id,input.environment_id])?;
            if !environment.is_some_and(|e|e.revision==input.environment_revision && !e.revoked) {
                return Err(validation::invalid("environment revision","is missing, revoked, stale or belongs to another app"));
            }
            capacity(tx,"SELECT count(*) FROM owner_candidates WHERE app_id=?1",&input.app_id,1000,"candidates")?;
            let result=OwnerCandidateRevisionDto { id:input.id.clone(),app_id:input.app_id.clone(),environment_id:input.environment_id.clone(),environment_revision:input.environment_revision,
                identity:input.identity.clone(),source:input.source,trust:input.trust,observed_at:input.observed_at };
            tx.execute("INSERT INTO owner_candidates(id,app_id,environment_id,environment_revision,payload) VALUES(?1,?2,?3,?4,?5)",params![result.id,result.app_id,result.environment_id,result.environment_revision,to_json(&result)?]).during("append owner candidate")?;
            Ok(result)
        })
    }

    pub fn list_candidates(
        &self,
        app_id: &str,
        input: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<OwnerCandidateRevisionDto>> {
        validation::uuid(app_id, "app id")?;
        self.read(|conn|page(conn,"SELECT id,payload FROM owner_candidates WHERE app_id=?1 AND id>?2 ORDER BY id LIMIT ?3",app_id,input))
    }

    pub fn select_candidate(&self, input: SelectOwnerCandidateInput) -> AppResult<OwnerAppDto> {
        validation::uuid(&input.app_id, "app id")?;
        validation::uuid(&input.candidate_id, "candidate id")?;
        self.mutate(input.operation_id.clone(),"select_candidate",input,|tx,clean| {
            let input=clean.get(); require_app(tx,&input.app_id)?;
            let candidate:OwnerCandidateRevisionDto=row(tx,"SELECT payload FROM owner_candidates WHERE app_id=?1 AND id=?2",params![input.app_id,input.candidate_id])?
                .ok_or_else(||validation::invalid("candidate","does not belong to this app"))?;
            let latest:OwnerEnvironmentRevisionDto=row(tx,"SELECT payload FROM owner_environment_revisions WHERE app_id=?1 AND id=?2 ORDER BY revision DESC LIMIT 1",params![input.app_id,candidate.environment_id])?
                .ok_or_else(||validation::invalid("environment","does not belong to this app"))?;
            if latest.revoked || latest.revision!=candidate.environment_revision { return Err(validation::invalid("candidate","environment configuration changed; create a candidate for the current revision")); }
            let mut app:OwnerAppDto=row(tx,"SELECT payload FROM owner_apps WHERE id=?1",[&input.app_id])?.ok_or_else(||validation::invalid("app","does not exist"))?;
            app.version=expected(Some(app.version),input.expected_version)?;
            app.current_candidate_id=Some(candidate.id); app.updated_at=crate::domain::now_ms();
            tx.execute("UPDATE owner_apps SET version=?2,payload=?3 WHERE id=?1",params![app.id,app.version,to_json(&app)?]).during("select owner candidate")?;
            Ok(app)
        })
    }
}
