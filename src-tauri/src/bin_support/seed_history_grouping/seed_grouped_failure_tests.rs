use super::*;

#[path = "seed_grouped_failure_fixtures.rs"]
pub(super) mod seed_grouped_failure_fixtures;
use seed_grouped_failure_fixtures::{assert_failed_second_group, FailureKind};

#[test]
fn event_failure_rolls_back_failed_group_and_keeps_committed_prefix() {
    assert_failed_second_group(FailureKind::Event, 7, 6_000, 6);
}

#[test]
fn graph_failure_rolls_back_successful_event_body_and_aborts_later_units() {
    assert_failed_second_group(FailureKind::Graph, 6, 6_000, 5);
}

#[test]
fn deferred_commit_failure_reports_completed_bodies_but_no_failed_group_commit() {
    assert_failed_second_group(FailureKind::Commit, 10, 10_000, 10);
}

#[test]
fn sqlite_raise_rollback_already_ended_transaction_keeps_primary_error() {
    assert_failed_second_group(FailureKind::AlreadyRolledBack, 7, 6_000, 6);
}

#[test]
fn admission_failure_does_not_claim_an_attempted_group_or_publish_state() {
    let fixture = seed_grouped_fixture::OwnedFixture::new();
    fixture.db().close();
    let run = seed_grouped_events(
        fixture.db(),
        &seed_grouped_failure_fixtures::options(),
        seed_grouped_failure_fixtures::END_AT,
        &mut SeedTimings::default(),
    );
    assert!(run.value.is_err());
    assert_eq!(run.attribution.attempted_groups, 0);
    assert_eq!(run.attribution.failed_groups, 0);
    assert_eq!(run.attribution.committed_input_rows, 0);
    assert_eq!(run.attribution.group_total, std::time::Duration::ZERO);
    assert_eq!(run.progress.rows, 0);
    assert_eq!(run.progress.groups, 0);
    assert_eq!(run.progress.notifications, 0);
}

#[test]
fn error_representation_preserves_primary_and_cleanup_causes() {
    use what_changed_lib::error::{AppError, DbError};
    // Representation only: do not claim to have induced an actual SQLite
    // rollback failure, enable hooks, or sabotage global filesystem resources.
    let error = seed_grouped_failure::GroupFailure::new(
        AppError::InvalidInput {
            what: "primary operation",
            reason: "failed".to_owned(),
        },
        Some(AppError::Db(DbError::Sqlite {
            operation: "rollback operation",
            source: rusqlite::Error::InvalidQuery,
        })),
    );
    assert!(matches!(
        error.primary(),
        AppError::InvalidInput {
            what: "primary operation",
            ..
        }
    ));
    assert!(matches!(
        error.rollback(),
        Some(AppError::Db(DbError::Sqlite {
            operation: "rollback operation",
            source: rusqlite::Error::InvalidQuery,
        }))
    ));
    let report = error.to_string();
    assert!(report.contains("primary operation"));
    assert!(report.contains("rollback operation"));
    assert!(report.contains("rollback also failed"));
}
