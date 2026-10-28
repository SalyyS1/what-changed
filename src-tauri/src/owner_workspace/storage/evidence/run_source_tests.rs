//! Cached responses preserve their observation source across every lifecycle operation.
#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

#[path = "run_source_test_fixture.rs"]
mod fixture;

fn assert_source_conflict<T: std::fmt::Debug>(result: AppResult<T>) {
    assert!(matches!(
        result,
        Err(crate::error::AppError::InvalidInput {
            what: "operation_id",
            ..
        })
    ));
}

fn exercise_source_cache(source: VerificationMethod) {
    let (store, input) = fixture::create();
    let mut approved = store.preview_browser_manifest(&input).unwrap();
    approved.semantics_version = crate::preview_runner::SEMANTICS.into();
    let start = |method| match method {
        VerificationMethod::Automated => store.start_browser_run(input.clone(), approved.clone()),
        _ => store.start_manual_run(input.clone()),
    };
    let other = if source == VerificationMethod::Automated {
        VerificationMethod::OwnerManual
    } else {
        VerificationMethod::Automated
    };
    let run = start(source).unwrap();
    assert_eq!(run.method, source);
    assert_eq!(start(source).unwrap(), run);
    assert_source_conflict(start(other));

    let scenario = &run.manifest.scenarios[0];
    let step = &scenario.recipe.steps[0];
    let result = RecordOwnerManualResultInput {
        operation_id: fixture::id(),
        id: fixture::id(),
        app_id: run.app_id.clone(),
        run_id: run.id.clone(),
        expected_run_version: run.version,
        scenario_id: scenario.id.clone(),
        scenario_revision: scenario.revision,
        observer: "Synthetic fixture observer".into(),
        observed_at: crate::domain::now_ms(),
        observed_steps: vec![ManualStepObservationDto {
            step_id: step.id.clone(),
            observed: "Loaded the named fixture surface".into(),
        }],
        assertions: vec![ManualAssertionObservationInput {
            assertion_id: step.assertions[0].id.clone(),
            status: ResultStatus::Passed,
            observed: "Named fixture surface is visible".into(),
            limitation: None,
        }],
        limitation: None,
    };
    let record = |method| match method {
        VerificationMethod::Automated => store.record_browser_result(result.clone()),
        _ => store.record_manual_result(result.clone()),
    };
    let observation = record(source).unwrap();
    assert_eq!(observation.method, source);
    assert_eq!(record(source).unwrap(), observation);
    assert_source_conflict(record(other));

    let finish = FinishOwnerManualRunInput {
        operation_id: fixture::id(),
        app_id: run.app_id.clone(),
        run_id: run.id.clone(),
        expected_run_version: run.version,
        lifecycle: RunLifecycle::Completed,
    };
    let finish = |method| match method {
        VerificationMethod::Automated => store.finish_browser_run(finish.clone(), None),
        _ => store.finish_manual_run(finish.clone()),
    };
    let completed = finish(source).unwrap();
    assert_eq!(completed.method, source);
    assert_eq!(finish(source).unwrap(), completed);
    assert_source_conflict(finish(other));
    let review = store.get_manual_run(&run.app_id, &run.id).unwrap();
    assert_eq!(review.run, completed);
    assert_eq!(review.scenarios.len(), 1);
    assert_eq!(review.scenarios[0].result, Some(observation));
    store.shutdown().unwrap();
}

#[test]
fn cached_manual_responses_cannot_satisfy_browser_operations() {
    exercise_source_cache(VerificationMethod::OwnerManual);
}

#[test]
fn cached_browser_responses_cannot_satisfy_manual_operations() {
    exercise_source_cache(VerificationMethod::Automated);
}
