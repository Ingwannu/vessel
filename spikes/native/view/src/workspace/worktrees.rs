//! What removing a worktree would lose, and which choices the removal
//! dialog offers (the page's `remove-worktree-dialog.tsx` and
//! `snapshotCovers` in `worktree-words.ts`): every reason the node gave is
//! listed, a snapshot is offered only when it keeps everything listed, and
//! discarding is always offered, last, as the destructive one.

use vessel_proto::workspace::{IfLost, KeptReason};

/// Whether a snapshot can keep what `reason` names: a session in it or a
/// failed inspection is not something a snapshot saves.
#[must_use]
pub const fn snapshot_covers(reason: KeptReason) -> bool {
    match reason {
        KeptReason::Uncommitted { .. }
        | KeptReason::UniqueCommits { .. }
        | KeptReason::FlaggedFiles
        | KeptReason::WorktreeRefs => true,
        KeptReason::Operation { .. }
        | KeptReason::Submodules
        | KeptReason::InUse
        | KeptReason::InspectionFailed
        | KeptReason::Unknown => false,
    }
}

/// The removal dialog's choices for the reasons a worktree was kept, in the
/// order drawn: a snapshot first when it keeps every reason, then discard.
#[must_use]
pub fn removal_choices(reasons: &[KeptReason]) -> Vec<IfLost> {
    let mut choices = Vec::with_capacity(2);
    if !reasons.is_empty() && reasons.iter().copied().all(snapshot_covers) {
        choices.push(IfLost::Snapshot);
    }
    choices.push(IfLost::Discard);
    choices
}

#[cfg(test)]
mod tests {
    use vessel_proto::workspace::GitOperation;

    use super::*;

    #[test]
    fn offers_a_snapshot_only_for_what_a_snapshot_keeps() {
        assert!(snapshot_covers(KeptReason::Uncommitted { files: 1 }));
        assert!(!snapshot_covers(KeptReason::InUse));
        assert!(!snapshot_covers(KeptReason::Submodules));
        assert_eq!(
            removal_choices(&[
                KeptReason::Uncommitted { files: 2 },
                KeptReason::WorktreeRefs
            ]),
            [IfLost::Snapshot, IfLost::Discard]
        );
        assert_eq!(
            removal_choices(&[
                KeptReason::UniqueCommits { commits: 1 },
                KeptReason::Operation {
                    operation: GitOperation::Rebase
                }
            ]),
            [IfLost::Discard]
        );
    }
}
