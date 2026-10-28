//! Application wiring: where the files live, the state every command reaches
//! through, and the parts of the app that are about being an application
//! rather than about recording change — the tray, the window's close
//! behaviour, the throttled event bus, and input validation.
//!
//! Contains no business rules. A rule about *what a change means* belongs in
//! `domain/` or `analysis/`; a rule about *what happens when the user closes
//! the window* belongs here.

pub mod data_lifecycle;
pub mod data_restore;
pub mod event_bus;
pub mod lifecycle;
pub mod monitoring;
pub mod paths;
pub mod rate_limit;
pub mod state;
pub mod tray;
pub mod validate;
pub mod window;

pub use data_lifecycle::DataLifecycle;
pub use event_bus::EventBus;
pub use monitoring::Monitoring;
pub use paths::AppPaths;
pub use state::{AppState, StartupGuard, TrayReflector};
