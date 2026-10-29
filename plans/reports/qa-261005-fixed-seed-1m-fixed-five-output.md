# Fixed seed-grouping 1M fixed-five output — 2026-10-05

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
seed grouping disk admission: events=1000000 route=fixed_five available_bytes=2993717248 required_bytes=2853306368 bytes_per_input=2048 wal_temp_reserve_bytes=536870912 build_headroom_bytes=268435456
seed grouping SQLite: events=1000000 route=fixed_five version=3.53.2 journal_mode=wal
seed grouping SQLite numeric: events=1000000 route=fixed_five name=synchronous value=1
seed grouping SQLite numeric: events=1000000 route=fixed_five name=foreign_keys value=1
seed grouping SQLite numeric: events=1000000 route=fixed_five name=temp_store value=2
seed grouping SQLite numeric: events=1000000 route=fixed_five name=cache_size value=-8000
seed grouping SQLite numeric: events=1000000 route=fixed_five name=wal_autocheckpoint value=1000
seed grouping SQLite numeric: events=1000000 route=fixed_five name=busy_timeout value=5000
seed grouping SQLite numeric: events=1000000 route=fixed_five name=auto_vacuum value=2
seed grouping SQLite numeric: events=1000000 route=fixed_five name=page_size value=4096
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
seed grouping full seed: events=1000000 route=fixed_five elapsed_ms=332180.967 clock=before_marker_registration_through_finish_seed_and_flush ownership_snapshot_included=1 fixture_open_settings_close_validation_digest_report_cleanup_excluded=1
seed grouping actual groups: events=1000000 route=fixed_five attempted_groups=200 committed_groups=200 failed_groups=0 attempted_input_rows=1000000 started_units=1000 completed_body_rows=1000000 completed_units=1000 committed_input_rows=1000000 progress_rows=1000000 progress_groups=200 progress_notifications=100 begin_ms=2.639 graph_ms=14516.166 commit_ms=128794.004 group_total_ms=317549.675
seed grouping actual bodies: events=1000000 route=fixed_five bodies=1000 attempted_rows=1000000 completed_bodies=1000 completed_rows=1000000 failed_bodies=0 empty_bodies=0 completed_phases=3000 preparation_ms=2.993 event_entity_loop_ms=173585.102 reconciliation_ms=14.562 body_total_ms=173602.658 body_success_is_not_commit=1
seed grouping coarse: events=1000000 route=fixed_five make_batch_ms=9355.780 inventory_ms=22.758 checkpoints_ms=210.062 investigations_ms=5008.040 legacy_insert_graph_fields_not_reinterpreted=1
seed grouping digest: events=1000000 route=fixed_five table=events rows=1000000 sha256=b909c1fbb807a4b039f424d61f2b87be58d5f7391993529d0eeb2ae02db1b938
seed grouping digest: events=1000000 route=fixed_five table=entities rows=3000 sha256=512faf9afecceb56115e065d1bc735260c95aac4eea62f33778e77ee75c7321d
seed grouping digest: events=1000000 route=fixed_five table=entity_relationships rows=150000 sha256=04cab33cf1bbbbb3f853d6d3c9c109d2842d247de8b1633dd56cb71ace098b99
seed grouping digest: events=1000000 route=fixed_five table=checkpoints rows=40 sha256=3614f4593e05c826dbefe77eda60dd66479389ca5eaeb1d2ae34d615d19802fb
seed grouping digest: events=1000000 route=fixed_five table=snapshots rows=9800 sha256=221badfdc3e0e4e3a13f9d26afeb366eb5f6bc91cf9983811963942aa1499c81
seed grouping digest: events=1000000 route=fixed_five table=investigations rows=20 sha256=fd86fa9a63d877324fa590b9f167ec18ac59da4edbec5910e8bd5e0092466432
seed grouping digest: events=1000000 route=fixed_five table=analysis_sessions rows=20 sha256=ecb3dcb59386dc033970edb6c1c73d7c9dc97d97de088e8ed852e709ce782f0c
seed grouping digest: events=1000000 route=fixed_five table=analysis_results rows=400 sha256=7895f204b5df12ad4089cc1fe6840f83d37980c5746ff0a347f5d6a261175f05
seed grouping digest: events=1000000 route=fixed_five table=settings rows=8 sha256=3b6dfe471447fd69ca1ad8d08852eb4f9023136a912995dfa47c6321ba4df06e
seed grouping digest: events=1000000 route=fixed_five table=event_category_counts rows=15 sha256=d1124f46bb2684291525d3f26db50780a2327a57d19a0cf77d6ed2d95833c886
seed grouping digest: events=1000000 route=fixed_five table=event_graph_ownership rows=2774 sha256=e29e370e2d04f5779955f3fd44cc7214898d18fc28f71deaa44518ca44a0f8a0
seed grouping cleanup: events=1000000 route=fixed_five closed_joined_removed_verified=1
seed grouping fixed pair result: events=1000000 count_content_ownership_parity=1 baseline_elapsed_ms=369085.288 fixed_five_elapsed_ms=332180.967 measured_pairs=1 order_effect_uncontrolled=1 acceptance_claimed=0
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out; finished in 736.74s

```

