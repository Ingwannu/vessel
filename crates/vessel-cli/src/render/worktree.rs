//! What became of a worktree a removal or a deletion tidied (protocol 10):
//! `vessel worktree remove` and `vessel session delete` say it in the same
//! words; and why a worktree was not made, a setup failed or where a clone
//! is, in words rather than the wire's names.

use vessel_proto::conversation::SessionDeleted;
use vessel_proto::workspace::{
    CloneStage, GitOperation, KeptReason, RemoveWorktreeOutcome, SetupFailure, WorktreeRefusal,
};

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
        RemoveWorktreeOutcome::Kept { reasons, .. } => format!(
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
        RemoveWorktreeOutcome::Kept { reasons, .. } => format!(
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

/// Why `worktree.create` made no worktree.
pub(crate) fn refusal(reason: &WorktreeRefusal) -> String {
    match reason {
        WorktreeRefusal::NotGit => "the project is not a git repository".to_owned(),
        WorktreeRefusal::NotReady => {
            "the project is not ready (cloning, failed, or its folder is gone)".to_owned()
        }
        WorktreeRefusal::NoCommit => "the repository has no commit yet".to_owned(),
        WorktreeRefusal::BranchExists => "the new branch exists; name another".to_owned(),
        WorktreeRefusal::CheckedOutElsewhere { path } => {
            format!("the branch is checked out in {}", path.display())
        }
        WorktreeRefusal::NoSuchBase => "the base is not a commit".to_owned(),
        WorktreeRefusal::NoSuchBranch => "no such branch".to_owned(),
        WorktreeRefusal::NoSuchSnapshot => "no snapshot of that worktree".to_owned(),
        WorktreeRefusal::Limit => {
            "as many worktrees as allowed already (on the node, or for this agent)".to_owned()
        }
        WorktreeRefusal::Unknown => "for a reason this vessel does not know".to_owned(),
    }
}

/// How a setup failed.
pub(crate) fn setup_failure(failure: SetupFailure) -> String {
    match failure {
        SetupFailure::Exit {
            command,
            code: Some(code),
        } => format!("command {} exited with {code}", command.saturating_add(1)),
        SetupFailure::Exit {
            command,
            code: None,
        } => format!("command {} was stopped", command.saturating_add(1)),
        SetupFailure::TimedOut => "it ran past its time limit".to_owned(),
        SetupFailure::Spawn => "the shell did not start".to_owned(),
        SetupFailure::Unknown => "for a reason this vessel does not know".to_owned(),
    }
}

/// Where a clone is.
pub(crate) const fn clone_stage(stage: CloneStage) -> &'static str {
    match stage {
        CloneStage::Connecting => "connecting",
        CloneStage::Counting => "counting objects",
        CloneStage::Compressing => "compressing objects",
        CloneStage::Receiving => "receiving objects",
        CloneStage::Resolving => "resolving deltas",
        CloneStage::CheckingOut => "checking out files",
        CloneStage::Unknown => "working",
    }
}
