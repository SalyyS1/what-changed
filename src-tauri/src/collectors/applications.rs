//! Installed-application collector.
//!
//! The provider reads the three uninstall registry views. This layer owns the
//! filtering and durable diff so an incomplete or quirky registry entry never
//! becomes an invented application event.

mod diff;
mod observations;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::collectors::application_install_map::{
    ApplicationInstallEvidence, ApplicationInstallMap,
};
use crate::collectors::service_evidence_map::ServiceEvidenceMap;
use crate::collectors::stateful;
use crate::collectors::{
    AckedState, CancelToken, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot,
    CommitToken, EntityIdentity, ObservationClock, RawEntity, RawObservation, RawRelationshipSpec,
    SnapshotContext, SystemObservationClock, WakeHandle,
};
use crate::domain::RelationshipKind;
use crate::error::{AppError, AppResult};
use crate::storage::{Baseline, CollectorCommitReceipt};

/// Canonical collector id.
pub const ID: &str = "applications";
/// Durable state kind owned by this collector.
pub const BASELINE_KIND: &str = "installed_applications";

/// Version is the only application mutation surfaced by the live collector;
/// metadata refreshes only advance its durable baseline.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    diff::checkpoint_fields(before, after)
}

/// One of the three uninstall registry views the product reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UninstallHive {
    LocalMachine64,
    LocalMachine32,
    CurrentUser,
}

impl UninstallHive {
    /// Stable text used by the application entity-id grammar.
    pub const fn identity(self) -> &'static str {
        match self {
            Self::LocalMachine64 => "HKLM64",
            Self::LocalMachine32 => "HKLM32",
            Self::CurrentUser => "HKCU",
        }
    }
}

/// One raw uninstall entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallEntry {
    pub hive: UninstallHive,
    pub subkey_name: String,
    pub display_name: String,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub install_date: Option<String>,
    pub install_location: Option<String>,
    pub system_component: bool,
}

/// Keeps registry notification threads alive until the collector stops.
pub trait RegistryNotificationGuard: Send {}

/// Read-only seam around uninstall registry enumeration and notifications.
pub trait RegistryUninstallProvider: Send + Sync + 'static {
    fn enumerate(&self) -> AppResult<Vec<UninstallEntry>>;

    fn enumerate_into(&self, output: &mut Vec<UninstallEntry>) -> AppResult<()> {
        output.clear();
        output.extend(self.enumerate()?);
        Ok(())
    }

    /// Register one-shot notifications which re-arm themselves after a wake.
    ///
    /// Failure is deliberately non-fatal: the descriptor's 300-second sweep
    /// remains the correctness path.
    fn register_notifications(
        &self,
        _wake: WakeHandle,
        _cancel: CancelToken,
    ) -> AppResult<Box<dyn RegistryNotificationGuard>> {
        Err(AppError::Platform {
            operation: "watch uninstall registry hives",
            reason: "notifications are unavailable".to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct ApplicationState {
    acknowledged: BTreeMap<String, TrackedApplication>,
    #[serde(default)]
    pending: Option<stateful::PendingTarget<BTreeMap<String, TrackedApplication>>>,
    established: bool,
}

impl ApplicationState {
    fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        stateful::hydrate(baseline, now_ms, "the installed-application table")
    }
}

/// Registry entry reduced to durable, diffable fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TrackedApplication {
    hive: UninstallHive,
    subkey_name: String,
    display_name: String,
    display_version: Option<String>,
    publisher: Option<String>,
    install_date: Option<String>,
    /// Session-only path evidence. Raw install roots are intentionally absent
    /// from the durable baseline and are republished from the next live sweep.
    #[serde(skip)]
    install_location: Option<String>,
}

impl TrackedApplication {
    fn key(&self) -> String {
        format!("{}:{}", self.hive.identity(), self.subkey_name)
    }
}

/// Diffs installed applications against the last acknowledged registry view.
pub struct ApplicationCollector<P, C = SystemObservationClock> {
    provider: P,
    clock: C,
    descriptor: CollectorDescriptor,
    install_map: ApplicationInstallMap,
    service_map: ServiceEvidenceMap,
    await_service_evidence: bool,
    service_readiness_checked: bool,
    state: AckedState<ApplicationState>,
    context: Option<CollectorContext>,
    notifications: Option<Box<dyn RegistryNotificationGuard>>,
    downtime: bool,
    downtime_clear_token: Option<CommitToken>,
    rows: Vec<UninstallEntry>,
}

impl<P: RegistryUninstallProvider> ApplicationCollector<P, SystemObservationClock> {
    pub fn new(
        provider: P,
        install_map: ApplicationInstallMap,
        descriptor: CollectorDescriptor,
    ) -> Self {
        Self::with_clock(provider, install_map, descriptor, SystemObservationClock)
    }

    pub fn with_evidence_maps(
        provider: P,
        install_map: ApplicationInstallMap,
        service_map: ServiceEvidenceMap,
        descriptor: CollectorDescriptor,
    ) -> Self {
        Self::build(
            provider,
            install_map,
            service_map,
            descriptor,
            SystemObservationClock,
            true,
        )
    }
}

impl<P: RegistryUninstallProvider, C: ObservationClock> ApplicationCollector<P, C> {
    pub fn with_clock(
        provider: P,
        install_map: ApplicationInstallMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self::build(
            provider,
            install_map,
            ServiceEvidenceMap::new(),
            descriptor,
            clock,
            false,
        )
    }

    pub fn with_clock_and_maps(
        provider: P,
        install_map: ApplicationInstallMap,
        service_map: ServiceEvidenceMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self::build(provider, install_map, service_map, descriptor, clock, true)
    }

    fn build(
        provider: P,
        install_map: ApplicationInstallMap,
        service_map: ServiceEvidenceMap,
        descriptor: CollectorDescriptor,
        clock: C,
        await_service_evidence: bool,
    ) -> Self {
        Self {
            provider,
            clock,
            descriptor,
            install_map,
            service_map,
            await_service_evidence,
            service_readiness_checked: false,
            state: AckedState::new(ApplicationState::default()),
            context: None,
            notifications: None,
            downtime: false,
            downtime_clear_token: None,
            rows: Vec::with_capacity(192),
        }
    }

    fn stage(
        &mut self,
        token: CommitToken,
        next: ApplicationState,
        now_ms: i64,
        observations: Vec<RawObservation>,
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

    fn publish_install_evidence(
        &self,
        applications: &BTreeMap<String, TrackedApplication>,
        cancel: &CancelToken,
    ) {
        self.install_map.publish(
            applications.values().filter_map(|application| {
                let install_location = application.install_location.as_deref()?.trim();
                if install_location.is_empty() {
                    return None;
                }
                Some(ApplicationInstallEvidence {
                    hive: application.hive.identity().to_owned(),
                    registry_key: application.subkey_name.clone(),
                    display_name: application.display_name.clone(),
                    install_location: install_location.to_owned(),
                })
            }),
            cancel,
            Instant::now(),
        );
    }
}

impl<P: RegistryUninstallProvider, C: ObservationClock> Collector for ApplicationCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let (state, downtime) =
            ApplicationState::hydrate(context.state.baseline(BASELINE_KIND)?, self.clock.now_ms());
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.service_readiness_checked = false;
        self.notifications = match self
            .provider
            .register_notifications(context.wake.clone(), context.cancel.clone())
        {
            Ok(guard) => Some(guard),
            Err(error) => {
                tracing::debug!(reason = %error, "registry notifications unavailable; using fallback sweep");
                None
            }
        };
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.notifications = None;
        if let Some(context) = &self.context {
            self.install_map.invalidate(&context.cancel);
        }
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "application collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        self.provider.enumerate_into(&mut self.rows)?;
        if cancel.is_cancelled() {
            self.rows.clear();
            return Ok(CollectorSnapshot::Noop);
        }
        let current = diff::tracked(&self.rows);
        self.rows.clear();
        self.publish_install_evidence(&current, &cancel);
        if self.await_service_evidence && !self.service_readiness_checked {
            for _ in 0..40 {
                if cancel.is_cancelled()
                    || self.service_map.wait_for_current(Duration::from_millis(50))
                {
                    break;
                }
            }
            self.service_readiness_checked = true;
            if cancel.is_cancelled() {
                return Ok(CollectorSnapshot::Noop);
            }
        }

        if !self.state.committed().established {
            return self.stage(
                context.token(),
                ApplicationState {
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
        let changes = diff::changes(&self.state.committed().acknowledged, &target);
        if changes.is_empty() {
            self.publish_install_evidence(&target, &cancel);
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }

        let mut acknowledged = self.state.committed().acknowledged.clone();
        let mut capacity = stateful::CommitCapacity::full();
        let mut drained = 0usize;
        let mut observations = Vec::new();
        let mut relationships = Vec::new();
        for change in &changes {
            let observation = observations::for_change(change, observed_at, uncertain);
            let change_relationships = observation
                .as_ref()
                .map(|_| {
                    provides_relationships(
                        change,
                        &self.install_map,
                        &self.service_map,
                        observations.len(),
                    )
                })
                .unwrap_or_default();
            let evidence_rows = change_relationships.len();
            if observation.is_some() && !capacity.spend(1, evidence_rows, evidence_rows) {
                if !stateful::CommitCapacity::full().fits(1, evidence_rows, evidence_rows) {
                    return Err(AppError::InvalidInput {
                        what: "application relationship evidence",
                        reason: "one application change exceeds the atomic relationship bounds"
                            .to_owned(),
                    });
                }
                break;
            }
            diff::apply(&mut acknowledged, change);
            observations.extend(observation);
            relationships.extend(change_relationships);
            drained += 1;
        }
        let fully_drained = drained == changes.len();
        let next = ApplicationState {
            acknowledged,
            pending: (!fully_drained).then_some(stateful::PendingTarget {
                target,
                observed_at,
                occurred_at_is_uncertain: uncertain,
            }),
            established: true,
        };
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
        if let Some(context) = &self.context {
            self.publish_install_evidence(&self.state.committed().acknowledged, &context.cancel);
        }
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
    services: &ServiceEvidenceMap,
    evidence_observation: usize,
) -> Vec<RawRelationshipSpec> {
    let mut relationships = Vec::new();
    let application = match change {
        diff::Change::Installed(application)
        | diff::Change::Updated {
            after: application, ..
        } => application,
        diff::Change::Removed(_) | diff::Change::Metadata { .. } => return relationships,
    };
    let Some(install_location) = application.install_location.as_deref() else {
        return relationships;
    };
    for service in services.under_install_location(install_location) {
        let owns_service = applications
            .providers_of(&service.binary_path)
            .iter()
            .any(|provider| {
                provider.hive == application.hive.identity()
                    && provider.registry_key == application.subkey_name
            });
        if !owns_service {
            continue;
        }
        relationships.push(RawRelationshipSpec {
            evidence_observation,
            source: RawEntity {
                identity: EntityIdentity::Application {
                    hive: application.hive.identity().to_owned(),
                    registry_key: application.subkey_name.clone(),
                },
                display_name: application.display_name.clone(),
            },
            target: RawEntity {
                identity: EntityIdentity::Service { name: service.name },
                display_name: service.display_name,
            },
            kind: RelationshipKind::Provides,
        });
    }
    relationships
}

/// Descriptor from the one canonical collector catalog.
pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the application collector is not in the canonical catalog".to_owned(),
        })
}
