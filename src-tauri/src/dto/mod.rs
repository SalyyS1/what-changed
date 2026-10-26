//! The transfer types the webview sees.
//!
//! Everything here is `Serialize + Deserialize + TS`: the TypeScript in
//! `src/lib/tauri/bindings.ts` is generated from these declarations, so a
//! renamed field breaks the build rather than the UI at runtime.
//!
//! A DTO exists when the wire shape differs from the internal one, or when the
//! shape belongs to a phase that has not landed yet. Where the domain type
//! *is* the wire shape — an event summary, a page, a retention policy — it is
//! exported directly rather than copied, because two definitions of one shape
//! is how they drift.

pub mod analysis_dto;
#[cfg(any(test, feature = "devtools"))]
pub mod bindings;
pub mod checkpoint_dto;
pub mod error_dto;
pub mod event_dto;
pub mod settings_dto;
pub mod status_dto;
pub mod trust_dto;

pub use analysis_dto::{
    ChangeMapData, ChangeMapEdge, ChangeMapEdgeKind, ChangeMapLane, ChangeMapLaneData,
    ChangeMapNode, ChangeMapRequest, EntityKindAvailability, EntityRef, InvestigationDto,
    InvestigationRequestDto, SuspectDto, SystemMapData, SystemMapEdge, SystemMapNode,
    SystemMapRecentEvent,
};
pub use checkpoint_dto::{
    CheckpointCategoryDiffDto, CheckpointDiffDto, CheckpointDto, DiffItemDto,
};
pub use error_dto::{AppErrorDto, CommandResult, ErrorCode};
pub use event_dto::{
    CheckpointRef, DashboardCountsDto, DashboardSummaryDto, EventDetailsDto, EventPageDto,
    ProjectDto, SearchResultDto, WatchedLocationDto,
};
// Settings are domain state, so the wire shape *is* the domain shape. These
// aliases keep the `Dto` suffix at the command boundary without a second
// definition of the same fields.
pub use settings_dto::{
    MonitoringProfile, SettingsDto, SettingsUpdateDto, Theme, VisualizationMode,
};
pub use status_dto::{
    AppStatusDto, CollectorState, CollectorStatusDto, DiagnosticsDto, MonitoringDto, UpdateCheckDto,
};
pub use trust_dto::{
    OnboardingStateDto, PrivacyCategoryCountDto, PrivacySummaryDto, PurgeReportDto,
};
