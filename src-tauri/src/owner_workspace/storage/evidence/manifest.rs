//! Freeze live inputs once; observation commits can never retarget this manifest.

use super::*;
use std::collections::BTreeMap;

/// Shared by freeze, retry, observation admission and historical projections.
/// A manual observation cannot stand in for a missing method or another target.
pub(super) fn validate_scenario(
    scenario: &ScenarioRevisionDto,
    environment: &OwnerEnvironmentRevisionDto,
) -> AppResult<()> {
    validation::recipe(&scenario.recipe)?;
    if scenario
        .recipe
        .adapters
        .iter()
        .any(|adapter| !environment.origins.contains(&adapter.origin))
    {
        return Err(validation::invalid(
            "scenario adapter origin",
            "every route must match an exact origin in the run's frozen environment",
        ));
    }
    Ok(())
}

pub(super) fn freeze(
    conn: &Connection,
    input: &StartOwnerManualRunInput,
) -> AppResult<OwnerManualManifestDto> {
    let app: OwnerAppDto = row(
        conn,
        "SELECT payload FROM owner_apps WHERE id=?1",
        [&input.app_id],
    )?
    .ok_or_else(|| validation::invalid("app", "does not exist"))?;
    let change: OwnerChangeDraftDto = row(
        conn,
        "SELECT payload FROM owner_change_drafts WHERE app_id=?1 AND id=?2",
        params![input.app_id, input.change_id],
    )?
    .ok_or_else(|| validation::invalid("change", "does not belong to this app"))?;
    if change.version != input.expected_change_version {
        return Err(validation::invalid(
            "expected_change_version",
            "conflict: reload the saved change",
        ));
    }
    if change.candidate_id.as_ref() != Some(&input.candidate_id)
        || app.current_candidate_id.as_ref() != Some(&input.candidate_id)
    {
        return Err(validation::invalid(
            "candidate",
            "must match the change and the app's selected candidate",
        ));
    }
    let candidate: OwnerCandidateRevisionDto = row(
        conn,
        "SELECT payload FROM owner_candidates WHERE app_id=?1 AND id=?2",
        params![input.app_id, input.candidate_id],
    )?
    .ok_or_else(|| validation::invalid("candidate", "does not belong to this app"))?;
    let environment:OwnerEnvironmentRevisionDto=row(conn,"SELECT payload FROM owner_environment_revisions WHERE app_id=?1 AND id=?2 ORDER BY revision DESC LIMIT 1",params![input.app_id,candidate.environment_id])?
        .ok_or_else(||validation::invalid("environment","does not belong to this app"))?;
    if environment.revoked || environment.revision != candidate.environment_revision {
        return Err(validation::invalid(
            "environment",
            "configuration changed or was revoked; select a current candidate",
        ));
    }
    let mut scenarios = Vec::new();
    let mut rules = BTreeMap::new();
    for reference in &change.preserved_rules {
        rules.insert(
            (reference.rule_id.clone(), reference.revision),
            reference.clone(),
        );
    }
    for reference in &input.scenarios {
        let scenario:ScenarioRevisionDto=row(conn,"SELECT payload FROM owner_scenario_revisions WHERE app_id=?1 AND id=?2 ORDER BY revision DESC LIMIT 1",params![input.app_id,reference.scenario_id])?
            .ok_or_else(||validation::invalid("scenario","does not belong to this app"))?;
        if scenario.revision != reference.revision {
            return Err(validation::invalid(
                "scenario revision",
                "is no longer the current confirmed revision",
            ));
        }
        validate_scenario(&scenario, &environment)?;
        let archived: Option<bool> = conn
            .query_row(
                "SELECT archived FROM owner_capabilities WHERE app_id=?1 AND id=?2",
                params![input.app_id, scenario.capability_id],
                |r| r.get(0),
            )
            .optional()
            .during("check manual scenario capability")?;
        if archived != Some(false) {
            return Err(validation::invalid(
                "scenario capability",
                "is missing or archived",
            ));
        }
        for reference in &scenario.rules {
            rules.insert(
                (reference.rule_id.clone(), reference.revision),
                reference.clone(),
            );
        }
        scenarios.push(scenario);
    }
    scenarios.sort_by(|a, b| a.id.cmp(&b.id).then(a.revision.cmp(&b.revision)));
    let refs = rules.into_values().collect::<Vec<_>>();
    let rules = super::super::rules::rule_refs(conn, &input.app_id, &refs)?;
    for rule in &rules {
        let revision: u32 = conn
            .query_row(
                "SELECT MAX(revision) FROM owner_rule_revisions WHERE app_id=?1 AND rule_id=?2",
                params![input.app_id, rule.rule_id],
                |r| r.get(0),
            )
            .during("check frozen manual rule")?;
        if revision != rule.revision {
            return Err(validation::invalid(
                "rule revision",
                "a preserved rule changed; update the change/scenario before starting",
            ));
        }
    }
    let fixtures = to_json(
        &scenarios
            .iter()
            .map(|scenario| {
                (
                    &scenario.id,
                    scenario.revision,
                    &scenario.recipe.fixture_refs,
                    &scenario.recipe.adapters,
                )
            })
            .collect::<Vec<_>>(),
    )?;
    clean_record(OwnerManualManifestDto {
        schema_version: 1,
        change,
        candidate,
        environment,
        rules,
        scenarios,
        fixture_schema_digest: digest(fixtures.as_bytes()),
        semantics_version: crate::owner_workspace::evidence::MANUAL_SEMANTICS_VERSION.into(),
    })
}

pub(super) fn retry_manifest(
    conn: &Connection,
    input: &StartOwnerManualRunInput,
) -> AppResult<OwnerManualManifestDto> {
    let Some(prior_id) = &input.retry_of else {
        return freeze(conn, input);
    };
    let prior = get_run(conn, &input.app_id, prior_id)?;
    if prior.change_id != input.change_id
        || prior.manifest.change.version != input.expected_change_version
        || prior.manifest.candidate.id != input.candidate_id
    {
        return Err(validation::invalid(
            "retry",
            "must reference the same frozen change version and candidate",
        ));
    }
    if matches!(
        prior.lifecycle,
        RunLifecycle::Running | RunLifecycle::Queued
    ) {
        return Err(validation::invalid(
            "retry",
            "finish or cancel the original attempt first",
        ));
    }
    let mut selected = input
        .scenarios
        .iter()
        .map(|r| (&r.scenario_id, r.revision))
        .collect::<Vec<_>>();
    selected.sort();
    let mut frozen = prior
        .manifest
        .scenarios
        .iter()
        .map(|s| (&s.id, s.revision))
        .collect::<Vec<_>>();
    frozen.sort();
    if selected != frozen {
        return Err(validation::invalid(
            "retry scenarios",
            "must match the original frozen manifest",
        ));
    }
    let (freshness, _) = read::freshness(conn, &prior)?;
    if freshness == EvidenceFreshness::Stale {
        return Err(validation::invalid(
            "retry",
            "inputs changed; start a new run instead of reusing old expectations",
        ));
    }
    for scenario in &prior.manifest.scenarios {
        validate_scenario(scenario, &prior.manifest.environment)?;
    }
    Ok(prior.manifest)
}
