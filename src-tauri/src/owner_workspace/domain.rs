//! Closed, versioned recipe and evidence vocabulary. No executable script field exists.

mod recipe;
mod scrub;

pub use recipe::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

macro_rules! words {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}

words!(RuleProvenance {
    Owner,
    Template,
    ImportedUntrusted,
    ExternalAi
});
words!(EnvironmentKind { Test, Local });
words!(CandidateSource {
    OwnerLabel,
    GitCommit,
    DeployBuildId,
    RevisionMarker
});
words!(CandidateTrust {
    SelfReported,
    SourceOnly,
    EnvironmentReported
});
words!(ChangeState {
    Draft,
    ReadyForReview,
    Deferred
});
words!(VerificationMethod {
    Automated,
    OwnerManual,
    ImportedUntrusted
});
words!(RunLifecycle {
    Queued,
    Running,
    Completed,
    Cancelled,
    Interrupted
});
words!(ResultStatus {
    Passed,
    Failed,
    Blocked,
    NotRun
});
words!(EvidenceFreshness {
    Current,
    Stale,
    Unknown
});
words!(DecisionKind {
    Pending,
    Accepted,
    AcceptedWithExceptions,
    Rejected,
    Deferred
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RuleRefDto {
    pub rule_id: String,
    pub revision: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRefDto {
    pub scenario_id: String,
    pub revision: u32,
}

/// An origin is parsed and normalized locally; saving it never opens a socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OriginDto {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct FrozenManifestDto {
    pub candidate_id: String,
    pub environment_id: String,
    pub environment_revision: u32,
    pub rules: Vec<RuleRefDto>,
    pub scenarios: Vec<ScenarioRefDto>,
    pub fixture_schema_digest: String,
    pub runner_semantics: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AssertionOutcomeDto {
    pub assertion_id: String,
    pub status: ResultStatus,
    pub expected: String,
    pub observed: String,
    pub limitation: Option<String>,
}
