//! Cfg-free canonical collector catalog and factory activation overlay.

use std::sync::Arc;
use std::time::Duration;

use crate::collectors::Collector;
use crate::domain::{CollectorCadence, CollectorCost, CollectorDescriptor};
use crate::error::{AppError, AppResult};

pub type CollectorFactory = Arc<dyn Fn() -> Box<dyn Collector> + Send + Sync>;

#[derive(Clone)]
pub struct RegistryEntry {
    pub descriptor: CollectorDescriptor,
    pub factory: Option<CollectorFactory>,
}

impl RegistryEntry {
    pub fn active(
        descriptor: CollectorDescriptor,
        factory: impl Fn() -> Box<dyn Collector> + Send + Sync + 'static,
    ) -> Self {
        Self {
            descriptor,
            factory: Some(Arc::new(factory)),
        }
    }

    pub fn unavailable(descriptor: CollectorDescriptor) -> Self {
        Self {
            descriptor,
            factory: None,
        }
    }
}

#[derive(Clone, Default)]
pub struct CollectorRegistry {
    entries: Vec<RegistryEntry>,
}

impl CollectorRegistry {
    pub fn new(entries: Vec<RegistryEntry>) -> Self {
        Self { entries }
    }
    pub fn entries(&self) -> &[RegistryEntry] {
        &self.entries
    }

    /// Activate one descriptor already present in the catalog.
    ///
    /// Platform composition passes only a constructor. The canonical id,
    /// presentation copy, cadence, cost and privilege declaration therefore
    /// remain owned by this cfg-free registry.
    pub fn with_factory(mut self, id: &str, factory: CollectorFactory) -> AppResult<Self> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.descriptor.id == id)
            .ok_or_else(|| registry_error("collector id is not in the canonical catalog"))?;
        if entry.descriptor.requires_admin {
            return Err(registry_error(
                "administrator-required collectors cannot be activated",
            ));
        }
        if entry.factory.is_some() {
            return Err(registry_error("collector is already active"));
        }
        entry.factory = Some(factory);
        Ok(self)
    }

    /// Closure-friendly form of [`CollectorRegistry::with_factory`].
    pub fn with_constructor(
        self,
        id: &str,
        constructor: impl Fn() -> Box<dyn Collector> + Send + Sync + 'static,
    ) -> AppResult<Self> {
        self.with_factory(id, Arc::new(constructor))
    }

    /// The product catalog. Later collector phases overlay unavailable entries
    /// with platform-backed factories without changing Settings copy or ids.
    pub fn production() -> Self {
        let definitions = [
            (
                "process",
                "Processes",
                // The blind spot is stated where the user reads it, not only in
                // the docs: detection is a one-second diff because both
                // event-tracing routes need elevation this app never asks for.
                "programs starting and stopping (checked every second — programs that live less than a second may be missed)",
                "shows which program changed the machine",
                CollectorCost::Cheap,
                CollectorCadence::Poll(Duration::from_secs(1)),
            ),
            (
                "listeners",
                "Listening ports",
                "TCP and UDP ports that begin or stop listening",
                "reveals local servers and endpoints that appeared or disappeared",
                CollectorCost::Cheap,
                CollectorCadence::Poll(Duration::from_secs(2)),
            ),
            (
                "files",
                "Watched files",
                "configuration files in folders you choose",
                "configuration changes often explain regressions",
                CollectorCost::Moderate,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(60)),
            ),
            (
                "services",
                "Services",
                "Windows services starting, stopping, being installed or removed, and changing start type",
                "background service changes can affect applications and devices",
                CollectorCost::Cheap,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(5)),
            ),
            (
                "applications",
                "Applications",
                "installed applications and version changes",
                "installs and updates are common regression boundaries",
                CollectorCost::Moderate,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(5 * 60)),
            ),
            (
                "updates",
                "Windows Update",
                "installed and failed Windows updates",
                "system updates can alter many components at once",
                CollectorCost::Cheap,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(15 * 60)),
            ),
            (
                "drivers",
                "Drivers",
                "device drivers added, removed, and version-changed",
                "driver changes can affect hardware and stability",
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(6 * 60 * 60)),
            ),
            (
                "network",
                "Network",
                "adapters, addresses, routes, DNS, and proxy settings",
                "network changes often explain sudden connectivity failures",
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(30 * 60)),
            ),
            (
                "startup",
                "Startup and logon tasks",
                "startup programs and scheduled logon tasks",
                "startup changes affect boot and background behavior",
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(60 * 60)),
            ),
            (
                "eventlog",
                "Windows event log",
                "crashes, service failures, and unexpected shutdowns",
                "corroborates changes with failures reported by Windows",
                CollectorCost::Cheap,
                CollectorCadence::Notify,
            ),
            (
                "docker",
                "Docker",
                "containers, images and volumes starting, stopping and being rebuilt",
                "container rebuilds and mappings affect local services",
                CollectorCost::Cheap,
                // Live changes wake from the stream; this sweep is the
                // absence-safe named-pipe re-probe required every 60 seconds.
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(60)),
            ),
            (
                "dev_env",
                "Development environment",
                "environment variables, PATH, installed tool versions, and the branch of folders you watch. Values are never stored.",
                "developer-tooling changes often explain local build regressions",
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(6 * 60 * 60)),
            ),
        ];
        Self::new(
            definitions
                .into_iter()
                .map(|(id, name, watches, matters, cost, cadence)| {
                    RegistryEntry::unavailable(CollectorDescriptor {
                        id,
                        display_name: name,
                        what_it_watches: watches,
                        why_it_matters: matters,
                        cost_hint: cost,
                        cadence,
                        requires_admin: false,
                    })
                })
                .collect(),
        )
    }
}

fn registry_error(reason: &'static str) -> AppError {
    AppError::InvalidInput {
        what: "collector factory",
        reason: reason.to_owned(),
    }
}
