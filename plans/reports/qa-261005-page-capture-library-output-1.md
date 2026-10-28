# Page capture full library output — part1/4 — 2026-10-05

[Frozen source](qa-261005-page-capture-source-freeze.md).
Command: `cargo test --release --locked --all-features --lib -- --test-threads=1`.
Terminal exit0:525 passed/zero failed/three intentional diagnostics ignored,
21.97s after0.88s cached preparation. All528 test records are retained in order.
[part1](qa-261005-page-capture-library-output-1.md), [part2](qa-261005-page-capture-library-output-2.md), [part3](qa-261005-page-capture-library-output-3.md), [part4](qa-261005-page-capture-library-output-4.md). Concatenating code blocks restores full stdout/stderr text;
no per-test line or observed failure is excluded. Caller/lint/canonical checks
are separate and remain pending in this receipt.

```text
    Finished `release` profile [optimized] target(s) in 0.88s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-708106f6a2e3e24a.exe)

running 528 tests
test analysis::explain::tests::deterministic_provider_matches_the_existing_copy_for_every_class ... ok
test analysis::explain::tests::injected_provider_changes_only_explanation_copy ... ok
test analysis::graph::builder_tests::benchmark::isolated_synthetic_old_and_new_builder_durations ... ignored, Isolated synthetic diagnostic; explicitly compare builders after equality tests
test analysis::graph::builder_tests::full_graph_matches_original_for_parallel_kinds_self_loops_and_event_overlap ... ok
test analysis::graph::builder_tests::generated_duplicate_and_mostly_unique_graphs_match_original_in_both_input_orders ... ok
test analysis::graph::tests::multi_source_distances_match_per_candidate_bfs_for_cycles_and_unknown_nodes ... ok
test analysis::score::tests::importance_includes_notable_term ... ok
test analysis::score::tests::temporal_boundaries_are_explicit ... ok
test analysis::stage_timings::enabled::tests::active_record_has_exact_static_fields_and_exclusive_sum ... DIAGNOSTIC analysis_stages status=error total_inclusive_ms=0.052 preprocess_exclusive_ms=0.052 structural_graph_exclusive_ms=0.000 chain_index_exclusive_ms=0.000 failure_index_exclusive_ms=0.000 candidate_preparation_exclusive_ms=0.000 candidate_scoring_exclusive_ms=0.000 selection_explanations_exclusive_ms=0.000
ok
test analysis::stage_timings::enabled::tests::disabled_has_no_active_record ... ok
test analysis::stage_timings::enabled::tests::incomplete_or_unwinding_records_are_errors ... ok
test analysis::tests::nonempty_investigation_matches_per_event_bfs_scores_rank_and_confidence ... ok
test app::event_bus::tests::a_notice_collected_inside_an_open_window_survives_shutdown ... ok
test app::event_bus::tests::a_single_notice_is_still_delivered ... ok
test app::event_bus::tests::a_thousand_notices_in_a_hundred_milliseconds_produce_at_most_two_emits ... ok
test app::event_bus::tests::merging_bounds_event_ids_to_the_newest_window ... ok
test app::event_bus::tests::merging_keeps_the_meaning_of_every_field ... ok
test app::monitoring::tests::a_fresh_gate_is_running_and_says_so ... ok
test app::monitoring::tests::a_timed_pause_carries_its_deadline_and_expires_on_its_own ... ok
test app::monitoring::tests::an_indefinite_pause_stays_paused_and_offers_no_resume_time ... ok
test app::monitoring::tests::set_paused_is_the_toggle_the_tray_and_the_ui_share ... ok
test app::monitoring::tests::the_pause_state_and_its_deadline_are_read_together ... ok
test app::paths::tests::everything_sits_under_the_one_data_folder ... ok
test app::paths::tests::the_resolved_folder_is_named_by_the_identifier ... ok
test app::rate_limit::tests::the_first_ten_calls_pass_and_the_eleventh_does_not ... ok
test app::rate_limit::tests::the_window_resets_and_the_cap_applies_again ... ok
test app::state::tests::a_garbled_marker_counts_as_zero_rather_than_failing ... ok
test app::state::tests::a_state_with_no_tray_says_so_which_is_what_keeps_the_window_closable ... ok
test app::state::tests::app_state_opens_and_shuts_down_cleanly ... ok
test app::state::tests::attaching_a_tray_hands_it_the_current_monitoring_state ... ok
test app::state::tests::concurrent_collector_configuration_keeps_database_and_runtime_equal ... ok
test app::state::tests::the_clean_shutdown_marker_is_written_once_however_many_paths_ask ... ok
test app::state::tests::three_unclean_starts_trigger_safe_mode_and_a_clean_shutdown_resets_it ... ok
test app::tray::tests::a_timed_pause_labels_the_resume_deadline ... ok
test app::tray::tests::checkpoint_menu_uses_the_same_application_operation_as_the_command ... ok
test app::tray::tests::every_embedded_tray_variant_decodes_as_a_small_png ... ok
test app::tray::tests::every_id_is_distinct ... ok
test app::tray::tests::every_route_the_tray_emits_exists_in_the_frontends_route_table ... ok
test app::tray::tests::exactly_the_three_screen_items_carry_a_route ... ok
test app::tray::tests::forced_colors_overrides_either_system_theme ... ok
test app::tray::tests::pause_and_resume_are_both_present_so_the_menu_never_changes_shape ... ok
test app::tray::tests::system_theme_selects_the_matching_tray_mark ... ok
test app::tray::tests::the_menu_is_exactly_the_seven_entries_in_the_products_order ... ok
test app::validate::tests::a_file_is_not_a_folder ... ok
test app::validate::tests::a_folder_that_does_not_exist_is_refused_before_anything_else ... ok
test app::validate::tests::a_name_is_trimmed_and_bounded_in_characters ... ok
test app::validate::tests::a_neighbouring_directory_is_not_inside_the_forbidden_one ... ok
test app::validate::tests::a_real_folder_outside_the_forbidden_roots_is_accepted ... ok
test app::validate::tests::a_rejection_never_repeats_the_value_it_rejected ... ok
test app::validate::tests::nested_redirections_are_rejected_at_collection_time ... ok
test app::validate::tests::ranges_and_depths_are_closed_intervals ... ok
test app::validate::tests::the_apps_own_data_folder_is_never_watchable ... ok
test app::validate::tests::the_root_cap_is_reached_rather_than_exceeded ... ok
test app::validate::tests::the_verbatim_prefix_never_reaches_the_user ... ok
test app::validate::tests::the_windows_directory_is_refused ... ok
test app::window::tests::the_balloon_says_recording_continues_and_where_to_look ... ok
test app::window::tests::the_window_only_hides_when_there_is_a_tray_to_get_it_back_from ... ok
test checkpoint::auto::tests::automatic_queue_is_bounded_and_nonblocking ... ok
test checkpoint::auto::tests::five_installer_starts_in_ten_minutes_fire_once ... ok
test checkpoint::compare::tests::skips_a_collector_missing_from_either_side ... ok
test checkpoint::compare::tests::unknown_collectors_are_skipped_instead_of_compared_structurally ... ok
test checkpoint::create::tests::preparation_sanitizes_state_and_normalizes_manifests ... ok
test checkpoint::service::tests::checkpoint_intents_only_coalesce_when_all_details_match ... ok
test checkpoint::service::tests::disabled_and_unavailable_collectors_are_skipped_not_missing ... ok
test checkpoint::service::tests::pending_durable_cycle_stays_missing_until_the_final_chunk_is_committed ... ok
test checkpoint::service::tests::queued_automatic_intent_runs_after_a_different_active_flight ... ok
test checkpoint::service::tests::shutdown_gate_rejects_new_work_and_waits_for_active_work ... ok
test checkpoint::service::tests::side_specific_missing_includes_manifest_asymmetry ... ok
test checkpoint::state_items::tests::acknowledged_maps_become_one_item_per_entity ... ok
test checkpoint::state_items::tests::config_checkpoints_ignore_format_and_redacted_value_uncertainty ... ok
test checkpoint::state_items::tests::empty_update_queue_is_drained_but_non_empty_queue_is_incomplete ... ok
test checkpoint::state_items::tests::network_adapters_are_individual_added_and_removed_items ... ok
test checkpoint::state_items::tests::network_gateway_routes_and_proxy_follow_live_change_cardinality ... ok
test checkpoint::state_items::tests::one_truncated_config_snapshot_makes_files_incomplete_on_either_side ... ok
test checkpoint::state_items::tests::pending_chunk_is_excluded_until_the_final_target_is_drained ... ok
test checkpoint::state_items::tests::recycled_pid_is_removed_and_added_while_readability_is_not_a_change ... ok
test checkpoint::state_items::tests::remaining_structural_collectors_use_owned_checkpoint_semantics ... ok
test checkpoint::state_items::tests::required_baseline_contract_is_only_the_p09_to_p13_structural_collectors ... ok
test checkpoint::state_items::tests::service_checkpoint_diff_matches_running_boundary_and_start_type_rules ... ok
test checkpoint::state_items::tests::startup_task_runtime_metadata_is_not_a_checkpoint_change ... ok
test checkpoint::state_items::tests::truncated_baseline_makes_the_whole_collector_incomplete ... ok
test checkpoint::state_items::tests::weak_process_identity_promotes_without_restart_but_pid_reuse_does_not ... ok
test collectors::application_install_map::tests::cancelled_or_stale_publication_cannot_supply_an_edge ... ok
test collectors::application_install_map::tests::only_component_bounded_install_roots_match_a_service_binary ... ok
test collectors::application_install_map::tests::the_most_specific_install_root_owns_the_service ... ok
test collectors::dev_env::observations::tests::reads_normal_and_indirected_heads ... ok
test collectors::docker::stream::tests::regression::full_drain_wakes_remaining_frames_after_producer_eof ... ok
test collectors::docker::stream::tests::regression::partial_frame_survives_repeated_idle_timeouts ... ok
test collectors::docker::stream::tests::regression::timeout_cannot_reset_the_partial_frame_memory_limit ... ok
test collectors::eventlog::tests::parser_ignores_machine_and_security_fields ... ok
test collectors::files::queue::tests::editor_temp_lifecycle_collapses_to_one_logical_modification ... ok
test collectors::files::queue::tests::follow_up_create_keeps_the_rename_source ... ok
test collectors::files::queue::tests::follow_up_delete_keeps_the_source_and_terminal_deletion ... ok
test collectors::files::queue::tests::ignored_dependency_storm_never_enters_the_queue ... ok
test collectors::files::queue::tests::temp_rename_keeps_the_meaningful_destination ... ok
test collectors::files::tests::bounded_sweep_pages_continue_past_the_first_four_thousand_paths ... ok
test collectors::files::tests::config_rename_and_delete_produce_logical_events_and_snapshot_deletes ... ok
test collectors::files::tests::descriptor_uses_the_canonical_notify_plus_sweep_cadence ... ok
test collectors::files::tests::exact_port_change_links_only_to_current_listener_evidence ... ok
test collectors::files::tests::most_specific_overlapping_root_owns_depth_policy ... ok
test collectors::files::tests::oversized_and_malformed_configs_fall_back_without_parsing_failure ... ok
test collectors::files::tests::oversized_port_relationship_evidence_is_rejected_instead_of_truncated ... ok
test collectors::files::tests::oversized_service_relationship_evidence_is_rejected_instead_of_truncated ... ok
test collectors::files::tests::restart_sweep_emits_uncertain_delete_for_a_missing_persisted_config ... ok
test collectors::files::tests::restart_sweep_marks_a_newer_config_diff_uncertain ... ok
test collectors::files::tests::sweep_only_queues_config_candidates_and_obeys_cancellation ... ok
test collectors::files::tests::truncated_snapshot_rebaseline_never_fabricates_key_additions ... ok
test collectors::files::tests::unchanged_nested_port_still_links_another_config_change ... ok
test collectors::ignore_set::tests::dependency_and_build_trees_are_rejected_before_files ... ok
test collectors::ignore_set::tests::depth_and_size_are_hard_bounds ... ok
test collectors::ignore_set::tests::meaningful_lockfiles_survive_the_lock_rule ... ok
test collectors::ignore_set::tests::only_git_head_passes_through_the_git_ignore_boundary ... ok
test collectors::listeners::diff::tests::a_close_carries_no_owner_evidence_but_a_reowner_does ... ok
test collectors::listeners::diff::tests::an_endpoint_survives_serialization_so_a_recreation_resumes_it ... ok
test collectors::listeners::diff::tests::an_owner_change_on_a_still_bound_endpoint_is_one_modification ... ok
test collectors::listeners::diff::tests::an_unchanged_endpoint_with_the_same_owners_is_not_a_change ... ok
test collectors::listeners::diff::tests::applying_changes_makes_the_acknowledged_view_match_the_machine ... ok
test collectors::listeners::diff::tests::opens_and_closes_are_reported_separately_and_in_key_order ... ok
test collectors::listeners::diff::tests::reusable_index_matches_the_ordered_snapshot_without_losing_capacity ... ok
test collectors::listeners::diff::tests::rows_sharing_an_endpoint_fold_into_one_owner_set ... ok
test collectors::listeners::observations::tests::a_burst_stops_at_the_relationship_bound_and_leaves_the_rest_for_next_cycle ... ok
test collectors::listeners::observations::tests::a_close_names_the_port_and_carries_no_owner_claim ... ok
test collectors::listeners::observations::tests::a_downtime_diff_marks_its_timestamps_uncertain ... ok
test collectors::listeners::observations::tests::a_low_confidence_owner_keeps_the_event_low_confidence ... ok
test collectors::listeners::observations::tests::a_mixed_owner_set_only_persists_the_high_confidence_edge ... ok
test collectors::listeners::observations::tests::a_reowned_endpoint_is_one_modification_naming_both_owner_sets ... ok
test collectors::listeners::observations::tests::an_opened_port_names_its_owner_the_way_the_worked_example_asks ... ok
test collectors::listeners::observations::tests::an_owner_nobody_identified_still_records_at_low_confidence ... ok
test collectors::listeners::observations::tests::each_resolved_owner_earns_both_directions_of_the_edge ... ok
test collectors::listeners::observations::tests::evidence_indices_point_at_the_event_that_proved_each_edge ... ok
test collectors::listeners::observations::tests::one_transition_larger_than_the_atomic_edge_bound_fails_visibly ... ok
test collectors::listeners::observations::tests::two_pids_of_one_executable_are_one_entity_and_one_pair_of_edges ... ok
test collectors::listeners::tests::a_burst_larger_than_one_commit_arrives_whole_over_several_cycles ... ok
test collectors::listeners::tests::a_deferred_short_lived_listener_survives_recreation_with_original_edges_and_time ... ok
test collectors::listeners::tests::a_downtime_diff_marks_its_timestamps_uncertain_and_then_stops ... ok
test collectors::listeners::tests::a_first_snapshot_records_a_baseline_and_invents_no_opens ... ok
test collectors::listeners::tests::a_quiet_cycle_is_a_noop ... ok
test collectors::listeners::tests::a_stale_or_absent_publisher_yields_no_edges_rather_than_a_guess ... ok
test collectors::listeners::tests::an_opened_port_lands_with_both_directions_of_its_edge ... ok
test collectors::listeners::tests::an_oversized_single_listener_transition_errors_without_state_advance ... ok
test collectors::listeners::tests::an_owner_change_is_one_modification_and_not_a_close_and_an_open ... ok
test collectors::listeners::tests::quiet_polls_reuse_row_storage_and_the_endpoint_index ... ok
test collectors::listeners::tests::the_catalog_owns_the_descriptor ... ok
test collectors::listening_port_map::tests::cancellation_invalidation_and_staleness_all_remove_evidence ... ok
test collectors::listening_port_map::tests::exact_port_lookup_returns_every_current_endpoint_in_stable_order ... ok
test collectors::listening_port_map::tests::replacement_is_complete_and_duplicates_are_folded ... ok
test collectors::normalize::tests::atomic_input_rejects_foreign_collectors_and_missing_evidence ... ok
test collectors::normalize::tests::atomic_relationship_must_include_its_primary_observation_entity ... ok
test collectors::process::churn::tests::a_window_closes_on_time_and_not_before ... ok
test collectors::process::churn::tests::an_episode_survives_serialization_so_a_recreation_resumes_it ... ok
test collectors::process::churn::tests::due_summaries_beyond_the_commit_bound_remain_for_later ... ok
test collectors::process::churn::tests::each_image_gets_its_own_episode ... ok
test collectors::process::churn::tests::five_pairs_is_quiet_and_six_is_one_summary ... ok
test collectors::process::churn::tests::one_weak_pair_keeps_the_whole_summary_low_confidence ... ok
test collectors::process::churn::tests::the_window_is_anchored_at_the_first_pair_and_does_not_slide ... ok
```
