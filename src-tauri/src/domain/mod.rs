//! The domain: the one event shape every collector emits, the entities those
//! events are about, and the identifiers that keep an entity the same thing
//! across restarts.
//!
//! Nothing here touches the operating system or SQL. That is a build gate, not
//! a convention: `tests/architecture.rs` fails on `#[cfg(windows)]` or a SQL
//! statement anywhere under this module.

pub mod checkpoint;
pub mod clock;
pub mod collector;
pub mod driver_naming;
pub mod entity;
pub mod enums;
pub mod event;
pub mod ids;
pub mod importance;
pub mod investigation;
pub mod page;
pub mod retention;
pub mod settings;
pub mod tool;

pub use checkpoint::{
    category_for_collector, CategoryCheckpointDiff, Checkpoint, CheckpointCapture,
    CheckpointComparisonTarget, CheckpointDiff, CheckpointDiffItem, CheckpointItem,
    CheckpointItemKey, CheckpointKind, NewCheckpoint,
};
pub use clock::now_ms;
pub use collector::{
    CollectorCadence, CollectorCadenceDto, CollectorCost, CollectorDescriptor,
    CollectorDescriptorDto, CollectorStatusDetails, CollectorWakeTier, DockerCollectorState,
};
pub use entity::{Entity, Relationship, RelationshipKind};
pub use enums::{Category, Confidence, EntityType, EventType, ExitKind, Importance, Severity};
pub use event::{EventSignature, NormalizedEvent, SystemEvent};
pub use ids::{entity_type_of, ProcessId};
pub use importance::{classify, ClassifierContext};
pub use investigation::{
    importance_value, AnalysisInput, ChainStep, ConfidenceClass, Coverage, Investigation,
    InvestigationWindow, NoCauseReason, ResolvedInvestigationWindow, ScoreBreakdown, Suspect,
    SymptomModel, SymptomTag,
};
pub use page::{
    Cursor, CursorDecodeError, EventQuery, EventSummary, Page, TimeRange, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE, MAX_RANGE_MS,
};
pub use retention::{RetentionPolicy, DAY_MS};
pub use settings::{MonitoringProfile, Settings, SettingsUpdate, Theme, VisualizationMode};
