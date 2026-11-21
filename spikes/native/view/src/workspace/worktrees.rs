//! What removing a worktree would lose, and which choices the removal
//! dialog offers (the page's `remove-worktree-dialog.tsx` and
//! `snapshotCovers` in `worktree-words.ts`): every reason the node gave is
//! listed; while something uses the worktree (a live session, a terminal,
//! setup) neither removal is offered; else a snapshot is offered only when
//! it keeps everything listed, and discarding always, as the destructive
//! one.

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

/// Whether something uses the worktree, so that neither removal can run.
#[must_use]
pub fn in_use(reasons: &[KeptReason]) -> bool {
    reasons.contains(&KeptReason::InUse)
}

/// The removal dialog's choices for the reasons a worktree was kept: a
/// snapshot when it keeps every reason, and discard; none while it is in
/// use.
#[must_use]
pub fn removal_choices(reasons: &[KeptReason]) -> Vec<IfLost> {
    let mut choices = Vec::with_capacity(2);
    if in_use(reasons) {
        return choices;
    }
    if !reasons.is_empty() && reasons.iter().copied().all(snapshot_covers) {
        choices.push(IfLost::Snapshot);
    }
    choices.push(IfLost::Discard);
    choices
}

/// The choices once the live terminals that alone keep it in use are to be
/// ended first (ADR 0121): those the other reasons allow, the same buttons
/// as without terminals. With no other reason both, as the page offers
/// them: nothing is lost, so the node removes it either way and makes no
/// snapshot.
#[must_use]
pub fn choices_ending_terminals(reasons: &[KeptReason]) -> Vec<IfLost> {
    let others: Vec<KeptReason> = reasons
        .iter()
        .copied()
        .filter(|reason| *reason != KeptReason::InUse)
        .collect();
    if others.is_empty() {
        return vec![IfLost::Snapshot, IfLost::Discard];
    }
    removal_choices(&others)
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
        assert!(
            removal_choices(&[KeptReason::InUse, KeptReason::Uncommitted { files: 1 }]).is_empty()
        );
    }

    #[test]
    fn ending_its_terminals_offers_what_the_other_reasons_allow() {
        assert_eq!(
            choices_ending_terminals(&[KeptReason::InUse]),
            [IfLost::Snapshot, IfLost::Discard]
        );
        assert_eq!(
            choices_ending_terminals(&[KeptReason::InUse, KeptReason::Uncommitted { files: 1 }]),
            [IfLost::Snapshot, IfLost::Discard]
        );
        assert_eq!(
            choices_ending_terminals(&[KeptReason::InUse, KeptReason::Submodules]),
            [IfLost::Discard]
        );
    }
}
