//! Bounded manual run lifecycle and immutable, app-scoped observation commits.

use super::*;

impl OwnerStore {
    /// A new native process never resumes an abandoned mutating browser attempt.
    pub(crate) fn reconcile_browser_runs(&self) -> AppResult<()> {
        self.run(|conn|{
            let tx=conn.transaction().during("recover interrupted browser attempts")?;
            let runs={let mut query=tx.prepare("SELECT payload FROM owner_runs WHERE json_extract(payload,'$.method')='automated' AND json_extract(payload,'$.lifecycle') IN ('running','queued')").during("read interrupted browser attempts")?;
                let rows=query.query_map([],|row|row.get::<_,String>(0)).during("read interrupted browser attempts")?.collect::<rusqlite::Result<Vec<_>>>().during("decode interrupted browser attempts")?;rows};
            for raw in runs{let mut run:OwnerManualRunDto=crate::storage::from_json("interrupted browser run",&raw)?;
                run.lifecycle=RunLifecycle::Interrupted;run.version=run.version.saturating_add(1);run.ended_at=Some(crate::domain::now_ms());run.execution=None;
                tx.execute("UPDATE owner_runs SET payload=?2 WHERE id=?1",params![run.id,to_json(&run)?]).during("mark browser attempt interrupted")?;
            }tx.commit().during("commit browser recovery")
        })
    }
    pub fn start_manual_run(
        &self,
        input: StartOwnerManualRunInput,
    ) -> AppResult<OwnerManualRunDto> {
        self.start_observation_run(input, VerificationMethod::OwnerManual, None)
    }

    pub(crate) fn start_browser_run(
        &self,
        input: StartOwnerManualRunInput,
        approved: OwnerManualManifestDto,
    ) -> AppResult<OwnerManualRunDto> {
        self.start_observation_run(input, VerificationMethod::Automated, Some(approved))
    }

    fn start_observation_run(
        &self,
        input: StartOwnerManualRunInput,
        method: VerificationMethod,
        approved: Option<OwnerManualManifestDto>,
    ) -> AppResult<OwnerManualRunDto> {
        validate_run_ids(&input.app_id, &input.id)?;
        validation::uuid(&input.change_id, "change id")?;
        validation::uuid(&input.candidate_id, "candidate id")?;
        if let Some(id) = &input.retry_of {
            validation::uuid(id, "retry run id")?;
        }
        if input.scenarios.is_empty() || input.scenarios.len() > MAX_SCENARIOS_PER_APP {
            return Err(validation::invalid(
                "manual scenarios",
                "select between 1 and 100 confirmed scenarios",
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for reference in &input.scenarios {
            validation::uuid(&reference.scenario_id, "scenario id")?;
            if reference.revision == 0 || !ids.insert(&reference.scenario_id) {
                return Err(validation::invalid(
                    "manual scenarios",
                    "each scenario must appear once with a positive confirmed revision",
                ));
            }
        }
        self.mutate(input.operation_id.clone(),if method==VerificationMethod::Automated{"start_browser_run"}else{"start_manual_run"},input,move|tx,clean| {
            let input=clean.get();require_app(tx,&input.app_id)?;
            if row::<OwnerManualRunDto>(tx,"SELECT payload FROM owner_runs WHERE id=?1",[&input.id])?.is_some() {return Err(validation::invalid("run id","already exists"));}
            capacity(tx,"SELECT count(*) FROM owner_runs WHERE app_id=?1",&input.app_id,1000,"manual runs")?;
            let mut manifest=manifest::retry_manifest(tx,input)?;
            if method==VerificationMethod::Automated { manifest.semantics_version=crate::preview_runner::SEMANTICS.into(); }
            if approved.as_ref().is_some_and(|approved|approved!=&manifest){return Err(validation::invalid("browser consent","the frozen inputs changed before run creation; review a new scope"));}
            let manifest_digest=digest(to_json(&manifest)?.as_bytes());
            let result=clean_record(OwnerManualRunDto {id:input.id.clone(),app_id:input.app_id.clone(),change_id:input.change_id.clone(),version:1,
                method,lifecycle:RunLifecycle::Running,retry_of:input.retry_of.clone(),manifest,manifest_digest,started_at:crate::domain::now_ms(),ended_at:None,execution:None})?;
            tx.execute("INSERT INTO owner_runs(id,app_id,candidate_id,environment_id,environment_revision,payload) VALUES(?1,?2,?3,?4,?5,?6)",
                params![result.id,result.app_id,result.manifest.candidate.id,result.manifest.environment.id,result.manifest.environment.revision,to_json(&result)?]).during("start frozen manual run")?;
            Ok(result)
        })
    }

    pub fn record_manual_result(
        &self,
        input: RecordOwnerManualResultInput,
    ) -> AppResult<OwnerManualResultDto> {
        self.record_observation_result(input, VerificationMethod::OwnerManual)
    }

    pub(crate) fn record_browser_result(
        &self,
        input: RecordOwnerManualResultInput,
    ) -> AppResult<OwnerManualResultDto> {
        self.record_observation_result(input, VerificationMethod::Automated)
    }

    fn record_observation_result(
        &self,
        input: RecordOwnerManualResultInput,
        method: VerificationMethod,
    ) -> AppResult<OwnerManualResultDto> {
        validate_run_ids(&input.app_id, &input.run_id)?;
        validation::uuid(&input.id, "result id")?;
        validation::uuid(&input.scenario_id, "scenario id")?;
        validation::required(&input.observer, "manual observer")?;
        if input.observed_at < 0
            || input.observed_at > crate::domain::now_ms().saturating_add(5 * 60 * 1000)
        {
            return Err(validation::invalid(
                "observed_at",
                "must be an epoch time at or before the current observation",
            ));
        }
        if input.observed_steps.len() > MAX_STEPS_PER_SCENARIO
            || input.assertions.len() > MAX_ASSERTIONS_PER_SCENARIO
        {
            return Err(validation::invalid(
                "manual observations",
                "at most 50 steps and 1000 assertions can be recorded",
            ));
        }
        self.mutate(input.operation_id.clone(),if method==VerificationMethod::Automated{"record_browser_result"}else{"record_manual_result"},input,move|tx,clean| {
            let input=clean.get();require_app(tx,&input.app_id)?;
            let run=get_run(tx,&input.app_id,&input.run_id)?;
            if run.method!=method {return Err(validation::invalid("observation method","only the owning observation source can append results"));}
            if run.version!=input.expected_run_version || run.lifecycle!=RunLifecycle::Running {return Err(validation::invalid("run version","conflict or run has already finished"));}
            let scenario=run.manifest.scenarios.iter().find(|s|s.id==input.scenario_id && s.revision==input.scenario_revision)
                .ok_or_else(||validation::invalid("scenario","is not part of this run's frozen manifest"))?;
            manifest::validate_scenario(scenario, &run.manifest.environment)?;
            if row::<OwnerManualResultDto>(tx,"SELECT payload FROM owner_results WHERE id=?1 OR (run_id=?2 AND scenario_id=?3 AND scenario_revision=?4)",params![input.id,run.id,scenario.id,scenario.revision])?.is_some() {
                return Err(validation::invalid("manual result","already exists; retry in a separate run to preserve this observation"));
            }
            let mut result=crate::owner_workspace::evidence::derive_result(input,scenario)?;
            result.method=method;
            let result=clean_record(result)?;
            tx.execute("INSERT INTO owner_results(id,app_id,run_id,scenario_id,scenario_revision,payload) VALUES(?1,?2,?3,?4,?5,?6)",params![result.id,result.app_id,result.run_id,result.scenario_id,result.scenario_revision,to_json(&result)?]).during("commit immutable manual result")?;
            Ok(result)
        })
    }

    pub fn finish_manual_run(
        &self,
        input: FinishOwnerManualRunInput,
    ) -> AppResult<OwnerManualRunDto> {
        self.finish_observation_run(input, VerificationMethod::OwnerManual, None)
    }

    pub(crate) fn finish_browser_run(
        &self,
        input: FinishOwnerManualRunInput,
        execution: Option<OwnerExecutionSummaryDto>,
    ) -> AppResult<OwnerManualRunDto> {
        self.finish_observation_run(input, VerificationMethod::Automated, execution)
    }

    fn finish_observation_run(
        &self,
        input: FinishOwnerManualRunInput,
        method: VerificationMethod,
        execution: Option<OwnerExecutionSummaryDto>,
    ) -> AppResult<OwnerManualRunDto> {
        validate_run_ids(&input.app_id, &input.run_id)?;
        if !matches!(
            input.lifecycle,
            RunLifecycle::Completed | RunLifecycle::Cancelled | RunLifecycle::Interrupted
        ) {
            return Err(validation::invalid(
                "run lifecycle",
                "finish with completed, cancelled or interrupted",
            ));
        }
        self.mutate(
            input.operation_id.clone(),
            if method == VerificationMethod::Automated {
                "finish_browser_run"
            } else {
                "finish_manual_run"
            },
            input,
            move |tx, clean| {
                let input = clean.get();
                require_app(tx, &input.app_id)?;
                let mut run = get_run(tx, &input.app_id, &input.run_id)?;
                if run.method != method {
                    return Err(validation::invalid(
                        "run method",
                        "only its owning supervisor may finish this run",
                    ));
                }
                if run.lifecycle != RunLifecycle::Running {
                    return Err(validation::invalid(
                        "run lifecycle",
                        "already finished; completed observations are unchanged",
                    ));
                }
                run.version = expected(Some(run.version), input.expected_run_version)?;
                run.lifecycle = input.lifecycle;
                run.ended_at = Some(crate::domain::now_ms());
                run.execution = execution;
                tx.execute(
                    "UPDATE owner_runs SET payload=?2 WHERE id=?1",
                    params![run.id, to_json(&run)?],
                )
                .during("finish manual run")?;
                Ok(run)
            },
        )
    }
}
