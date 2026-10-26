use std::collections::BTreeMap;

use what_changed_lib::analysis::analyze;
use what_changed_lib::domain::*;

#[test]
fn identical_empty_inputs_are_byte_stable() {
    let input = AnalysisInput {
        problem: "unknown symptom".into(),
        window: InvestigationWindow::Today,
        resolved_window: ResolvedInvestigationWindow {
            from_ms: 1,
            to_ms: 2,
        },
        symptom_time_ms: 2,
        created_at: 3,
        events: vec![],
        boundary_events: vec![],
        entities: vec![],
        relationships: vec![],
        prior_occurrences_90d: BTreeMap::new(),
        total_events_in_window: 0,
        dropped_events: 0,
        coverage: Coverage::default(),
    };
    let first = analyze(input.clone());
    let second = analyze(input);
    assert_eq!(first, second);
}
