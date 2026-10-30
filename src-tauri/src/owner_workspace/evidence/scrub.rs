//! New evidence roots use the same Sanitizer chokepoint as the owner repositories.

use crate::dto::owner_evidence_dto::*;
use crate::dto::owner_workspace_dto::{
    OwnerCandidateRevisionDto, OwnerChangeDraftDto, OwnerEnvironmentRevisionDto,
    OwnerRuleRevisionDto, ScenarioRevisionDto,
};
use crate::privacy::{Redaction, Sanitizer, Scrub};

macro_rules! fields {
    ($ty:ty; $($field:ident),* $(,)?) => {
        impl Scrub for $ty {
            fn scrub_in_place(&mut self, sanitizer:&Sanitizer, parent:&str, report:&mut Vec<Redaction>) {
                $(self.$field.scrub_in_place(sanitizer,&format!("{parent}.{}",stringify!($field)),report);)*
                let _ = (sanitizer,parent,report);
            }
        }
    };
}
fields!(StartOwnerManualRunInput;);
fields!(FinishOwnerManualRunInput;);
fields!(RecordOwnerManualResultInput; observer, observed_steps, assertions, limitation);
fields!(ManualStepObservationDto; observed);
fields!(ManualAssertionObservationInput; observed, limitation);
fields!(OwnerChangeDraftDto; intent, example, scope);
fields!(OwnerRuleRevisionDto; expected_behavior, confirmed_by);
fields!(OwnerCandidateRevisionDto; identity);
fields!(ScenarioRevisionDto; title, expected_behavior, confirmed_by, recipe);
fields!(OwnerManualManifestDto; change, candidate, environment, rules, scenarios);
fields!(OwnerExecutionSummaryDto; revision);
fields!(OwnerManualRunDto; manifest, execution);
fields!(OwnerManualResultDto; observer, observed_steps, assertions, limitation);
fields!(RecordOwnerEvidenceDecisionInput; reason);
fields!(OwnerEvidenceDecisionDto; reason);
fields!(OwnerScenarioEvidenceDto; scenario, result);
fields!(OwnerEvidenceReviewDto; run, freshness_reasons, scenarios, decisions, decision_blockers, limitations);

impl Scrub for OwnerEnvironmentRevisionDto {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, parent: &str, report: &mut Vec<Redaction>) {
        self.label
            .scrub_in_place(sanitizer, &format!("{parent}.label"), report);
        // Live environments already satisfy the credential-free typed-address
        // policy; shape/pair sanitation still runs at export/storage boundaries.
        self.preview_url
            .scrub_in_place(sanitizer, &format!("{parent}.preview_address"), report);
        self.origins
            .scrub_in_place(sanitizer, &format!("{parent}.origins"), report);
    }
}
