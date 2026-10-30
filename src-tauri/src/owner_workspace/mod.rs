//! Offline owner-controlled product memory, isolated from machine history.

pub mod domain;
pub(crate) mod storage;
pub(crate) mod validation;

pub use crate::dto::owner_workspace_dto::*;
pub use domain::{
    AdapterEffect, CandidateSource, CandidateTrust, ChangeState, DecisionKind, EnvironmentKind,
    EvidenceFreshness, LocatorDto, NamedAssertionDto, ReadBackAdapterDto, RecipeActionDto,
    RecipeAssertionDto, RecipeStepDto, ResultStatus, RuleProvenance, RunLifecycle,
    ScenarioRecipeDto, VerificationMethod,
};
pub mod archive;
pub mod brief;
pub mod evidence;
pub mod portability;
pub use storage::{AdmissionEpoch, OwnerStore, OwnerStoreState};

pub const MAX_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_APPS: usize = 20;
pub const MAX_CAPABILITIES_PER_APP: usize = 200;
pub const MAX_RULES_PER_APP: usize = 500;
pub const MAX_SCENARIOS_PER_APP: usize = 100;
pub const MAX_STEPS_PER_SCENARIO: usize = 50;
/// All expected outcomes must fit one immutable manual result, including gaps.
pub const MAX_ASSERTIONS_PER_SCENARIO: usize = 1000;
pub const MAX_PAGE_SIZE: u32 = 200;
