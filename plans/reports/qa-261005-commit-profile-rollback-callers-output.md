# Restored caller verification output — 2026-10-05

Status: PASS, exit0. Command:
`cargo test --release --locked --features devtools --test query_service --test repository --test seed_history_purge --test seed_insertion_profile --test page_plan_capture_profile -- --test-threads=1`.
One build job, existing release target. Compilation11m12s.
Query9/repository24/purge9/insertion2/PAGEfixture3 pass: 47 passed,
0 failed, two deliberately ignored diagnostics. All49 records below.
The PAGE canonical diagnostic and insertion attribution canonical diagnostic
are not rerun here. Ordinary purge correctness cases do create their own small
owned corpora; this is not a repeat of the fixed seed pairs or PROFILE experiment.
No p95, seed-time, current-source installer or phase acceptance claimed.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 11m 12s
     Running tests\page_plan_capture_profile.rs (target\release\deps\page_plan_capture_profile-231292b0146e9340.exe)

running 4 tests
test canonical_10k_actual_first_and_tenth_page_plans ... ignored, one unchanged canonical10k actual page1/page10 diagnostic; root executes after review
test perf_fixture::app_state_fixture_joins_maintenance_and_removes_its_owned_directory ... ok
test perf_fixture::assertion_failure_still_closes_and_removes_the_fixture ... ok
test perf_fixture::dropping_a_fixture_removes_only_its_owned_directory ... ok

test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.37s

     Running tests\query_service.rs (target\release\deps\query_service-e304c0772d6b058d.exe)

running 9 tests
test a_docker_burst_clusters_and_the_cursor_still_follows_raw_rows ... ok
test clustered_pages_keep_the_raw_cursor_without_skipping_the_next_page ... ok
test cursor_encoding_is_opaque_round_trippable_and_rejects_invalid_values ... ok
test details_return_related_rows_and_prior_occurrences ... ok
test every_palette_fixture_term_and_numeric_port_is_searchable ... ok
test every_ui_filter_and_cursor_combination_uses_an_index ... ok
test page_capture_service_tests::captured_tied_cluster_pages_preserve_all_raw_ids_and_next_cursor ... ok
test search_and_dashboard_summary_use_the_real_index_and_counts ... ok
test synthetic_profiles_are_sanitized_deterministic_and_calm ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s

     Running tests\repository.rs (target\release\deps\repository-e2f16f6a12ab83dc.exe)

running 24 tests
test a_baseline_survives_a_restart_byte_identical ... ok
test a_limit_over_the_cap_is_clamped_and_a_zero_limit_returns_one_row ... ok
test an_oversized_baseline_is_replaced_by_a_marker_not_stored ... ok
test batched_prior_occurrences_keep_exclusive_before_inclusive_since_and_zero_counts ... ok
test batched_search_summaries_keep_rank_order_and_missing_rows_are_skipped ... ok
test config_snapshots_key_on_the_canonical_path_and_go_with_their_folder ... ok
test counts_and_prior_occurrences_answer_the_details_drawer ... ok
test dense_relationship_scan_matches_keyed_lookup_with_time_and_endpoint_filters ... ok
test event_summaries_round_trip_before_and_after_values_on_the_wire ... ok
test every_filter_path_uses_an_index_rather_than_scanning_events ... ok
test filters_narrow_by_category_importance_and_entity ... ok
test full_text_search_ranks_by_importance_then_timestamp ... ok
test inserted_events_come_back_with_ids_and_their_entities_recorded ... ok
test keyset_pages_are_stable_and_non_overlapping_under_concurrent_inserts ... ok
test large_sorted_recurrence_matches_individual_index_counts_and_batch_overwrites ... ok
test queued_writes_publish_one_notice_per_batch_with_no_bodies ... ok
test removing_a_watched_location_deletes_its_snapshots_in_the_same_transaction ... ok
test removing_an_overlapping_parent_preserves_child_baselines_until_the_child_is_removed ... ok
test retention_deletes_only_expired_rows_keeps_important_longer_and_spares_checkpoint_snapshots ... ok
test search_finds_ports_prefixes_and_phrases ... ok
test settings_are_seeded_and_round_trip ... ok
test skewed_dense_relationship_scan_retries_keyed_even_after_early_matches ... ok
test ten_thousand_events_insert_in_batches_within_one_measured_run ... ok
test watched_location_removal_rolls_back_when_snapshot_cleanup_fails ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.60s

     Running tests\seed_history_purge.rs (target\release\deps\seed_history_purge-015922782ed19b2f.exe)

running 9 tests
test cli_refuses_missing_data_dir_and_nonempty_seed_directory ... ok
test collector_payload_sensitivity_keeps_corpus_identity_and_persists_checked_in_shapes ... ok
test concurrent_purge_refuses_while_seed_holds_directory_lock ... ok
test no_relations_seed_has_neither_related_ids_nor_graph_edges ... ok
test nonfresh_database_with_real_entity_and_edge_is_rejected_unchanged ... ok
test registered_purge_is_exact_idempotent_and_preserves_real_lookalikes ... ok
test retention_then_purge_preserves_edges_without_attributable_event_provenance ... ok
test ten_thousand_seed_has_required_distribution_and_purge_removes_its_artifacts ... ok
test wrong_owner_marker_and_stale_lock_refuse_purge ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.03s

     Running tests\seed_insertion_profile.rs (target\release\deps\seed_insertion_profile-4bc2eef69d59356e.exe)

running 3 tests
test canonical_seed_reports_aggregate_insertion_attribution ... ignored, Explicit diagnostic: one fresh canonical corpus with insertion attribution; no acceptance budget
test captured_writer_bridge_preserves_results_and_recovers_after_rollback ... ok
test seed_insertion_fixture::diagnostic_fixture_cleanup_closes_db_and_preserves_siblings_on_unwind ... ok

test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.24s

```

Unresolved questions: lint/final rollback checks and remaining plan obligations.

