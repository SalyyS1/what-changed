use crate::dto::{owner_evidence_dto::*, owner_memory_dto::ProjectDecisionRevisionDto};
use crate::error::AppResult;
use crate::owner_workspace::{validation, *};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableRecords {
    pub app: OwnerAppDto,
    pub capabilities: Vec<OwnerCapabilityDto>,
    pub rules: Vec<OwnerRuleRevisionDto>,
    pub changes: Vec<OwnerChangeDraftDto>,
    pub environments: Vec<OwnerEnvironmentRevisionDto>,
    pub candidates: Vec<OwnerCandidateRevisionDto>,
    pub scenarios: Vec<ScenarioRevisionDto>,
    pub project_decisions: Vec<ProjectDecisionRevisionDto>,
    pub runs: Vec<OwnerManualRunDto>,
    pub results: Vec<OwnerManualResultDto>,
    pub decisions: Vec<OwnerEvidenceDecisionDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableHistory {
    pub source_digest: String,
    pub records: PortableRecords,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableWorkspace {
    pub schema_version: u32,
    pub records: PortableRecords,
    pub history: Vec<PortableHistory>,
}

/// Compare the typed re-encoding recursively so unknown fields in older DTOs
/// cannot be silently discarded by their more permissive serde derives.
pub fn reject_unknown(original: &Value, typed: &Value) -> AppResult<()> {
    match (original, typed) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                let other = b.get(key).ok_or_else(|| {
                    validation::invalid("bundle schema", "contains unsupported fields")
                })?;
                reject_unknown(value, other)?;
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Err(validation::invalid("bundle schema", "record count changed"));
            }
            for (x, y) in a.iter().zip(b) {
                reject_unknown(x, y)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl PortableWorkspace {
    pub fn validate(&self) -> AppResult<()> {
        if self.schema_version != super::PORTABLE_SCHEMA || self.history.len() > 20 {
            return Err(validation::invalid(
                "bundle schema",
                "unsupported schema or history count",
            ));
        }
        self.records.validate()?;
        for history in &self.history {
            history.records.validate()?;
        }
        Ok(())
    }
}

impl PortableRecords {
    pub fn validate(&self) -> AppResult<()> {
        validation::uuid(&self.app.id, "source app id")?;
        validation::required(&self.app.name, "source app name")?;
        let value = serde_json::to_value(self)
            .map_err(|_| validation::invalid("bundle", "cannot encode"))?;
        // Bound each record independently; the complete bundle has a separate
        // 16 MiB bound and a maximum of 2,000 records per collection.
        for (key, records) in value
            .as_object()
            .ok_or_else(|| validation::invalid("bundle", "requires records"))?
        {
            if key == "app" {
                validation::payload(records)?;
                continue;
            }
            let records = records
                .as_array()
                .ok_or_else(|| validation::invalid("bundle", "requires record arrays"))?;
            if records.len() > super::MAX_RECORDS {
                return Err(validation::invalid("bundle", "too many records"));
            }
            for record in records {
                validation::payload(record)?;
                if record.get("app_id").and_then(Value::as_str) != Some(self.app.id.as_str()) {
                    return Err(validation::invalid(
                        "bundle ownership",
                        "record belongs to another app",
                    ));
                }
            }
        }
        let mut capabilities = BTreeMap::new();
        for cap in &self.capabilities {
            validation::uuid(&cap.id, "capability id")?;
            if capabilities.insert(&cap.id, cap).is_some() {
                return Err(validation::invalid("bundle", "duplicate capability"));
            }
        }
        let mut rules = BTreeMap::new();
        for rule in &self.rules {
            validation::uuid(&rule.rule_id, "rule id")?;
            validation::required(&rule.expected_behavior, "behavior rule")?;
            if rule.revision == 0
                || !capabilities.contains_key(&rule.capability_id)
                || rules.insert((&rule.rule_id, rule.revision), rule).is_some()
            {
                return Err(validation::invalid(
                    "bundle rules",
                    "duplicate or invalid revision/reference",
                ));
            }
        }
        for scenario in &self.scenarios {
            validation::uuid(&scenario.id, "scenario id")?;
            if scenario.revision == 0 || !capabilities.contains_key(&scenario.capability_id) {
                return Err(validation::invalid(
                    "bundle scenario",
                    "invalid capability/revision",
                ));
            }
            // Portable scenario definitions remain archival on import. Current
            // evidence semantics must not erase older immutable expectations.
            validation::archived_recipe(&scenario.recipe)?;
            for rule in &scenario.rules {
                if !rules.contains_key(&(&rule.rule_id, rule.revision)) {
                    return Err(validation::invalid(
                        "bundle scenario",
                        "missing rule revision",
                    ));
                }
            }
        }
        for change in &self.changes {
            validation::uuid(&change.id, "change id")?;
            for rule in &change.preserved_rules {
                if !rules.contains_key(&(&rule.rule_id, rule.revision)) {
                    return Err(validation::invalid(
                        "bundle change",
                        "missing rule revision",
                    ));
                }
            }
        }
        Ok(())
    }
}
