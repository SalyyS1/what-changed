use crate::domain::{ConfidenceClass, ScoreBreakdown, SystemEvent};

pub fn classify(
    event: &SystemEvent,
    score: &ScoreBreakdown,
    has_structural_chain: bool,
    boundary_head: bool,
) -> ConfidenceClass {
    let structural = score.relationship > 0.0 || has_structural_chain;
    if has_structural_chain
        && score.failure_proximity == 1.0
        && !event.event.occurred_at_is_uncertain
        && !boundary_head
    {
        ConfidenceClass::Confirmed
    } else if structural {
        ConfidenceClass::Likely
    } else {
        ConfidenceClass::Correlated
    }
}
