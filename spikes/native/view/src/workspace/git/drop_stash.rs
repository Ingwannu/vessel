//! Stash deletion is prepared before the confirmation opens; the root,
//! index and commit are the ones the person saw, never a later selection.

use vessel_proto::workspace::{StashChanges, StashEntry, WorkspaceRoot};

use super::{StashChoice, stash_request};

/// An exact stash deletion reviewed by the person in the confirm dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropStash {
    root: WorkspaceRoot,
    entry: StashEntry,
}

impl DropStash {
    /// Prepare the immutable identity the confirmation will name.
    #[must_use]
    pub fn of(root: WorkspaceRoot, entry: StashEntry) -> Self {
        Self { root, entry }
    }

    /// The entry named in the dialog.
    #[must_use]
    pub const fn entry(&self) -> &StashEntry {
        &self.entry
    }

    /// Confirm the exact root and pinned entry, consuming the decision.
    #[must_use]
    pub fn confirm(self) -> StashChanges {
        stash_request(self.root, &self.entry, StashChoice::Drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace::StashAction;

    #[test]
    fn confirmed_drop_keeps_the_root_and_both_parts_of_entry_identity() {
        let root = WorkspaceRoot::Project {
            project: "6f1c2a3e-0000-4000-8000-000000000001".parse().unwrap(),
        };
        let entry = StashEntry {
            index: 7,
            commit: "1111111111111111111111111111111111111111".parse().unwrap(),
            message: "Saved local edits".to_owned(),
            time: "2026-11-12T09:00:00Z".parse().unwrap(),
        };
        let decision = DropStash::of(root, entry.clone());
        assert_eq!(decision.entry(), &entry);
        assert_eq!(
            decision.confirm(),
            StashChanges {
                root,
                action: StashAction::Drop {
                    index: 7,
                    commit: entry.commit
                },
            }
        );
    }
}
