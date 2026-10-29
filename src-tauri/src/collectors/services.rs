//! Windows service snapshots and durable diffing.
//!
//! The provider is deliberately read-only. It reports a complete table or an
//! error, and the collector advances that table only with the event rows that
//! describe the transition. A missing durable baseline establishes silently;
//! a baseline from a previous recorder session exposes downtime changes with
//! uncertain occurrence times.

mod diff;
mod observations;
mod state;

use std::time::Duration;

use crate::collectors::application_install_map::ApplicationInstallMap;
use crate::collectors::service_evidence_map::{ServiceEvidence, ServiceEvidenceMap};
use crate::collectors::stateful;
use crate::collectors::{
    AckedState, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot, CommitToken,
    EntityIdentity, ObservationClock, RawEntity, RawRelationshipSpec, SnapshotContext,
    SystemObservationClock, WakeHandle,
};
use crate::domain::RelationshipKind;
use crate::error::{AppError, AppResult};
use crate::storage::CollectorCommitReceipt;

pub use state::{ServiceRecord, ServiceRunState, ServiceSnapshot, ServiceStartType, BASELINE_KIND};

/// Canonical collector id.
pub const ID: &str = "services";

/// Compare durable service rows using the live collector's running-boundary
/// and readable start-type rules.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    diff::checkpoint_fields(before, after)
}

/// A complete, read-only view of the Windows service control manager.
pub trait ServiceProvider: Send + Sync + 'static {
    fn services(&self) -> AppResult<Vec<ServiceRecord>>;

    /// Register a best-effort wake for service creation/removal.
    ///
    /// Returning `false` is normal: the five-second sweep remains the source
    /// of correctness and the collector must keep running without this hint.
    fn try_register_notify(&self, _wake: WakeHandle) -> bool {
        false
    }
}

pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the services collector is not in the canonical catalog".to_owned(),
        })
}

/// Diffs the service table on its catalog cadence.
pub struct ServiceCollector<P, C = SystemObservationClock> {
    provider: P,
    clock: C,
    descriptor: CollectorDescriptor,
    evidence: ServiceEvidenceMap,
    applications: ApplicationInstallMap,
    await_application_evidence: bool,
    application_readiness_checked: bool,
    state: AckedState<ServiceSnapshot>,
    context: Option<CollectorContext>,
    downtime: bool,
    downtime_clear_token: Option<CommitToken>,
}

impl<P: ServiceProvider> ServiceCollector<P, SystemObservationClock> {
    pub fn new(provider: P, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, descriptor, SystemObservationClock)
    }

    pub fn with_evidence_map(
        provider: P,
        evidence: ServiceEvidenceMap,
        descriptor: CollectorDescriptor,
    ) -> Self {
        Self::with_clock_and_map(provider, evidence, descriptor, SystemObservationClock)
    }

    pub fn with_evidence_maps(
        provider: P,
        evidence: ServiceEvidenceMap,
        applications: ApplicationInstallMap,
        descriptor: CollectorDescriptor,
    ) -> Self {
        Self::build(
            provider,
            evidence,
            applications,
            descriptor,
            SystemObservationClock,
            true,
        )
    }
}

impl<P: ServiceProvider, C: ObservationClock> ServiceCollector<P, C> {
    pub fn with_clock(provider: P, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self::build(
            provider,
            ServiceEvidenceMap::new(),
            ApplicationInstallMap::new(),
            descriptor,
            clock,
            false,
        )
    }

    pub fn with_clock_and_map(
        provider: P,
        evidence: ServiceEvidenceMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self::build(
            provider,
            evidence,
            ApplicationInstallMap::new(),
            descriptor,
            clock,
            false,
        )
    }

    pub fn with_clock_and_maps(
        provider: P,
        evidence: ServiceEvidenceMap,
        applications: ApplicationInstallMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self::build(provider, evidence, applications, descriptor, clock, true)
    }

    fn build(
        provider: P,
        evidence: ServiceEvidenceMap,
        applications: ApplicationInstallMap,
        descriptor: CollectorDescriptor,
        clock: C,
        await_application_evidence: bool,
    ) -> Self {
        Self {
            provider,
            clock,
            descriptor,
            evidence,
            applications,
            await_application_evidence,
            application_readiness_checked: false,
            state: AckedState::new(ServiceSnapshot::default()),
            context: None,
            downtime: false,
            downtime_clear_token: None,
        }
    }

    fn stage(
        &mut self,
        token: CommitToken,
        next: ServiceSnapshot,
        now_ms: i64,
        observations: Vec<crate::collectors::RawObservation>,
        relationships: Vec<RawRelationshipSpec>,
        clears_downtime: bool,
    ) -> AppResult<CollectorSnapshot> {
        let baseline = stateful::baseline_for(BASELINE_KIND, &next, now_ms)?;
        self.state.stage(token, next)?;
        self.downtime_clear_token = clears_downtime.then_some(token);
        Ok(CollectorSnapshot::Atomic {
            token,
            observations,
            relationships,
            baseline: Some(baseline),
            config_snapshots: Vec::new(),
        })
    }
}

impl<P: ServiceProvider, C: ObservationClock> Collector for ServiceCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let now_ms = self.clock.now_ms();
        let baseline = context.state.baseline(BASELINE_KIND)?;
        let (state, downtime) = ServiceSnapshot::hydrate(baseline, now_ms);
        if !self.provider.try_register_notify(context.wake.clone()) {
            tracing::debug!("service notification unavailable; continuing with five-second sweeps");
        }
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.application_readiness_checked = false;
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(context) = &self.context {
            self.evidence.invalidate(&context.cancel);
        }
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<crate::collectors::RawObservation>> {
        Err(AppError::InvalidInput {
            what: "service collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        let rows = self.provider.services()?;
        if cancel.is_cancelled() {
            return Ok(CollectorSnapshot::Noop);
        }
        let mut current = state::index(rows)?;
        self.evidence.publish(
            current.values().filter_map(|row| {
                row.binary_path.as_ref().map(|binary_path| ServiceEvidence {
                    name: row.name.clone(),
                    display_name: row.display_name.clone(),
                    binary_path: binary_path.clone(),
                })
            }),
            &cancel,
            std::time::Instant::now(),
        );
        if self.await_application_evidence && !self.application_readiness_checked {
            for _ in 0..40 {
                if cancel.is_cancelled()
                    || self
                        .applications
                        .wait_for_current(Duration::from_millis(50))
                {
                    break;
                }
            }
            self.application_readiness_checked = true;
            if cancel.is_cancelled() {
                return Ok(CollectorSnapshot::Noop);
            }
        }
        state::preserve_unavailable_config(&mut current, &self.state.committed().acknowledged);

        if !self.state.committed().established {
            return self.stage(
                context.token(),
                ServiceSnapshot {
                    acknowledged: current,
                    pending: None,
                    established: true,
                },
                now_ms,
                Vec::new(),
                Vec::new(),
                true,
            );
        }

        let pending = self.state.committed().pending.clone();
        let (target, observed_at, uncertain) = pending
            .as_ref()
            .map(|pending| {
                (
                    pending.target.clone(),
                    pending.observed_at,
                    pending.occurred_at_is_uncertain,
                )
            })
            .unwrap_or((current, now_ms, self.downtime));
        let changes = diff::between(&self.state.committed().acknowledged, &target);
        let mut capacity = stateful::CommitCapacity::full();
        let mut drained = 0usize;
        let mut observations = Vec::new();
        let mut relationships = Vec::new();
        for change in &changes {
            let evidence_observation = observations.len();
            let change_relationships =
                provides_relationships(change, &self.applications, evidence_observation);
            let evidence_rows = change_relationships.len();
            if !capacity.spend(1, evidence_rows, evidence_rows) {
                if !stateful::CommitCapacity::full().fits(1, evidence_rows, evidence_rows) {
                    return Err(AppError::InvalidInput {
                        what: "service relationship evidence",
                        reason: "one service change exceeds the atomic relationship bounds"
                            .to_owned(),
                    });
                }
                break;
            }
            observations.push(observations::for_change(change, observed_at, uncertain));
            relationships.extend(change_relationships);
            drained += 1;
        }
        let mut acknowledged = self.state.committed().acknowledged.clone();
        for change in &changes[..drained] {
            diff::apply(&mut acknowledged, change);
        }
        if drained == changes.len() {
            // Refresh non-event evidence (display name, pid and binary path)
            // only after every event-bearing transition reached this commit.
            // During a bounded drain, the per-change apply above intentionally
            // leaves later dimensions untouched so they cannot disappear.
            acknowledged = target.clone();
        }
        let next = ServiceSnapshot {
            acknowledged,
            pending: (drained != changes.len()).then_some(stateful::PendingTarget {
                target,
                observed_at,
                occurred_at_is_uncertain: uncertain,
            }),
            established: true,
        };

        if changes.is_empty() && next == *self.state.committed() {
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }
        self.stage(
            context.token(),
            next,
            now_ms,
            observations,
            relationships,
            pending.is_none(),
        )
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        self.state.ack(token)?;
        if self.downtime_clear_token == Some(token) {
            self.downtime = false;
        }
        self.downtime_clear_token = None;
        Ok(())
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        let result = self.state.reject(token);
        if self.downtime_clear_token == Some(token) {
            self.downtime_clear_token = None;
        }
        result
    }
}

fn provides_relationships(
    change: &diff::Change,
    applications: &ApplicationInstallMap,
    evidence_observation: usize,
) -> Vec<RawRelationshipSpec> {
    let mut relationships = Vec::new();
    let diff::Change::Installed(service) = change else {
        return relationships;
    };
    let Some(binary_path) = service.binary_path.as_deref() else {
        return relationships;
    };
    for application in applications.providers_of(binary_path) {
        relationships.push(RawRelationshipSpec {
            evidence_observation,
            source: RawEntity {
                identity: EntityIdentity::Application {
                    hive: application.hive,
                    registry_key: application.registry_key,
                },
                display_name: application.display_name,
            },
            target: RawEntity {
                identity: EntityIdentity::Service {
                    name: service.name.clone(),
                },
                display_name: service.display_name.clone(),
            },
            kind: RelationshipKind::Provides,
        });
    }
    relationships
}
