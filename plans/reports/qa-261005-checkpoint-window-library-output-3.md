# Checkpoint window full-library output — part 3 — 2026-10-05

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
test platform::windows::named_pipe::tests::rejects_request_injection ... ok
test platform::windows::named_pipe::tests::stream_idle_timeout_renews_each_operation ... ok
test platform::windows::named_pipe::tests::truncated_valid_json_prefix_fails_declared_content_length ... ok
test platform::windows::named_pipe::tests::undrained_cancellation_poison_prevents_second_operation ... ok
test platform::windows::network::tests::native_wake_callback_contains_a_panicking_runtime_closure ... ok
test platform::windows::network::tests::vpn_detection_does_not_treat_hyper_v_as_a_vpn ... ok
test platform::windows::process::tests::a_path_denial_is_reported_as_absence_rather_than_as_a_failure ... ok
test platform::windows::process::tests::a_snapshot_of_this_machine_names_the_process_reading_it ... ok
test platform::windows::process::tests::a_wide_buffer_becomes_text_at_its_terminator ... ok
test platform::windows::process::tests::caller_owned_row_storage_is_reused_across_snapshots ... ok
test platform::windows::process::tests::enumerating_repeatedly_does_not_accumulate_handles ... ok
test platform::windows::process::tests::no_pid_appears_twice_in_one_snapshot ... ok
test platform::windows::process::tests::repeated_snapshots_reuse_the_large_path_workspace ... ok
test platform::windows::registry::environment_tests::environment_scopes_merge_case_insensitively_with_user_precedence ... ok
test platform::windows::registry::environment_tests::expandable_non_path_values_are_expanded_with_the_same_bound ... ok
test platform::windows::registry::environment_tests::path_expansion_is_case_insensitive_recursive_and_cycle_safe ... ok
test platform::windows::registry::environment_tests::path_expansion_rejects_output_beyond_the_hard_bound ... ok
test platform::windows::resume::tests::a_sleep_gap_is_distinct_from_ordinary_elapsed_time_and_clock_noise ... ok
test platform::windows::services::tests::a_non_elevated_real_snapshot_is_complete_and_named ... ok
test platform::windows::services::tests::image_path_extraction_handles_quotes_arguments_and_drivers ... ok
test platform::windows::socket_rows::tests::a_row_becomes_the_product_fact_the_diff_keys_on ... ok
test platform::windows::socket_rows::tests::an_ipv4_address_reads_in_octet_order ... ok
test platform::windows::socket_rows::tests::an_ipv6_address_compresses_and_keeps_its_scope ... ok
test platform::windows::socket_rows::tests::port_four_thousand_survives_the_byte_order_conversion ... ok
test platform::windows::socket_rows::tests::the_ipv6_rows_carry_their_address_inline_and_their_scope_beside_it ... ok
test platform::windows::sockets::tests::a_count_larger_than_the_buffer_fails_the_whole_table ... ok
test platform::windows::sockets::tests::a_payload_ending_part_way_through_a_row_fails_visibly ... ok
test platform::windows::sockets::tests::a_real_provider_reads_the_listeners_this_machine_has ... ok
test platform::windows::sockets::tests::a_sub_header_probe_size_is_malformed_for_every_allowed_status ... ok
test platform::windows::sockets::tests::a_successful_read_that_initializes_less_than_a_header_is_malformed ... ok
test platform::windows::sockets::tests::a_table_that_keeps_growing_gives_up_after_three_attempts ... ok
test platform::windows::sockets::tests::an_empty_or_truncated_buffer_fails_instead_of_forging_closes ... ok
test platform::windows::sockets::tests::an_impossible_table_size_is_refused_before_it_is_allocated ... ok
test platform::windows::sockets::tests::caller_owned_socket_rows_are_reused_across_polls ... ok
test platform::windows::sockets::tests::no_data_for_a_family_is_an_answer_rather_than_a_failure ... ok
test platform::windows::sockets::tests::repeated_table_reads_reuse_the_existing_allocation ... ok
test platform::windows::sockets::tests::rows_are_read_out_of_the_buffer_the_api_wrote ... ok
test platform::windows::sockets::tests::spare_complete_bytes_are_not_mistaken_for_declared_rows ... ok
test platform::windows::sockets::tests::the_size_probe_loop_grows_the_buffer_until_the_platform_is_satisfied ... ok
test platform::windows::tasks::tests::failed_task_partition_does_not_hide_other_startup_sources ... ok
test platform::windows::tasks::tests::notification_callback_wakes_only_while_live ... ok
test platform::windows::tasks::tests::ole_date_conversion_rejects_never_run_and_converts_epoch ... ok
test platform::windows::tool_probe::tests::path_parser_respects_quotes_and_ignores_empty_entries ... ok
test platform::windows::tool_probe::tests::registered_probe_pid_is_excluded_only_for_its_executable ... ok
test platform::windows::tool_probe::tests::resolution_refuses_names_outside_the_allowlist ... ok
test platform::windows::wua::tests::ole_automation_epoch_conversion_is_stable ... ok
test platform::windows::wua::tests::runtime_recreation_does_not_overlap_timed_out_wua_query_and_recovers ... ok
test platform::windows::wua::tests::timed_out_request_does_not_make_provider_drop_wait_for_apartment ... ok
test privacy::entropy::tests::fires_on_an_opaque_encoded_blob ... ok
test privacy::entropy::tests::measures_the_reference_cases ... ok
test privacy::entropy::tests::spares_a_repetitive_string_that_only_looks_encoded ... ok
test privacy::entropy::tests::spares_the_values_the_product_exists_to_show ... ok
test privacy::keys::tests::accepts_the_documented_over_redaction ... ok
test privacy::keys::tests::catches_a_glued_name_with_no_word_boundary ... ok
test privacy::keys::tests::leaves_names_that_only_look_sensitive ... ok
test privacy::keys::tests::normalizes_every_spelling_of_the_same_key ... ok
test privacy::keys::tests::redacts_by_last_word_whatever_the_prefix ... ok
test privacy::keys::tests::redacts_the_documented_sensitive_names ... ok
test privacy::keys::tests::splits_on_separators_and_case_transitions ... ok
test privacy::keys::tests::the_cheap_filter_never_rejects_something_the_rules_would_catch ... ok
test privacy::keys::tests::the_cheap_filter_rejects_an_ordinary_name ... ok
test privacy::keys::tests::treats_a_trailing_qualifier_as_part_of_the_same_thing ... ok
test privacy::patterns::tests::defers_the_generic_rule_to_the_entropy_gate ... ok
test privacy::patterns::tests::every_pattern_compiles ... ok
test privacy::patterns::tests::keeps_the_host_of_a_uri_and_drops_the_credentials ... ok
test privacy::patterns::tests::keeps_the_readable_part_of_a_bearer_header ... ok
test privacy::patterns::tests::kind_names_are_unique_and_stable ... ok
test privacy::patterns::tests::leaves_a_uri_without_credentials_alone ... ok
test privacy::patterns::tests::redacts_each_shape_and_names_the_rule ... ok
test privacy::patterns::tests::redacts_every_occurrence_in_one_line ... ok
test privacy::patterns::tests::the_run_scan_finds_what_the_generic_rule_needs ... ok
test privacy::patterns::tests::the_two_pre_filters_cover_the_whole_table_between_them ... ok
test privacy::sanitize::tests::a_bare_string_is_scrubbed_by_shape ... ok
test privacy::sanitize::tests::a_sensitive_key_hides_a_whole_subtree ... ok
test privacy::sanitize::tests::an_absent_option_is_left_alone ... ok
test privacy::sanitize::tests::an_entity_scrubs_only_its_display_and_metadata ... ok
test privacy::sanitize::tests::an_event_is_scrubbed_field_by_field_and_marked_sensitive ... ok
test privacy::sanitize::tests::json_redacts_by_key_at_any_depth_and_names_the_path ... ok
test privacy::sanitize::tests::scrubbing_is_idempotent ... ok
test privacy::scrub::tests::a_sensitive_key_cannot_leave_a_credential_behind_it ... ok
test privacy::scrub::tests::a_sensitive_key_replaces_the_whole_value ... ok
test privacy::scrub::tests::a_shape_fires_under_an_innocuous_key ... ok
test privacy::scrub::tests::a_value_whose_quote_is_never_closed_is_still_hidden ... ok
test privacy::scrub::tests::an_escaped_quote_does_not_end_the_value_early ... ok
test privacy::scrub::tests::an_ordinary_key_leaves_an_ordinary_value_borrowed ... ok
test privacy::scrub::tests::keeps_a_quoted_value_with_spaces_inside_the_quotes ... ok
test privacy::scrub::tests::keeps_the_words_after_a_redacted_value ... ok
test privacy::scrub::tests::leaves_a_path_that_contains_a_sensitive_word ... ok
test privacy::scrub::tests::leaves_an_ordinary_line_untouched_and_unallocated ... ok
test privacy::scrub::tests::reads_the_key_immediately_left_of_the_separator ... ok
test privacy::scrub::tests::scrubs_a_pair_inside_free_text ... ok
test privacy::scrub::tests::scrubs_a_quoted_json_pair ... ok
test privacy::scrub::tests::scrubs_a_shape_with_no_key_in_sight ... ok
test privacy::scrub::tests::scrubs_every_sensitive_pair_on_one_line ... ok
test privacy::scrub::tests::the_other_kind_of_quote_does_not_end_the_value ... ok
test services::investigation_service::stage_timings::enabled::tests::disabled_has_no_active_record ... ok
test services::investigation_service::stage_timings::enabled::tests::record_has_positive_aggregate_static_fields_and_error_until_complete ... DIAGNOSTIC investigation_stages status=complete total_inclusive_ms=2.039 input_inclusive_ms=1.039 request_setup_exclusive_ms=2.000 window_hydration_exclusive_ms=0.039 boundary_queries_exclusive_ms=0.000 entity_load_exclusive_ms=0.000 relationships_exclusive_ms=0.000 recurrence_exclusive_ms=0.000 input_finalize_exclusive_ms=0.000 analysis_scoring_exclusive_ms=0.000 result_sanitization_exclusive_ms=0.000 persistence_exclusive_ms=0.000
ok
test services::query_service::tests::calendar_ranges_resolve_each_midnight_independently ... ok
test services::query_service::tests::custom_range_accepts_the_limit_and_rejects_backwards_or_wider_ranges ... ok
test storage::checkpoint_repository::tests::create_is_atomic_and_delete_cascades_items ... ok
test storage::checkpoint_repository::tests::eleventh_automatic_prunes_oldest_and_never_prunes_manual ... ok
test storage::checkpoint_repository::tests::event_window_is_inclusive_ordered_and_bounded_by_contract ... ok
test storage::checkpoint_repository::tests::state_json_is_redacted_before_it_can_reach_storage ... ok
test storage::checkpoint_window_plan_capture::owner_tests::actual_prepare_and_incompatible_schema_decode_errors_remain_incomplete ... ok
test storage::checkpoint_window_plan_capture::owner_tests::actual_unknown_schema_index_changes_only_plan_evidence ... ok
test storage::checkpoint_window_plan_capture::owner_tests::actual_window_cap_preserves_first_ten_thousand_tied_ids ... ok
test storage::checkpoint_window_plan_capture::owner_tests::normalized_inclusive_bounds_preserve_timestamp_id_ties_and_empty_windows ... ok
test storage::checkpoint_window_plan_capture::tests::inactive_and_missing_scopes_do_not_claim_an_invocation ... ok
test storage::checkpoint_window_plan_capture::tests::multiple_actual_calls_preserve_both_original_results ... ok
test storage::checkpoint_window_plan_capture::tests::nested_scope_preserves_outer_and_rejects_before_inner_closure ... ok
test storage::checkpoint_window_plan_capture::tests::primitive_binding_mismatch_does_not_replace_operation_result ... ok
test storage::checkpoint_window_plan_capture::tests::primitive_explain_failure_and_execution_transitions_are_independent ... ok
test storage::checkpoint_window_plan_capture::tests::scopes_are_thread_local_and_clear_after_unwind ... ok
test storage::collector_commit::tests::a_commit_cannot_mix_event_and_baseline_owners ... ok
test storage::collector_commit::tests::atomic_command_weight_limits_accept_boundaries_and_reject_one_more ... ok
test storage::collector_commit::tests::baseline_size_and_event_time_are_bounded_before_admission ... ok
test storage::collector_commit::tests::entity_prefixes_and_relationship_direction_are_checked ... ok
test storage::collector_commit::tests::one_commit_cannot_assign_the_same_edge_to_two_evidence_events ... ok
test storage::collector_commit::tests::relationship_mapping_is_structural_and_endpoints_are_checked ... ok
test storage::connection::tests::a_fresh_writer_is_in_wal_mode_with_the_documented_pragmas ... ok
test storage::connection::tests::a_reader_cannot_write ... ok
test storage::db::tests::a_fresh_open_is_ready_at_the_latest_schema ... ok
test storage::db::tests::a_second_recovery_is_rejected_before_it_can_touch_shared_files ... ok
test storage::db::tests::a_writer_job_cannot_wait_reentrantly_on_the_same_writer ... ok
test storage::db::tests::close_finishes_an_admitted_ticket_without_holding_the_inner_lock ... ok
test storage::db::tests::close_waits_for_recovery_and_cannot_be_reopened_behind_it ... ok
test storage::db::tests::command_recovery_rejects_destructive_actions_on_a_healthy_database ... ok
test storage::db::tests::lifecycle_calls_from_a_writer_callback_do_not_wait_on_the_inner_lock ... ok
test storage::db::tests::recovery_cannot_overtake_a_corruption_transition_drain ... ok
test storage::db::tests::state_serializes_as_a_tagged_word ... ok
test storage::db::tests::the_companion_files_sit_beside_the_database ... ok
test storage::event_repository::insertion_timing_capture::tests::aggregation_adds_counts_and_durations_without_retaining_history ... ok
test storage::event_repository::insertion_timing_capture::tests::caller_unwinding_and_other_threads_cannot_leave_capture_state ... ok
test storage::event_repository::insertion_timing_capture::tests::captured_insert_keeps_actual_rows_ids_and_entity_ownership ... ok
test storage::event_repository::insertion_timing_capture::tests::deferred_commit_failure_includes_rollback_but_no_committed_rows ... ok
test storage::event_repository::insertion_timing_capture::tests::disabled_insert_paths_read_no_attribution_clocks ... ok
test storage::event_repository::insertion_timing_capture::tests::preparation_error_and_empty_capture_do_not_claim_commits ... ok
test storage::event_repository::insertion_timing_capture::tests::real_mid_batch_failure_rolls_back_and_next_capture_is_fresh ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::body_error_returns_no_ownership_and_requires_outer_abandonment ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::body_success_is_uncommitted_and_outer_drop_rolls_back ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::caller_unwind_rolls_back_completed_body_and_leaves_no_capture_state ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::disabled_body_capture_reads_no_clocks_on_success_error_or_empty ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::outer_commit_matches_legacy_rows_and_preserves_real_entities ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::preparation_error_and_empty_body_keep_transaction_ownership ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::successive_bodies_refresh_owned_entities_without_committing ... ok
test storage::event_repository::page_capture_owner_tests::concrete_page_bindings_preserve_exact_sql_order_and_variable_arities ... ok
test storage::event_repository::page_capture_owner_tests::prepare_and_lookahead_decode_errors_remain_incomplete_and_clear_scope ... ok
test storage::event_repository::page_capture_owner_tests::tied_keyset_pages_decode_lookahead_and_complete_empty_pages ... ok
test storage::event_repository::page_capture_owner_tests::unknown_actual_schema_index_fails_only_plan_evidence_not_page_execution ... ok
test storage::event_repository::recurrence_incremental_prototype::benchmark::two_rounds_eleven_alternating_pairs_compare_transaction_local_counts ... ignored, Explicit synthetic matched recurrence benchmark; two rounds of eleven pairs per family
test storage::event_repository::recurrence_incremental_prototype::parity::autocommit_falls_back_directly_without_starting_a_transaction ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::checked_arithmetic_rejects_underflow_and_overflow_without_saturation ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::clustered_edge_history_preserves_parity_despite_short_delta_spans ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::deterministic_histories_and_varied_input_orders_preserve_complete_maps ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::duplicate_ids_keep_original_chunks_and_later_chunk_overwrites ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::duplicate_timestamps_and_all_signature_fields_are_counted_exactly ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::empty_and_threshold_inputs_match_in_one_read_transaction ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::half_open_extreme_and_invalid_windows_preserve_counts_and_probe_rules ... ok
test storage::event_repository::recurrence_incremental_prototype::parity::missing_table_retains_the_existing_typed_prepare_error ... ok
```
