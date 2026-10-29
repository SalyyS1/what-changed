//! One fixed-order smoke pair and one fresh canonical pair, explicitly ignored.

use std::time::Duration;

use super::seed_grouped_fixture::OwnedFixture;
use crate::{Options, DEFAULT_DAYS, DEFAULT_SEED};

#[path = "seed_grouped_admission.rs"]
mod admission;
#[path = "seed_grouped_content_digest.rs"]
mod content_digest;
#[path = "seed_grouped_corpus_validation.rs"]
mod corpus_validation;
#[path = "seed_grouped_measurement_run.rs"]
mod measurement_run;

use admission::FrozenSettings;
use content_digest::ContentDigest;
use measurement_run::{measure, Route};

const END_AT: i64 = 1_800_000_000_000;

struct Evidence {
    elapsed: Duration,
    corpus: Vec<ContentDigest>,
    owned: ContentDigest,
    settings: FrozenSettings,
}

fn one_run(rows: usize, route: Route, expected_settings: Option<&FrozenSettings>) -> Evidence {
    // Check immediately before exclusive fixture construction, not before
    // compilation or while the other run's corpus still occupies the volume.
    admission::preflight(rows, route.label());
    let fixture = OwnedFixture::new();
    let root = fixture.db().paths().root.clone();
    let settings = fixture
        .db()
        .write(|conn| Ok(admission::settings(conn)))
        .unwrap();
    if let Some(expected) = expected_settings {
        assert!(
            settings == *expected,
            "frozen SQLite settings differ before seeding"
        );
    }
    settings.print(rows, route.label());
    let options = Options {
        events: rows,
        days: DEFAULT_DAYS,
        seed: DEFAULT_SEED,
        relations: true,
        collector_payload_sensitivity: false,
        timings: true,
        insertion_timings: true,
        end_at: Some(END_AT),
        ..Options::default()
    };
    let measured = measure(&fixture, &options, route);
    measured.validate(rows);
    let corpus = fixture
        .db()
        .read(|conn| {
            corpus_validation::validate(conn, rows, END_AT);
            Ok(content_digest::corpus(conn))
        })
        .unwrap();
    corpus_validation::validate_burst(fixture.db());
    let owned = content_digest::ownership(&measured.owned);
    measured.print(rows, route);
    for digest in corpus.iter().chain(std::iter::once(&owned)) {
        println!(
            "seed grouping digest: events={rows} route={} table={} rows={} sha256={}",
            route.label(),
            digest.table,
            digest.rows,
            digest.sha256
        );
    }
    let elapsed = measured.elapsed;
    drop(measured);
    // Existing guard drains/joins the exclusive writer, validates the exact
    // canonical child and surfaces normal/unwind cleanup failures.
    drop(fixture);
    match std::fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => panic!("owned seed-pair fixture must be absent after close/join/cleanup"),
    }
    println!(
        "seed grouping cleanup: events={rows} route={} closed_joined_removed_verified=1",
        route.label()
    );
    Evidence {
        elapsed,
        corpus,
        owned,
        settings,
    }
}

fn fixed_pair(rows: usize) {
    assert!(matches!(rows, 10_000 | 1_000_000));
    assert_eq!(DEFAULT_DAYS, 90);
    assert_eq!(DEFAULT_SEED, 0xC0DE_2026);
    println!("seed grouping frozen pair: events={rows} order=baseline_then_fixed_five pairs=1 days={DEFAULT_DAYS} seed={DEFAULT_SEED} end_at={END_AT} relations=1 relation_percent=15 bursts_percent=8 entities=3000 checkpoints=40 snapshots=9800 investigations=20 collector_payload_sensitivity=0 insertion_timings=1 stage_timings=0 logical_batch_rows=1000 grouped_logical_units=5 public_default_unchanged=1");
    // Source bounds describe retained slots, NOT native working-set evidence
    // or a byte upper bound for strings/JSON/HashSet/SQLite allocations. The
    // unchanged generator also has a history-sized selection mask and a
    // temporary eligible-index vector; neither is hidden by this accounting.
    println!("seed grouping source memory: events={rows} baseline_max_prepared_rows=1000 fixed_five_max_prepared_rows=5000 maximum_relation_ordinals=3000 grouped_provisional_ordinal_clone=3000 ownership_snapshot_max=3000 ownership_digest_sorted_refs_max=3000 digest_max_retained_corpus_rows=1 existing_selection_mask_max={rows} existing_temporary_eligible_indices_max={rows} native_peak_unmeasured=1 dynamic_payload_bytes_unmeasured=1 memory_acceptance_claimed=0");
    let baseline = one_run(rows, Route::Baseline, None);
    // one_run returns ONLY bounded scalar/digest evidence, after verified
    // deletion. Never hold two owned corpora or open their handles together.
    let grouped = one_run(rows, Route::FixedFive, Some(&baseline.settings));
    assert!(
        baseline.settings == grouped.settings,
        "frozen SQLite settings differ"
    );
    assert!(
        baseline.corpus == grouped.corpus,
        "complete normalized corpus count/content digest mismatch; stop this experiment"
    );
    assert!(
        baseline.owned == grouped.owned,
        "complete sorted event/graph ownership digest mismatch; stop this experiment"
    );
    println!("seed grouping fixed pair result: events={rows} count_content_ownership_parity=1 baseline_elapsed_ms={:.3} fixed_five_elapsed_ms={:.3} measured_pairs=1 order_effect_uncontrolled=1 acceptance_claimed=0",
        baseline.elapsed.as_secs_f64() * 1000.0, grouped.elapsed.as_secs_f64() * 1000.0);
}

#[test]
#[ignore = "owned fixed-order 10k smoke pair; root executes once after review"]
fn fixed_seed_grouping_10k_pair() {
    fixed_pair(10_000);
}

#[test]
#[ignore = "owned fixed-order fresh canonical 1M pair; root executes once after reviewed smoke"]
fn fixed_seed_grouping_1m_pair() {
    fixed_pair(1_000_000);
}
