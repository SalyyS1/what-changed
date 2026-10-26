//! Cross-platform collector contracts and runtime.

pub mod application_install_map;
pub mod applications;
pub mod config_diff;
pub mod dev_env;
pub mod docker;
pub mod docker_protocol;
pub mod drivers;
pub mod eventlog;
pub mod files;
pub mod ignore_set;
pub mod listeners;
pub mod listening_port_map;
pub mod network;
pub mod normalize;
pub mod process;
pub mod process_map;
pub mod registry;
pub mod runtime;
pub mod schedule;
pub mod service_evidence_map;
pub mod services;
pub mod snapshot;
pub mod startup;
pub mod stateful;
#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub mod testing;
pub mod updates;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::Value;

use crate::app::Monitoring;
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};
use crate::error::AppResult;

pub use crate::domain::{CollectorCadence, CollectorCost, CollectorDescriptor};
pub use snapshot::{
    AckedState, CollectorSnapshot, CommitToken, RawBaseline, RawConfigSnapshotMutation,
    RawRelationshipSpec, SnapshotContext, StateReadHandle,
};

/// Collector-owned semantic field change used by checkpoint comparison.
/// Persistence/session details must not become user-visible changes.
pub(crate) type CheckpointFieldChange = (String, Value, Value);

/// The observation kind and the human-readable summaries it contributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationKind {
    pub category: Category,
    pub event_type: EventType,
    pub severity: Severity,
    pub confidence: Confidence,
    pub user_friendly_summary: String,
    pub technical_summary: String,
    pub classifier: ClassifierContext,
}

/// Identity material understood by the P04 entity-id grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityIdentity {
    Process {
        executable: String,
    },
    /// A process whose stable id is already computed.
    ///
    /// A stateful collector persists ids, not paths, so that hydrating a
    /// durable baseline never recomputes an identity from text the sanitizer
    /// rewrote. See [`crate::domain::ids::ProcessId`].
    ProcessById {
        id: crate::domain::ids::ProcessId,
    },
    Port {
        protocol: crate::domain::ids::Protocol,
        address: String,
        port: u16,
    },
    Service {
        name: String,
    },
    Application {
        hive: String,
        registry_key: String,
    },
    Driver {
        device_instance_id: String,
    },
    File {
        path: String,
    },
    Adapter {
        guid: String,
    },
    Container {
        docker_id: String,
    },
    Image {
        repository_tag: String,
    },
    DockerVolume {
        name: String,
    },
    DockerNetwork {
        name: String,
    },
    Project {
        path: String,
    },
    Update {
        id: String,
    },
    Task {
        path: String,
    },
    System,
}

/// An entity before stable-id normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntity {
    pub identity: EntityIdentity,
    pub display_name: String,
}

/// A platform-neutral fact produced by a collector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawObservation {
    pub collector_id: &'static str,
    pub kind: ObservationKind,
    pub entity: RawEntity,
    pub before: Option<Value>,
    pub after: Option<Value>,
    /// Structured config key when the observation proves one key changed.
    pub field: Option<String>,
    pub occurred_at: i64,
    pub details: Option<Value>,
    pub related_entity_ids: Vec<String>,
    pub occurred_at_is_uncertain: bool,
}

/// Cooperative cancellation checked by collector/provider page loops.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// The clock a collector timestamps its observations with.
///
/// Injectable because a collector's window logic is the part most worth testing
/// and the least worth waiting for: a churn episode closing after sixty seconds
/// of wall clock is a test that either sleeps for a minute or guesses. The
/// trait is narrow on purpose — an observation timestamp is the only time a
/// collector needs, so there is no monotonic reading here.
pub trait ObservationClock: Send + Sync + 'static {
    /// Now, as epoch milliseconds UTC.
    fn now_ms(&self) -> i64;
}

/// The production clock: the product's one reading of the system clock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SystemObservationClock;

impl ObservationClock for SystemObservationClock {
    fn now_ms(&self) -> i64 {
        crate::domain::now_ms()
    }
}

/// Shared monitoring pause gate.
#[derive(Clone)]
pub struct MonitorHandle(pub(crate) Arc<Monitoring>);

impl MonitorHandle {
    pub fn is_paused(&self) -> bool {
        self.0.is_paused()
    }
}

/// Narrow route used by notification-based collectors.
#[derive(Clone)]
pub struct EmitHandle {
    inner: Arc<dyn Fn(Vec<RawObservation>) -> AppResult<()> + Send + Sync>,
}

impl EmitHandle {
    pub(crate) fn new(
        emit: impl Fn(Vec<RawObservation>) -> AppResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(emit),
        }
    }

    pub fn emit(&self, observations: Vec<RawObservation>) -> AppResult<()> {
        (self.inner)(observations)
    }
}

/// Narrow notification callback used to request an immediate collector snapshot.
///
/// The runtime owns all scheduling and lifecycle checks. Platform notification
/// adapters only retain this handle and call it when their watched source
/// changes; the callback never performs collection or storage work itself.
#[derive(Clone)]
pub struct WakeHandle {
    inner: Arc<dyn Fn() -> AppResult<()> + Send + Sync>,
}

/// Closed, value-free reasons an OS collector may use to request a checkpoint.
/// No process path or Event Log payload crosses this route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomaticCheckpointTrigger {
    WindowsUpdateInstallStarted,
    InstallerProcessStarted,
}

/// Nonblocking, bounded route from a collector to checkpoint orchestration.
#[derive(Clone)]
pub struct AutomaticCheckpointHandle {
    inner: Arc<dyn Fn(AutomaticCheckpointTrigger) -> bool + Send + Sync>,
}

impl AutomaticCheckpointHandle {
    pub(crate) fn new(
        submit: impl Fn(AutomaticCheckpointTrigger) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(submit),
        }
    }

    #[cfg(any(test, debug_assertions, feature = "devtools"))]
    pub(crate) fn noop() -> Self {
        Self::new(|_| false)
    }

    /// Returns `false` when the bounded queue is full or the context is stale.
    pub fn try_request(&self, trigger: AutomaticCheckpointTrigger) -> bool {
        (self.inner)(trigger)
    }
}

impl WakeHandle {
    pub(crate) fn new(wake: impl Fn() -> AppResult<()> + Send + Sync + 'static) -> Self {
        Self {
            inner: Arc::new(wake),
        }
    }

    pub fn request_snapshot(&self) -> AppResult<()> {
        (self.inner)()
    }
}

/// Lifecycle-fenced collector-specific diagnostics.
///
/// Values are deliberately small, structured facts such as a driver's active
/// notification tier. A stale collector instance cannot overwrite the status
/// of its replacement.
#[derive(Clone)]
pub struct StatusDetailsHandle {
    inner:
        Arc<dyn Fn(Option<crate::domain::CollectorStatusDetails>) -> AppResult<()> + Send + Sync>,
}

impl StatusDetailsHandle {
    pub(crate) fn new(
        update: impl Fn(Option<crate::domain::CollectorStatusDetails>) -> AppResult<()>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(update),
        }
    }

    pub fn replace(&self, details: Option<crate::domain::CollectorStatusDetails>) -> AppResult<()> {
        (self.inner)(details)
    }
}

/// Capabilities passed to a collector, and nothing else.
#[derive(Clone)]
pub struct CollectorContext {
    pub monitor: MonitorHandle,
    pub emit: EmitHandle,
    pub wake: WakeHandle,
    pub status_details: StatusDetailsHandle,
    pub automatic_checkpoint: AutomaticCheckpointHandle,
    pub state: StateReadHandle,
    pub cancel: CancelToken,
}

/// One independent source of system observations.
pub trait Collector: Send + 'static {
    fn id(&self) -> &'static str;
    fn descriptor(&self) -> CollectorDescriptor;
    fn start(&mut self, ctx: CollectorContext) -> AppResult<()>;
    fn stop(&mut self);
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>>;

    fn snapshot_batch(&mut self, _context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        self.snapshot().map(CollectorSnapshot::Legacy)
    }

    fn commit_succeeded(
        &mut self,
        _token: CommitToken,
        _receipt: &crate::storage::CollectorCommitReceipt,
    ) -> AppResult<()> {
        Ok(())
    }

    fn commit_failed(&mut self, _token: CommitToken) -> AppResult<()> {
        Ok(())
    }
}
