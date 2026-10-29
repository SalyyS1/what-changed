//! Opt-in aggregate timings around executed analysis stages.

#[derive(Clone, Copy)]
pub(super) enum Stage {
    #[cfg(feature = "devtools")]
    Preprocess,
    StructuralGraph,
    ChainIndex,
    FailureIndex,
    CandidatePreparation,
    CandidateScoring,
    SelectionExplanations,
}

pub(super) struct StageTimings {
    #[cfg(feature = "devtools")]
    active: Option<enabled::Active>,
}

impl StageTimings {
    pub(super) fn new() -> Self {
        #[cfg(feature = "devtools")]
        {
            static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            Self::with_enabled(*ENABLED.get_or_init(|| {
                std::env::var_os("WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS")
                    .is_some_and(|value| value == "1")
            }))
        }
        #[cfg(not(feature = "devtools"))]
        Self {}
    }

    #[cfg(feature = "devtools")]
    fn with_enabled(enabled: bool) -> Self {
        Self {
            active: enabled.then(enabled::Active::new),
        }
    }

    #[inline]
    pub(super) fn enter(&mut self, _stage: Stage) {
        #[cfg(feature = "devtools")]
        if let Some(active) = &mut self.active {
            active.enter(_stage);
        }
    }

    #[inline]
    pub(super) fn complete(&mut self) {
        #[cfg(feature = "devtools")]
        if let Some(active) = &mut self.active {
            active.complete = true;
        }
    }
}

#[cfg(feature = "devtools")]
impl Drop for StageTimings {
    fn drop(&mut self) {
        use std::io::Write;
        if let Some(active) = &self.active {
            let _ = writeln!(std::io::stderr().lock(), "{}", active.record());
        }
    }
}

#[cfg(feature = "devtools")]
#[path = "stage_timings_enabled.rs"]
mod enabled;
