# Checkpoint window full-library output — part 4 — 2026-10-05

Status: terminal PASS,535 passed,0 failed,3 deliberate ignored,538 total
records; 23.54s, cached build0.86s. Consecutive presentation of the retained
55,075-character stdout stored before response rendering. The four fenced
payloads together contain55,076 characters: Markdown adds one final LF.
Removing exactly that documented separator LF reconstructs the retained text;
this is character-content equality, not a byte-for-byte file identity claim.
Command: `cargo test --release --locked --all-features --lib -- --test-threads=1`.
One build job, existing release target. [Source freeze](qa-261005-checkpoint-window-source-freeze.md).
No canonical diagnostic or installed-app acceptance in this command.

```text
test storage::event_repository::recurrence_incremental_prototype::parity::span_guard_retains_valid_duplicates_and_single_delta_probes ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::span_guard_rolls_only_below_the_full_span_cost ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::wal_read_snapshot_survives_an_external_commit ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::widened_endpoints_preserve_guard_boundaries_at_i64_extremes ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::empty_inputs_have_zero_statistics ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::identical_windows_ignore_event_ids_and_count_every_adjacent_overlap ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::invalid_windows_count_as_distinct_keys_and_interrupt_adjacency ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::maximum_group_size_includes_repeated_queries_and_the_last_group ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::nesting_and_decreasing_before_are_distinct_from_monotone_overlap ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::signature_boundaries_include_entity_category_and_event_type ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::statistics_walk_the_supplied_execution_order ... ok
test storage::event_repository::recurrence_plan_capture::recurrence_window_statistics::tests::touching_and_disjoint_half_open_windows_do_not_overlap ... ok
test storage::event_repository::recurrence_plan_capture::tests::actual_five_thousand_counts_match_captured_input_statistics ... ok
test storage::event_repository::recurrence_plan_capture::tests::actual_sorted_invocation_reports_index_and_completed_counts_without_bind_values ... ok
test storage::event_repository::recurrence_plan_capture::tests::delta_explain_error_is_static_and_preserves_operation_value ... ok
test storage::event_repository::recurrence_plan_capture::tests::explain_error_is_static_and_does_not_replace_operation_result ... ok
test storage::event_repository::recurrence_plan_capture::tests::inactive_capture_does_not_explain ... ok
test storage::event_repository::recurrence_plan_capture::tests::multiple_invocations_are_rejected_without_changing_repository_results ... ok
test storage::event_repository::recurrence_plan_capture::tests::nested_scope_rejection_preserves_outer_capture ... ok
test storage::event_repository::recurrence_plan_capture::tests::returned_error_and_unwinding_clear_thread_local_scope ... ok
test storage::event_repository::recurrence_plan_capture::tests::transaction_capture_classifies_actual_full_and_two_delta_probes ... ok
test storage::event_repository::recurrence_plan_capture::tests::transaction_capture_omits_zero_width_deltas_and_rejects_wide_shifts ... ok
test storage::event_repository::recurrence_plan_capture::tests::transaction_capture_reuses_five_thousand_identical_valid_windows ... ok
test storage::event_repository::recurrence_plan_capture::tests::transaction_error_and_unwinding_clear_capture_after_real_queries ... ok
test storage::event_repository::recurrence_plan_capture::tests::transaction_nested_and_multiple_scopes_preserve_repository_results ... ok
test storage::investigations::tests::a_result_insert_failure_rolls_back_metadata_session_and_results ... ok
test storage::investigations::tests::create_is_sanitized_atomic_and_reopens_without_events ... ok
test storage::investigations::tests::maintenance_keeps_ninety_days_of_investigation_history ... ok
test storage::investigations::tests::validation_rolls_back_the_whole_investigation ... ok
test storage::migrations::tests::a_checksum_is_hex_sha256 ... ok
test storage::migrations::tests::the_search_migration_has_exactly_one_body ... ok
test storage::migrations::tests::versions_are_consecutive_from_one_and_the_files_are_distinct ... ok
test storage::page_plan_capture::page_plan_explain::tests::actual_sqlite_projection_preserves_all_node_ids_parents_and_order ... ok
test storage::page_plan_capture::page_plan_explain::tests::pinned_access_sort_and_or_vocabulary_preserves_structural_fields ... ok
test storage::page_plan_capture::page_plan_explain::tests::unknown_names_literals_and_vocabulary_are_not_retained_or_classified ... ok
test storage::page_plan_capture::tests::capture_is_thread_local_and_clears_after_unwind ... ok
test storage::page_plan_capture::tests::inactive_and_missing_scopes_do_not_claim_an_invocation ... ok
test storage::page_plan_capture::tests::multiple_actual_calls_and_variant_mismatch_preserve_operation_results ... ok
test storage::page_plan_capture::tests::nested_scope_preserves_outer_capture_and_does_not_run_inner_operation ... ok
test storage::page_plan_capture::tests::primitive_binding_mismatch_does_not_override_real_operation ... ok
test storage::page_plan_capture::tests::primitive_explain_sqlite_failure_is_independent_of_successful_real_operation ... ok
test storage::purge::tests::full_purge_keeps_reviewed_preferences_and_clears_history_tables ... ok
test storage::reader_pool::tests::a_connection_goes_back_to_the_pool_when_dropped ... ok
test storage::search_index::tests::a_pasted_wall_of_text_is_cut_to_the_limit ... ok
test storage::search_index::tests::tokens_become_quoted_phrases_and_the_last_is_a_prefix ... ok
test storage::settings_repository::tests::a_collector_flag_round_trips_and_an_unknown_id_is_refused ... ok
test storage::settings_repository::tests::a_garbled_row_falls_back_to_the_default_rather_than_failing_the_screen ... ok
test storage::settings_repository::tests::a_multi_field_update_rolls_back_when_a_later_field_is_invalid ... ok
test storage::settings_repository::tests::an_update_touches_only_what_it_names_and_survives_a_reopen ... ok
test storage::settings_repository::tests::retention_is_bounded_so_zero_days_can_never_mean_delete_everything ... ok
test storage::settings_repository::tests::the_first_hide_flag_flips_once_and_stays_flipped ... ok
test storage::settings_repository::tests::the_seeded_defaults_are_what_the_screen_first_renders ... ok
test storage::settings_repository::tests::visualization_preference_round_trips_without_a_schema_key_seed ... ok
test storage::writer::tests::a_full_queue_counts_admission_drops_without_calling_them_commit_failures ... ok
test storage::writer::tests::a_rejected_baseline_only_cycle_has_cycle_telemetry_and_cannot_advance ... ok
test storage::writer::tests::a_ticket_transferred_to_its_writer_rejects_wait_without_blocking ... ok
test storage::writer::tests::an_injected_commit_failure_counts_rows_by_collector_not_as_queue_drops ... ok
test testing::synthetic::tests::only_documented_simulation_kinds_are_accepted ... ok
test testing::synthetic::tests::profile_selectors_parse_and_relations_are_optional ... ok
test testing::synthetic::tests::profiles_are_deterministic_and_always_label_the_source ... ok
test testing::tests::adding_collector_summaries_does_not_resequence_legacy_corpora ... ok
test tests::autostart_never_hides_the_only_window_when_the_tray_fails ... ok
test tests::executable_rejects_reserved_qa_directory_names ... ok
test tests::the_registered_handler_matches_the_documented_command_list ... ok

test result: ok. 535 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 23.54s


```
