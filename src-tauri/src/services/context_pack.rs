//! Deterministic, redacted Markdown handoff for external AI assistants.
//!
//! This module consumes persisted investigation presentation data only. It
//! never re-queries raw event metadata, reads a project folder, or contacts an
//! AI provider. The final rendered text passes through the same sanitizer used
//! by persistence before it crosses the IPC boundary.

use crate::domain::{ConfidenceClass, Investigation, NoCauseReason, Suspect};
use crate::privacy::Sanitizer;
use crate::storage::project_repository::Project;

pub const MAX_PACK_CHARS: usize = 24_000;
const MAX_SUSPECTS: usize = 8;
const MAX_CHAIN_STEPS: usize = 6;

pub fn investigation_markdown(investigation: &Investigation, project: Option<&Project>) -> String {
    let mut output = String::new();
    output.push_str("# What Changed — AI troubleshooting context\n\n");
    output.push_str("Use this as local evidence, not proof of causation. What Changed reports recorded correlation and relationships; it does not claim that a change caused the problem.\n\n");
    output.push_str("## Symptom\n\n");
    output.push_str(&format!("{}\n\n", line(&investigation.problem)));
    output.push_str("## Window\n\n");
    output.push_str(&format!(
        "- Requested: {}\n- Resolved UTC milliseconds: {} to {}\n- Recorded changes: {}\n- Analysed changes: {}\n",
        line(&format!("{:?}", investigation.window)),
        investigation.resolved_window.from_ms,
        investigation.resolved_window.to_ms,
        investigation.total_events_in_window,
        investigation.analyzed_events,
    ));
    if investigation.dropped_events > 0 {
        output.push_str(&format!(
            "- Routine changes omitted by the analysis bound: {}\n",
            investigation.dropped_events
        ));
    }
    output.push('\n');

    output.push_str("## Workspace\n\n");
    match project {
        Some(project) => {
            output.push_str(&format!("- Name: {}\n", line(&project.name)));
            match project.baseline_checkpoint_id {
                Some(id) => output.push_str(&format!(
                    "- Known-good baseline: checkpoint {} ({})\n",
                    id,
                    project
                        .baseline_checkpoint_name
                        .as_deref()
                        .unwrap_or("unnamed")
                )),
                None => output.push_str("- Known-good baseline: not set\n"),
            }
        }
        None if investigation.project_id.is_some() => output.push_str(
            "- Workspace: unavailable (the registered project was removed or is no longer readable).\n",
        ),
        None => output.push_str("- No workspace was selected for this handoff.\n"),
    }
    output.push('\n');

    match (
        &investigation.no_cause_reason,
        investigation.no_cause_message.as_deref(),
    ) {
        (Some(reason), message) => {
            output.push_str("## Result\n\n");
            output.push_str(&format!(
                "- No ranked change cleared the reporting threshold.\n- Reason: {}\n",
                no_cause(*reason)
            ));
            if let Some(message) = message {
                output.push_str(&format!("- Guidance: {}\n", line(message)));
            }
            output.push_str("\n## Suggested next step\n\nWiden the window or keep recording until the next meaningful change is observed.\n");
        }
        (None, _) => {
            output.push_str("## Ranked evidence\n\n");
            for (index, suspect) in investigation.suspects.iter().take(MAX_SUSPECTS).enumerate() {
                append_suspect(&mut output, index + 1, suspect);
            }
            if investigation.suspects.is_empty() {
                output.push_str("No suspect cleared the reporting threshold.\n");
            }
            output.push('\n');
            output.push_str("## Suggested next step\n\n");
            if let Some(suspect) = investigation.suspects.first() {
                output.push_str(&line(&suspect.what_to_try));
            } else {
                output.push_str("Describe what changed next, then run another investigation.");
            }
            output.push('\n');
        }
    }
    output.push_str("\n## Privacy boundary\n\nValues are redacted before storage and before this handoff. This pack contains only the bounded investigation snapshot and optional workspace label; it does not include raw event metadata, secret values, or a full event dump.\n");

    let (sanitized, _) = Sanitizer::new().sanitize(output);
    let mut output = sanitized.into_inner();
    if output.chars().count() > MAX_PACK_CHARS {
        output = output.chars().take(MAX_PACK_CHARS).collect();
        output.push_str("\n\n[Pack truncated at the privacy-safe size limit.]");
    }
    output
}

fn append_suspect(output: &mut String, rank: usize, suspect: &Suspect) {
    output.push_str(&format!(
        "### {}. {}\n\n- Event id: {}\n- Time UTC milliseconds: {}\n- Confidence: {}\n- Score: {:.3}\n- Evidence sentence: {}\n",
        rank,
        line(&suspect.title),
        suspect.event_id,
        suspect.timestamp_ms,
        confidence(suspect.confidence),
        suspect.score,
        line(&suspect.confidence_text),
    ));
    if !suspect.why_ranked.is_empty() {
        output.push_str("- Why it ranked:\n");
        for reason in suspect.why_ranked.iter().take(6) {
            output.push_str(&format!("  - {}\n", line(reason)));
        }
    }
    if !suspect.chain.is_empty() {
        output.push_str("- Recorded relationship chain:\n");
        for step in suspect.chain.iter().take(MAX_CHAIN_STEPS) {
            output.push_str(&format!("  - {}\n", line(&step.label)));
        }
    }
    output.push_str(&format!(
        "- What to verify next: {}\n\n",
        line(&suspect.what_to_try)
    ));
}

fn line(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

fn confidence(value: ConfidenceClass) -> &'static str {
    match value {
        ConfidenceClass::Confirmed => "confirmed",
        ConfidenceClass::Likely => "likely",
        ConfidenceClass::Correlated => "correlated",
    }
}

fn no_cause(value: NoCauseReason) -> &'static str {
    match value {
        NoCauseReason::NothingChanged => "nothing meaningful changed",
        NoCauseReason::OnlyRoutineChanges => "only routine changes were observed",
        NoCauseReason::WindowTooNarrow => "the window may be too narrow",
        NoCauseReason::NotWatchingYet => "recording had not started for this window",
        NoCauseReason::NoRecordedEvidenceInWorkspace => {
            "no recorded evidence was associated with this workspace"
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::{InvestigationWindow, ResolvedInvestigationWindow, ScoreBreakdown};

    fn investigation(problem: &str) -> Investigation {
        Investigation {
            id: 1,
            project_id: None,
            problem: problem.to_owned(),
            window: InvestigationWindow::Today,
            resolved_window: ResolvedInvestigationWindow {
                from_ms: 10,
                to_ms: 20,
            },
            suspects: Vec::new(),
            no_cause_reason: Some(NoCauseReason::NothingChanged),
            no_cause_message: Some("Try a wider window".to_owned()),
            total_events_in_window: 2,
            analyzed_events: 2,
            dropped_events: 0,
            created_at: 20,
        }
    }

    #[test]
    fn empty_pack_is_deterministic_and_mentions_the_privacy_boundary() {
        let output = investigation_markdown(&investigation("API stopped; token=secret"), None);
        assert!(output.contains("No ranked change cleared"));
        assert!(output.contains("Privacy boundary"));
        assert!(!output.contains("token=secret"));
        assert!(output.contains("[REDACTED]"));
        assert_eq!(
            output,
            investigation_markdown(&investigation("API stopped; token=secret"), None)
        );
    }

    #[test]
    fn pack_bounds_large_persisted_text() {
        let mut value = investigation("problem");
        value.no_cause_reason = None;
        value.suspects = (0..20)
            .map(|index| crate::domain::Suspect {
                event_id: index,
                title: "x".repeat(4_000),
                timestamp_ms: index,
                score: 1.0,
                breakdown: ScoreBreakdown::default(),
                confidence: ConfidenceClass::Correlated,
                confidence_text: "correlated".to_owned(),
                relative_time: String::new(),
                why_ranked: Vec::new(),
                chain: Vec::new(),
                what_to_try: "check".to_owned(),
            })
            .collect();
        assert!(investigation_markdown(&value, None).chars().count() <= MAX_PACK_CHARS + 80);
    }

    #[test]
    fn removed_workspace_is_explicit_without_blocking_export() {
        let mut value = investigation("workspace failure");
        value.project_id = Some("project:removed".to_owned());
        let output = investigation_markdown(&value, None);
        assert!(output.contains("unavailable"));
        assert!(output.contains("project was removed"));
    }
}
