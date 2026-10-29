//! Local notification policy and delivery primitives.
//!
//! The policy is intentionally independent from Tauri.  A collector can
//! evaluate an event on a worker thread without touching the UI runtime, and
//! tests can replay a day of history without creating a Windows toast.

pub mod copy;
pub mod delivery;
pub mod policy;
pub mod rate_limit;

pub use copy::why_you_might_care;
pub use policy::{should_notify, NotificationContext, NotificationCopy};
pub use rate_limit::{NotificationRateLimiter, QuietHours, RateLimitDecision};
