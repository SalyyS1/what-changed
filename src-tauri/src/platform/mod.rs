//! The operating-system boundary.
//!
//! `traits` is platform-neutral and is what collectors compile against.
//! `windows` is the only place a Win32 call may appear, and the only place that
//! knows a factory exists for a given collector. `tests/architecture.rs` keeps
//! `domain/`, `analysis/` and `collectors/` free of a target `cfg`, so the
//! collection *policy* stays testable on any host and the collection
//! *mechanism* stays in one audited place.

pub mod memory;
pub mod safe_file;
pub mod traits;

#[cfg(windows)]
pub mod windows;

/// The product catalog with every platform-backed factory this host can offer.
///
/// Activation is best-effort and never fatal: a factory that cannot be
/// installed leaves its catalog entry unavailable, so a partial platform still
/// boots the rest of the product. Descriptor copy, cadence, cost and the
/// non-elevation promise stay owned by the cfg-free registry — composition
/// passes constructors and nothing else.
#[cfg(windows)]
pub use windows::composition::production_registry;

#[cfg(windows)]
pub fn start_resume_monitor(
    wake: crate::collectors::runtime::ResumeWakeHandle,
) -> crate::error::AppResult<Box<dyn Send>> {
    Ok(Box::new(windows::resume::WindowsResumeMonitor::start(
        wake,
    )?))
}

/// No platform-backed collector exists on this host, so every catalog entry
/// stays unavailable and the app still runs.
#[cfg(not(windows))]
pub fn production_registry() -> crate::collectors::registry::CollectorRegistry {
    crate::collectors::registry::CollectorRegistry::production()
}

#[cfg(not(windows))]
pub fn start_resume_monitor(
    _wake: crate::collectors::runtime::ResumeWakeHandle,
) -> crate::error::AppResult<Box<dyn Send>> {
    Ok(Box::new(()))
}
