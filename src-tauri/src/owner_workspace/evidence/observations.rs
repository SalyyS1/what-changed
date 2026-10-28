//! Derive outcomes from frozen assertion IDs, never from an empty success signal.

use crate::dto::owner_evidence_dto::*;
use crate::dto::owner_workspace_dto::ScenarioRevisionDto;
use crate::error::{AppResult, DbError};
use crate::owner_workspace::{validation, AssertionOutcomeDto, ResultStatus, VerificationMethod};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn summarize(statuses: impl Iterator<Item = ResultStatus>) -> ResultStatus {
    let mut any = false;
    let mut failed = false;
    let mut blocked = false;
    let mut not_run = false;
    for status in statuses {
        any = true;
        match status {
            ResultStatus::Failed => failed = true,
            ResultStatus::Blocked => blocked = true,
            ResultStatus::NotRun => not_run = true,
            ResultStatus::Passed => {}
        }
    }
    if failed {
        ResultStatus::Failed
    } else if blocked {
        ResultStatus::Blocked
    } else if not_run || !any {
        ResultStatus::NotRun
    } else {
        ResultStatus::Passed
    }
}

pub(crate) fn derive_result(
    input: &RecordOwnerManualResultInput,
    scenario: &ScenarioRevisionDto,
) -> AppResult<OwnerManualResultDto> {
    // Older frozen scenarios must satisfy the same method and result budget as
    // newly confirmed recipes before any supplied status can become evidence.
    validation::recipe(&scenario.recipe)?;
    let mut steps = BTreeSet::new();
    for observed in &input.observed_steps {
        validation::uuid(&observed.step_id, "observed step id")?;
        validation::required(&observed.observed, "observed step")?;
        if !scenario
            .recipe
            .steps
            .iter()
            .any(|step| step.id == observed.step_id)
            || !steps.insert(&observed.step_id)
        {
            return Err(validation::invalid(
                "observed step",
                "is duplicated or does not belong to the frozen scenario",
            ));
        }
    }
    let mut supplied = BTreeMap::new();
    for observed in &input.assertions {
        validation::uuid(&observed.assertion_id, "assertion id")?;
        if supplied.insert(&observed.assertion_id, observed).is_some() {
            return Err(validation::invalid(
                "assertion observation",
                "duplicate assertion IDs are not allowed",
            ));
        }
    }
    let mut assertions = Vec::new();
    for step in &scenario.recipe.steps {
        for expected in &step.assertions {
            let supplied = supplied.remove(&expected.id);
            let criterion =
                serde_json::to_string(&expected.assertion).map_err(|_| DbError::Encoding {
                    reason: "could not encode frozen assertion".into(),
                })?;
            let mut outcome = AssertionOutcomeDto {
                assertion_id: expected.id.clone(),
                expected: criterion,
                status: ResultStatus::NotRun,
                observed: String::new(),
                limitation: Some(
                    "No manual observation was recorded for this expected assertion.".into(),
                ),
            };
            if let Some(supplied) = supplied {
                if matches!(supplied.status, ResultStatus::Passed | ResultStatus::Failed) {
                    validation::required(&supplied.observed, "assertion observation")?;
                }
                if supplied.status == ResultStatus::Blocked
                    && supplied.observed.trim().is_empty()
                    && supplied
                        .limitation
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())
                {
                    return Err(validation::invalid(
                        "blocked assertion",
                        "requires an observed limitation",
                    ));
                }
                outcome.status = supplied.status;
                outcome.observed = supplied.observed.clone();
                outcome.limitation = supplied.limitation.clone();
                if supplied.status == ResultStatus::Passed && !steps.contains(&step.id) {
                    outcome.status = ResultStatus::NotRun;
                    outcome.limitation=Some("The action step has no manual observation; this assertion cannot count as passed.".into());
                }
            }
            assertions.push(outcome);
        }
    }
    if !supplied.is_empty() {
        return Err(validation::invalid(
            "assertion observation",
            "references an assertion outside the frozen scenario",
        ));
    }
    let result = OwnerManualResultDto {
        id: input.id.clone(),
        app_id: input.app_id.clone(),
        run_id: input.run_id.clone(),
        scenario_id: input.scenario_id.clone(),
        scenario_revision: input.scenario_revision,
        method: VerificationMethod::OwnerManual,
        status: summarize(assertions.iter().map(|a| a.status)),
        observer: input.observer.clone(),
        observed_at: input.observed_at,
        observed_steps: input.observed_steps.clone(),
        assertions,
        limitation: input.limitation.clone(),
        recorded_at: crate::domain::now_ms(),
    };
    validation::payload(&result)?;
    Ok(result)
}
