use crate::domain::InvestigationWindow;

pub fn infer(problem: &str) -> InvestigationWindow {
    let text = problem.to_lowercase();
    if text.contains("yesterday") {
        InvestigationWindow::SinceYesterday
    } else {
        // "this morning" and unsupported relative phrases such as "after the
        // update" deliberately stay bounded to Today rather than inventing a
        // timestamp for an update the engine has not resolved yet.
        InvestigationWindow::Today
    }
}
