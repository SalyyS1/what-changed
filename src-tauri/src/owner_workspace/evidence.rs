//! Manual evidence preserves observations and frozen expectations separately.

mod observations;
mod scrub;

pub use crate::dto::owner_evidence_dto::*;
pub(crate) use observations::{derive_result, summarize};
pub const MANUAL_SEMANTICS_VERSION: &str = "owner-manual/v1";
