//! Body-state precedence and empty-state navigation, never decided in drawing.

use super::{Comparison, ComparisonMode, ReadFailure, Whitespace};
use vessel_proto::workspace::{DiffSummary, GitStatus};

/// Demand belongs to the retained pane and connection, not a render timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demand {
    /// Retained pane or folded section does not request new work.
    Inactive,
    /// Visible content waits for the actual node's connection.
    Disconnected,
    /// Visible, connected content may request work.
    Active,
}

/// Repository facts have a distinct not-git state; absence is not emptiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryRead<'a> {
    /// No status answer yet.
    Waiting,
    /// The authoritative project descriptor says this is not a repository.
    NotGit,
    /// The current status read failed; retry is a person action.
    Failed(&'a ReadFailure),
    /// Facts from the current root's successful read.
    Ready(&'a GitStatus),
}

/// Diff lifecycle, including a payload too large to preview as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRead<'a> {
    /// No answer for this comparison yet.
    Waiting,
    /// The node refused the whole document's size.
    TooLarge,
    /// The current comparison failed.
    Failed(&'a ReadFailure),
    /// A bounded summary, possibly incomplete.
    Ready(&'a DiffSummary),
}

/// A useful alternative after a genuinely empty comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyAction {
    /// A committed-only comparison is empty but working changes exist.
    Uncommitted,
    /// A working comparison is empty but known branch commits exist.
    Committed,
}

/// Ordered UI state after Paseo `DiffBodyContent`; data never masquerades as empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body<'a> {
    /// Retained content is not requesting work.
    Inactive,
    /// Wait for the node before requesting either read.
    Connecting,
    /// Repository facts must arrive before choosing a diff state.
    CheckingRepository,
    /// Status failed, even if a prior diff succeeded.
    RepositoryFailed(&'a ReadFailure),
    /// The authoritative root has files but no git.
    NotRepository,
    /// The repository is usable; comparison has not answered yet.
    ReadingDiff,
    /// A bounded preview cannot fit this comparison.
    DiffTooLarge,
    /// The comparison failed after repository facts succeeded.
    DiffFailed(&'a ReadFailure),
    /// A complete summary is empty, with a possible alternative mode.
    Empty(Option<EmptyAction>),
    /// Data or a partial result; neither may be labelled empty.
    Files(&'a DiffSummary),
}

/// Whitespace filtering suppresses the suggestion because the other mode may
/// also look empty. Unknown ahead counts never invent committed changes.
#[must_use]
pub fn empty_action(comparison: &Comparison, status: &GitStatus) -> Option<EmptyAction> {
    if comparison.whitespace() == Whitespace::Ignore {
        return None;
    }
    match comparison.mode() {
        ComparisonMode::Committed if !status.files.is_empty() || status.more => {
            Some(EmptyAction::Uncommitted)
        }
        ComparisonMode::Uncommitted if status.base.as_ref().is_some_and(|base| base.ahead > 0) => {
            Some(EmptyAction::Committed)
        }
        ComparisonMode::All | ComparisonMode::Committed | ComparisonMode::Uncommitted => None,
    }
}

/// Project the body once, preserving the reference's exact failure precedence.
#[must_use]
pub fn body<'a>(
    demand: Demand,
    repository: RepositoryRead<'a>,
    diff: DiffRead<'a>,
    comparison: &Comparison,
) -> Body<'a> {
    match demand {
        Demand::Inactive => return Body::Inactive,
        Demand::Disconnected => return Body::Connecting,
        Demand::Active => {}
    }
    let status = match repository {
        RepositoryRead::Waiting => return Body::CheckingRepository,
        RepositoryRead::Failed(problem) => return Body::RepositoryFailed(problem),
        RepositoryRead::NotGit => return Body::NotRepository,
        RepositoryRead::Ready(status) => status,
    };
    match diff {
        DiffRead::Waiting => Body::ReadingDiff,
        DiffRead::TooLarge => Body::DiffTooLarge,
        DiffRead::Failed(problem) => Body::DiffFailed(problem),
        DiffRead::Ready(summary) if summary.files.is_empty() && !summary.more => {
            Body::Empty(empty_action(comparison, status))
        }
        DiffRead::Ready(summary) => Body::Files(summary),
    }
}
