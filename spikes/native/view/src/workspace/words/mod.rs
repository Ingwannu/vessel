//! The workspace's words, in English and Korean: one table per language
//! ([`En`], [`Ko`]), each a whole implementation of [`WorkspaceWords`], so
//! a sentence added to one language and not the other does not compile.
//! The English is the page's, word for word (`data/workspace/*-words.ts`,
//! `changes.ts`, `git-quick.ts`, `place.ts`, `branch-name.ts`), and the
//! page's test cases check it.
//!
//! [Decision Log]
//! - Purpose: every string in English and Korean (ADR 0036, "words"),
//!   from the first line, with the page's English as the specification.
//! - Constraint: the app's string table (`Text`, the desktop lead's) is a
//!   struct of fields in one shared file; this area has over a hundred
//!   sentences, most of them chosen by a closed set from the wire (a
//!   refusal, a state), and other tracks edit that file at the same time.
//! - Alternatives: fields in the shared `Text` (one field per variant of
//!   every set, in a file every track touches); a key-string table (a typo
//!   found at run time).
//! - Choice: a trait with one method per set, matched exhaustively, and one
//!   implementation per language in a file of its own; the language picks
//!   the implementation ([`words`]). The same guarantee as `Text` (both
//!   languages, or no build), with the area's words in the area's files.
//! - Cost: a third language is a third implementation; a new variant on
//!   the wire is two arms to write.

mod en;
mod ko;

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::{
    ChangeStatus, CloneFailure, CloneStage, DiffScope, GitOperation, GitRefusal, GitSupport,
    KeptReason, ProjectState, RelPath, SetupFailure, SetupState, WorktreeRefusal,
};

pub use en::En;
pub use ko::Ko;

use super::branch_name::BranchNameProblem;
use super::changes::ScopeKind;
use super::command_changes::Compared;
use super::git_quick::{Blocked, GitAction};
use super::place::PlaceProblem;
use super::problem::Problem;
use super::save::SaveFailure;

/// A refusal said plainly: what happened, what to do, and what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalWords<'a> {
    /// What happened, one sentence.
    pub title: String,
    /// What to do about it, one sentence.
    pub next: String,
    /// The paths it concerns, at most 100 (the node's bound).
    pub paths: &'a [RelPath],
    /// Text to show as it came (a hook's output, the remote's message).
    pub output: Option<&'a str>,
}

/// A time and the day it is read on, in the reader's zone: what "today"
/// means for [`WorkspaceWords::compared`].
#[derive(Debug, Clone, Copy)]
pub struct Local {
    /// The reader's zone.
    pub offset: FixedOffset,
    /// Today, in that zone.
    pub today: NaiveDate,
}

impl Local {
    /// "14:03" when `at` is today, else "2026-11-12 14:03" (the page's
    /// `when`, in its formats: 24-hour time, an ISO date).
    #[must_use]
    pub fn when(&self, at: DateTime<Utc>) -> (bool, String) {
        let local = at.with_timezone(&self.offset);
        let time = local.format("%H:%M").to_string();
        if local.date_naive() == self.today {
            (true, time)
        } else {
            (false, format!("{} {time}", local.format("%Y-%m-%d")))
        }
    }
}

/// Every sentence of the workspace, in one language.
pub trait WorkspaceWords: Sync + std::fmt::Debug {
    /// Why a typed name is not a branch name.
    fn branch_name(&self, problem: BranchNameProblem) -> String;
    /// Why Send cannot start in a place.
    fn place(&self, problem: PlaceProblem) -> String;
    /// `count` with its noun.
    fn files(&self, count: u64) -> String;
    /// What a clone is doing.
    fn clone_stage(&self, stage: CloneStage) -> &'static str;
    /// Why a clone failed, and what to do.
    fn clone_failure(&self, reason: CloneFailure) -> &'static str;
    /// A project's state in one short phrase (`None` when ready).
    fn project_state(&self, state: &ProjectState) -> Option<String>;
    /// The machine's git, when it cannot be used.
    fn git_support(&self, git: &GitSupport) -> Option<String>;
    /// Why setup failed, naming the command when the list holds it.
    fn setup_failure(&self, failure: &SetupFailure, commands: &[&str]) -> String;
    /// A worktree's setup in one short phrase.
    fn setup_state(&self, setup: &SetupState) -> &'static str;
    /// What would be lost, one line per reason.
    fn kept_reason(&self, reason: KeptReason) -> String;
    /// Why a worktree was not made, and what to do.
    fn worktree_refusal(&self, reason: &WorktreeRefusal) -> String;
    /// A git operation in a sentence ("a rebase").
    fn operation(&self, operation: GitOperation) -> &'static str;
    /// A refusal's title and what to do.
    fn refusal_lines(&self, refusal: &GitRefusal) -> (String, String);
    /// A git action's button.
    fn action(&self, action: GitAction) -> &'static str;
    /// Why a git action is off.
    fn blocked(&self, blocked: Blocked) -> &'static str;
    /// What a script or a service is called in a sentence.
    fn script_kind(&self, kind: ScriptKind) -> &'static str;
    /// What differs between two entries of one name.
    fn script_change(&self, was: &ScriptEntry, now: &ScriptEntry) -> String;
    /// What the approval dialog says above the lists.
    fn compared(&self, compared: &Compared, local: &Local) -> String;
    /// What the picker calls a comparison.
    fn scope_label(&self, scope: &DiffScope) -> String;
    /// A comparison in one line under its name.
    fn scope_detail(&self, kind: ScopeKind) -> &'static str;
    /// An empty comparison.
    fn scope_empty(&self, scope: &DiffScope) -> &'static str;
    /// A file's status in a word.
    fn status_word(&self, status: ChangeStatus) -> &'static str;
    /// A call that did not answer.
    fn problem(&self, problem: &Problem) -> String;
    /// A save that wrote nothing.
    fn save_failure(&self, failure: &SaveFailure) -> String;

    /// `refusal` in plain words, with the paths and output it names.
    fn refusal<'a>(&self, refusal: &'a GitRefusal) -> RefusalWords<'a> {
        let (title, next) = self.refusal_lines(refusal);
        let (paths, output): (&[RelPath], Option<&str>) = match refusal {
            GitRefusal::WouldOverwrite { paths } | GitRefusal::Conflicts { paths } => (paths, None),
            GitRefusal::HookFailed { output } => (&[], Some(output)),
            GitRefusal::RemoteFailed { message } => (&[], Some(message)),
            GitRefusal::NeedsCredential
            | GitRefusal::TimedOut
            | GitRefusal::NonFastForward
            | GitRefusal::Diverged
            | GitRefusal::NothingToCommit
            | GitRefusal::NoIdentity
            | GitRefusal::NoUpstream
            | GitRefusal::Detached
            | GitRefusal::CheckedOutElsewhere { .. }
            | GitRefusal::NothingToStash
            | GitRefusal::StashMoved
            | GitRefusal::BranchExists
            | GitRefusal::NoSuchRef
            | GitRefusal::Operation { .. }
            | GitRefusal::RepositoryConfig { .. }
            | GitRefusal::Unknown => (&[], None),
        };
        RefusalWords {
            title,
            next,
            paths,
            output,
        }
    }
}

#[cfg(test)]
mod tests;
