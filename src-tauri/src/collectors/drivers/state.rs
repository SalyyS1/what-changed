use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::collectors::stateful;
use crate::storage::Baseline;

/// One present device and the driver package properties used for diffing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRecord {
    pub device_instance_id: String,
    pub device_description: String,
    pub class_name: String,
    pub manufacturer: String,
    pub driver_version: Option<String>,
    pub driver_date: Option<String>,
    pub provider_name: Option<String>,
}

impl DriverRecord {
    pub(super) fn key(&self) -> String {
        self.device_instance_id.to_ascii_uppercase()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DriverSnapshot {
    pub acknowledged: BTreeMap<String, DriverRecord>,
    #[serde(default)]
    pub pending: Option<stateful::PendingTarget<BTreeMap<String, DriverRecord>>>,
    pub established: bool,
}

impl DriverSnapshot {
    pub(super) fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        stateful::hydrate(baseline, now_ms, "the present-driver table")
    }
}

pub(super) fn index(rows: Vec<DriverRecord>) -> BTreeMap<String, DriverRecord> {
    rows.into_iter().map(|row| (row.key(), row)).collect()
}
