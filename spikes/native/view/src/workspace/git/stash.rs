//! Stash entries keep their identity when another worktree changes the
//! shared stash. Never reconstruct an entry from its list index alone.

use vessel_proto::workspace::{StashAction, StashChanges, StashEntry, WorkspaceRoot};

/// What to do with a specific stash entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashChoice {
    /// Apply it and keep it in the stash.
    Apply,
    /// Apply it, dropping it only after success.
    Pop,
    /// Delete it, after the kit's confirm dialog.
    Drop,
}

/// Prepare an action with both the entry's position and its commit. The
/// node's `stash_moved` refusal then protects against a concurrent save.
#[must_use]
pub fn stash_request(root: WorkspaceRoot, entry: &StashEntry, choice: StashChoice) -> StashChanges {
    let index = entry.index;
    let commit = entry.commit.clone();
    let action = match choice {
        StashChoice::Apply => StashAction::Apply { index, commit },
        StashChoice::Pop => StashAction::Pop { index, commit },
        StashChoice::Drop => StashAction::Drop { index, commit },
    };
    StashChanges { root, action }
}
