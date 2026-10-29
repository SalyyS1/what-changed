//! Opt-in Phase 20 evidence for routine retention and incremental reclaim.
//!
//! This is separate from `perf_db_size`: retention deliberately keeps graph
//! edges after their evidence event expires, so the tagged synthetic purge's
//! exact-artifact assertions belong to a fresh fixture.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::domain::{RetentionPolicy, DAY_MS};
use what_changed_lib::storage::{maintenance, Db};

#[path = "support/perf_fixture.rs"]
mod perf_fixture;
use perf_fixture::PerfFixture;

const RETENTION_ROWS_ENV: &str = "WHAT_CHANGED_RETENTION_ROWS";
const RECLAIM_PAGE_BUDGET: u64 = 256;

#[derive(Debug)]
struct PageStats {
    pages: u64,
    freelist: u64,
    auto_vacuum: i64,
}

fn page_stats(db: &Db) -> PageStats {
    db.read(|conn| {
        let pragma = |name: &str| -> i64 {
            conn.pragma_query_value(None, name, |row| row.get(0))
                .expect("read retention page statistic")
        };
        Ok(PageStats {
            pages: u64::try_from(pragma("page_count")).expect("nonnegative page count"),
            freelist: u64::try_from(pragma("freelist_count")).expect("nonnegative freelist count"),
            auto_vacuum: pragma("auto_vacuum"),
        })
    })
    .expect("inspect retention page statistics")
}

fn eligible_events(db: &Db, policy: RetentionPolicy, now: i64) -> usize {
    db.read(|conn| {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE \
                   (importance IN ('important', 'critical') AND timestamp < ?2) \
                   OR (importance IN ('informational', 'notable') AND timestamp < ?1)",
                (policy.events_cutoff(now), policy.important_cutoff(now)),
                |row| row.get(0),
            )
            .expect("count deterministic retention candidates");
        Ok(usize::try_from(count).expect("nonnegative candidate count"))
    })
    .expect("inspect retention candidates")
}

fn protected_important_events(db: &Db, policy: RetentionPolicy, now: i64) -> usize {
    db.read(|conn| {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE importance IN ('important', 'critical') \
                   AND timestamp < ?1 AND timestamp >= ?2",
                (policy.events_cutoff(now), policy.important_cutoff(now)),
                |row| row.get(0),
            )
            .expect("count important events within their longer retention lifetime");
        Ok(usize::try_from(count).expect("nonnegative protected count"))
    })
    .expect("inspect protected important events")
}

fn selected_retention_rows() -> usize {
    match std::env::var(RETENTION_ROWS_ENV) {
        Err(std::env::VarError::NotPresent) => 10_000,
        Ok(value) => match value.parse::<usize>() {
            Ok(rows @ (10_000 | 100_000 | 1_000_000)) => rows,
            _ => panic!("{RETENTION_ROWS_ENV} must be 10000, 100000, or 1000000"),
        },
        Err(error) => panic!("read {RETENTION_ROWS_ENV}: {error}"),
    }
}

#[test]
#[ignore = "Phase 20 routine-retention gate; defaults to canonical 10k, opt into 100k/1M with WHAT_CHANGED_RETENTION_ROWS"]
fn canonical_retention_pass_incrementally_reclaims_pages() {
    let rows = selected_retention_rows();
    let (fixture, corpus) = PerfFixture::new_seeded_db(rows);
    let db = fixture.db();
    let policy = RetentionPolicy::default();
    // Thirty-day routine retention at this fixed instant expires the first
    // fifteen days of a ninety-day corpus; important rows retain their longer
    // lifetime. The date does not depend on the test machine's clock.
    let now = corpus.from + 45 * DAY_MS;
    let eligible = eligible_events(db, policy, now);
    let protected = protected_important_events(db, policy, now);
    assert!(
        eligible > 0 && eligible < rows,
        "retention fixture needs a partial purge"
    );
    assert!(
        protected > 0,
        "retention fixture needs important rows to preserve"
    );
    let before = page_stats(db);
    assert_eq!(
        before.auto_vacuum, 2,
        "SQLite must use incremental auto-vacuum"
    );

    let report = maintenance::run_pass(db, policy, now).expect("run production retention pass");
    let after = page_stats(db);
    let remaining = db
        .read(|conn| {
            Ok(conn
                .query_row("SELECT COUNT(*) FROM events", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("count retained events"))
        })
        .expect("read retained event count");
    println!(
        "phase20 routine_retention rows={rows} eligible={} deleted={} remaining={} batches={} page_count={}=>{} freelist_count={}=>{} duration_ms={}",
        eligible,
        report.events_deleted,
        remaining,
        report.batches,
        before.pages,
        after.pages,
        before.freelist,
        after.freelist,
        report.duration_ms
    );
    assert_eq!(
        report.events_deleted, eligible,
        "retention skipped eligible events"
    );
    assert_eq!(
        remaining,
        i64::try_from(rows - eligible).expect("remaining fits i64")
    );
    assert_eq!(
        eligible_events(db, policy, now),
        0,
        "expired events remain after retention"
    );
    assert_eq!(
        protected_important_events(db, policy, now),
        protected,
        "routine retention deleted important events before their longer deadline"
    );
    if rows == 10_000 {
        assert_eq!(
            after.freelist, 0,
            "routine incremental vacuum left free pages"
        );
        assert!(
            after.pages < before.pages,
            "routine retention did not return pages: before={before:?} after={after:?}"
        );
        return;
    }

    // Deletes can create new free pages while vacuum drains old ones, so the
    // first pass's freelist delta cannot establish the 256-page cap. An
    // immediate second production pass has no eligible rows and isolates the
    // incremental vacuum without changing the corpus or retention policy.
    let followup = maintenance::run_pass(db, policy, now).expect("run follow-up reclaim pass");
    let followup_pages = page_stats(db);
    println!(
        "phase20 routine_reclaim_followup rows={rows} deleted={} snapshot_deleted={} investigation_deleted={} batches={} page_count={}=>{} freelist_count={}=>{} duration_ms={}",
        followup.events_deleted,
        followup.snapshots_deleted,
        followup.investigations_deleted,
        followup.batches,
        after.pages,
        followup_pages.pages,
        after.freelist,
        followup_pages.freelist,
        followup.duration_ms
    );
    assert_eq!(followup.events_deleted, 0, "follow-up deleted events");
    assert_eq!(followup.snapshots_deleted, 0, "follow-up deleted snapshots");
    assert_eq!(
        followup.investigations_deleted, 0,
        "follow-up deleted investigations"
    );
    assert_eq!(
        protected_important_events(db, policy, now),
        protected,
        "follow-up deleted protected important events"
    );
    assert_eq!(followup_pages.auto_vacuum, 2);
    assert!(
        followup_pages.pages <= after.pages,
        "no-delete follow-up grew the database: after={after:?} followup={followup_pages:?}"
    );
    assert!(
        followup_pages.freelist <= after.freelist,
        "no-delete follow-up grew the freelist: after={after:?} followup={followup_pages:?}"
    );
    // The PRAGMA's 256 limit applies to freelist pages; physical page_count
    // can fall by an extra auto-vacuum bookkeeping page at a map boundary.
    assert!(
        after.freelist - followup_pages.freelist <= RECLAIM_PAGE_BUDGET,
        "follow-up drained more than {RECLAIM_PAGE_BUDGET} free pages: after={after:?} followup={followup_pages:?}"
    );
    if after.freelist > 0 {
        assert!(
            followup_pages.pages < after.pages,
            "routine follow-up did not return pages: after={after:?} followup={followup_pages:?}"
        );
    }
}

#[test]
fn routine_reclaim_stops_after_page_budget() {
    let fixture = PerfFixture::new();
    let db = fixture.db();
    db.write(|conn| {
        conn.execute_batch(
            "CREATE TABLE reclaim_probe (payload BLOB NOT NULL); \
             WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400) \
             INSERT INTO reclaim_probe(payload) SELECT zeroblob(8192) FROM n; \
             DELETE FROM reclaim_probe;",
        )
        .expect("create a freelist larger than the routine reclaim budget");
        Ok(())
    })
    .expect("seed reclaim budget probe");
    let before = page_stats(db);
    assert!(before.freelist > 512, "probe needs over two page budgets");

    maintenance::run_pass(db, RetentionPolicy::default(), 45 * DAY_MS)
        .expect("run bounded production maintenance pass");
    let after = page_stats(db);
    assert!(
        after.pages < before.pages,
        "vacuum did not return any pages"
    );
    assert!(
        after.freelist < before.freelist,
        "vacuum did not drain rows"
    );
    assert!(
        after.freelist > 0,
        "routine vacuum exceeded its page budget"
    );
    assert!(
        before.freelist - after.freelist <= RECLAIM_PAGE_BUDGET,
        "routine vacuum reclaimed more than {RECLAIM_PAGE_BUDGET} free pages: before={before:?} after={after:?}"
    );
}
