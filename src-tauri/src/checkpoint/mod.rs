//! Checkpoint capture preparation and collector-neutral comparison.

pub mod auto;
pub mod compare;
pub mod create;
pub mod service;
pub mod state_items;

pub use compare::compare_item_sets;
pub use create::{default_checkpoint_name, prepare_capture};
pub use service::CheckpointService;
