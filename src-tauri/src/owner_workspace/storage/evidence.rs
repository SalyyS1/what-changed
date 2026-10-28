//! Transactional manual evidence; all SQL remains in the owner storage boundary.

mod decision;
mod manifest;
mod read;
mod run;
#[cfg(test)]
#[path = "evidence/run_source_tests.rs"]
mod run_source_tests;

use super::repository::{capacity, expected, require_app, row};
use super::{migrations::digest, OwnerStore};
use crate::dto::owner_evidence_dto::*;
use crate::error::AppResult;
use crate::owner_workspace::{validation, *};
use crate::privacy::{Sanitizer, Scrub};
use crate::storage::{to_json, SqlContext};
use rusqlite::{params, Connection, OptionalExtension};

/// Reuse the caller's read transaction when projecting portable evidence.
/// Decision-time fields remain immutable; current freshness is recomputed.
pub(super) fn review_in_snapshot(
    conn: &Connection,
    app_id: &str,
    run_id: &str,
) -> AppResult<OwnerEvidenceReviewDto> {
    validate_run_ids(app_id, run_id)?;
    read::review(conn, app_id, run_id)
}

fn validate_run_ids(app_id: &str, run_id: &str) -> AppResult<()> {
    validation::uuid(app_id, "app id")?;
    validation::uuid(run_id, "run id")
}

fn get_run(conn: &Connection, app_id: &str, run_id: &str) -> AppResult<OwnerManualRunDto> {
    row(
        conn,
        "SELECT payload FROM owner_runs WHERE app_id=?1 AND id=?2",
        params![app_id, run_id],
    )?
    .ok_or_else(|| validation::invalid("run", "does not belong to this app"))
}

fn clean_record<T: Scrub + serde::Serialize>(record: T) -> AppResult<T> {
    let (clean, _) = Sanitizer::new().sanitize(record);
    validation::payload(clean.get())?;
    Ok(clean.into_inner())
}

impl OwnerStore {
    /// Preparing scope is read-only and never grants browser authority.
    pub(crate) fn preview_browser_manifest(
        &self,
        input: &StartOwnerManualRunInput,
    ) -> AppResult<OwnerManualManifestDto> {
        validate_run_ids(&input.app_id, &input.id)?;
        if input.scenarios.is_empty() || input.scenarios.len() > 10 {
            return Err(validation::invalid(
                "browser scenarios",
                "choose between one and ten confirmed scenarios",
            ));
        }
        self.read(|conn| {
            let tx = conn
                .unchecked_transaction()
                .during("preview frozen browser manifest")?;
            manifest::freeze(&tx, input)
        })
    }
}
