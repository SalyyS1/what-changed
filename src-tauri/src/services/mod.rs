//! Application services over stored events.
//!
//! Commands validate transport input and delegate here. These modules own
//! query composition that spans repositories and the product copy derived from
//! results; no Tauri type crosses this boundary.

pub mod cluster;
pub mod context_pack;
pub mod investigation_service;
pub mod query_service;
pub mod search_service;
pub mod summary_service;
