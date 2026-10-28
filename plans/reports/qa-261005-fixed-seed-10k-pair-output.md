# Fixed seed-grouping 10k pair output — 2026-10-05

[Reviewed immutable protocol](qa-261005-fixed-seed-pair-harness.md).
Command: `cargo test --release --locked --features devtools --bin seed_history seed_grouped_events::measurement::fixed_seed_grouping_10k_pair -- --exact --ignored --nocapture --test-threads=1`.
Exit0, one pair/one test, all table/count/ownership digests equal; both fixtures
closed/joined/deleted before return. Baseline1,109.966ms versus grouped1,257.987ms:
this single fixed-order 10k observation is slower, not a performance win.
It is not extrapolated to1M or a profiler-free acceptance/default promotion.
Complete stdout/stderr follows; no observations omitted.

```text
Finished `release` profile [optimized] target(s) in 0.66s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 1 test
test seed_grouped_events::measurement::fixed_seed_grouping_10k_pair ... seed grouping frozen pair: events=10000 order=baseline_then_fixed_five pairs=1 days=90 seed=3235782694 end_at=1800000000000 relations=1 relation_percent=15 bursts_percent=8 entities=3000 checkpoints=40 snapshots=9800 investigations=20 collector_payload_sensitivity=0 insertion_timings=1 stage_timings=0 logical_batch_rows=1000 grouped_logical_units=5 public_default_unchanged=1
seed grouping source memory: events=10000 baseline_max_prepared_rows=1000 fixed_five_max_prepared_rows=5000 maximum_relation_ordinals=3000 grouped_provisional_ordinal_clone=3000 ownership_snapshot_max=3000 ownership_digest_sorted_refs_max=3000 digest_max_retained_corpus_rows=1 existing_selection_mask_max=10000 existing_temporary_eligible_indices_max=10000 native_peak_unmeasured=1 dynamic_payload_bytes_unmeasured=1 memory_acceptance_claimed=0
seed grouping disk admission: events=10000 route=baseline available_bytes=2994294784 required_bytes=825786368 bytes_per_input=2048 wal_temp_reserve_bytes=536870912 build_headroom_bytes=268435456
seed grouping SQLite: events=10000 route=baseline version=3.53.2 journal_mode=wal
seed grouping SQLite numeric: events=10000 route=baseline name=synchronous value=1
seed grouping SQLite numeric: events=10000 route=baseline name=foreign_keys value=1
seed grouping SQLite numeric: events=10000 route=baseline name=temp_store value=2
seed grouping SQLite numeric: events=10000 route=baseline name=cache_size value=-8000
seed grouping SQLite numeric: events=10000 route=baseline name=wal_autocheckpoint value=1000
seed grouping SQLite numeric: events=10000 route=baseline name=busy_timeout value=5000
seed grouping SQLite numeric: events=10000 route=baseline name=auto_vacuum value=2
seed grouping SQLite numeric: events=10000 route=baseline name=page_size value=4096
seeded 10000/10000 synthetic events
seed grouping full seed: events=10000 route=baseline elapsed_ms=1109.966 clock=before_marker_registration_through_finish_seed_and_flush ownership_snapshot_included=1 fixture_open_settings_close_validation_digest_report_cleanup_excluded=1
seed insertion timings: batches=10 attempted_rows=10000 committed_batches=10 committed_rows=10000 failed_batches=0 empty_batches=0 completed_phases=40 begin_prepare_ms=0.299 event_entity_loop_ms=600.636 reconciliation_ms=0.105 commit_ms=92.148 repository_total_ms=693.195 caller_insert_ms=697.995 caller_writer_overhead_ms=4.799
seed timings: elapsed_make_batch_ms=53.771 insert_events_ms=697.995 graph_ms=23.641 inventory_ms=37.904 checkpoints_ms=81.775 investigations_ms=214.106 total_run_ms=1109.966
seed grouping digest: events=10000 route=baseline table=events rows=10000 sha256=1ddb78200543695eb3dac961cc0dcaa6ee1315960c38750a8b95029b9b472c72
seed grouping digest: events=10000 route=baseline table=entities rows=3000 sha256=beb19d0342cba2154bb2c3cec057f8b60cff8a4720c8ade825560b281f67a90a
seed grouping digest: events=10000 route=baseline table=entity_relationships rows=1500 sha256=bba03b0668a0226c4cc243a1fc5f9b5691a5b67b5cea75fcf3d056f576eebde3
seed grouping digest: events=10000 route=baseline table=checkpoints rows=40 sha256=7e9e2e1d7455ed606907de2f442df740daeee2ec22b36ffe396311e9f1449de6
seed grouping digest: events=10000 route=baseline table=snapshots rows=9800 sha256=78ad855d44db68458d2607a04017d426bd124941f33a920e54efd53ec9701cc4
seed grouping digest: events=10000 route=baseline table=investigations rows=20 sha256=c06a54ce65bee75eff948bba3f248616897394b6d794a32461abec35b06e3223
seed grouping digest: events=10000 route=baseline table=analysis_sessions rows=20 sha256=fac39a749941d5478f1e496803edfdba7b57ab2e418018738b600a28bfdc6a5e
seed grouping digest: events=10000 route=baseline table=analysis_results rows=400 sha256=379c72c9c90726e756d33755006e1a565ca239739d725ab81958b1d13fe0ca23
seed grouping digest: events=10000 route=baseline table=settings rows=8 sha256=3b6dfe471447fd69ca1ad8d08852eb4f9023136a912995dfa47c6321ba4df06e
seed grouping digest: events=10000 route=baseline table=event_category_counts rows=15 sha256=2f944a9e388177a2312bc824f0f16b33dcaa7fed40657288d40b69d0893df2bc
seed grouping digest: events=10000 route=baseline table=event_graph_ownership rows=2578 sha256=9de3df7cb0f8b2bb5ee4e829d960a4b92fe2341f74b434be0efe1738b9576a8e
seed grouping cleanup: events=10000 route=baseline closed_joined_removed_verified=1
seed grouping disk admission: events=10000 route=fixed_five available_bytes=2989178880 required_bytes=825786368 bytes_per_input=2048 wal_temp_reserve_bytes=536870912 build_headroom_bytes=268435456
seed grouping SQLite: events=10000 route=fixed_five version=3.53.2 journal_mode=wal
seed grouping SQLite numeric: events=10000 route=fixed_five name=synchronous value=1
seed grouping SQLite numeric: events=10000 route=fixed_five name=foreign_keys value=1
seed grouping SQLite numeric: events=10000 route=fixed_five name=temp_store value=2
seed grouping SQLite numeric: events=10000 route=fixed_five name=cache_size value=-8000
seed grouping SQLite numeric: events=10000 route=fixed_five name=wal_autocheckpoint value=1000
seed grouping SQLite numeric: events=10000 route=fixed_five name=busy_timeout value=5000
seed grouping SQLite numeric: events=10000 route=fixed_five name=auto_vacuum value=2
seed grouping SQLite numeric: events=10000 route=fixed_five name=page_size value=4096
seeded 10000/10000 synthetic events
seed grouping full seed: events=10000 route=fixed_five elapsed_ms=1257.987 clock=before_marker_registration_through_finish_seed_and_flush ownership_snapshot_included=1 fixture_open_settings_close_validation_digest_report_cleanup_excluded=1
seed grouping actual groups: events=10000 route=fixed_five attempted_groups=2 committed_groups=2 failed_groups=0 attempted_input_rows=10000 started_units=10 completed_body_rows=10000 completed_units=10 committed_input_rows=10000 progress_rows=10000 progress_groups=2 progress_notifications=1 begin_ms=0.025 graph_ms=17.165 commit_ms=79.421 group_total_ms=852.211
seed grouping actual bodies: events=10000 route=fixed_five bodies=10 attempted_rows=10000 completed_bodies=10 completed_rows=10000 failed_bodies=0 empty_bodies=0 completed_phases=30 preparation_ms=0.296 event_entity_loop_ms=750.412 reconciliation_ms=0.099 body_total_ms=750.807 body_success_is_not_commit=1
seed grouping coarse: events=10000 route=fixed_five make_batch_ms=65.701 inventory_ms=22.527 checkpoints_ms=91.123 investigations_ms=225.594 legacy_insert_graph_fields_not_reinterpreted=1
seed grouping digest: events=10000 route=fixed_five table=events rows=10000 sha256=1ddb78200543695eb3dac961cc0dcaa6ee1315960c38750a8b95029b9b472c72
seed grouping digest: events=10000 route=fixed_five table=entities rows=3000 sha256=beb19d0342cba2154bb2c3cec057f8b60cff8a4720c8ade825560b281f67a90a
seed grouping digest: events=10000 route=fixed_five table=entity_relationships rows=1500 sha256=bba03b0668a0226c4cc243a1fc5f9b5691a5b67b5cea75fcf3d056f576eebde3
seed grouping digest: events=10000 route=fixed_five table=checkpoints rows=40 sha256=7e9e2e1d7455ed606907de2f442df740daeee2ec22b36ffe396311e9f1449de6
seed grouping digest: events=10000 route=fixed_five table=snapshots rows=9800 sha256=78ad855d44db68458d2607a04017d426bd124941f33a920e54efd53ec9701cc4
seed grouping digest: events=10000 route=fixed_five table=investigations rows=20 sha256=c06a54ce65bee75eff948bba3f248616897394b6d794a32461abec35b06e3223
seed grouping digest: events=10000 route=fixed_five table=analysis_sessions rows=20 sha256=fac39a749941d5478f1e496803edfdba7b57ab2e418018738b600a28bfdc6a5e
seed grouping digest: events=10000 route=fixed_five table=analysis_results rows=400 sha256=379c72c9c90726e756d33755006e1a565ca239739d725ab81958b1d13fe0ca23
seed grouping digest: events=10000 route=fixed_five table=settings rows=8 sha256=3b6dfe471447fd69ca1ad8d08852eb4f9023136a912995dfa47c6321ba4df06e
seed grouping digest: events=10000 route=fixed_five table=event_category_counts rows=15 sha256=2f944a9e388177a2312bc824f0f16b33dcaa7fed40657288d40b69d0893df2bc
seed grouping digest: events=10000 route=fixed_five table=event_graph_ownership rows=2578 sha256=9de3df7cb0f8b2bb5ee4e829d960a4b92fe2341f74b434be0efe1738b9576a8e
seed grouping cleanup: events=10000 route=fixed_five closed_joined_removed_verified=1
seed grouping fixed pair result: events=10000 count_content_ownership_parity=1 baseline_elapsed_ms=1109.966 fixed_five_elapsed_ms=1257.987 measured_pairs=1 order_effect_uncontrolled=1 acceptance_claimed=0
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out; finished in 2.59s
```

