# Required C3 runtime case inventory

Source code: ce9667ac5e843b326a978ae5de445cc6a1404ce4. These are named
source cases, NOT execution results. The exact ordinary changed-crate library
selection must report each applicable Linux case as passed before handoff.
No public wire/number/native/OS assertion follows from the list.

## crates/vessel-workspace/src/git/diff/all_index/tests.rs

- `git::diff::all_index::tests::recreated_base_file_is_compared_once_and_the_real_index_is_unchanged`
- `git::diff::all_index::tests::identically_recreated_file_is_unchanged_in_summary_and_body`
- `git::diff::all_index::tests::restored_paths_are_literal_and_preserve_other_staged_renames`
- `git::diff::all_index::tests::batches_bound_path_count_and_conservatively_escaped_argv`
- `git::diff::all_index::tests::malformed_tree_entries_are_errors_instead_of_dropped_paths`
- `git::diff::all_index::tests::entry_decode_failures_retain_the_original_error_source`
- `git::diff::all_index::tests::private_index_copy_enforces_its_byte_budget_and_private_mode`
- `git::diff::all_index::tests::private_index_copy_does_not_follow_a_source_symlink`
- `git::diff::all_index::tests::a_relative_scratch_parent_is_refused_before_reading_a_comparison`
## crates/vessel-workspace/src/git/diff/comparison/tests.rs

- `git::diff::comparison::tests::both_entry_points_finish_the_same_all_comparison_without_changing_the_real_index`
- `git::diff::comparison::tests::cancellation_after_preparation_still_closes_the_actual_private_index`
## crates/vessel-workspace/src/git/diff/comparison_tests.rs

- `git::diff::comparison_tests::committed_only_excludes_staged_unstaged_and_untracked_changes_and_preserves_index`
- `git::diff::comparison_tests::no_base_is_explicit_and_an_invalid_requested_base_does_not_fall_back`
- `git::diff::comparison_tests::committed_only_before_the_first_commit_is_empty_even_with_new_files`
- `git::diff::comparison_tests::comparison_prefers_origin_while_qualified_bases_keep_their_exact_stream`
- `git::diff::comparison_tests::qualified_missing_or_noncommit_refs_never_fall_back_to_a_valid_local_branch`
## crates/vessel-workspace/src/git/panel_history/tests.rs

- `git::panel_history::tests::all_workspace_commits_precede_ten_base_commits_without_a_legacy_page_cap`
- `git::panel_history::tests::recent_base_history_is_capped_and_a_short_saved_missing_base_uses_the_default`
- `git::panel_history::tests::context_starts_at_the_fork_and_publication_uses_any_remote_not_just_upstream`
- `git::panel_history::tests::a_local_ahead_base_wins_ties_but_a_more_advanced_origin_wins`
- `git::panel_history::tests::detached_and_unborn_checkouts_have_no_branch_history_and_abbreviations_stay_observed`
## crates/vessel-node/src/workspace/tests/comparison.rs

- `workspace::tests::comparison::node_comparisons_keep_committed_working_and_all_distinct_and_reject_invalid_bases`
- `workspace::tests::comparison::an_unknown_root_fails_before_a_job_and_a_stopped_node_refuses_the_resolved_one`
- `workspace::tests::comparison::a_folder_cannot_be_requested_as_a_file_diff`
- `workspace::tests::comparison::the_same_resolved_snapshot_admits_only_the_recorded_worktree_stream`
- `workspace::tests::comparison::explicit_refresh_bypasses_cached_working_changes_and_a_commit_invalidates_them`
- `workspace::tests::comparison::a_cached_payload_cannot_bypass_revalidation_of_the_recorded_folder`
## crates/vessel-node/src/workspace/tests/git_history.rs

- `workspace::tests::git_history::history_uses_recorded_and_default_refs_from_one_root_snapshot`
- `workspace::tests::git_history::a_live_history_read_revalidates_its_root_and_shutdown_refuses_before_disk_validation`
## crates/vessel-node/src/workspace/tests/file_mutations.rs

- `workspace::tests::file_mutations::legacy_file_mutations_invalidate_the_actual_cached_working_tree`
- `workspace::tests::file_mutations::an_owned_file_task_keeps_invalidation_until_physical_completion_after_waiter_loss`
- `workspace::tests::file_mutations::the_future_captures_the_original_root_before_its_tracked_task_is_scheduled`
- `workspace::tests::file_mutations::partial_physical_work_still_invalidates_when_its_following_disk_step_fails`
- `workspace::tests::file_mutations::ordinary_non_git_files_remain_writable_while_the_git_wrapper_still_refuses`
- `workspace::tests::file_mutations::duplication_uses_the_same_cache_guard_and_actual_returned_sibling_path`
- `workspace::tests::file_mutations::shutdown_refuses_a_prepared_file_future_before_unknown_root_or_any_disk_step`
## crates/vessel-node/src/workspace/tests/queued_roots.rs

- `workspace::tests::queued_roots::queued_reads_do_not_follow_replaced_paths_or_use_superseded_base_facts`
## crates/vessel-node/src/workspace/tests/git_writes.rs

- `workspace::tests::git_writes::queued_mutation_revalidates_after_the_project_reservation`
## crates/vessel-node/src/workspace/git_writes/tests.rs

- `workspace::git_writes::tests::one_project_waits_while_another_can_write_and_releasing_all_frees_the_map`
- `workspace::git_writes::tests::cancelled_waiters_never_enter_and_release_their_admission`
- `workspace::git_writes::tests::a_full_writer_reservation_refuses_before_queueing_more_work`
- `workspace::git_writes::tests::same_project_waiters_keep_the_existing_mutex_fifo_order`
## crates/vessel-node/src/workspace/comparison/cache/tests.rs

- `workspace::comparison::cache::tests::a_forced_build_waits_for_cancellation_cleanup_and_fresh_waiters_share_the_next_one`
- `workspace::comparison::cache::tests::dropping_every_external_receiver_does_not_abandon_the_owned_build`
- `workspace::comparison::cache::tests::shared_failures_keep_one_typed_source_and_do_not_poison_a_retry`
- `workspace::comparison::cache::tests::the_write_guard_retires_a_read_that_started_between_its_two_boundaries`
- `workspace::comparison::cache::tests::the_public_read_requests_revalidation_only_after_the_old_cleanup_finishes`
## crates/vessel-node/src/workspace/comparison/cache/core/tests.rs

- `workspace::comparison::cache::core::tests::forced_readers_wait_for_one_fresh_build_after_the_old_owned_job_finishes`
- `workspace::comparison::cache::core::tests::invalidated_reads_cannot_refill_the_cache_and_duplicate_completions_are_inert`
- `workspace::comparison::cache::core::tests::active_work_survives_completed_eviction_and_a_failed_job_can_retry`
- `workspace::comparison::cache::core::tests::working_and_history_changes_invalidate_all_for_each_base_but_not_other_roots`
- `workspace::comparison::cache::core::tests::freshness_and_two_second_suppression_match_the_reference_boundaries`
- `workspace::comparison::cache::core::tests::key_normalization_uses_defaults_only_for_comparisons_that_need_a_base`
- `workspace::comparison::cache::core::tests::reasons_are_not_query_identity_and_every_forced_reason_bypasses_freshness`
- `workspace::comparison::cache::core::tests::repository_invalidation_retires_immutable_bodies_but_working_and_history_do_not`
- `workspace::comparison::cache::core::tests::active_admission_and_identifier_exhaustion_never_start_unowned_work`
- `workspace::comparison::cache::core::tests::a_forced_summary_invalidates_its_body_pages_but_not_another_comparison`
- `workspace::comparison::cache::core::tests::recently_used_completed_payloads_survive_eviction_of_an_older_one`
- `workspace::comparison::cache::core::tests::shared_ref_changes_retire_sibling_project_caches_but_working_changes_stay_local`
## crates/vessel-node/src/workspace/comparison/jobs/tests.rs

- `workspace::comparison::jobs::tests::losing_the_receiver_does_not_drop_running_cleanup_or_release_its_reservation_early`
- `workspace::comparison::jobs::tests::the_queue_refuses_at_64_and_shutdown_does_not_start_waiting_operations`
- `workspace::comparison::jobs::tests::shutdown_waits_for_started_work_to_finish_its_cleanup`
## crates/vessel-node/src/workspace/comparison/pinned_base/tests.rs

- `workspace::comparison::pinned_base::tests::a_project_without_a_pinned_base_keeps_the_complete_requested_comparison`
- `workspace::comparison::pinned_base::tests::an_exact_ref_or_its_display_alias_keeps_the_recorded_stream_for_both_modes`
- `workspace::comparison::pinned_base::tests::another_qualified_stream_is_not_authorized_by_the_same_display_name`
- `workspace::comparison::pinned_base::tests::immutable_commits_and_uncommitted_reads_do_not_apply_branch_base_admission`

## Successor cache-hit authority regression (not in ce9667ac)

- `workspace::tests::comparison::cached_root_probe::cache_hits_refuse_registry_changes_during_the_original_root_open`

This added case runs actual cached summary and file entry points against path
and default-branch registry changes during their queued blocking root open.
It is a source case until separately executed on the successor. Keep the66
original-case inventory and old candidate results separately attributed.
