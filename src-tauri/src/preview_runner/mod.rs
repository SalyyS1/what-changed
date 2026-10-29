//! Native authority for frozen, explicitly permitted test-preview sessions.
mod bundle;
pub mod captures;
#[cfg(windows)]
mod clock_guard;
mod consent;
mod coordinator;
mod export;
mod protocol;
mod retention;
mod session_control;
mod types;
#[cfg(windows)]
mod windows_job;
mod worker;
pub use coordinator::PreviewRunner;
pub use export::{export_recipe, recipe_archive};
pub use retention::CaptureRetention;
pub use types::*;
pub const SEMANTICS: &str = "owner-browser/v1";
pub(crate) fn invalid(reason: &str) -> crate::error::AppError {
    crate::error::AppError::InvalidInput {
        what: "preview runner",
        reason: reason.into(),
    }
}
pub(super) fn digest(value: &impl serde::Serialize) -> crate::error::AppResult<String> {
    use sha2::{Digest, Sha256};
    let bytes =
        serde_json::to_vec(value).map_err(|_| invalid("Run inputs could not be encoded"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
