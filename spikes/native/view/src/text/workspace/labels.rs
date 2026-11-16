//! The workspace screens' fixed words: one constant per language, a field
//! per word (the shape of the app's `Text`), so a word missing in one
//! language does not compile. Words with values in them are methods of
//! [`super::WorkspaceWords`].

/// One language's fixed words of the workspace's screens.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the words it holds")]
pub struct Labels {
    pub projects: &'static str,
    pub projects_lead: &'static str,
    pub add_project: &'static str,
    pub add_folder: &'static str,
    pub clone_repository: &'static str,
    pub start_new: &'static str,
    pub no_projects: &'static str,
    pub no_git: &'static str,
    pub no_commit_yet: &'static str,
    pub remove_project: &'static str,
    pub dismiss: &'static str,
    pub loading: &'static str,
    pub no_machines: &'static str,
    pub title_add_folder: &'static str,
    pub title_clone: &'static str,
    pub title_new: &'static str,
    pub repository_url: &'static str,
    pub clone_into: &'static str,
    pub folder: &'static str,
    pub url_placeholder: &'static str,
    pub parent_placeholder: &'static str,
    pub folder_placeholder: &'static str,
    pub cancel: &'static str,
    pub clone: &'static str,
    pub create: &'static str,
    pub add: &'static str,
    pub default_branch: &'static str,
    pub added: &'static str,
    pub none_yet: &'static str,
    pub not_git_repository: &'static str,
    pub commands: &'static str,
    pub reading_file: &'static str,
    pub no_commands: &'static str,
    pub approved: &'static str,
    pub not_approved: &'static str,
    pub withdraw: &'static str,
    pub review: &'static str,
    pub worktrees: &'static str,
    pub no_worktrees: &'static str,
    pub no_live_session: &'static str,
    pub detached: &'static str,
    pub run_setup: &'static str,
    pub remove: &'static str,
    pub snapshots: &'static str,
    pub drop: &'static str,
    pub keep: &'static str,
    pub drop_for_good: &'static str,
    pub approval_title: &'static str,
    pub setup: &'static str,
    pub teardown: &'static str,
    pub scripts_and_services: &'static str,
    pub none: &'static str,
    pub start_without_setup: &'static str,
    /// Why a digest containing unshown terminal commands is refused.
    pub approval_unseen: &'static str,
    pub approve_and_run: &'static str,
    pub script_warning: &'static str,
    pub badge_new: &'static str,
    pub badge_changed: &'static str,
    pub badge_removed: &'static str,
    pub would_lose: &'static str,
    pub end_session_first: &'static str,
    pub snapshot_keeps: &'static str,
    pub snapshot_cannot: &'static str,
    pub discard_and_remove: &'static str,
    pub keep_snapshot_and_remove: &'static str,
    pub snapshot_refused: &'static str,
    pub nothing_removed: &'static str,
    pub folders: &'static str,
    pub isolation: &'static str,
    pub local: &'static str,
    pub new_worktree: &'static str,
    pub start_from: &'static str,
    pub search_places: &'static str,
    pub search_branches: &'static str,
    pub no_matching_refs: &'static str,
    pub reading_branches: &'static str,
    pub default_ref: &'static str,
    pub making_worktree: &'static str,
    pub waiting_approval: &'static str,
    pub setting_up: &'static str,
    pub setup_finished: &'static str,
    pub setup_failed: &'static str,
    pub setup_skipped: &'static str,
    pub no_output: &'static str,
    pub starting_session: &'static str,
    pub try_again: &'static str,
    pub start_anyway: &'static str,
    pub stop: &'static str,
    pub start_stopped: &'static str,
    pub worktree: &'static str,
    pub commands_unreadable: &'static str,
    pub setup_not_followed: &'static str,
    pub tip_place: &'static str,
    pub tip_isolation: &'static str,
    pub tip_base: &'static str,
    pub all_projects: &'static str,
    pub fewer_projects: &'static str,
    pub panel_changes: &'static str,
    pub panel_files: &'static str,
    pub panel_toggle: &'static str,
    pub panel_close: &'static str,
    pub not_in_project: &'static str,
    pub show_rest: &'static str,
    pub binary_file: &'static str,
    pub too_large: &'static str,
    pub submodule: &'static str,
    pub no_content_change: &'static str,
    pub unknown_content: &'static str,
    pub empty_folder: &'static str,
    pub more_entries: &'static str,
    pub file_continues: &'static str,
    pub back: &'static str,
    pub read_failed: &'static str,
    pub list_failed: &'static str,
    /// The Files tab's search field, finding a file by its name.
    pub go_to_file: &'static str,
    /// The Files tab's search field, searching inside files (Enter runs it).
    pub search_in_files: &'static str,
    /// The Files tab's search: the segment that finds files by name.
    pub by_name: &'static str,
    /// The Files tab's search: the segment that searches inside files.
    pub by_content: &'static str,
    /// The Files tab's search results when nothing matched.
    pub nothing_found: &'static str,
    /// The Files tab's search results when the machine could not search; its reason is shown under it.
    pub search_failed: &'static str,
    /// The Files tab's search results when the query cannot be sent (more than one line, or past the length the machine takes).
    pub search_refused: &'static str,
    /// The Files tree's row where a new file's, folder's or entry's name is typed.
    pub name_placeholder: &'static str,
    /// Over the Files tree when the typed name cannot be a path (a slash, empty, or `..`).
    pub name_refused: &'static str,
    /// A tree entry's menu (Paseo's file actions).
    pub open_file: &'static str,
    /// A tree entry's menu.
    pub collapse_folder: &'static str,
    /// A tree entry's menu.
    pub new_file: &'static str,
    /// A tree entry's menu.
    pub new_folder: &'static str,
    /// A tree entry's menu.
    pub copy_path: &'static str,
    /// A tree entry's menu.
    pub copy_relative_path: &'static str,
    /// A tree entry's menu: the path into the session's draft.
    pub add_to_chat: &'static str,
    /// A tree entry's menu.
    pub rename: &'static str,
    /// A tree entry's menu: a copy beside it (Paseo's `fileActions.duplicate`).
    pub duplicate: &'static str,
    /// A tree entry's menu, and the delete dialog's button.
    pub delete: &'static str,
    /// The delete dialog's title for a file.
    pub delete_file_title: &'static str,
    /// The delete dialog's title for a folder.
    pub delete_folder_title: &'static str,
    /// The delete dialog's sentence; the entry's name is shown under it.
    pub delete_message: &'static str,
    /// Over the Files tree when the machine refused a new entry, a rename, a duplicate or a delete; its reason is shown under it.
    pub change_refused: &'static str,
    /// The file editor's conflict bar: the file changed on disk while it was edited (Paseo's `changedOnDisk`).
    pub editor_changed_on_disk: &'static str,
    /// The file editor's conflict bar: the file is gone from disk (Paseo's `deletedTitle`).
    pub editor_deleted_title: &'static str,
    /// The file editor's conflict bar, under the deleted title (Paseo's `preservedDescription`).
    pub editor_preserved: &'static str,
    /// The file editor's conflict bar, under "Changed on disk" when Overwrite is offered (Paseo's `conflictDescription`).
    pub editor_conflict_description: &'static str,
    /// The file editor's conflict bar: write the buffer over the disk's file.
    pub editor_overwrite: &'static str,
    /// The file editor's conflict bar and the reload dialog's button: take the disk's file.
    pub editor_reload: &'static str,
    /// The reload dialog's title, when the buffer has changes (Paseo's `reloadTitle`).
    pub editor_reload_title: &'static str,
    /// The reload dialog's sentence (Paseo's `reloadMessage`).
    pub editor_reload_message: &'static str,
    /// The file editor's toolbar while a save is out (Paseo's `saving`).
    pub editor_saving: &'static str,
    /// The file editor's toolbar after a save failed (Paseo's `saveFailed`); the reason is in the tooltip.
    pub editor_save_failed: &'static str,
    /// The file editor's toolbar: the dot's tooltip when the buffer has unsaved changes (Paseo's `unsavedChanges`).
    pub editor_unsaved: &'static str,
}

pub use super::labels_en::EN;
pub use super::labels_ko::KO;
