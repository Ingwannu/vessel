//! Starting a session in a new worktree, in the order the node wants (the
//! page's `features/compose/start-worktree.ts`): `worktree.create`; if the
//! base's commands are not approved, the person approves them (or starts
//! without setup) and it is asked again; then setup's output is followed
//! with `worktree.output` until setup is no longer running; a failed setup
//! is shown and the person chooses (run it again, start anyway, stop);
//! only then `session.create { place: worktree }`, since a session in a
//! worktree whose setup runs is refused.
//!
//! What a screen shows at each step and what a person can answer are here;
//! the walk itself is the link's (`link::workspace::start`), which sends a
//! [`Step`] at each point and waits for the answer.
//!
//! [Decision Log]
//! - Purpose: where a session runs, with nothing started by a side door:
//!   approval is a person's choice, never automatic, and setup runs only
//!   through the node's own rule.
//! - Constraint: the node answers `worktree.create` at once and runs setup
//!   in the background (it continues when the app goes away); setup's
//!   output is paged (64 KiB) with a wait of at most 30 s.
//! - Choice: the steps as values; setup's output travels as text to add
//!   ([`Step::Output`]), so the screen keeps one [`SetupLog`] and a page of
//!   output is never copied with the whole log.
//! - Leaving: the composer's leaving (or another place chosen) stops the
//!   walk: no session is started and nothing more is asked after it. The
//!   worktree already made stays, set up, with no session in it.
//!
//! [`SetupLog`]: super::setup_log::SetupLog

use vessel_proto::WorktreeId;
use vessel_proto::workspace::{ProjectCommands, SetupState, WorktreeInfo, WorktreeRefusal};

use super::command_changes::Compared;
use super::problem::Problem;

/// How many reads of setup's output in a row may fail before the walk says
/// so (the worktree is made by then).
pub const TRIES: u32 = 3;

/// What the composer shows while a worktree is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing (cancelled, or stopped by the person).
    Idle,
    /// `worktree.create` was sent.
    Making,
    /// The commands need an approval.
    Approval {
        /// The set asked for (listed).
        commands: ProjectCommands,
        /// What it is compared with.
        compared: Box<Compared>,
        /// Whether the approval is being sent.
        busy: bool,
    },
    /// The worktree was not made, or its setup could not be followed.
    Refused(Refusal),
    /// Setup runs or ended.
    Setup {
        /// The branch, in words (`None`: a detached HEAD).
        branch: Option<String>,
        /// Its state now.
        setup: SetupState,
        /// The setup commands, to name the one that failed.
        commands: Vec<String>,
    },
    /// More of setup's output, decoded, to add to the log the screen keeps.
    Output(String),
}

/// Why a start stopped with something to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The node refused to make the worktree.
    Worktree(WorktreeRefusal),
    /// The project's commands could not be read for an approval.
    CommandsUnreadable,
    /// Made, but setup could not be followed.
    NotFollowed {
        /// The branch, in words.
        branch: Option<String>,
        /// Why.
        problem: Problem,
    },
}

/// The person's answer to an approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalAnswer {
    /// Approve and run.
    Approve,
    /// Start without setup.
    Skip,
    /// Cancel.
    Cancel,
}

/// The person's answer to a failed setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailedAnswer {
    /// Run setup again.
    Retry,
    /// Start anyway.
    Start,
    /// Stop here.
    Stop,
}

/// How a worktree's making ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Made {
    /// Ready: start the session in it.
    Ready(WorktreeId),
    /// Stopped (by the person, by leaving, or after a refusal shown).
    Stopped,
    /// A call failed.
    Failed(Problem),
}

/// The branch a worktree is on (`None`: a detached HEAD).
#[must_use]
pub fn branch_of(info: &WorktreeInfo) -> Option<String> {
    info.branch
        .as_ref()
        .or(info.created_branch.as_ref())
        .map(ToString::to_string)
}

/// Whether a page of setup's output ends the following: setup no longer
/// runs and the page brought nothing (the rest was read).
#[must_use]
pub const fn setup_ended(setup: &SetupState, page_empty: bool) -> bool {
    page_empty && !matches!(setup, SetupState::Running { .. })
}

/// Only a known terminal setup state authorizes automatic session creation.
/// A newer state must not be silently presented as successful setup.
#[must_use]
pub const fn setup_allows_session(setup: &SetupState) -> bool {
    match setup {
        SetupState::None | SetupState::Skipped | SetupState::Succeeded { .. } => true,
        SetupState::Running { .. } | SetupState::Failed { .. } | SetupState::Unknown => false,
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    use chrono::DateTime;
    use vessel_proto::workspace::SetupFailure;
    #[test]
    fn only_known_completed_or_explicitly_skipped_setup_allows_automatic_creation() {
        for state in [
            SetupState::None,
            SetupState::Skipped,
            SetupState::Succeeded {
                finished_at: DateTime::UNIX_EPOCH,
            },
        ] {
            assert!(setup_allows_session(&state));
        }
        for state in [
            SetupState::Running {
                started_at: DateTime::UNIX_EPOCH,
            },
            SetupState::Failed {
                failure: SetupFailure::Spawn,
                finished_at: DateTime::UNIX_EPOCH,
            },
            SetupState::Unknown,
        ] {
            assert!(!setup_allows_session(&state));
        }
    }
}
