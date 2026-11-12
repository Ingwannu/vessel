//! What became of a worktree a removal or a deletion tidied (protocol 10):
//! `vessel worktree remove` and `vessel session delete` say it in the same
//! words.

use vessel_proto::conversation::SessionDeleted;
use vessel_proto::workspace::{GitOperation, KeptReason, RemoveWorktreeOutcome};

/// A removal's outcome, ending with a newline; a kept worktree says why
/// and how to remove it anyway.
pub(crate) fn removal(outcome: &RemoveWorktreeOutcome) -> String {
    match outcome {
        RemoveWorktreeOutcome::Removed { snapshot: None } => "removed\n".to_owned(),
        RemoveWorktreeOutcome::Removed {
            snapshot: Some(snapshot),
        } => format!(
            "removed; its work is kept as snapshot {} (vessel worktree snapshots)\n",
            snapshot.commit
        ),
        RemoveWorktreeOutcome::Kept { reasons } => format!(
            "kept: {}\n(--snapshot keeps a snapshot first; --discard removes it anyway)\n",
            kept(reasons)
        ),
    }
}

/// What `session.delete` did with the session's worktree: nothing to say
/// when it had none.
pub(crate) fn deleted(deleted: &SessionDeleted) -> String {
    let Some(tidied) = &deleted.worktree else {
        return String::new();
    };
    match &tidied.outcome {
        RemoveWorktreeOutcome::Removed { .. } => {
            format!("its worktree {} was removed\n", tidied.worktree)
        }
        RemoveWorktreeOutcome::Kept { reasons } => format!(
            "its worktree {id} was kept: {why}\n(vessel worktree remove {id} --snapshot or \
             --discard removes it)\n",
            id = tidied.worktree,
            why = kept(reasons)
        ),
    }
}

/// Every reason, in a few words each.
pub(crate) fn kept(reasons: &[KeptReason]) -> String {
    let words: Vec<String> = reasons.iter().copied().map(reason).collect();
    if words.is_empty() {
        "for a reason the node did not give".to_owned()
    } else {
        words.join("; ")
    }
}

fn reason(reason: KeptReason) -> String {
    match reason {
        KeptReason::Uncommitted { files: 1 } => "1 file not committed".to_owned(),
        KeptReason::Uncommitted { files } => format!("{files} files not committed"),
        KeptReason::UniqueCommits { commits: 1 } => {
            "1 commit no other branch or tag has".to_owned()
        }
        KeptReason::UniqueCommits { commits } => {
            format!("{commits} commits no other branch or tag has")
        }
        KeptReason::Operation { operation } => {
            format!("{} in progress", operation_name(operation))
        }
        KeptReason::Submodules => "it has submodules".to_owned(),
        KeptReason::FlaggedFiles => "files marked assume-unchanged or skip-worktree".to_owned(),
        KeptReason::WorktreeRefs => "refs of its own".to_owned(),
        KeptReason::InUse => "a session is in it, or its setup runs".to_owned(),
        KeptReason::InspectionFailed => "git could not inspect it".to_owned(),
        KeptReason::Unknown => "a reason this vessel does not know".to_owned(),
    }
}

const fn operation_name(operation: GitOperation) -> &'static str {
    match operation {
        GitOperation::Merge => "a merge",
        GitOperation::Rebase => "a rebase",
        GitOperation::CherryPick => "a cherry-pick",
        GitOperation::Revert => "a revert",
        GitOperation::Bisect => "a bisect",
    }
}
