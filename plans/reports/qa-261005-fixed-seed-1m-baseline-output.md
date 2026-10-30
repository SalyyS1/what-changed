# Fixed seed-grouping 1M baseline output — 2026-10-05

[Immutable reviewed protocol](qa-261005-fixed-seed-pair-harness.md).
One command: `cargo test --release --locked --features devtools --bin seed_history seed_grouped_events::measurement::fixed_seed_grouping_1m_pair -- --exact --ignored --nocapture --test-threads=1`.
Terminal exit0: one pair/test, complete count/content/ownership parity,
zero failures; test736.74s after1.60s cached preparation. The two linked
receipts partition complete stdout/stderr at baseline's verified cleanup line;
concatenating their code blocks restores the full original output.
[Baseline](qa-261005-fixed-seed-1m-baseline-output.md),
[fixed-five and terminal result](qa-261005-fixed-seed-1m-fixed-five-output.md).
Total seed clocks369,085.288 versus332,180.967ms are diagnostic, not acceptance.

```text
    Finished `release` profile [optimized] target(s) in 1.60s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 1 test
test seed_grouped_events::measurement::fixed_seed_grouping_1m_pair ... seed grouping frozen pair: events=1000000 order=baseline_then_fixed_five pairs=1 days=90 seed=3235782694 end_at=1800000000000 relations=1 relation_percent=15 bursts_percent=8 entities=3000 checkpoints=40 snapshots=9800 investigations=20 collector_payload_sensitivity=0 insertion_timings=1 stage_timings=0 logical_batch_rows=1000 grouped_logical_units=5 public_default_unchanged=1
seed grouping source memory: events=1000000 baseline_max_prepared_rows=1000 fixed_five_max_prepared_rows=5000 maximum_relation_ordinals=3000 grouped_provisional_ordinal_clone=3000 ownership_snapshot_max=3000 ownership_digest_sorted_refs_max=3000 digest_max_retained_corpus_rows=1 existing_selection_mask_max=1000000 existing_temporary_eligible_indices_max=1000000 native_peak_unmeasured=1 dynamic_payload_bytes_unmeasured=1 memory_acceptance_claimed=0
seed grouping disk admission: events=1000000 route=baseline available_bytes=2994294784 required_bytes=2853306368 bytes_per_input=2048 wal_temp_reserve_bytes=536870912 build_headroom_bytes=268435456
seed grouping SQLite: events=1000000 route=baseline version=3.53.2 journal_mode=wal
seed grouping SQLite numeric: events=1000000 route=baseline name=synchronous value=1
seed grouping SQLite numeric: events=1000000 route=baseline name=foreign_keys value=1
seed grouping SQLite numeric: events=1000000 route=baseline name=temp_store value=2
seed grouping SQLite numeric: events=1000000 route=baseline name=cache_size value=-8000
seed grouping SQLite numeric: events=1000000 route=baseline name=wal_autocheckpoint value=1000
seed grouping SQLite numeric: events=1000000 route=baseline name=busy_timeout value=5000
seed grouping SQLite numeric: events=1000000 route=baseline name=auto_vacuum value=2
seed grouping SQLite numeric: events=1000000 route=baseline name=page_size value=4096
seeded 10000/1000000 synthetic events
seeded 20000/1000000 synthetic events
seeded 30000/1000000 synthetic events
seeded 40000/1000000 synthetic events
seeded 50000/1000000 synthetic events
seeded 60000/1000000 synthetic events
seeded 70000/1000000 synthetic events
seeded 80000/1000000 synthetic events
seeded 90000/1000000 synthetic events
seeded 100000/1000000 synthetic events
seeded 110000/1000000 synthetic events
seeded 120000/1000000 synthetic events
seeded 130000/1000000 synthetic events
seeded 140000/1000000 synthetic events
seeded 150000/1000000 synthetic events
seeded 160000/1000000 synthetic events
seeded 170000/1000000 synthetic events
seeded 180000/1000000 synthetic events
seeded 190000/1000000 synthetic events
seeded 200000/1000000 synthetic events
seeded 210000/1000000 synthetic events
seeded 220000/1000000 synthetic events
seeded 230000/1000000 synthetic events
seeded 240000/1000000 synthetic events
seeded 250000/1000000 synthetic events
seeded 260000/1000000 synthetic events
seeded 270000/1000000 synthetic events
seeded 280000/1000000 synthetic events
seeded 290000/1000000 synthetic events
seeded 300000/1000000 synthetic events
seeded 310000/1000000 synthetic events
seeded 320000/1000000 synthetic events
seeded 330000/1000000 synthetic events
seeded 340000/1000000 synthetic events
seeded 350000/1000000 synthetic events
seeded 360000/1000000 synthetic events
seeded 370000/1000000 synthetic events
seeded 380000/1000000 synthetic events
seeded 390000/1000000 synthetic events
seeded 400000/1000000 synthetic events
seeded 410000/1000000 synthetic events
seeded 420000/1000000 synthetic events
seeded 430000/1000000 synthetic events
seeded 440000/1000000 synthetic events
seeded 450000/1000000 synthetic events
seeded 460000/1000000 synthetic events
seeded 470000/1000000 synthetic events
seeded 480000/1000000 synthetic events
seeded 490000/1000000 synthetic events
seeded 500000/1000000 synthetic events
seeded 510000/1000000 synthetic events
seeded 520000/1000000 synthetic events
seeded 530000/1000000 synthetic events
seeded 540000/1000000 synthetic events
seeded 550000/1000000 synthetic events
seeded 560000/1000000 synthetic events
seeded 570000/1000000 synthetic events
seeded 580000/1000000 synthetic events
seeded 590000/1000000 synthetic events
seeded 600000/1000000 synthetic events
seeded 610000/1000000 synthetic events
seeded 620000/1000000 synthetic events
seeded 630000/1000000 synthetic events
seeded 640000/1000000 synthetic events
seeded 650000/1000000 synthetic events
seeded 660000/1000000 synthetic events
seeded 670000/1000000 synthetic events
seeded 680000/1000000 synthetic events
seeded 690000/1000000 synthetic events
seeded 700000/1000000 synthetic events
seeded 710000/1000000 synthetic events
seeded 720000/1000000 synthetic events
seeded 730000/1000000 synthetic events
seeded 740000/1000000 synthetic events
seeded 750000/1000000 synthetic events
seeded 760000/1000000 synthetic events
seeded 770000/1000000 synthetic events
seeded 780000/1000000 synthetic events
seeded 790000/1000000 synthetic events
seeded 800000/1000000 synthetic events
seeded 810000/1000000 synthetic events
seeded 820000/1000000 synthetic events
seeded 830000/1000000 synthetic events
seeded 840000/1000000 synthetic events
seeded 850000/1000000 synthetic events
seeded 860000/1000000 synthetic events
seeded 870000/1000000 synthetic events
seeded 880000/1000000 synthetic events
seeded 890000/1000000 synthetic events
seeded 900000/1000000 synthetic events
seeded 910000/1000000 synthetic events
seeded 920000/1000000 synthetic events
seeded 930000/1000000 synthetic events
seeded 940000/1000000 synthetic events
seeded 950000/1000000 synthetic events
seeded 960000/1000000 synthetic events
seeded 970000/1000000 synthetic events
seeded 980000/1000000 synthetic events
seeded 990000/1000000 synthetic events
seeded 1000000/1000000 synthetic events
seed grouping full seed: events=1000000 route=baseline elapsed_ms=369085.288 clock=before_marker_registration_through_finish_seed_and_flush ownership_snapshot_included=1 fixture_open_settings_close_validation_digest_report_cleanup_excluded=1
seed insertion timings: batches=1000 attempted_rows=1000000 committed_batches=1000 committed_rows=1000000 failed_batches=0 empty_batches=0 completed_phases=4000 begin_prepare_ms=17.344 event_entity_loop_ms=147203.355 reconciliation_ms=13.970 commit_ms=195463.706 repository_total_ms=342699.734 caller_insert_ms=343441.233 caller_writer_overhead_ms=741.499
seed timings: elapsed_make_batch_ms=8800.063 insert_events_ms=343441.233 graph_ms=14614.650 inventory_ms=42.689 checkpoints_ms=91.376 investigations_ms=2076.653 total_run_ms=369085.288
seed grouping digest: events=1000000 route=baseline table=events rows=1000000 sha256=b909c1fbb807a4b039f424d61f2b87be58d5f7391993529d0eeb2ae02db1b938
seed grouping digest: events=1000000 route=baseline table=entities rows=3000 sha256=512faf9afecceb56115e065d1bc735260c95aac4eea62f33778e77ee75c7321d
seed grouping digest: events=1000000 route=baseline table=entity_relationships rows=150000 sha256=04cab33cf1bbbbb3f853d6d3c9c109d2842d247de8b1633dd56cb71ace098b99
seed grouping digest: events=1000000 route=baseline table=checkpoints rows=40 sha256=3614f4593e05c826dbefe77eda60dd66479389ca5eaeb1d2ae34d615d19802fb
seed grouping digest: events=1000000 route=baseline table=snapshots rows=9800 sha256=221badfdc3e0e4e3a13f9d26afeb366eb5f6bc91cf9983811963942aa1499c81
seed grouping digest: events=1000000 route=baseline table=investigations rows=20 sha256=fd86fa9a63d877324fa590b9f167ec18ac59da4edbec5910e8bd5e0092466432
seed grouping digest: events=1000000 route=baseline table=analysis_sessions rows=20 sha256=ecb3dcb59386dc033970edb6c1c73d7c9dc97d97de088e8ed852e709ce782f0c
seed grouping digest: events=1000000 route=baseline table=analysis_results rows=400 sha256=7895f204b5df12ad4089cc1fe6840f83d37980c5746ff0a347f5d6a261175f05
seed grouping digest: events=1000000 route=baseline table=settings rows=8 sha256=3b6dfe471447fd69ca1ad8d08852eb4f9023136a912995dfa47c6321ba4df06e
seed grouping digest: events=1000000 route=baseline table=event_category_counts rows=15 sha256=d1124f46bb2684291525d3f26db50780a2327a57d19a0cf77d6ed2d95833c886
seed grouping digest: events=1000000 route=baseline table=event_graph_ownership rows=2774 sha256=e29e370e2d04f5779955f3fd44cc7214898d18fc28f71deaa44518ca44a0f8a0
seed grouping cleanup: events=1000000 route=baseline closed_joined_removed_verified=1
```

