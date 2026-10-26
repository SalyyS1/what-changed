//! Startup composition for this platform.
//!
//! This module overlays the canonical, cfg-free catalog with the factories this
//! host can actually build. It passes constructors and nothing else: descriptor
//! copy, cadence, cost and the non-elevation promise stay owned by the registry,
//! so a platform wiring change cannot quietly rewrite what Settings says a source
//! watches.

use crate::collectors::application_install_map::ApplicationInstallMap;
use crate::collectors::applications::{self, ApplicationCollector};
use crate::collectors::dev_env::{self, DevEnvironmentCollector};
use crate::collectors::docker::{self, DockerCollector};
use crate::collectors::drivers::{self, DriverCollector, DriverQueryCoordinator};
use crate::collectors::eventlog::{self, EventLogCollector};
use crate::collectors::files::{self, FileCollector};
use crate::collectors::listeners::{self, ListenerCollector};
use crate::collectors::listening_port_map::ListeningPortMap;
use crate::collectors::network::{self, NetworkCollector, NetworkQueryCoordinator};
use crate::collectors::process::{self, ProcessCollector};
use crate::collectors::process_map::ProcessMap;
use crate::collectors::registry::CollectorRegistry;
use crate::collectors::service_evidence_map::ServiceEvidenceMap;
use crate::collectors::services::{self, ServiceCollector};
use crate::collectors::startup::{self, StartupCollector, StartupQueryCoordinator};
use crate::collectors::updates::{self, UpdateCollector};
use crate::error::AppResult;
use crate::platform::windows::devices::WindowsDriverProvider;
use crate::platform::windows::eventlog::WindowsEventLogSource;
use crate::platform::windows::named_pipe::WindowsNamedPipeTransport;
use crate::platform::windows::network::WindowsNetworkProvider;
use crate::platform::windows::process::WindowsProcessProvider;
use crate::platform::windows::registry::WindowsDevEnvironmentProvider;
use crate::platform::windows::registry::WindowsRegistryUninstallProvider;
use crate::platform::windows::services::WindowsServiceProvider;
use crate::platform::windows::sockets::WindowsSocketProvider;
use crate::platform::windows::tasks::WindowsStartupProvider;
use crate::platform::windows::wua::{WindowsUpdateProvider, WuaQueryCoordinator};

/// The product catalog with every implemented Windows collector activated.
pub fn production_registry() -> CollectorRegistry {
    match activate() {
        Ok(registry) => registry,
        Err(error) => {
            // A factory that cannot be installed leaves its catalog entry
            // unavailable. Startup is not the place to fail over it: the rest of
            // the product still records, Diagnostics says which source is
            // missing, and an app that refuses to boot keeps no history at all.
            tracing::error!(
                %error,
                "collector composition failed; falling back to the canonical catalog"
            );
            CollectorRegistry::production()
        }
    }
}

/// Overlay both factories on the canonical catalog.
///
/// One process map is created here and shared by the two factories, so a single
/// enumeration serves both collectors and a port is attributed to a process the
/// recorder actually saw. The map is session-local by design: the next poll
/// rebuilds it and nothing about it is restored from disk.
fn activate() -> AppResult<CollectorRegistry> {
    let process_map = ProcessMap::new();
    let listener_map = process_map.clone();
    let port_map = ListeningPortMap::new();
    let listener_port_map = port_map.clone();
    let service_map = ServiceEvidenceMap::new();
    let file_service_map = service_map.clone();
    let application_service_map = service_map.clone();
    let application_map = ApplicationInstallMap::new();
    let service_application_map = application_map.clone();
    let driver_query_coordinator = DriverQueryCoordinator::default();
    let wua_query_coordinator = WuaQueryCoordinator::default();
    let network_query_coordinator = NetworkQueryCoordinator::default();
    let startup_query_coordinator = StartupQueryCoordinator::default();
    let process_descriptor = process::catalog_descriptor()?;
    let listener_descriptor = listeners::catalog_descriptor()?;
    let file_descriptor = files::catalog_descriptor()?;
    let service_descriptor = services::catalog_descriptor()?;
    let application_descriptor = applications::catalog_descriptor()?;
    let update_descriptor = updates::catalog_descriptor()?;
    let driver_descriptor = drivers::catalog_descriptor()?;
    let network_descriptor = network::catalog_descriptor()?;
    let startup_descriptor = startup::catalog_descriptor()?;
    let eventlog_descriptor = eventlog::catalog_descriptor()?;
    let docker_descriptor = docker::catalog_descriptor()?;
    let dev_env_descriptor = dev_env::catalog_descriptor()?;
    CollectorRegistry::production()
        .with_constructor(process::ID, move || {
            Box::new(ProcessCollector::new(
                WindowsProcessProvider,
                process_map.clone(),
                process_descriptor.clone(),
            ))
        })?
        .with_constructor(listeners::ID, move || {
            Box::new(ListenerCollector::with_port_map(
                WindowsSocketProvider,
                listener_map.clone(),
                listener_port_map.clone(),
                listener_descriptor.clone(),
            ))
        })?
        .with_constructor(files::ID, move || {
            Box::new(FileCollector::with_service_map(
                port_map.clone(),
                file_service_map.clone(),
                file_descriptor.clone(),
            ))
        })?
        .with_constructor(services::ID, move || {
            Box::new(ServiceCollector::with_evidence_maps(
                WindowsServiceProvider::default(),
                service_map.clone(),
                service_application_map.clone(),
                service_descriptor.clone(),
            ))
        })?
        .with_constructor(applications::ID, move || {
            Box::new(ApplicationCollector::with_evidence_maps(
                WindowsRegistryUninstallProvider,
                application_map.clone(),
                application_service_map.clone(),
                application_descriptor.clone(),
            ))
        })?
        .with_constructor(updates::ID, move || {
            let provider = WindowsUpdateProvider::with_query_coordinator(
                wua_query_coordinator.clone(),
            ).unwrap_or_else(|error| {
                tracing::debug!(%error, "WUA apartment unavailable; collector will report the failure");
                WindowsUpdateProvider::failed(error.to_string())
            });
            Box::new(UpdateCollector::new(provider, update_descriptor.clone()))
        })?
        .with_constructor(drivers::ID, move || {
            Box::new(DriverCollector::with_query_coordinator(
                WindowsDriverProvider::default(),
                driver_descriptor.clone(),
                driver_query_coordinator.clone(),
            ))
        })?
        .with_constructor(network::ID, move || {
            Box::new(NetworkCollector::with_query_coordinator(
                WindowsNetworkProvider::default(),
                network_descriptor.clone(),
                network_query_coordinator.clone(),
            ))
        })?
        .with_constructor(startup::ID, move || {
            let provider = WindowsStartupProvider::new().unwrap_or_else(|error| {
                tracing::debug!(%error, "Task Scheduler apartment unavailable; collector will report the failure");
                WindowsStartupProvider::failed(error.to_string())
            });
            Box::new(StartupCollector::with_query_coordinator(
                provider,
                startup_descriptor.clone(),
                startup_query_coordinator.clone(),
            ))
        })?
        .with_constructor(eventlog::ID, move || {
            Box::new(EventLogCollector::new(
                WindowsEventLogSource::new(),
                eventlog_descriptor.clone(),
            ))
        })?
        .with_constructor(docker::ID, move || {
            Box::new(DockerCollector::new(
                WindowsNamedPipeTransport::docker(),
                docker_descriptor.clone(),
            ))
        })?
        .with_constructor(dev_env::ID, move || {
            Box::new(DevEnvironmentCollector::new(
                WindowsDevEnvironmentProvider,
                dev_env_descriptor.clone(),
            ))
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn composition_activates_exactly_the_sources_this_platform_backs() {
        let composed = production_registry();
        let canonical = CollectorRegistry::production();
        assert_eq!(
            composed.entries().len(),
            canonical.entries().len(),
            "composition adds factories, never catalog entries"
        );

        let mut activated = Vec::new();
        for (composed, canonical) in composed.entries().iter().zip(canonical.entries()) {
            assert_eq!(
                composed.descriptor, canonical.descriptor,
                "the catalog still owns the copy, the cadence and the cost"
            );
            if composed.factory.is_some() {
                activated.push(composed.descriptor.id);
            }
        }
        assert_eq!(
            activated,
            vec![
                "process",
                "listeners",
                "files",
                "services",
                "applications",
                "updates",
                "drivers",
                "network",
                "startup",
                "eventlog",
                "docker",
                "dev_env",
            ]
        );
    }

    #[test]
    fn every_composed_collector_reports_the_id_the_catalog_gave_it() {
        for entry in production_registry().entries() {
            let Some(factory) = entry.factory.as_ref() else {
                continue;
            };
            let collector = factory();
            assert_eq!(collector.id(), entry.descriptor.id);
            assert_eq!(collector.descriptor(), entry.descriptor);
        }
    }
}
