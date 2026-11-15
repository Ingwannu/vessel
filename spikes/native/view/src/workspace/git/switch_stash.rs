//! Paseo's "Stash & Switch" (`hooks/use-branch-switcher.ts`): a switch
//! refused because it would overwrite local changes asks to stash them,
//! and on yes saves them, then switches to the same branch.
//!
//! [Decision Log]
//! - Purpose: a branch with local edits can still be switched from in one
//!   decision, as in Paseo, without losing the edits.
//! - Constraints: the node does not stash on its own; Paseo recognises the
//!   case from git's message text, which the node types instead
//!   (`GitRefusal::WouldOverwrite`). Two writes, one after the other, in
//!   the repository's one write slot.
//! - Alternatives: show the refusal and leave stashing to the menu (the
//!   page's way); stash first on every switch.
//! - Choice: offer only for that refusal of a branch switch (not a new
//!   branch, which carries the edits along), and send the switch only after
//!   the save succeeded.
//! - Why: no edit is put away unasked, and a failed save stops before the
//!   switch.
//! - Cost: Paseo's later offer to restore an entry saved for the branch
//!   switched back to is not here: the node's saved entries carry no
//!   branch of their own (Paseo tags its own entries).

use vessel_proto::workspace::{GitRefusal, StashAction, StashChanges, Switch};

use super::{StashScope, Success, WriteIntent, Written};
use crate::workspace::problem::Problem;

/// Whether this answer to `intent` is the refusal Paseo answers with
/// "Stash & Switch".
#[must_use]
pub const fn offers_stash(intent: &WriteIntent, result: &Result<Written, Problem>) -> bool {
    matches!(
        (intent, result),
        (
            WriteIntent::SwitchBranch,
            Ok(Written::Refused(GitRefusal::WouldOverwrite { .. }))
        )
    )
}

/// The save sent first, for the switch's root. Tracked changes only:
/// untracked files are what git does not overwrite on a switch.
#[must_use]
pub fn stash_for(switch: &Switch) -> (StashChanges, WriteIntent) {
    (
        StashChanges {
            root: switch.root,
            action: StashAction::Save {
                message: None,
                untracked: false,
            },
        },
        WriteIntent::SaveStash(StashScope::Tracked),
    )
}

/// Whether the save answered so that the waiting switch may be sent.
#[must_use]
pub const fn saved(result: &Result<Written, Problem>) -> bool {
    matches!(result, Ok(Written::Said(Success::Stashed(_))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::git_quick::GitAction;
    use vessel_proto::workspace::{StashList, WorkspaceRoot};

    fn root() -> WorkspaceRoot {
        WorkspaceRoot::Project {
            project: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
        }
    }

    fn overwrite() -> Written {
        Written::Refused(GitRefusal::WouldOverwrite {
            paths: vec!["README.md".parse().unwrap()],
        })
    }

    #[test]
    fn only_a_branch_switch_refused_for_local_changes_offers_to_stash() {
        assert!(offers_stash(&WriteIntent::SwitchBranch, &Ok(overwrite())));
        assert!(!offers_stash(&WriteIntent::CreateBranch, &Ok(overwrite())));
        assert!(!offers_stash(
            &WriteIntent::Quick(GitAction::Pull),
            &Ok(overwrite())
        ));
        assert!(!offers_stash(
            &WriteIntent::SwitchBranch,
            &Ok(Written::Refused(GitRefusal::TimedOut))
        ));
        assert!(!offers_stash(
            &WriteIntent::SwitchBranch,
            &Err(Problem::Older)
        ));
    }

    #[test]
    fn the_save_keeps_new_files_where_they_are_and_names_its_scope() {
        let switch = Switch {
            root: root(),
            branch: "main".parse().unwrap(),
            create: None,
        };
        assert_eq!(
            stash_for(&switch),
            (
                StashChanges {
                    root: root(),
                    action: StashAction::Save {
                        message: None,
                        untracked: false,
                    },
                },
                WriteIntent::SaveStash(StashScope::Tracked),
            )
        );
    }

    #[test]
    fn the_switch_waits_for_a_save_that_succeeded() {
        let list = StashList {
            entries: Vec::new(),
            more: false,
        };
        assert!(saved(&Ok(Written::Said(Success::Stashed(list)))));
        assert!(!saved(&Ok(Written::Refused(GitRefusal::TimedOut))));
        assert!(!saved(&Err(Problem::Unsent)));
    }
}
