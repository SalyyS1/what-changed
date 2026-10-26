//! Debounced watched-folder collector.
//!
//! Notification callbacks do no parsing and touch no database state: they
//! normalize paths into a bounded queue and request one targeted runtime
//! cycle. The serial collector snapshot owns root reconciliation, parsing and
//! the atomic event/baseline commit.

mod observations;
mod queue;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, Debouncer, RecommendedCache};

use crate::collectors::listening_port_map::ListeningPortMap;
use crate::collectors::service_evidence_map::ServiceEvidenceMap;
use crate::collectors::{
    config_diff, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot,
    EntityIdentity, RawConfigSnapshotMutation, RawEntity, RawObservation, RawRelationshipSpec,
    SnapshotContext, StateReadHandle,
};
use crate::domain::{EventType, RelationshipKind};
use crate::error::{AppError, AppResult};
use crate::storage::CollectorCommitReceipt;
use crate::storage::{
    MAX_ATOMIC_OBSERVATIONS, MAX_ATOMIC_RELATED_ENTITIES, MAX_ATOMIC_RELATIONSHIPS,
};

use queue::{ChangeQueue, LogicalChange, RootSpec, SharedRoots};

pub const ID: &str = "files";

/// Compare persisted config keys through the live collector's bounded,
/// redaction-aware contract. Parser format is persistence metadata.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    #[derive(serde::Deserialize)]
    struct ConfigState {
        keys: BTreeMap<String, String>,
    }
    let before: ConfigState = serde_json::from_value(before.clone()).ok()?;
    let after: ConfigState = serde_json::from_value(after.clone()).ok()?;
    Some(
        config_diff::ConfigDiff::between(&before.keys, &after.keys)
            .changes
            .into_iter()
            .map(|change| {
                (
                    change.field,
                    serde_json::json!(change.before),
                    serde_json::json!(change.after),
                )
            })
            .collect(),
    )
}
const DEBOUNCE: Duration = Duration::from_millis(500);
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
const MAX_SWEEP_ENTRIES_PER_PAGE: usize = 4_096;
// One parsed file per atomic commit keeps the worst-case 2 MiB source plus
// JSON/event overhead below the storage transaction's 4 MiB payload bound.
const PATHS_PER_BATCH: usize = 1;
const MAX_SNAPSHOT_JSON_BYTES: usize = 3 * 1024 * 1024;
// Project process evidence is a bounded supplement to the primary config
// event. Leave room for independent live port/service evidence so a large
// project cannot make an otherwise valid config change fail the whole commit.
const MAX_PROJECT_RELATIONSHIPS_PER_EVENT: usize = MAX_ATOMIC_RELATIONSHIPS / 2;

type FileDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;

pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the file collector is not in the canonical catalog".to_owned(),
        })
}

pub struct FileCollector {
    descriptor: CollectorDescriptor,
    port_map: ListeningPortMap,
    service_map: ServiceEvidenceMap,
    context: Option<CollectorContext>,
    debouncer: Option<FileDebouncer>,
    queue: ChangeQueue,
    roots: SharedRoots,
    watched: BTreeSet<PathBuf>,
    last_sweep: Option<Instant>,
    sweep_backlog: VecDeque<LogicalChange>,
    sweep_walker: Option<SweepWalker>,
    pending: Option<(crate::collectors::CommitToken, Vec<LogicalChange>)>,
}

impl FileCollector {
    pub fn new(port_map: ListeningPortMap, descriptor: CollectorDescriptor) -> Self {
        Self::with_service_map(port_map, ServiceEvidenceMap::new(), descriptor)
    }

    pub fn with_service_map(
        port_map: ListeningPortMap,
        service_map: ServiceEvidenceMap,
        descriptor: CollectorDescriptor,
    ) -> Self {
        Self {
            descriptor,
            port_map,
            service_map,
            context: None,
            debouncer: None,
            queue: ChangeQueue::default(),
            roots: Arc::new(RwLock::new(Vec::new())),
            watched: BTreeSet::new(),
            last_sweep: None,
            sweep_backlog: VecDeque::new(),
            sweep_walker: None,
            pending: None,
        }
    }

    fn start_debouncer(&mut self, context: &CollectorContext) -> AppResult<()> {
        let callback = queue::callback(
            self.queue.clone(),
            Arc::clone(&self.roots),
            context.wake.clone(),
        );
        self.debouncer =
            Some(
                new_debouncer(DEBOUNCE, None, callback).map_err(|_| AppError::Platform {
                    operation: "start watched-folder notifications",
                    reason: "the filesystem watcher could not start".to_owned(),
                })?,
            );
        Ok(())
    }

    fn replace_roots(&mut self, roots: Vec<RootSpec>) {
        let mut shared = self
            .roots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *shared = roots;
    }

    fn reconcile_roots(&mut self, state: &StateReadHandle) -> AppResult<bool> {
        let desired: BTreeMap<PathBuf, u8> = state
            .watched_locations()?
            .into_iter()
            .filter(|root| root.enabled)
            .map(|root| (PathBuf::from(root.path), root.max_depth))
            .collect();
        let available: BTreeSet<PathBuf> = desired
            .keys()
            .filter(|path| crate::app::validate::watched_root_still_safe(path))
            .cloned()
            .collect();
        let mut changed = false;
        if let Some(debouncer) = self.debouncer.as_mut() {
            let removed = self
                .watched
                .difference(&available)
                .cloned()
                .collect::<Vec<_>>();
            for path in removed {
                let _ = debouncer.unwatch(&path);
                self.watched.remove(&path);
                changed = true;
            }
            let added = available
                .difference(&self.watched)
                .cloned()
                .collect::<Vec<_>>();
            for path in added {
                if debouncer.watch(&path, RecursiveMode::Recursive).is_ok() {
                    self.watched.insert(path);
                    changed = true;
                }
            }
        }
        self.replace_roots(
            desired
                .into_iter()
                .filter(|(path, _)| self.watched.contains(path))
                .map(|(path, max_depth)| RootSpec { path, max_depth })
                .collect(),
        );
        Ok(changed)
    }

    fn sweep_due(&self) -> bool {
        self.last_sweep
            .is_none_or(|last| last.elapsed() >= SWEEP_INTERVAL)
    }
}

impl Collector for FileCollector {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        self.start_debouncer(&context)?;
        self.queue.request_sweep();
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.debouncer = None;
        self.context = None;
        self.replace_roots(Vec::new());
        self.watched.clear();
        self.last_sweep = None;
        self.sweep_backlog.clear();
        self.sweep_walker = None;
        self.pending = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "file collector",
            reason: "the file collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, snapshot: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(state) = self.context.as_ref().map(|context| context.state.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        if self.queue.take_watcher_reset() {
            self.debouncer = None;
            self.watched.clear();
            let context = self.context.as_ref().cloned();
            if context
                .as_ref()
                .is_none_or(|context| self.start_debouncer(context).is_err())
            {
                self.queue.request_watcher_reset();
                self.replace_roots(Vec::new());
            }
        }
        let roots_changed = self.reconcile_roots(&state)?;
        if roots_changed {
            self.sweep_backlog.clear();
            self.sweep_walker = None;
        }
        let (mut queued, requested_sweep) = self.queue.drain(PATHS_PER_BATCH);
        let sweep = requested_sweep || roots_changed || self.sweep_due();
        if sweep && self.sweep_walker.is_none() {
            self.last_sweep = Some(Instant::now());
            let roots = self
                .roots
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            self.sweep_walker = Some(SweepWalker::new(roots));
            let root_paths = self
                .roots
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .map(|root| root.path.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            for stored in state.config_snapshot_paths_under(&root_paths)? {
                let path = PathBuf::from(stored);
                if !path.exists() && safe_removed_under_roots(&self.roots, &path) {
                    self.sweep_backlog
                        .push_back(LogicalChange::SweptDeleted(path));
                }
            }
        }
        if self.sweep_backlog.is_empty() {
            if let Some(walker) = self.sweep_walker.as_mut() {
                let page = walker.next_page(self.context.as_ref().map(|context| &context.cancel));
                self.sweep_backlog
                    .extend(page.paths.into_iter().map(LogicalChange::Swept));
                if page.finished || page.cancelled {
                    self.sweep_walker = None;
                }
            }
        }
        if queued.is_empty() {
            queued.extend(self.sweep_backlog.pop_front());
        }
        queued.sort_by(|left, right| change_path(left).cmp(change_path(right)));
        queued.dedup_by(|left, right| change_path(left) == change_path(right));
        let overflow = queued.split_off(queued.len().min(PATHS_PER_BATCH));
        for change in overflow {
            self.queue.push(change);
        }

        let now_ms = crate::domain::now_ms();
        let mut batch = Batch::default();
        let mut attempted = Vec::with_capacity(queued.len());
        let mut remaining = VecDeque::from(queued);
        while let Some(change) = remaining.pop_front() {
            let retry = change.clone();
            let path = change_path(&change).to_path_buf();
            let first_event_index = batch.observations.len();
            process_change(
                change,
                now_ms,
                &state,
                &self.port_map,
                &self.roots,
                &mut batch,
            )?;
            if config_candidate(&path) && batch.observations.len() > first_event_index {
                if let Some(root) = most_specific_root(&self.roots, &path) {
                    add_service_relationships(
                        &path,
                        &root.path,
                        first_event_index,
                        &self.service_map,
                        &mut batch,
                    );
                }
                add_project_relationships(&path, first_event_index, &state, &mut batch)?;
            }
            if let Err(error) = validate_batch_capacity(&batch) {
                // Nothing in this candidate reached storage. Requeue the
                // current path, every unprocessed path, and every earlier path
                // represented by the discarded candidate. PATHS_PER_BATCH is
                // deliberately one today, which also isolates a poison path.
                for attempted_change in attempted {
                    self.queue.push(attempted_change);
                }
                self.queue.push(retry);
                for unprocessed in remaining {
                    self.queue.push(unprocessed);
                }
                return Err(error);
            }
            attempted.push(retry);
        }
        if self.queue.len() > 0 || !self.sweep_backlog.is_empty() || self.sweep_walker.is_some() {
            if let Some(context) = &self.context {
                let _ = context.wake.request_snapshot();
            }
        }
        if batch.observations.is_empty() && batch.config_snapshots.is_empty() {
            return Ok(CollectorSnapshot::Noop);
        }
        debug_assert!(batch.observations.len() <= MAX_ATOMIC_OBSERVATIONS);
        self.pending = Some((snapshot.token(), attempted));
        Ok(CollectorSnapshot::Atomic {
            token: snapshot.token(),
            observations: batch.observations,
            relationships: batch.relationships,
            baseline: None,
            config_snapshots: batch.config_snapshots,
        })
    }

    fn commit_succeeded(
        &mut self,
        token: crate::collectors::CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        if self
            .pending
            .as_ref()
            .is_some_and(|(pending, _)| *pending == token)
        {
            self.pending = None;
            Ok(())
        } else {
            Err(AppError::InvalidInput {
                what: "file collector state",
                reason: "the acknowledgement token does not match".to_owned(),
            })
        }
    }

    fn commit_failed(&mut self, token: crate::collectors::CommitToken) -> AppResult<()> {
        let Some((pending, changes)) = self.pending.take() else {
            return Err(AppError::InvalidInput {
                what: "file collector state",
                reason: "there is no pending file batch".to_owned(),
            });
        };
        if pending != token {
            self.pending = Some((pending, changes));
            return Err(AppError::InvalidInput {
                what: "file collector state",
                reason: "the rejection token does not match".to_owned(),
            });
        }
        for change in changes {
            self.queue.push(change);
        }
        Ok(())
    }
}

#[derive(Default)]
struct Batch {
    observations: Vec<RawObservation>,
    relationships: Vec<RawRelationshipSpec>,
    config_snapshots: Vec<RawConfigSnapshotMutation>,
}

#[allow(clippy::too_many_arguments)]
fn process_change(
    change: LogicalChange,
    now_ms: i64,
    state: &StateReadHandle,
    port_map: &ListeningPortMap,
    roots: &SharedRoots,
    batch: &mut Batch,
) -> AppResult<()> {
    match change {
        LogicalChange::Deleted(path) => {
            if ignored(roots, &path, false) || !safe_removed_under_roots(roots, &path) {
                return Ok(());
            }
            batch.observations.push(observations::generic(
                &path,
                EventType::Deleted,
                None,
                None,
                now_ms,
                false,
            ));
            if state.config_snapshot(&path.to_string_lossy())?.is_some() {
                batch
                    .config_snapshots
                    .push(RawConfigSnapshotMutation::Delete {
                        path: path.to_string_lossy().into_owned(),
                    });
            }
        }
        LogicalChange::Renamed { from, to } => {
            if ignored(roots, &to, false) || !safe_under_roots(roots, &to) {
                return Ok(());
            }
            let from_snapshot = state.config_snapshot(&from.to_string_lossy())?;
            // Editors commonly save by renaming a temporary file over an
            // existing config path. The durable baseline belongs to the
            // destination in that case, so this is a modification/diff, not
            // a file rename that should replace the baseline blindly.
            if from_snapshot.is_none()
                && config_candidate(&to)
                && state.config_snapshot(&to.to_string_lossy())?.is_some()
            {
                return process_file(
                    &to,
                    EventType::Modified,
                    false,
                    now_ms,
                    state,
                    port_map,
                    roots,
                    batch,
                );
            }
            let metadata = std::fs::metadata(&to).ok();
            batch.observations.push(observations::renamed(
                &from,
                &to,
                metadata.as_ref().map(Metadata::len),
                metadata.as_ref().and_then(modified_ms),
                now_ms,
            ));
            if from_snapshot.is_some() {
                batch
                    .config_snapshots
                    .push(RawConfigSnapshotMutation::Delete {
                        path: from.to_string_lossy().into_owned(),
                    });
            }
            if config_candidate(&to) {
                if let Some(root) = safe_root_for_path(roots, &to) {
                    if let Ok(config_diff::ReadOutcome::Parsed(parsed)) =
                        config_diff::read_under(&root.path, &to)
                    {
                        let fits = snapshot_keys_fit(&parsed.keys);
                        let keys = if fits { parsed.keys } else { BTreeMap::new() };
                        let format = snapshot_format(parsed.format.as_str(), fits);
                        batch
                            .config_snapshots
                            .push(RawConfigSnapshotMutation::Upsert {
                                path: to.to_string_lossy().into_owned(),
                                keys,
                                captured_at: now_ms,
                                format,
                            });
                    }
                }
            }
        }
        LogicalChange::RenamedThenDeleted { from, to } => {
            if ignored(roots, &to, false) || !safe_removed_under_roots(roots, &to) {
                return Ok(());
            }
            batch.observations.push(observations::generic(
                &to,
                EventType::Deleted,
                None,
                None,
                now_ms,
                false,
            ));
            let baseline_paths = [&from, &to]
                .into_iter()
                .map(|path| crate::domain::ids::canonical_path(&path.to_string_lossy()))
                .collect::<BTreeSet<_>>();
            for path in baseline_paths {
                if state.config_snapshot(&path)?.is_some() {
                    batch
                        .config_snapshots
                        .push(RawConfigSnapshotMutation::Delete { path });
                }
            }
        }
        LogicalChange::Created(path) => {
            if ignored(roots, &path, false) || !safe_under_roots(roots, &path) {
                return Ok(());
            }
            process_file(
                &path,
                EventType::Created,
                false,
                now_ms,
                state,
                port_map,
                roots,
                batch,
            )?;
        }
        LogicalChange::Modified(path) => {
            if ignored(roots, &path, false) || !safe_under_roots(roots, &path) {
                return Ok(());
            }
            process_file(
                &path,
                EventType::Modified,
                false,
                now_ms,
                state,
                port_map,
                roots,
                batch,
            )?;
        }
        LogicalChange::Swept(path) => {
            if ignored(roots, &path, false) || !safe_under_roots(roots, &path) {
                return Ok(());
            }
            process_file(
                &path,
                EventType::Modified,
                true,
                now_ms,
                state,
                port_map,
                roots,
                batch,
            )?;
        }
        LogicalChange::SweptDeleted(path) => {
            if ignored(roots, &path, false) || !safe_removed_under_roots(roots, &path) {
                return Ok(());
            }
            batch.observations.push(observations::generic(
                &path,
                EventType::Deleted,
                None,
                None,
                now_ms,
                true,
            ));
            batch
                .config_snapshots
                .push(RawConfigSnapshotMutation::Delete {
                    path: path.to_string_lossy().into_owned(),
                });
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_file(
    path: &std::path::Path,
    generic_type: EventType,
    sweep: bool,
    now_ms: i64,
    state: &StateReadHandle,
    port_map: &ListeningPortMap,
    roots: &SharedRoots,
    batch: &mut Batch,
) -> AppResult<()> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(());
    };
    if !metadata.is_file() {
        return Ok(());
    }
    let size = metadata.len();
    let modified_at = modified_ms(&metadata);
    if !config_candidate(path) {
        if !sweep {
            batch.observations.push(observations::generic(
                path,
                generic_type,
                Some(size),
                modified_at,
                now_ms,
                false,
            ));
        }
        return Ok(());
    }

    let path_text = path.to_string_lossy().into_owned();
    let previous = state.config_snapshot(&path_text)?;
    if sweep
        && previous
            .as_ref()
            .is_some_and(|old| modified_at.is_some_and(|mtime| mtime <= old.captured_at))
    {
        return Ok(());
    }
    if !crate::collectors::ignore_set::IgnoreSet::new().may_read_contents(size) {
        if !sweep || previous.is_some() {
            batch.observations.push(observations::generic(
                path,
                generic_type,
                Some(size),
                modified_at,
                now_ms,
                sweep,
            ));
        }
        return Ok(());
    }
    let Some(root) = safe_root_for_path(roots, path) else {
        return Ok(());
    };
    let parsed = match config_diff::read_under(&root.path, path) {
        Ok(config_diff::ReadOutcome::Parsed(parsed)) => parsed,
        Ok(config_diff::ReadOutcome::Unsupported | config_diff::ReadOutcome::TooLarge) | Err(_) => {
            if !sweep || previous.is_some() {
                batch.observations.push(observations::generic(
                    path,
                    generic_type,
                    Some(size),
                    modified_at,
                    now_ms,
                    sweep,
                ));
            }
            return Ok(());
        }
    };
    if !snapshot_keys_fit(&parsed.keys) {
        if previous.is_some() {
            batch.observations.push(observations::generic(
                path,
                generic_type,
                Some(size),
                modified_at,
                now_ms,
                sweep,
            ));
        }
        batch
            .config_snapshots
            .push(RawConfigSnapshotMutation::Upsert {
                path: path_text,
                keys: BTreeMap::new(),
                captured_at: now_ms,
                format: snapshot_format(parsed.format.as_str(), false),
            });
        return Ok(());
    }
    let Some(previous) = previous else {
        batch
            .config_snapshots
            .push(RawConfigSnapshotMutation::Upsert {
                path: path_text,
                keys: parsed.keys,
                captured_at: now_ms,
                format: parsed.format.as_str().to_owned(),
            });
        return Ok(());
    };
    if previous.format.ends_with("+truncated") {
        batch.observations.push(observations::generic(
            path,
            generic_type,
            Some(size),
            modified_at,
            now_ms,
            sweep,
        ));
        batch
            .config_snapshots
            .push(RawConfigSnapshotMutation::Upsert {
                path: path_text,
                keys: parsed.keys,
                captured_at: now_ms,
                format: parsed.format.as_str().to_owned(),
            });
        return Ok(());
    }
    let old: BTreeMap<String, String> = serde_json::from_value(previous.keys).unwrap_or_default();
    let diff = config_diff::ConfigDiff::between(&old, &parsed.keys);
    let first_event_index = batch.observations.len();
    for change in diff.changes {
        batch.observations.push(observations::config_change(
            path,
            change.field.clone(),
            change.before,
            change.after,
            now_ms,
            sweep,
        ));
    }
    if diff.omitted > 0 {
        batch.observations.push(observations::config_summary(
            path,
            diff.omitted,
            now_ms,
            sweep,
        ));
    }
    if diff.sensitive_uncertain {
        batch
            .observations
            .push(observations::sensitive_uncertain(path, now_ms, sweep));
    }
    if batch.observations.len() > first_event_index {
        add_port_relationships(path, &parsed.keys, first_event_index, port_map, batch);
    }
    batch
        .config_snapshots
        .push(RawConfigSnapshotMutation::Upsert {
            path: path_text,
            keys: parsed.keys,
            captured_at: now_ms,
            format: parsed.format.as_str().to_owned(),
        });
    Ok(())
}

fn add_port_relationships(
    path: &std::path::Path,
    keys: &BTreeMap<String, String>,
    event_index: usize,
    port_map: &ListeningPortMap,
    batch: &mut Batch,
) {
    let ports = keys
        .iter()
        .filter(|(field, _)| port_shaped_key(field))
        .filter_map(|(_, value)| value.parse::<u16>().ok())
        .collect::<BTreeSet<_>>();
    for port in ports {
        for endpoint in port_map.endpoints(port) {
            let display_name = endpoint.display_name();
            batch.relationships.push(RawRelationshipSpec {
                evidence_observation: event_index,
                source: observations::file_entity(path),
                target: RawEntity {
                    identity: EntityIdentity::Port {
                        protocol: endpoint.protocol,
                        address: endpoint.address,
                        port: endpoint.port,
                    },
                    display_name,
                },
                kind: RelationshipKind::ConfigOf,
            });
        }
    }
}

fn add_service_relationships(
    path: &std::path::Path,
    root: &std::path::Path,
    event_index: usize,
    service_map: &ServiceEvidenceMap,
    batch: &mut Batch,
) {
    for service in service_map.under_root(&root.to_string_lossy()) {
        batch.relationships.push(RawRelationshipSpec {
            evidence_observation: event_index,
            source: observations::file_entity(path),
            target: RawEntity {
                identity: EntityIdentity::Service { name: service.name },
                display_name: service.display_name,
            },
            kind: RelationshipKind::ConfigOf,
        });
    }
}

fn add_project_relationships(
    path: &std::path::Path,
    event_index: usize,
    state: &StateReadHandle,
    batch: &mut Batch,
) -> AppResult<()> {
    let evidence = state.project_processes_for_config_path(&path.to_string_lossy())?;
    // Service and port relationships are independent evidence and are added
    // before this supplement.  Price project edges against both atomic
    // limits, and reserve half the relationship budget for stronger facts.
    let relationship_budget = MAX_ATOMIC_RELATIONSHIPS.saturating_sub(batch.relationships.len());
    let related_entity_budget =
        MAX_ATOMIC_RELATED_ENTITIES.saturating_sub(batch.relationships.len());
    let allowed = relationship_budget
        .min(related_entity_budget)
        .min(MAX_PROJECT_RELATIONSHIPS_PER_EVENT);
    let more_available = evidence.iter().any(|process| process.truncated);
    let omitted_by_budget = evidence.len().saturating_sub(allowed);
    let mut invalid_evidence = false;
    let mut recorded = 0usize;
    for process in evidence.into_iter().take(allowed) {
        let Some(id) = crate::domain::ids::ProcessId::parse(&process.entity_id) else {
            // A legacy/corrupt process row must not poison an otherwise valid
            // config commit.  It is omitted and called out below; the
            // normalizer still receives only typed ProcessById identities.
            invalid_evidence = true;
            continue;
        };
        batch.relationships.push(RawRelationshipSpec {
            evidence_observation: event_index,
            source: observations::file_entity(path),
            target: RawEntity {
                identity: EntityIdentity::ProcessById { id },
                display_name: process.display_name,
            },
            kind: RelationshipKind::ConfigOf,
        });
        recorded += 1;
    }
    if more_available || omitted_by_budget > 0 || invalid_evidence {
        // Keep the config event and any live port/service edges durable. The
        // metadata says that graph evidence was bounded without pretending an
        // omitted process was observed.
        if let Some(observation) = batch.observations.get_mut(event_index) {
            let mut details = observation
                .details
                .take()
                .filter(|value| value.is_object())
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(object) = details.as_object_mut() {
                object.insert(
                    "project_process_evidence_bounded".to_owned(),
                    serde_json::Value::Bool(true),
                );
                object.insert(
                    "project_process_evidence_recorded".to_owned(),
                    serde_json::Value::from(recorded as u64),
                );
                object.insert(
                    "project_process_evidence_more_available".to_owned(),
                    serde_json::Value::Bool(
                        more_available || omitted_by_budget > 0 || invalid_evidence,
                    ),
                );
            }
            observation.details = Some(details);
        }
    }
    Ok(())
}

fn validate_batch_capacity(batch: &Batch) -> AppResult<()> {
    if batch.observations.len() > MAX_ATOMIC_OBSERVATIONS {
        return Err(AppError::InvalidInput {
            what: "file collector batch",
            reason: "one file produces too many observations for an atomic commit".to_owned(),
        });
    }
    if batch.relationships.len() > MAX_ATOMIC_RELATIONSHIPS
        || batch.relationships.len() > MAX_ATOMIC_RELATED_ENTITIES
    {
        return Err(AppError::InvalidInput {
            what: "file relationship evidence",
            reason: "one file produces too many relationships for an atomic commit".to_owned(),
        });
    }
    Ok(())
}

fn port_shaped_key(field: &str) -> bool {
    field
        .rsplit(|character: char| !character.is_ascii_alphanumeric())
        .find(|part| !part.is_empty())
        .is_some_and(|part| part.eq_ignore_ascii_case("port"))
}

fn ignored(roots: &SharedRoots, path: &std::path::Path, is_directory: bool) -> bool {
    let Some(root) = most_specific_root(roots, path) else {
        return true;
    };
    crate::collectors::ignore_set::IgnoreSet::new().ignores(
        &root.path,
        path,
        is_directory,
        None,
        root.max_depth,
    )
}

fn safe_under_roots(roots: &SharedRoots, path: &std::path::Path) -> bool {
    safe_root_for_path(roots, path).is_some()
}

fn safe_root_for_path(roots: &SharedRoots, path: &std::path::Path) -> Option<RootSpec> {
    let root = most_specific_root(roots, path)?;
    crate::app::validate::watched_path_still_safe(&root.path, path).then_some(root)
}

fn most_specific_root(roots: &SharedRoots, path: &std::path::Path) -> Option<RootSpec> {
    let roots = roots
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    roots
        .iter()
        .filter(|root| path.starts_with(&root.path))
        .max_by_key(|root| root.path.components().count())
        .cloned()
}

fn safe_removed_under_roots(roots: &SharedRoots, path: &std::path::Path) -> bool {
    let roots = roots
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    roots
        .iter()
        .filter(|root| path.starts_with(&root.path))
        .max_by_key(|root| root.path.components().count())
        .is_some_and(|root| crate::app::validate::watched_removed_path_still_safe(&root.path, path))
}

fn config_candidate(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if name == ".env" || name.starts_with(".env.") {
        return true;
    }
    matches!(
        path.extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .as_deref(),
        Some("json" | "yaml" | "yml" | "toml" | "ini" | "conf" | "properties" | "xml")
    ) || matches!(name.as_str(), "dockerfile" | "caddyfile" | "hosts")
}

fn modified_ms(metadata: &Metadata) -> Option<i64> {
    let duration = metadata
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?;
    i64::try_from(duration.as_millis()).ok()
}

fn snapshot_keys_fit(keys: &BTreeMap<String, String>) -> bool {
    serde_json::to_vec(keys)
        .map(|encoded| encoded.len() <= MAX_SNAPSHOT_JSON_BYTES)
        .unwrap_or(false)
}

fn snapshot_format(format: &str, fits: bool) -> String {
    if fits {
        format.to_owned()
    } else {
        format!("{format}+truncated")
    }
}

fn change_path(change: &LogicalChange) -> &std::path::Path {
    match change {
        LogicalChange::Created(path)
        | LogicalChange::Modified(path)
        | LogicalChange::Swept(path)
        | LogicalChange::SweptDeleted(path)
        | LogicalChange::Deleted(path) => path,
        LogicalChange::Renamed { to, .. } | LogicalChange::RenamedThenDeleted { to, .. } => to,
    }
}

struct SweepPage {
    paths: Vec<PathBuf>,
    finished: bool,
    cancelled: bool,
}

struct DirectoryFrame {
    entries: std::fs::ReadDir,
    root: RootSpec,
}

struct SweepWalker {
    roots: Vec<RootSpec>,
    next_root: usize,
    stack: Vec<DirectoryFrame>,
}

impl SweepWalker {
    fn new(roots: Vec<RootSpec>) -> Self {
        Self {
            roots,
            next_root: 0,
            stack: Vec::new(),
        }
    }

    fn open_next_root(&mut self) -> bool {
        while let Some(root) = self.roots.get(self.next_root).cloned() {
            self.next_root += 1;
            if !crate::app::validate::watched_root_still_safe(&root.path) {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&root.path) {
                self.stack.push(DirectoryFrame { entries, root });
                return true;
            }
        }
        false
    }

    fn next_page(&mut self, cancel: Option<&crate::collectors::CancelToken>) -> SweepPage {
        let mut paths = Vec::new();
        let mut visited = 0usize;
        let mut cancelled = false;
        while visited < MAX_SWEEP_ENTRIES_PER_PAGE {
            if cancel.is_some_and(crate::collectors::CancelToken::is_cancelled) {
                cancelled = true;
                break;
            }
            if self.stack.is_empty() && !self.open_next_root() {
                break;
            }
            let (entry, root) = {
                let Some(frame) = self.stack.last_mut() else {
                    continue;
                };
                (frame.entries.next(), frame.root.clone())
            };
            let Some(entry) = entry else {
                self.stack.pop();
                continue;
            };
            visited += 1;
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if crate::app::validate::is_reparse_point(&path)
                || !crate::app::validate::watched_path_still_safe(&root.path, &path)
            {
                continue;
            }
            let file_type = metadata.file_type();
            if crate::collectors::ignore_set::IgnoreSet::new().ignores(
                &root.path,
                &path,
                file_type.is_dir(),
                None,
                root.max_depth,
            ) {
                continue;
            }
            if file_type.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    self.stack.push(DirectoryFrame { entries, root });
                }
            } else if file_type.is_file() && config_candidate(&path) {
                paths.push(path);
            }
        }
        paths.sort();
        paths.dedup();
        let finished = self.stack.is_empty() && self.next_root >= self.roots.len();
        SweepPage {
            paths,
            finished,
            cancelled,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::collectors::listeners::diff::TrackedSocket;
    use crate::collectors::service_evidence_map::ServiceEvidence;
    use crate::collectors::testing::live_context;
    use crate::collectors::CancelToken;
    use crate::domain::ids::Protocol;
    use crate::privacy::Sanitizer;
    use crate::storage::{config_snapshot_repository, ConfigSnapshot};

    #[test]
    fn descriptor_uses_the_canonical_notify_plus_sweep_cadence() {
        let descriptor = catalog_descriptor().unwrap();
        assert_eq!(
            descriptor.cadence,
            crate::domain::CollectorCadence::NotifyPlusSweep(Duration::from_secs(60))
        );
    }

    fn store_previous(
        live: &crate::collectors::testing::LiveContext,
        path: &std::path::Path,
        keys: serde_json::Value,
    ) {
        let snapshot = Sanitizer::new()
            .sanitize(ConfigSnapshot {
                path: path.to_string_lossy().into_owned(),
                keys,
                captured_at: 1,
                format: "env".to_owned(),
            })
            .0;
        live.db
            .write(move |connection| config_snapshot_repository::put(connection, &snapshot))
            .unwrap();
    }

    fn register_root(live: &crate::collectors::testing::LiveContext, root: &std::path::Path) {
        let path = root.to_string_lossy().into_owned();
        live.db
            .write(move |connection| {
                crate::storage::watched_location_repository::insert(connection, &path, 6, 1)
                    .map(|_| ())
            })
            .unwrap();
    }

    #[test]
    fn exact_port_change_links_only_to_current_listener_evidence() {
        let live = live_context(ID).unwrap();
        let path = live.directory.join(".env");
        std::fs::write(&path, b"PORT=4000\n").unwrap();
        store_previous(&live, &path, serde_json::json!({ "PORT": "3000" }));

        let map = ListeningPortMap::new();
        let token = CancelToken::default();
        map.publish(
            [TrackedSocket {
                protocol: Protocol::Tcp,
                address: "127.0.0.1".to_owned(),
                port: 4_000,
                owners: BTreeSet::new(),
            }],
            &token,
            Instant::now(),
        );
        let mut batch = Batch::default();
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: live.directory.clone(),
            max_depth: 6,
        }]));
        process_file(
            &path,
            EventType::Modified,
            false,
            2,
            &live.context.state,
            &map,
            &roots,
            &mut batch,
        )
        .unwrap();
        assert_eq!(batch.observations.len(), 1);
        assert_eq!(batch.observations[0].field.as_deref(), Some("PORT"));
        assert_eq!(batch.relationships.len(), 1);
        assert_eq!(batch.relationships[0].kind, RelationshipKind::ConfigOf);
        assert!(matches!(
            batch.relationships[0].target.identity,
            EntityIdentity::Port { port: 4_000, .. }
        ));
    }

    #[test]
    fn unchanged_nested_port_still_links_another_config_change() {
        let live = live_context(ID).unwrap();
        let path = live.directory.join("config.json");
        std::fs::write(&path, br#"{"server":{"port":4000},"mode":"prod"}"#).unwrap();
        store_previous(
            &live,
            &path,
            serde_json::json!({ "server.port": "4000", "mode": "dev" }),
        );
        let map = ListeningPortMap::new();
        let token = CancelToken::default();
        map.publish(
            [TrackedSocket {
                protocol: Protocol::Tcp,
                address: "127.0.0.1".to_owned(),
                port: 4_000,
                owners: BTreeSet::new(),
            }],
            &token,
            Instant::now(),
        );
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: live.directory.clone(),
            max_depth: 6,
        }]));
        let mut batch = Batch::default();
        process_file(
            &path,
            EventType::Modified,
            false,
            2,
            &live.context.state,
            &map,
            &roots,
            &mut batch,
        )
        .unwrap();
        assert_eq!(batch.observations.len(), 1);
        assert_eq!(batch.observations[0].field.as_deref(), Some("mode"));
        assert_eq!(batch.relationships.len(), 1);
        assert!(matches!(
            batch.relationships[0].target.identity,
            EntityIdentity::Port { port: 4_000, .. }
        ));
    }

    #[test]
    fn oversized_port_relationship_evidence_is_rejected_instead_of_truncated() {
        let live = live_context(ID).unwrap();
        let path = live.directory.join(".env");
        std::fs::write(&path, b"PORT=4000\n").unwrap();
        store_previous(&live, &path, serde_json::json!({ "PORT": "3000" }));
        let map = ListeningPortMap::new();
        let token = CancelToken::default();
        let sockets = (0..(MAX_ATOMIC_RELATIONSHIPS + 25)).map(|index| TrackedSocket {
            protocol: Protocol::Tcp,
            address: format!("10.0.{}.{}", index / 256, index % 256),
            port: 4_000,
            owners: BTreeSet::new(),
        });
        map.publish(sockets, &token, Instant::now());
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: live.directory.clone(),
            max_depth: 6,
        }]));
        let mut batch = Batch::default();
        process_file(
            &path,
            EventType::Modified,
            false,
            2,
            &live.context.state,
            &map,
            &roots,
            &mut batch,
        )
        .unwrap();
        assert!(batch.relationships.len() > MAX_ATOMIC_RELATIONSHIPS);
        assert!(validate_batch_capacity(&batch).is_err());
    }

    #[test]
    fn oversized_service_relationship_evidence_is_rejected_instead_of_truncated() {
        let root = crate::testing::temp_dir("files-service-relationship-bound");
        let path = root.join("appsettings.json");
        let map = ServiceEvidenceMap::new();
        let token = CancelToken::default();
        map.publish(
            (0..=MAX_ATOMIC_RELATIONSHIPS).map(|index| ServiceEvidence {
                name: format!("fixture-{index}"),
                display_name: format!("Fixture Service {index}"),
                binary_path: root
                    .join(format!("service-{index}.exe"))
                    .to_string_lossy()
                    .into_owned(),
            }),
            &token,
            Instant::now(),
        );
        let mut batch = Batch::default();
        add_service_relationships(&path, &root, 0, &map, &mut batch);

        assert!(batch.relationships.len() > MAX_ATOMIC_RELATIONSHIPS);
        assert!(validate_batch_capacity(&batch).is_err());
    }

    #[test]
    fn most_specific_overlapping_root_owns_depth_policy() {
        let root = crate::testing::temp_dir("files-overlap-root");
        let child = root.join("nested");
        let deep = child.join("one").join("two");
        std::fs::create_dir_all(&deep).unwrap();
        let path = deep.join("config.json");
        std::fs::write(&path, "{}").unwrap();
        let roots = Arc::new(RwLock::new(vec![
            RootSpec {
                path: root,
                max_depth: 1,
            },
            RootSpec {
                path: child,
                max_depth: 6,
            },
        ]));
        assert!(!ignored(&roots, &path, false));
    }

    #[test]
    fn sweep_only_queues_config_candidates_and_obeys_cancellation() {
        let root_path = crate::testing::temp_dir("files-bounded-sweep");
        for index in 0..100 {
            std::fs::write(root_path.join(format!("note-{index}.txt")), "ordinary").unwrap();
        }
        let config = root_path.join("config.json");
        std::fs::write(&config, "{}").unwrap();
        let root = RootSpec {
            path: root_path,
            max_depth: 6,
        };
        let mut walker = SweepWalker::new(vec![root.clone()]);
        let page = walker.next_page(None);
        assert_eq!(page.paths, vec![config]);
        assert!(page.finished);
        assert!(!page.cancelled);

        let cancelled = CancelToken::default();
        cancelled.cancel();
        let mut cancelled_walker = SweepWalker::new(vec![root]);
        let cancelled_page = cancelled_walker.next_page(Some(&cancelled));
        assert!(cancelled_page.paths.is_empty());
        assert!(cancelled_page.cancelled);
    }

    #[test]
    fn bounded_sweep_pages_continue_past_the_first_four_thousand_paths() {
        let root_path = crate::testing::temp_dir("files-paged-sweep");
        for index in 0..(MAX_SWEEP_ENTRIES_PER_PAGE + 4) {
            std::fs::write(root_path.join(format!("config-{index:05}.json")), "{}").unwrap();
        }
        let root = RootSpec {
            path: root_path,
            max_depth: 6,
        };
        let mut walker = SweepWalker::new(vec![root]);
        let first = walker.next_page(None);
        assert_eq!(first.paths.len(), MAX_SWEEP_ENTRIES_PER_PAGE);
        assert!(!first.finished);
        let second = walker.next_page(None);
        assert_eq!(second.paths.len(), 4);
        assert!(second.finished);
    }

    #[test]
    fn truncated_snapshot_rebaseline_never_fabricates_key_additions() {
        let live = live_context(ID).unwrap();
        let path = live.directory.join("config.json");
        std::fs::write(&path, br#"{"server":{"port":4000}}"#).unwrap();
        let snapshot = Sanitizer::new()
            .sanitize(ConfigSnapshot {
                path: path.to_string_lossy().into_owned(),
                keys: serde_json::json!({}),
                captured_at: 1,
                format: "json+truncated".to_owned(),
            })
            .0;
        live.db
            .write(move |connection| config_snapshot_repository::put(connection, &snapshot))
            .unwrap();
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: live.directory.clone(),
            max_depth: 6,
        }]));
        let mut batch = Batch::default();
        process_file(
            &path,
            EventType::Modified,
            false,
            2,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut batch,
        )
        .unwrap();
        assert_eq!(batch.observations.len(), 1);
        assert_eq!(batch.observations[0].kind.event_type, EventType::Modified);
        assert!(batch.observations[0].field.is_none());
        assert_eq!(batch.config_snapshots.len(), 1);
    }

    #[test]
    fn oversized_and_malformed_configs_fall_back_without_parsing_failure() {
        let live = live_context(ID).unwrap();
        let oversized = live.directory.join("oversized.json");
        std::fs::write(
            &oversized,
            vec![b'x'; crate::collectors::ignore_set::MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        let malformed = live.directory.join("malformed.json");
        std::fs::write(&malformed, b"{ definitely-not-json").unwrap();

        let mut batch = Batch::default();
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: live.directory.clone(),
            max_depth: 6,
        }]));
        for path in [&oversized, &malformed] {
            process_file(
                path,
                EventType::Modified,
                false,
                2,
                &live.context.state,
                &ListeningPortMap::new(),
                &roots,
                &mut batch,
            )
            .unwrap();
        }
        assert_eq!(batch.observations.len(), 2);
        assert!(batch
            .observations
            .iter()
            .all(|row| row.kind.event_type == EventType::Modified && row.field.is_none()));
        assert!(batch.config_snapshots.is_empty());
    }

    #[test]
    fn restart_sweep_emits_uncertain_delete_for_a_missing_persisted_config() {
        let live = live_context(ID).unwrap();
        let root = live.directory.join("watched-delete");
        std::fs::create_dir(&root).unwrap();
        let missing = root.join("removed-subtree").join("config.json");
        register_root(&live, &root);
        store_previous(&live, &missing, serde_json::json!({ "PORT": "3000" }));

        let mut collector =
            FileCollector::new(ListeningPortMap::new(), catalog_descriptor().unwrap());
        collector.start(live.context.clone()).unwrap();
        let snapshot = collector
            .snapshot_batch(SnapshotContext::new(crate::collectors::CommitToken::mint(
                91,
            )))
            .unwrap();
        let CollectorSnapshot::Atomic {
            observations,
            config_snapshots,
            ..
        } = snapshot
        else {
            panic!("missing persisted config must produce an atomic deletion");
        };
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].kind.event_type, EventType::Deleted);
        assert!(observations[0].occurred_at_is_uncertain);
        assert_eq!(
            config_snapshots,
            vec![RawConfigSnapshotMutation::Delete {
                path: missing.to_string_lossy().into_owned(),
            }]
        );
    }

    #[test]
    fn restart_sweep_marks_a_newer_config_diff_uncertain() {
        let live = live_context(ID).unwrap();
        let root = live.directory.join("watched");
        std::fs::create_dir(&root).unwrap();
        let path = root.join("config.json");
        std::fs::write(&path, br#"{"server":{"port":3000}}"#).unwrap();
        register_root(&live, &root);

        let mut first = FileCollector::new(ListeningPortMap::new(), catalog_descriptor().unwrap());
        first.start(live.context.clone()).unwrap();
        let initial = first
            .snapshot_batch(SnapshotContext::new(crate::collectors::CommitToken::mint(
                1,
            )))
            .unwrap();
        assert!(matches!(
            initial,
            CollectorSnapshot::Atomic {
                observations,
                config_snapshots,
                ..
            } if observations.is_empty() && config_snapshots.len() == 1
        ));
        first.stop();

        store_previous(&live, &path, serde_json::json!({ "server.port": "3000" }));
        std::fs::write(&path, br#"{"server":{"port":4000}}"#).unwrap();
        let mut resumed =
            FileCollector::new(ListeningPortMap::new(), catalog_descriptor().unwrap());
        resumed.start(live.context.clone()).unwrap();
        let restart = resumed
            .snapshot_batch(SnapshotContext::new(crate::collectors::CommitToken::mint(
                2,
            )))
            .unwrap();
        let CollectorSnapshot::Atomic { observations, .. } = restart else {
            panic!("restart should atomically report the downtime diff");
        };
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].field.as_deref(), Some("server.port"));
        assert!(observations[0].occurred_at_is_uncertain);
        resumed.stop();
    }

    #[test]
    fn config_rename_and_delete_produce_logical_events_and_snapshot_deletes() {
        let live = live_context(ID).unwrap();
        let root = live.directory.join("watched");
        std::fs::create_dir(&root).unwrap();
        let from = root.join(".env.old");
        let to = root.join(".env.new");
        std::fs::write(&to, b"PORT=4000\n").unwrap();
        store_previous(&live, &from, serde_json::json!({ "PORT": "3000" }));
        let roots = Arc::new(RwLock::new(vec![RootSpec {
            path: root,
            max_depth: 6,
        }]));
        let mut renamed = Batch::default();
        process_change(
            LogicalChange::Renamed {
                from: from.clone(),
                to: to.clone(),
            },
            2,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut renamed,
        )
        .unwrap();
        assert_eq!(renamed.observations.len(), 1);
        assert_eq!(renamed.observations[0].kind.event_type, EventType::Renamed);
        assert_eq!(renamed.config_snapshots.len(), 2);

        let tracked_from = to.with_file_name(".env.tracked-from");
        let tracked_to = to.with_file_name(".env.tracked-to");
        std::fs::write(&tracked_to, b"PORT=5000\n").unwrap();
        store_previous(&live, &tracked_from, serde_json::json!({ "PORT": "3000" }));
        store_previous(&live, &tracked_to, serde_json::json!({ "PORT": "4000" }));
        let mut tracked_rename = Batch::default();
        process_change(
            LogicalChange::Renamed {
                from: tracked_from.clone(),
                to: tracked_to,
            },
            2,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut tracked_rename,
        )
        .unwrap();
        assert_eq!(tracked_rename.observations.len(), 1);
        assert_eq!(
            tracked_rename.observations[0].kind.event_type,
            EventType::Renamed
        );
        assert!(tracked_rename.config_snapshots.iter().any(
            |mutation| matches!(mutation, RawConfigSnapshotMutation::Delete { path } if path == &tracked_from.to_string_lossy())
        ));
        assert!(tracked_rename
            .config_snapshots
            .iter()
            .any(|mutation| matches!(mutation, RawConfigSnapshotMutation::Upsert { .. })));

        let mut plain_rename = Batch::default();
        process_change(
            LogicalChange::Renamed {
                from: to.with_file_name("before.txt"),
                to: to.with_file_name("after.txt"),
            },
            2,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut plain_rename,
        )
        .unwrap();
        assert_eq!(plain_rename.observations.len(), 1);
        assert_eq!(
            plain_rename.observations[0].kind.event_type,
            EventType::Renamed
        );
        assert!(plain_rename.config_snapshots.is_empty());

        let destination = to.with_file_name(".env");
        std::fs::write(&destination, b"PORT=5000\n").unwrap();
        store_previous(&live, &destination, serde_json::json!({ "PORT": "3000" }));
        let mut editor_save = Batch::default();
        process_change(
            LogicalChange::Renamed {
                from: to.with_file_name("config.tmp"),
                to: destination,
            },
            2,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut editor_save,
        )
        .unwrap();
        assert_eq!(editor_save.observations.len(), 1);
        assert_eq!(
            editor_save.observations[0].kind.event_type,
            EventType::ConfigurationChanged
        );
        assert_eq!(editor_save.observations[0].field.as_deref(), Some("PORT"));
        assert_eq!(editor_save.config_snapshots.len(), 1);

        let delete_from = to.with_file_name(".env.delete-source");
        let delete_to = to.with_file_name(".env.delete-destination");
        store_previous(&live, &delete_from, serde_json::json!({ "PORT": "5000" }));
        store_previous(&live, &delete_to, serde_json::json!({ "PORT": "6000" }));
        let mut rename_deleted = Batch::default();
        process_change(
            LogicalChange::RenamedThenDeleted {
                from: delete_from,
                to: delete_to,
            },
            3,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut rename_deleted,
        )
        .unwrap();
        assert_eq!(rename_deleted.observations.len(), 1);
        assert_eq!(
            rename_deleted.observations[0].kind.event_type,
            EventType::Deleted
        );
        assert_eq!(rename_deleted.config_snapshots.len(), 2);
        assert!(rename_deleted
            .config_snapshots
            .iter()
            .all(|mutation| matches!(mutation, RawConfigSnapshotMutation::Delete { .. })));

        let mut deleted = Batch::default();
        process_change(
            LogicalChange::Deleted(from),
            3,
            &live.context.state,
            &ListeningPortMap::new(),
            &roots,
            &mut deleted,
        )
        .unwrap();
        assert_eq!(deleted.observations[0].kind.event_type, EventType::Deleted);
        assert!(matches!(
            deleted.config_snapshots.as_slice(),
            [RawConfigSnapshotMutation::Delete { .. }]
        ));
    }
}
