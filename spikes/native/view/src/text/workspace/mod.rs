//! The workspace's words (track F's area, `text/mod.rs`'s convention), in
//! English and Korean: one table per language
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
pub mod labels;
mod labels_en;
mod labels_ko;
mod script_words;

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::ServicePort;
use vessel_proto::workspace::{
    ChangeStatus, CloneFailure, CloneStage, DiffScope, GitOperation, GitRefusal, GitSupport,
    KeptReason, ProjectState, RelPath, SetupFailure, SetupState, WorktreeRefusal,
};

pub use en::En;
pub use ko::Ko;
pub use labels::Labels;

use crate::workspace::branch_name::BranchNameProblem;
use crate::workspace::changes::{FileNote, ScopeKind};
use crate::workspace::command_changes::Compared;
use crate::workspace::git_quick::{Blocked, GitAction};
use crate::workspace::place::PlaceProblem;
use crate::workspace::problem::Problem;
use crate::workspace::save::SaveFailure;
use crate::workspace::size::size_word;
use crate::workspace::start::Refusal as StartRefusal;

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

/// Which way a project is added (the add form's three titles and hints).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddForm {
    /// A folder on the machine.
    Folder,
    /// A repository cloned into a folder.
    Clone,
    /// A new repository in an empty folder.
    New,
}

/// The workspace's table of `lang`.
impl super::Lang {
    /// The workspace's words in this language.
    #[must_use]
    pub const fn workspace(self) -> &'static dyn WorkspaceWords {
        match self {
            Self::En => &En,
            Self::Ko => &Ko,
        }
    }

    /// The workspace screens' fixed words, where a `const fn` needs them
    /// (a route's name).
    #[must_use]
    pub const fn workspace_labels(self) -> &'static Labels {
        match self {
            Self::En => &labels::EN,
            Self::Ko => &labels::KO,
        }
    }
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
    /// The declared port, including an explicit absence when comparing changes.
    fn script_port(&self, port: Option<ServicePort>) -> String;
    /// What differs between two entries of one name, including declared ports.
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

    /// The screens' fixed words.
    fn labels(&self) -> &'static Labels;
    /// "Projects on laptop".
    fn projects_on(&self, machine: &str) -> String;
    /// The empty list's line.
    fn no_projects_body(&self, machine: &str) -> String;
    /// A machine older than projects.
    fn older_machine(&self, machine: &str) -> String;
    /// A project forgotten.
    fn forgot(&self, name: &str) -> String;
    /// Something not removed, and why.
    fn not_removed(&self, name: &str, why: &str) -> String;
    /// How many live sessions.
    fn live(&self, count: usize) -> String;
    /// Why tidying kept a worktree.
    fn kept(&self, reason: &str) -> String;
    /// "2 setup, 1 teardown, 3 scripts and services".
    fn commands_title(&self, setup: usize, teardown: usize, scripts: usize) -> String;
    /// A `vessel.toml` that does not parse.
    fn invalid_commands(&self, message: &str) -> String;
    /// An approval or a withdrawal refused.
    fn commands_not(&self, approved: bool, why: &str) -> String;
    /// Setup not started again.
    fn setup_not_started(&self, why: &str) -> String;
    /// A worktree removed, with the snapshot kept first.
    fn removed(&self, branch: &str, snapshot: Option<&str>) -> String;
    /// The removal dialog's title.
    fn remove_title(&self, branch: &str) -> String;
    /// What discarding destroys.
    fn discard_says(&self, path: &str, branch: &str) -> String;
    /// A snapshot not dropped.
    fn snapshot_not_dropped(&self, name: &str, why: &str) -> String;
    /// The add form's help line: `clone_target` is the folder and the
    /// repository's name a clone lands in, when both are known.
    fn add_hint(&self, form: AddForm, machine: &str, clone_target: Option<(&str, &str)>) -> String;
    /// The approval dialog's first paragraph.
    fn approval_copy(&self, place: &str, scripts: bool) -> String;
    /// Where a project's setup runs ("in each new worktree of shop on laptop").
    fn approval_place(&self, project: &str, machine: &str, each: bool) -> String;
    /// The set's digest line.
    fn digest(&self, digest: &str) -> String;
    /// The code panel's file view, its button when lines are chosen
    /// ("Add lines 12–20 to chat"); one line when `first` is `last`.
    fn add_lines(&self, first: usize, last: usize) -> String;
    /// The file editor's toolbar: how many lines the file has (Paseo's
    /// `lines`).
    fn editor_lines(&self, count: usize) -> String;
    /// The code panel's search by name, the line above its results: how
    /// many files match; `searched` files were looked at, every file of
    /// the folder when `complete`; `capped` when as many came as the
    /// panel asks for (there may be more: the best are shown).
    fn found_count(&self, files: usize, searched: u32, complete: bool, capped: bool) -> String;
    /// The code panel's search in files, the line above its results: how
    /// many lines in how many files; `more` when the machine stopped
    /// before the last line that matches.
    fn grepped_count(&self, lines: usize, files: usize, more: bool) -> String;
    /// An edited command's old text.
    fn was(&self, command: &str) -> String;

    /// A note in a file's place in the Changes list.
    fn file_note(&self, note: &FileNote) -> String {
        let labels = self.labels();
        match note {
            FileNote::Loading => labels.loading.to_owned(),
            FileNote::More => labels.show_rest.to_owned(),
            FileNote::Binary => labels.binary_file.to_owned(),
            FileNote::TooLarge { bytes } => {
                format!("{} ({})", labels.too_large, size_word(*bytes))
            }
            FileNote::Submodule { from, to } => format!(
                "{}: {} \u{2192} {}",
                labels.submodule,
                from.as_deref().unwrap_or("\u{2014}"),
                to.as_deref().unwrap_or("\u{2014}")
            ),
            FileNote::Unchanged => labels.no_content_change.to_owned(),
            FileNote::Unknown => labels.unknown_content.to_owned(),
            FileNote::Failed(message) => message.clone(),
        }
    }

    /// Why a start stopped, in a sentence.
    fn start_refusal(&self, refusal: &StartRefusal) -> String {
        let labels = self.labels();
        match refusal {
            StartRefusal::Worktree(reason) => self.worktree_refusal(reason),
            StartRefusal::CommandsUnreadable => labels.commands_unreadable.to_owned(),
            StartRefusal::NotFollowed { branch, problem } => format!(
                "{} ({}): {}",
                labels.setup_not_followed,
                branch.as_deref().unwrap_or(labels.detached),
                self.problem(problem)
            ),
        }
    }

    /// `refusal` in plain words, with the paths and output it names.
    fn refusal<'a>(&self, refusal: &'a GitRefusal) -> RefusalWords<'a> {
        let (title, next) = self.refusal_lines(refusal);
        let (paths, output): (&[RelPath], Option<&str>) = match refusal {
            GitRefusal::WouldOverwrite { paths, .. } | GitRefusal::Conflicts { paths, .. } => {
                (paths, None)
            }
            GitRefusal::HookFailed { output, .. } => (&[], Some(output)),
            GitRefusal::RemoteFailed { message, .. } => (&[], Some(message)),
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

/// A refusal's title and what to do, from anything that becomes a string
/// (both tables' refusal arms).
fn lines(title: impl Into<String>, next: impl Into<String>) -> (String, String) {
    (title.into(), next.into())
}

#[cfg(test)]
mod tests;
