//! Stateful collector snapshot and acknowledgement contracts.
//!
//! Collectors describe raw facts and stage candidate in-memory state. The
//! runtime alone normalizes, admits and acknowledges the durable transaction;
//! no database or writer capability crosses this boundary.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use crate::domain::RelationshipKind;
use crate::error::{AppError, AppResult};
use crate::storage::{
    baseline_repository, config_snapshot_repository, project_repository,
    watched_location_repository, Baseline, ConfigSnapshot, Db,
};

use super::{RawEntity, RawObservation};

/// Opaque identity for one serial collector snapshot cycle.
///
/// Values can only be minted inside the crate's runtime. Collectors receive a
/// token through [`SnapshotContext`] and must echo it in an atomic snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommitToken(u64);

impl CommitToken {
    pub(crate) const fn mint(value: u64) -> Self {
        Self(value)
    }
}

/// Runtime-owned context for one call to `Collector::snapshot_batch`.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotContext {
    token: CommitToken,
}

impl SnapshotContext {
    pub(crate) const fn new(token: CommitToken) -> Self {
        Self { token }
    }

    /// Construct a context for development-only end-to-end performance gates.
    #[cfg(any(debug_assertions, feature = "devtools"))]
    #[doc(hidden)]
    pub const fn for_devtools(value: u64) -> Self {
        Self {
            token: CommitToken::mint(value),
        }
    }

    pub const fn token(&self) -> CommitToken {
        self.token
    }
}

/// One raw graph edge and the observation that proves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRelationshipSpec {
    pub evidence_observation: usize,
    pub source: RawEntity,
    pub target: RawEntity,
    pub kind: RelationshipKind,
}

/// A candidate durable baseline. The runtime injects the collector id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBaseline {
    pub entity_kind: String,
    pub snapshot: Value,
    pub captured_at: i64,
}

/// Sanitized per-file config state to advance with its event rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawConfigSnapshotMutation {
    Upsert {
        path: String,
        keys: BTreeMap<String, String>,
        captured_at: i64,
        format: String,
    },
    Delete {
        path: String,
    },
}

/// Output from one provider snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectorSnapshot {
    /// Compatibility path: enqueue the events through the legacy async writer.
    Legacy(Vec<RawObservation>),
    /// The provider observed nothing and needs no storage operation.
    Noop,
    /// Events, graph evidence and baseline that must commit atomically.
    ///
    /// Once storage admits this batch the runtime never submits this value a
    /// second time, including when `commit_succeeded` panics or returns an
    /// error after durability. A recreated stateful collector must hydrate
    /// its durable baseline through [`StateReadHandle`] before it produces a
    /// new candidate, so provider replay cannot duplicate the acknowledged
    /// observation.
    Atomic {
        token: CommitToken,
        observations: Vec<RawObservation>,
        relationships: Vec<RawRelationshipSpec>,
        baseline: Option<RawBaseline>,
        config_snapshots: Vec<RawConfigSnapshotMutation>,
    },
}

/// Read-only durable state bound to the collector that received the context.
///
/// There is deliberately no collector-id parameter, generic query closure,
/// connection, writer or commit operation on this handle.
#[derive(Clone, Debug)]
pub struct StateReadHandle {
    db: Arc<Db>,
    collector_id: &'static str,
}

impl StateReadHandle {
    pub(crate) fn new(db: Arc<Db>, collector_id: &'static str) -> Self {
        Self { db, collector_id }
    }

    pub fn baseline(&self, entity_kind: &str) -> AppResult<Option<Baseline>> {
        self.db.read(|connection| {
            baseline_repository::load(connection, self.collector_id, entity_kind)
        })
    }

    /// Watched roots are visible to the files collector and to the
    /// developer-environment collector that tracks Git branch changes there.
    pub fn watched_locations(
        &self,
    ) -> AppResult<Vec<watched_location_repository::WatchedLocation>> {
        self.ensure_watched_location_reader()?;
        self.db.read(watched_location_repository::list)
    }

    /// Explicit development projects are independent from watched folders.
    pub fn projects(&self) -> AppResult<Vec<project_repository::Project>> {
        self.ensure_project_reader()?;
        self.db.read(project_repository::list)
    }

    /// Processes whose persisted start evidence proves their executable lived
    /// under a registered project containing this config file.
    pub fn project_processes_for_config_path(
        &self,
        path: &str,
    ) -> AppResult<Vec<project_repository::ProjectProcessEvidence>> {
        self.ensure_files()?;
        let path = path.to_owned();
        self.db.read(move |connection| {
            project_repository::processes_for_config_path(connection, &path)
        })
    }

    /// One persisted, already-sanitized config baseline.
    pub fn config_snapshot(&self, path: &str) -> AppResult<Option<ConfigSnapshot>> {
        self.ensure_files()?;
        let path = path.to_owned();
        self.db
            .read(move |connection| config_snapshot_repository::get(connection, &path))
    }

    /// Persisted config paths under currently safe roots, for sweep deletion
    /// reconciliation after watcher downtime.
    pub fn config_snapshot_paths_under(&self, roots: &[String]) -> AppResult<Vec<String>> {
        self.ensure_files()?;
        let roots = roots.to_vec();
        self.db
            .read(move |connection| config_snapshot_repository::paths_under(connection, &roots))
    }

    fn ensure_files(&self) -> AppResult<()> {
        if self.collector_id == "files" {
            Ok(())
        } else {
            Err(AppError::InvalidInput {
                what: "collector state",
                reason: "config state is available only to the files collector".to_owned(),
            })
        }
    }

    fn ensure_watched_location_reader(&self) -> AppResult<()> {
        if matches!(self.collector_id, "files" | "dev_env") {
            Ok(())
        } else {
            Err(AppError::InvalidInput {
                what: "collector state",
                reason: "watched locations are available only to the files and developer-environment collectors".to_owned(),
            })
        }
    }

    fn ensure_project_reader(&self) -> AppResult<()> {
        if self.collector_id == "dev_env" {
            Ok(())
        } else {
            Err(AppError::InvalidInput {
                what: "collector state",
                reason: "projects are available only to the developer-environment collector"
                    .to_owned(),
            })
        }
    }
}

/// Candidate state that advances only after the matching durable commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckedState<T> {
    committed: T,
    pending: Option<(CommitToken, T)>,
}

impl<T> AckedState<T> {
    pub const fn new(committed: T) -> Self {
        Self {
            committed,
            pending: None,
        }
    }

    pub const fn committed(&self) -> &T {
        &self.committed
    }

    pub fn stage(&mut self, token: CommitToken, next: T) -> AppResult<()> {
        if self.pending.is_some() {
            return Err(state_error("a state update is already pending"));
        }
        self.pending = Some((token, next));
        Ok(())
    }

    pub fn ack(&mut self, token: CommitToken) -> AppResult<()> {
        let Some((pending_token, _)) = self.pending.as_ref() else {
            return Err(state_error("no state update is pending"));
        };
        if *pending_token != token {
            return Err(state_error("the acknowledgement token does not match"));
        }
        let (_, next) = self
            .pending
            .take()
            .ok_or_else(|| state_error("no state update is pending"))?;
        self.committed = next;
        Ok(())
    }

    pub fn reject(&mut self, token: CommitToken) -> AppResult<()> {
        let Some((pending_token, _)) = self.pending.as_ref() else {
            return Err(state_error("no state update is pending"));
        };
        if *pending_token != token {
            return Err(state_error("the rejection token does not match"));
        }
        self.pending = None;
        Ok(())
    }
}

fn state_error(reason: &'static str) -> AppError {
    AppError::InvalidInput {
        what: "collector state",
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn state_promotes_only_after_the_matching_acknowledgement() {
        let first = CommitToken::mint(1);
        let other = CommitToken::mint(2);
        let mut state = AckedState::new("old");

        state.stage(first, "new").unwrap();
        assert_eq!(state.committed(), &"old");
        assert!(state.stage(other, "overlap").is_err());
        assert!(state.ack(other).is_err());
        assert_eq!(state.committed(), &"old");

        state.ack(first).unwrap();
        assert_eq!(state.committed(), &"new");
        assert!(state.ack(first).is_err());
    }

    #[test]
    fn rejection_discards_only_the_matching_candidate() {
        let first = CommitToken::mint(1);
        let other = CommitToken::mint(2);
        let mut state = AckedState::new("old");

        state.stage(first, "candidate").unwrap();
        assert!(state.reject(other).is_err());
        assert_eq!(state.committed(), &"old");
        assert!(state.stage(other, "overlap").is_err());

        state.reject(first).unwrap();
        assert_eq!(state.committed(), &"old");
        state.stage(other, "later").unwrap();
        state.ack(other).unwrap();
        assert_eq!(state.committed(), &"later");
    }
}
