//! Freshness and summaries are recomputed in the same read/decision transaction.

use super::*;

pub(super) fn freshness(
    conn: &Connection,
    run: &OwnerManualRunDto,
) -> AppResult<(EvidenceFreshness, Vec<String>)> {
    if digest(to_json(&run.manifest)?.as_bytes()) != run.manifest_digest {
        return Err(validation::invalid(
            "run manifest",
            "digest mismatch; preserve this history for recovery",
        ));
    }
    let mut reasons = Vec::new();
    let app: Option<OwnerAppDto> = row(
        conn,
        "SELECT payload FROM owner_apps WHERE id=?1",
        [&run.app_id],
    )?;
    if !app.as_ref().is_some_and(|app| {
        app.archived_at.is_none()
            && app.current_candidate_id.as_ref() == Some(&run.manifest.candidate.id)
    }) {
        reasons.push("The app was archived or its selected candidate changed.".into());
    }
    let change: Option<OwnerChangeDraftDto> = row(
        conn,
        "SELECT payload FROM owner_change_drafts WHERE app_id=?1 AND id=?2",
        params![run.app_id, run.change_id],
    )?;
    if change.as_ref() != Some(&run.manifest.change) {
        reasons.push("The saved change request changed after this run was frozen.".into());
    }
    let candidate: Option<OwnerCandidateRevisionDto> = row(
        conn,
        "SELECT payload FROM owner_candidates WHERE app_id=?1 AND id=?2",
        params![run.app_id, run.manifest.candidate.id],
    )?;
    if candidate.as_ref() != Some(&run.manifest.candidate) {
        reasons.push("The candidate identity no longer matches the frozen input.".into());
    }
    let environment:Option<OwnerEnvironmentRevisionDto>=row(conn,"SELECT payload FROM owner_environment_revisions WHERE app_id=?1 AND id=?2 ORDER BY revision DESC LIMIT 1",params![run.app_id,run.manifest.environment.id])?;
    if environment.as_ref() != Some(&run.manifest.environment)
        || environment.is_some_and(|e| e.revoked)
    {
        reasons.push("The environment configuration changed or was revoked.".into());
    }
    for rule in &run.manifest.rules {
        let current:Option<OwnerRuleRevisionDto>=row(conn,"SELECT payload FROM owner_rule_revisions WHERE app_id=?1 AND rule_id=?2 ORDER BY revision DESC LIMIT 1",params![run.app_id,rule.rule_id])?;
        if current.as_ref() != Some(rule) {
            reasons.push("A confirmed rule changed after this run was frozen.".into());
            break;
        }
    }
    for scenario in &run.manifest.scenarios {
        let current:Option<ScenarioRevisionDto>=row(conn,"SELECT payload FROM owner_scenario_revisions WHERE app_id=?1 AND id=?2 ORDER BY revision DESC LIMIT 1",params![run.app_id,scenario.id])?;
        let active: Option<bool> = conn
            .query_row(
                "SELECT archived=0 FROM owner_capabilities WHERE app_id=?1 AND id=?2",
                params![run.app_id, scenario.capability_id],
                |r| r.get(0),
            )
            .optional()
            .during("check evidence capability freshness")?;
        if current.as_ref() != Some(scenario) || active != Some(true) {
            reasons.push("A scenario, assertion, locator, fixture or capability changed.".into());
            break;
        }
    }
    let semantics = if run.method == VerificationMethod::Automated {
        crate::preview_runner::SEMANTICS
    } else {
        crate::owner_workspace::evidence::MANUAL_SEMANTICS_VERSION
    };
    if run.manifest.schema_version != 1 || run.manifest.semantics_version != semantics {
        reasons.push("Evidence schema or manual verification semantics changed.".into());
    }
    if !reasons.is_empty() {
        return Ok((EvidenceFreshness::Stale, reasons));
    }
    if run.method == VerificationMethod::Automated && run.lifecycle == RunLifecycle::Completed {
        if run.execution.as_ref().is_some_and(|execution| {
            execution.runner_semantics == semantics
                && execution.identity == "environment_reported"
                && execution.revision.as_ref() == Some(&run.manifest.candidate.identity)
        }) {
            return Ok((EvidenceFreshness::Current, Vec::new()));
        }
        return Ok((EvidenceFreshness::Unknown, vec!["The test preview did not report the same matching revision before and after this attempt.".into()]));
    }
    // A configured label, source commit or target-reported configuration does
    // not establish before/after deployed identity for a manual observation.
    Ok((
        EvidenceFreshness::Unknown,
        vec!["Manual observations do not establish the current deployed build identity.".into()],
    ))
}

pub(super) fn review(
    conn: &Connection,
    app_id: &str,
    run_id: &str,
) -> AppResult<OwnerEvidenceReviewDto> {
    let run = get_run(conn, app_id, run_id)?;
    let (freshness, freshness_reasons) = freshness(conn, &run)?;
    let mut statement=conn.prepare("SELECT payload FROM owner_results WHERE app_id=?1 AND run_id=?2 ORDER BY scenario_id,scenario_revision LIMIT 101").during("read manual results")?;
    let texts = statement
        .query_map(params![app_id, run_id], |r| r.get::<_, String>(0))
        .during("read manual results")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("decode manual results")?;
    if texts.len() > MAX_SCENARIOS_PER_APP {
        return Err(validation::invalid(
            "manual results",
            "stored result count exceeds supported limit",
        ));
    }
    let results = texts
        .iter()
        .map(|text| crate::storage::from_json::<OwnerManualResultDto>("manual result", text))
        .collect::<AppResult<Vec<_>>>()?;
    let mut scenarios = Vec::new();
    let mut unresolved_scenarios = Vec::new();
    let mut unsupported_recipe = false;
    let mut counts = OwnerEvidenceCountsDto {
        passed: 0,
        failed: 0,
        blocked: 0,
        not_run: 0,
    };
    for scenario in &run.manifest.scenarios {
        let result = results
            .iter()
            .find(|r| r.scenario_id == scenario.id && r.scenario_revision == scenario.revision)
            .cloned();
        let observed_status = result.as_ref().map_or(ResultStatus::NotRun, |r| r.status);
        let status = if manifest::validate_scenario(scenario, &run.manifest.environment).is_err() {
            unsupported_recipe = true;
            // Raw historical observations remain intact, including failures.
            // Only the authoritative projection rejects unsupported claims.
            crate::owner_workspace::evidence::summarize(
                [observed_status, ResultStatus::Blocked].into_iter(),
            )
        } else if run.method == VerificationMethod::Automated
            && run.lifecycle != RunLifecycle::Completed
        {
            // Preserve partial raw observations without presenting an unacknowledged
            // child transcript as a completed pass.
            crate::owner_workspace::evidence::summarize(
                [observed_status, ResultStatus::Blocked].into_iter(),
            )
        } else {
            observed_status
        };
        match status {
            ResultStatus::Passed => counts.passed += 1,
            ResultStatus::Failed => counts.failed += 1,
            ResultStatus::Blocked => counts.blocked += 1,
            ResultStatus::NotRun => counts.not_run += 1,
        };
        if status != ResultStatus::Passed {
            unresolved_scenarios.push(ScenarioRefDto {
                scenario_id: scenario.id.clone(),
                revision: scenario.revision,
            });
        }
        scenarios.push(OwnerScenarioEvidenceDto {
            scenario: scenario.clone(),
            status,
            result,
        });
    }
    let mut statement=conn.prepare("SELECT payload FROM owner_decisions WHERE app_id=?1 AND json_extract(payload,'$.run_id')=?2 ORDER BY id LIMIT 101").during("read owner evidence decisions")?;
    let texts = statement
        .query_map(params![app_id, run_id], |r| r.get::<_, String>(0))
        .during("read owner evidence decisions")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("decode owner evidence decisions")?;
    if texts.len() > 100 {
        return Err(validation::invalid(
            "owner decisions",
            "stored per-run decision count exceeds supported limit",
        ));
    }
    let mut decisions = texts
        .iter()
        .map(|text| {
            crate::storage::from_json::<OwnerEvidenceDecisionDto>("owner evidence decision", text)
        })
        .collect::<AppResult<Vec<_>>>()?;
    for decision in &mut decisions {
        decision.current_freshness = freshness;
    }
    let mut decision_blockers = Vec::new();
    if run.lifecycle != RunLifecycle::Completed {
        decision_blockers.push(
            "Finish this run before recording acceptance; saved observations remain unchanged."
                .into(),
        );
    }
    if freshness == EvidenceFreshness::Stale {
        decision_blockers.push(
            "Frozen inputs are stale. Start a new run for the current change and candidate.".into(),
        );
    }
    if freshness == EvidenceFreshness::Unknown {
        decision_blockers
            .push("Build currency is unknown; review an explicit exception decision.".into());
    }
    let mut limitations = vec![
        if run.method == VerificationMethod::Automated { "Browser observations cover only the granted test routes and roles; build identity is reported by that environment.".into() } else { "Every observation is owner_manual, not automated verification.".into() },
        "Only the selected frozen scenarios are covered; no universal correctness, security or persistence claim is made.".into(),
    ];
    if unsupported_recipe {
        let limitation = "A frozen recipe lacks a supported method, declared route/role, matching environment origin or supported assertion budget. Raw historical observations are preserved but cannot establish that scenario; review its blocked gap explicitly.".to_owned();
        limitations.push(limitation.clone());
        decision_blockers.push(limitation);
    }
    Ok(OwnerEvidenceReviewDto {
        status: crate::owner_workspace::evidence::summarize(scenarios.iter().map(|s| s.status)),
        can_accept: run.lifecycle == RunLifecycle::Completed
            && freshness == EvidenceFreshness::Current
            && !unsupported_recipe
            && !scenarios.is_empty()
            && scenarios
                .iter()
                .all(|scenario| scenario.status == ResultStatus::Passed),
        can_accept_with_exceptions: run.lifecycle == RunLifecycle::Completed
            && freshness != EvidenceFreshness::Stale,
        run,
        freshness,
        freshness_reasons,
        counts,
        scenarios,
        decisions,
        unresolved_scenarios,
        decision_blockers,
        limitations,
    })
}

impl OwnerStore {
    pub fn get_manual_run(&self, app_id: &str, run_id: &str) -> AppResult<OwnerEvidenceReviewDto> {
        validate_run_ids(app_id, run_id)?;
        self.read(|conn| {
            let tx = conn
                .unchecked_transaction()
                .during("begin manual evidence review")?;
            review(&tx, app_id, run_id)
        })
    }

    /// Export callers use this same projection rather than trusting stored status.
    pub fn export_manual_run_review(
        &self,
        app_id: &str,
        run_id: &str,
    ) -> AppResult<OwnerEvidenceReviewDto> {
        self.get_manual_run(app_id, run_id)
    }

    pub fn list_manual_runs(
        &self,
        app_id: &str,
        change_id: &str,
        page: OwnerPageInput,
    ) -> AppResult<OwnerPageDto<OwnerEvidenceReviewDto>> {
        validation::uuid(app_id, "app id")?;
        validation::uuid(change_id, "change id")?;
        validation::page(&page)?;
        self.read(|conn| {
            let tx=conn.unchecked_transaction().during("begin manual run list snapshot")?;
            let mut statement=tx.prepare("SELECT id FROM owner_runs WHERE app_id=?1 AND json_extract(payload,'$.change_id')=?2 AND id>?3 ORDER BY id LIMIT ?4").during("list manual runs")?;
            let ids=statement.query_map(params![app_id,change_id,page.after.unwrap_or_default(),page.limit+1],|r|r.get::<_,String>(0)).during("list manual runs")?.collect::<rusqlite::Result<Vec<_>>>().during("decode manual run list")?;
            let more=ids.len()>page.limit as usize;let visible=ids.into_iter().take(page.limit as usize).collect::<Vec<_>>();
            let next_cursor=if more{visible.last().cloned()}else{None};
            let items=visible.iter().map(|id|review(&tx,app_id,id)).collect::<AppResult<Vec<_>>>()?;
            Ok(OwnerPageDto{items,next_cursor})
        })
    }
}
