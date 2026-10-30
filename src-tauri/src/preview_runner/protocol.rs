//! Child output is untrusted; the parent owns assertion membership and completion.
use super::{invalid, types::PreviewRunnerEventDto};
use crate::dto::owner_evidence_dto::*;
use crate::error::AppResult;
use crate::owner_workspace::{LocatorDto, ResultStatus};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

pub(super) enum Action {
    None,
    Authorize(String),
    Complete(OwnerExecutionSummaryDto),
    Capture(Option<super::PreviewCaptureDto>),
}
pub(super) struct Transcript {
    run: OwnerManualRunDto,
    nonce: String,
    sequence: u32,
    scope: String,
    order: Vec<(String, String)>,
    next: usize,
    steps: BTreeSet<String>,
    assertions: BTreeSet<String>,
    pub results: BTreeMap<String, RecordOwnerManualResultInput>,
    pub login_roles: BTreeSet<String>,
    pub completed: bool,
    ready: bool,
    recording: bool,
    mode: String,
    capture: Option<(String, String)>,
}
fn text<'a>(frame: &'a Value, key: &str) -> AppResult<&'a str> {
    frame
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| text.len() <= 1000)
        .ok_or_else(|| invalid("Invalid runner protocol field"))
}
impl Transcript {
    pub fn new(run: OwnerManualRunDto, nonce: String, scope: String, mode: String) -> Self {
        let order = run
            .manifest
            .scenarios
            .iter()
            .flat_map(|scenario| {
                scenario
                    .recipe
                    .steps
                    .iter()
                    .map(move |step| (scenario.id.clone(), step.id.clone()))
            })
            .collect();
        let results = run
            .manifest
            .scenarios
            .iter()
            .map(|scenario| {
                (
                    scenario.id.clone(),
                    RecordOwnerManualResultInput {
                        operation_id: uuid::Uuid::new_v4().to_string(),
                        id: uuid::Uuid::new_v4().to_string(),
                        app_id: run.app_id.clone(),
                        run_id: run.id.clone(),
                        expected_run_version: run.version,
                        scenario_id: scenario.id.clone(),
                        scenario_revision: scenario.revision,
                        observer: "Packaged browser runner".into(),
                        observed_at: crate::domain::now_ms(),
                        observed_steps: vec![],
                        assertions: vec![],
                        limitation: None,
                    },
                )
            })
            .collect();
        Self {
            run,
            nonce,
            sequence: 0,
            scope,
            order,
            next: 0,
            steps: BTreeSet::new(),
            assertions: BTreeSet::new(),
            results,
            login_roles: BTreeSet::new(),
            completed: false,
            ready: false,
            recording: false,
            mode,
            capture: None,
        }
    }
    pub fn request_capture(&mut self, id: String, role: String) -> AppResult<()> {
        if self.capture.is_some() {
            return Err(invalid("A capture is already pending"));
        }
        self.capture = Some((id, role));
        Ok(())
    }
    pub fn accept(&mut self, frame: &Value) -> AppResult<(PreviewRunnerEventDto, Action)> {
        if self.completed
            || frame["protocol"] != 1
            || text(frame, "run_id")? != self.run.id
            || text(frame, "nonce")? != self.nonce
            || text(frame, "manifest_digest")? != self.run.manifest_digest
            || frame["sequence"].as_u64() != Some(u64::from(self.sequence) + 1)
        {
            return Err(invalid(
                "Runner message identity, order or completion mismatch",
            ));
        }
        let kind = text(frame, "type")?;
        let fields: &[&str] = match kind {
            "ready" => &["roles"],
            "needs_login" => &["role"],
            "recording" => &[],
            "locator" => &["locator"],
            "before_step" | "step" => &["scenario_id", "scenario_revision", "step_id", "observed"],
            "assertion" => &[
                "scenario_id",
                "scenario_revision",
                "step_id",
                "assertion_id",
                "status",
                "observed",
            ],
            "complete" => &["identity", "revision"],
            "capture" => &["capture_id", "role", "data_url"],
            "capture_unavailable" => &["capture_id", "role"],
            "interrupted" => &["reason"],
            _ => return Err(invalid("Unknown runner message")),
        };
        let object = frame
            .as_object()
            .ok_or_else(|| invalid("Invalid runner frame"))?;
        if object.keys().any(|key| {
            ![
                "protocol",
                "run_id",
                "nonce",
                "manifest_digest",
                "sequence",
                "type",
            ]
            .contains(&key.as_str())
                && !fields.contains(&key.as_str())
        }) {
            return Err(invalid("Unexpected runner message field"));
        }
        self.sequence += 1;
        let mut event = PreviewRunnerEventDto {
            sequence: self.sequence,
            kind: kind.into(),
            message: String::new(),
            scenario_id: None,
            assertion_id: None,
            status: None,
            locator: None,
        };
        let mut action = Action::None;
        match kind {
            "ready" => {
                let expected = self
                    .run
                    .manifest
                    .scenarios
                    .iter()
                    .flat_map(|scenario| scenario.recipe.role_refs.iter().cloned())
                    .collect::<BTreeSet<_>>();
                let roles: Vec<String> = serde_json::from_value(frame["roles"].clone())
                    .map_err(|_| invalid("Invalid ready roles"))?;
                if self.ready
                    || roles.len() != expected.len()
                    || roles.into_iter().collect::<BTreeSet<_>>() != expected
                {
                    return Err(invalid(
                        "Unexpected ready roles or repeated browser initialization",
                    ));
                }
                self.ready = true;
                event.message = "Isolated test browser ready.".into();
            }
            "needs_login" => {
                let role = text(frame, "role")?;
                if !self.ready
                    || !self.run.manifest.scenarios.iter().any(|scenario| {
                        scenario
                            .recipe
                            .role_refs
                            .iter()
                            .any(|expected| expected == role)
                    })
                    || !self.login_roles.insert(role.into())
                {
                    return Err(invalid("Unexpected test role"));
                }
                event.message = format!(
                    "Sign into the isolated {role} test session, then confirm login in Ownvero."
                );
            }
            "recording" => {
                if self.mode != "record" || self.recording {
                    return Err(invalid("Unexpected locator-selection state"));
                }
                self.recording = true;
                event.message =
                    "Select a supported element in the test browser; review its suggested locator."
                        .into();
            }
            "locator" => {
                if !self.recording {
                    return Err(invalid("Locator arrived outside selection mode"));
                }
                let locator: LocatorDto = serde_json::from_value(frame["locator"].clone())
                    .map_err(|_| invalid("Unsupported recorded locator"))?;
                let encoded =
                    serde_json::to_string(&locator).map_err(|_| invalid("Locator encoding"))?;
                let (_, redactions) = crate::privacy::Sanitizer::new().sanitize(encoded.clone());
                if encoded.len() > 500 || !redactions.is_empty() {
                    return Err(invalid(
                        "Recorded locator is too long or resembles secret data",
                    ));
                }
                event.locator = Some(locator);
                event.message =
                    "Review this locator before adding it to a new scenario revision.".into();
            }
            "capture" | "capture_unavailable" => {
                let (id, role) = self
                    .capture
                    .take()
                    .ok_or_else(|| invalid("Unrequested browser capture"))?;
                if text(frame, "capture_id")? != id || text(frame, "role")? != role {
                    return Err(invalid("Capture identity mismatch"));
                }
                let capture = if kind == "capture" {
                    let data = frame["data_url"]
                        .as_str()
                        .ok_or_else(|| invalid("Missing raw capture"))?;
                    Some(super::captures::pending(id, role, data.into())?)
                } else {
                    None
                };
                event.message = if capture.is_some() {
                    "Sensitive raw capture ready for review; it has not been retained."
                } else {
                    "The selected test surface could not be captured."
                }
                .into();
                action = Action::Capture(capture);
            }
            "before_step" | "step" | "assertion" => {
                let scenario_id = text(frame, "scenario_id")?;
                let step_id = text(frame, "step_id")?;
                let scenario = self
                    .run
                    .manifest
                    .scenarios
                    .iter()
                    .find(|scenario| {
                        scenario.id == scenario_id
                            && frame["scenario_revision"].as_u64()
                                == Some(u64::from(scenario.revision))
                    })
                    .ok_or_else(|| invalid("Unknown frozen scenario"))?;
                let step = scenario
                    .recipe
                    .steps
                    .iter()
                    .find(|step| step.id == step_id)
                    .ok_or_else(|| invalid("Unknown frozen step"))?;
                event.scenario_id = Some(scenario_id.into());
                if kind == "before_step" {
                    if self.mode != "run" || !self.ready || self.login_roles.is_empty() {
                        return Err(invalid("Action arrived before test-session preparation"));
                    }
                    if self.order.get(self.next) != Some(&(scenario_id.into(), step_id.into())) {
                        return Err(invalid("Runner action order changed"));
                    }
                    self.next += 1;
                    action = Action::Authorize(step_id.into());
                    event.message = "Rechecking the frozen scope before the next action.".into();
                } else {
                    if self.next == 0
                        || self.order.get(self.next - 1)
                            != Some(&(scenario_id.into(), step_id.into()))
                    {
                        return Err(invalid("Observation arrived outside its authorized action"));
                    }
                    let result = self
                        .results
                        .get_mut(scenario_id)
                        .ok_or_else(|| invalid("Missing scenario result"))?;
                    if kind == "step" {
                        if !self.steps.insert(step_id.into()) {
                            return Err(invalid("Duplicate step completion"));
                        }
                        result.observed_steps.push(ManualStepObservationDto {
                            step_id: step_id.into(),
                            observed: "Approved action completed.".into(),
                        });
                        event.message = "Approved action completed.".into();
                    } else {
                        let id = text(frame, "assertion_id")?;
                        if !step.assertions.iter().any(|assertion| assertion.id == id)
                            || !self.assertions.insert(id.into())
                        {
                            return Err(invalid("Unknown or duplicate assertion"));
                        }
                        let status: ResultStatus = serde_json::from_value(frame["status"].clone())
                            .map_err(|_| invalid("Invalid assertion status"))?;
                        if status == ResultStatus::Passed && !self.steps.contains(step_id) {
                            return Err(invalid("A passed assertion lacks an observed action"));
                        }
                        let observed = match status {
                            ResultStatus::Passed => "The named assertion matched.",
                            ResultStatus::Failed => "The named assertion did not match.",
                            _ => "The action, role, locator or adapter was unavailable.",
                        }
                        .to_owned();
                        result.assertions.push(ManualAssertionObservationInput {
                            assertion_id: id.into(),
                            status,
                            observed: observed.clone(),
                            limitation: if status == ResultStatus::Blocked {
                                Some("No pass inferred from unavailable evidence.".into())
                            } else {
                                None
                            },
                        });
                        event.assertion_id = Some(id.into());
                        event.status = Some(status);
                        event.message = observed;
                    }
                }
            }
            "complete" => {
                let expected = self
                    .run
                    .manifest
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.recipe.steps)
                    .map(|step| step.assertions.len())
                    .sum::<usize>();
                if self.next != self.order.len() || self.assertions.len() != expected {
                    return Err(invalid("Runner completion omitted expected assertions"));
                }
                let identity = text(frame, "identity")?;
                let revision = frame["revision"].as_str().map(str::to_owned);
                if self.mode != "run"
                    || !["unknown", "environment_reported"].contains(&identity)
                    || (identity == "environment_reported"
                        && revision.as_ref() != Some(&self.run.manifest.candidate.identity))
                    || (identity == "unknown" && !frame["revision"].is_null())
                {
                    return Err(invalid("Runner build identity mismatch"));
                }
                action = Action::Complete(OwnerExecutionSummaryDto {
                    runner_semantics: super::SEMANTICS.into(),
                    scope_digest: self.scope.clone(),
                    identity: identity.into(),
                    revision,
                });
                self.completed = true;
                event.message =
                    "All expected assertion outcomes received; owner review is still required."
                        .into();
            }
            "interrupted" => {
                return Err(invalid(
                    "Preview execution was interrupted; partial observations remain partial",
                ))
            }
            _ => unreachable!(),
        }
        Ok((event, action))
    }
}
