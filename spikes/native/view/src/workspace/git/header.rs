//! The narrow Changes head keeps the branch once and one short state.
//!
//! [Decision Log]
//! - Purpose: full comparison sentences repeated long branch names and
//!   occupied most of a 440-point panel before any changed file appeared.
//! - Constraints: no arrow-only state, lost comparison facts or duplicate
//!   file counts; the full sentences remain available in the tooltip.
//! - Alternatives: truncate names or remove the comparison information.
//! - Choice: typed compact state, implicit upstream name when it mirrors
//!   the checked-out branch, and a base only when it adds another target.
//! - Why: the branch wraps once; activity and incomplete status stay
//!   visible, while the tooltip can explain the full values.
//! - Cost: the renderer must attach that tooltip rather than treating the
//!   compact line as a complete description of the repository.

use vessel_proto::workspace::{BranchName, Distance, GitOperation, GitStatus};

use super::{Reading, Writing};

/// What is happening to the facts currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadActivity {
    /// Complete displayed facts, with no pending request.
    Idle,
    /// A read retains previous facts until its answer arrives.
    Reading,
    /// A write is changing the displayed repository.
    Writing,
}

/// The visible branch label, without a sentence around its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadName<'a> {
    /// The checked-out branch, shown without a sentence around it.
    Branch(&'a BranchName),
    /// A commit is checked out without a branch.
    Detached,
    /// Neither a branch nor a commit was returned.
    Missing,
}

/// Whether the compact distance needs a different branch's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceName {
    /// The upstream mirrors the current branch's name; its full ref is
    /// in the tooltip rather than repeated beside that same name.
    ImplicitUpstream,
    /// A different target, or the separate base comparison, stays named.
    Explicit,
}

/// A compact state, with its original typed facts retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadState<'a> {
    /// A write is pending.
    Working,
    /// Previous facts are being refreshed.
    Refreshing,
    /// Git is in an unfinished operation.
    Operation(GitOperation),
    /// The status reply omitted some changed files.
    Incomplete,
    /// The repository has no first commit yet.
    NoCommits,
    /// The exact commit is checked out without a branch.
    Detached(&'a vessel_proto::workspace::CommitId),
    /// A branch has no tracking target configured.
    NoUpstream,
    /// The branch's relationship to its upstream.
    Upstream {
        /// The complete distance returned by the machine.
        distance: &'a Distance,
        /// Whether its target needs a different name in the short line.
        name: DistanceName,
    },
}

/// The compact head; its file counts belong to F's existing toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHead<'a> {
    /// The branch name once, or why no branch name exists.
    pub name: HeadName<'a>,
    /// The short state appropriate to the current activity.
    pub state: HeadState<'a>,
    /// A base target that adds information beyond the current branch
    /// and its upstream; every target remains in the full tooltip.
    pub base: Option<&'a Distance>,
}

/// A write takes precedence over a read of previous facts.
#[must_use]
pub const fn head_activity(status: &Reading<GitStatus>, writing: &Writing) -> HeadActivity {
    if writing.busy() {
        HeadActivity::Writing
    } else if matches!(status, Reading::Loading { .. }) {
        HeadActivity::Reading
    } else {
        HeadActivity::Idle
    }
}

/// The head displays only the comparison target that adds information;
/// comparison with the current branch itself adds none. The tooltip
/// retains every comparison, even when it is redundant in this head.
#[must_use]
pub fn git_head(status: &GitStatus, activity: HeadActivity) -> GitHead<'_> {
    let name = match (&status.branch, &status.head) {
        (Some(branch), _) => HeadName::Branch(branch),
        (None, Some(_)) => HeadName::Detached,
        (None, None) => HeadName::Missing,
    };
    let state = match (
        activity,
        status.operation,
        &status.head,
        &status.branch,
        &status.upstream,
    ) {
        (HeadActivity::Writing, ..) => HeadState::Working,
        (HeadActivity::Reading, ..) => HeadState::Refreshing,
        (HeadActivity::Idle, Some(operation), ..) => HeadState::Operation(operation),
        (HeadActivity::Idle, None, ..) if status.more => HeadState::Incomplete,
        (HeadActivity::Idle, None, None, ..) => HeadState::NoCommits,
        (HeadActivity::Idle, None, Some(head), None, _) => HeadState::Detached(head),
        (HeadActivity::Idle, None, Some(_), Some(_), None) => HeadState::NoUpstream,
        (HeadActivity::Idle, None, Some(_), Some(branch), Some(distance)) => HeadState::Upstream {
            distance,
            name: if mirrors_branch(distance, branch) {
                DistanceName::ImplicitUpstream
            } else {
                DistanceName::Explicit
            },
        },
    };
    let base = status.base.as_ref().filter(|base| {
        activity == HeadActivity::Idle
            && status
                .branch
                .as_ref()
                .is_none_or(|branch| base.name.as_str() != branch.as_str())
            && status
                .upstream
                .as_ref()
                .is_none_or(|upstream| base.name != upstream.name)
    });
    GitHead { name, state, base }
}

fn mirrors_branch(distance: &Distance, branch: &BranchName) -> bool {
    let upstream = distance.name.as_str();
    upstream == branch.as_str()
        // RemoteName forbids '/', so strip exactly one remote component.
        // A suffix match would hide the distinct target origin/other/feature
        // when the checked-out branch is only feature.
        || upstream
            .split_once('/')
            .is_some_and(|(_, name)| name == branch.as_str())
}
