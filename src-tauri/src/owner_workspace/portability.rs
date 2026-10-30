//! Versioned portable memory. Imported observations are archival, never current.
pub mod cache;
mod records;
mod review;
pub use records::{PortableHistory, PortableRecords, PortableWorkspace};
pub use review::{prepare_bundle, validate_bundle};

pub const PORTABLE_SCHEMA: u32 = 1;
pub const MAX_RECORDS: usize = 2000;

pub const IMPORT_LIMITATIONS: &[&str] = &[
    "Imported evidence and scenario definitions are historical and untrusted; they cannot approve a current change.",
    "No project root, environment permission, browser session or execution consent is restored.",
    "Only behavior rules explicitly reviewed and confirmed below become active. Configure a new target and confirm scenarios before checking again.",
];
