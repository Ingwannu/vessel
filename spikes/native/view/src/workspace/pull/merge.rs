//! Merging a session's pull request from the git menu: Paseo's direct
//! merge actions (`git/policy.ts` `canMergePr`,
//! `buildDirectPullRequestMergeAction`): "Merge PR (squash)", "(merge)",
//! "(rebase)" on one choice, offered only when the forge would take it.
//!
//! [Decision Log]
//! - Purpose: merge without leaving the session, and never merge
//!   something other than what the person saw.
//! - Constraint: the node pins the number and the head the caller looked
//!   at (`MergePullRequest`); a push since is refused (`HeadMoved`). The
//!   methods the repository allows come with the whole pull request
//!   (`pr.get`), not with the card's summary.
//! - Alternatives: always offering the three rows and letting the node
//!   refuse (Paseo hides what cannot run); a confirmation dialog (Paseo
//!   merges on one choice, and the pinned head already guards the case a
//!   confirmation would).
//! - Choice: Paseo's rule with what the node sends: open, not a draft,
//!   mergeable (clean, or unstable: only checks that are not required
//!   fail), the folder clean and in step with its remote, its head the
//!   pull request's head, and (when the node knows the worktree's base)
//!   ahead of that base; the methods in Paseo's order, filtered by the
//!   repository's. The pull request is taken apart whole: a merge is a
//!   person's consent, and `MergePullRequest` is built whole too.
//! - Cost: a pull request whose summary is a moment behind a push shows
//!   no merge rows until the next read: the safe side.

use vessel_proto::SessionId;
use vessel_proto::forge::{
    MergeMethod, MergeOutcome, MergePullRequest, MergeRefusal, Mergeable, PullRequest, PullState,
    PullSummary,
};
use vessel_proto::workspace::GitStatus;

/// Paseo's order of the direct merge rows.
const ORDER: [MergeMethod; 3] = [MergeMethod::Squash, MergeMethod::Merge, MergeMethod::Rebase];

/// The merge rows to offer for `pull` while the session's folder is
/// `status`: none when the forge would not take a merge now.
#[must_use]
pub fn merge_methods(pull: &PullRequest, status: &GitStatus) -> Vec<MergeMethod> {
    // Whole on purpose (README, types that carry what a person accepts): a
    // merge row is a person's consent to merge what this pull request
    // shows, so a field a newer node adds breaks the build until this rule
    // reads it or says here why it does not decide a merge.
    let PullRequest {
        summary,
        author: _,
        head_branch: _,
        base: _,
        checks: _,
        reviews: _,
        created_at: _,
        merged_at: _,
        closed_at: _,
        merge_methods: allowed,
        read_at: _,
    } = pull;
    // The checks and the review are the forge's to weigh: it folds the
    // required ones into `mergeable` (Blocked, or Unstable when only checks
    // that are not required fail), as Paseo's `directMergeReady` does.
    let PullSummary {
        repo: _,
        number: _,
        url: _,
        title: _,
        state,
        draft,
        head_commit,
        checks: _,
        review: _,
        mergeable,
        updated_at: _,
    } = summary;
    let open = *state == PullState::Open && !draft;
    let mergeable = matches!(mergeable, Mergeable::Mergeable | Mergeable::Unstable);
    let clean = status.files.is_empty() && !status.more && status.operation.is_none();
    let in_step = status
        .upstream
        .as_ref()
        .is_some_and(|upstream| upstream.ahead == 0 && upstream.behind == 0);
    let same_head = status.head.as_ref() == Some(head_commit);
    // Paseo's `aheadCount > 0`: a branch with nothing over its base has
    // nothing to merge. The node knows the base only for a worktree made
    // from a branch; elsewhere the forge's own facts decide.
    let has_work = status.base.as_ref().is_none_or(|base| base.ahead > 0);
    if !(open && mergeable && clean && in_step && same_head && has_work) {
        return Vec::new();
    }
    ORDER
        .into_iter()
        .filter(|method| allowed.contains(method))
        .collect()
}

/// `pr.merge` for `session`'s pull request as it was seen.
#[must_use]
pub fn merge_request(
    session: SessionId,
    pull: &PullRequest,
    method: MergeMethod,
) -> MergePullRequest {
    MergePullRequest {
        session,
        number: pull.summary.number,
        method,
        head: pull.summary.head_commit.clone(),
    }
}

/// What `pr.merge` answered, as the git row acts on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Merged: Paseo's "PR merged".
    Done,
    /// Nothing was merged, and why.
    Refused(MergeRefusal),
}

/// `outcome` read for the git row.
#[must_use]
pub fn merge_outcome(outcome: MergeOutcome) -> Merged {
    match outcome {
        MergeOutcome::Merged { .. } => Merged::Done,
        MergeOutcome::Refused { refusal, .. } => Merged::Refused(refusal),
        MergeOutcome::Unknown => Merged::Refused(MergeRefusal::Unknown),
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
