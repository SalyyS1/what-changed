//! Small transaction helpers specific to the owner domain; sanitized inputs only.

use super::{migrations::digest, AdmissionEpoch, OwnerStore};
use crate::error::AppResult;
use crate::owner_workspace::{validation, OwnerPageDto, OwnerPageInput};
use crate::owner_workspace::{
    OwnerAppDto, OwnerCandidateRevisionDto, OwnerChangeDraftDto, OwnerEnvironmentRevisionDto,
    OwnerRuleRevisionDto, ScenarioRefDto, ScenarioRevisionDto,
};

/// All records needed by a brief are read in one SQLite snapshot. The renderer
/// never issues a second query while assembling copyable text.
pub(crate) struct BriefSnapshot {
    pub app: OwnerAppDto,
    pub change: OwnerChangeDraftDto,
    pub rules: Vec<OwnerRuleRevisionDto>,
    pub candidate: Option<OwnerCandidateRevisionDto>,
    pub environment: Option<OwnerEnvironmentRevisionDto>,
    pub scenarios: Vec<ScenarioRevisionDto>,
}

pub(crate) fn brief_snapshot(
    conn: &Connection,
    app_id: &str,
    change_id: &str,
    scenario_ids: &[ScenarioRefDto],
) -> AppResult<BriefSnapshot> {
    let tx = conn
        .unchecked_transaction()
        .during("begin owner brief snapshot")?;
    let app: OwnerAppDto = row(&tx, "SELECT payload FROM owner_apps WHERE id=?1", [app_id])?
        .ok_or_else(|| validation::invalid("app", "does not exist"))?;
    let change: OwnerChangeDraftDto = row(
        &tx,
        "SELECT payload FROM owner_change_drafts WHERE app_id=?1 AND id=?2",
        params![app_id, change_id],
    )?
    .ok_or_else(|| validation::invalid("change", "does not belong to this app"))?;
    let rules = super::rules::rule_refs(&tx, app_id, &change.preserved_rules)?;
    let candidate: Option<OwnerCandidateRevisionDto> = match &change.candidate_id {
        Some(id) => row(
            &tx,
            "SELECT payload FROM owner_candidates WHERE app_id=?1 AND id=?2",
            params![app_id, id],
        )?,
        None => None,
    };
    let environment = match &candidate {
        Some(candidate) => row(
            &tx,
            "SELECT payload FROM owner_environment_revisions WHERE app_id=?1 AND id=?2 AND revision=?3",
            params![app_id, candidate.environment_id, candidate.environment_revision],
        )?,
        None => None,
    };
    let mut scenarios: Vec<ScenarioRevisionDto> = if scenario_ids.is_empty() {
        let mut statement = tx.prepare("SELECT payload FROM owner_scenario_revisions s WHERE app_id=?1 AND revision=(SELECT MAX(revision) FROM owner_scenario_revisions latest WHERE latest.app_id=s.app_id AND latest.id=s.id) ORDER BY id LIMIT 101")
            .during("read owner brief scenarios")?;
        let texts = statement
            .query_map([app_id], |r| r.get::<_, String>(0))
            .during("read owner brief scenarios")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .during("decode owner brief scenarios")?;
        if texts.len() > crate::owner_workspace::MAX_SCENARIOS_PER_APP {
            return Err(validation::invalid(
                "scenarios",
                "stored scenario count exceeds supported limit",
            ));
        }
        texts
            .iter()
            .map(|text| from_json::<ScenarioRevisionDto>("owner scenario", text))
            .collect::<AppResult<Vec<_>>>()?
    } else {
        let mut selected = Vec::with_capacity(scenario_ids.len());
        for reference in scenario_ids {
            let scenario = row(&tx,"SELECT payload FROM owner_scenario_revisions WHERE app_id=?1 AND id=?2 AND revision=?3",
                params![app_id,reference.scenario_id,reference.revision])?
                .ok_or_else(|| validation::invalid("scenario selection", "selected revision is missing or belongs to another app"))?;
            selected.push(scenario);
        }
        selected
    };
    scenarios.sort_by(|a, b| a.id.cmp(&b.id).then(a.revision.cmp(&b.revision)));
    Ok(BriefSnapshot {
        app,
        change,
        rules,
        candidate,
        environment,
        scenarios,
    })
}
use crate::privacy::{Sanitized, Sanitizer, Scrub};
use crate::storage::{from_json, to_json, SqlContext};
use rusqlite::{params, Connection, OptionalExtension, Params, Transaction};
use serde::{de::DeserializeOwned, Serialize};

pub(super) fn row<T: DeserializeOwned>(
    conn: &Connection,
    sql: &str,
    parameters: impl Params,
) -> AppResult<Option<T>> {
    let text: Option<String> = conn
        .query_row(sql, parameters, |r| r.get(0))
        .optional()
        .during("read owner record")?;
    text.map(|text| from_json("owner payload", &text))
        .transpose()
}

pub(super) fn require_app(conn: &Connection, id: &str) -> AppResult<()> {
    let archived: Option<bool> = conn
        .query_row("SELECT archived FROM owner_apps WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()
        .during("check owner app")?;
    match archived {
        Some(false) => Ok(()),
        Some(true) => Err(validation::invalid(
            "app",
            "is archived; restore it before editing",
        )),
        None => Err(validation::invalid("app", "does not exist")),
    }
}

pub(super) fn expected(found: Option<u32>, expected: u32) -> AppResult<u32> {
    if found.unwrap_or(0) != expected || (found.is_none() && expected != 0) {
        return Err(validation::invalid(
            "expected_version",
            "conflict: reload the current record before saving",
        ));
    }
    expected
        .checked_add(1)
        .ok_or_else(|| validation::invalid("expected_version", "version limit reached"))
}

pub(super) fn capacity(
    conn: &Connection,
    sql: &str,
    app_id: &str,
    limit: usize,
    what: &'static str,
) -> AppResult<()> {
    let count: i64 = conn
        .query_row(sql, [app_id], |r| r.get(0))
        .during("check owner capacity")?;
    let count = usize::try_from(count)
        .map_err(|_| validation::invalid(what, "stored record count is invalid"))?;
    if count >= limit {
        return Err(validation::invalid(
            what,
            &format!("at most {limit} records are supported"),
        ));
    }
    Ok(())
}

pub(super) fn page<T: DeserializeOwned>(
    conn: &Connection,
    sql: &str,
    app_id: &str,
    input: OwnerPageInput,
) -> AppResult<OwnerPageDto<T>> {
    validation::page(&input)?;
    let mut statement = conn.prepare(sql).during("prepare owner page")?;
    let records = statement
        .query_map(
            params![app_id, input.after.unwrap_or_default(), input.limit + 1],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .during("read owner page")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("decode owner page")?;
    let has_more = records.len() > input.limit as usize;
    let mut items = Vec::new();
    let mut last = None;
    for (cursor, text) in records.into_iter().take(input.limit as usize) {
        items.push(from_json("owner page", &text)?);
        last = Some(cursor);
    }
    Ok(OwnerPageDto {
        items,
        next_cursor: if has_more { last } else { None },
    })
}

impl OwnerStore {
    pub(super) fn mutate<I, T>(
        &self,
        operation_id: String,
        kind: &'static str,
        input: I,
        action: impl FnOnce(&Transaction<'_>, &Sanitized<I>) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T>
    where
        I: Serialize + Scrub + Send + 'static,
        T: Serialize + DeserializeOwned + Send + 'static,
    {
        let epoch = self.capture_epoch()?;
        self.mutate_at(epoch, operation_id, kind, input, action)
    }

    pub(super) fn mutate_at<I, T>(
        &self,
        epoch: AdmissionEpoch,
        operation_id: String,
        kind: &'static str,
        input: I,
        action: impl FnOnce(&Transaction<'_>, &Sanitized<I>) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T>
    where
        I: Serialize + Scrub + Send + 'static,
        T: Serialize + DeserializeOwned + Send + 'static,
    {
        validation::uuid(&operation_id, "operation_id")?;
        validation::payload(&input)?;
        let (clean, _) = Sanitizer::new().sanitize(input);
        validation::payload(clean.get())?;
        // Never hash the original input: secrets with the same redaction are identical.
        let request_digest = digest(to_json(clean.get())?.as_bytes());
        self.write_at_epoch(epoch, move |tx| {
            let prior: Option<(String, String, String)> = tx.query_row(
                "SELECT kind,request_digest,response FROM owner_operations WHERE operation_id=?1", [&operation_id],
                |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().during("read owner operation")?;
            if let Some((prior_kind, prior_digest, response)) = prior {
                if prior_kind != kind || prior_digest != request_digest {
                    return Err(validation::invalid("operation_id", "was already used for a different request"));
                }
                return from_json("owner operation response", &response);
            }
            let result = action(tx, &clean)?;
            let response = to_json(&result)?;
            if response.len() > 1024 * 1024 { return Err(validation::invalid("owner response", "exceeds 1 MiB")); }
            tx.execute("INSERT INTO owner_operations(operation_id,kind,request_digest,response) VALUES(?1,?2,?3,?4)",
                params![operation_id,kind,request_digest,response]).during("record owner operation")?;
            Ok(result)
        })
    }
}
