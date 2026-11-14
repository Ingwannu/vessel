//! The "Create PR" form: what it starts with, the request it sends
//! (`pr.open`) and what its answer means to the person.
//!
//! [Decision Log]
//! - Purpose: Paseo's "Create PR" sends no title and its daemon writes one
//!   with a model (`generatePullRequestText`); this node takes a title it
//!   is given (`PullTitle`) and never asks an agent, so the person writes
//!   it, with title, description, base and draft (the task's form).
//! - Defaults: GitHub's `gh pr create --fill`: one new commit gives its
//!   subject as the title; several give the branch name with `-` and `_`
//!   read as spaces, and the description lists their subjects oldest
//!   first. The base is the one the worktree was made from (Paseo's stored
//!   base ref), else the node's default branch (sent as absent).
//! - Checks: the title's and description's rules are `vessel-proto`'s
//!   (`forge/names.rs`): checked here first so the form says which, then
//!   parsed by the wire's own type.
//! - Cost: a title the wire rejects for a rule added later reads as
//!   "not accepted" without naming the rule.

use vessel_proto::SessionId;
use vessel_proto::forge::{
    OpenOutcome, OpenPullRequest, OpenRefusal, PullBody, PullRequest, PullTitle,
};
use vessel_proto::workspace::{BranchName, CommitInfo};

/// The most characters of a title (`vessel-proto`'s `TITLE`).
const TITLE_CHARS: usize = 256;
/// The most bytes of a description (`vessel-proto`'s `BODY`).
const BODY_BYTES: usize = 64 * 1024;

/// What the form holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenDraft {
    /// The title as typed.
    pub title: String,
    /// The description as typed; empty sends none.
    pub body: String,
    /// The base chosen; `None` is the node's default branch.
    pub base: Option<BranchName>,
    /// Open it as a draft.
    pub draft: bool,
}

/// Why the form cannot be sent as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenProblem {
    /// The title is empty or only spaces.
    TitleBlank,
    /// The title is longer than 256 characters.
    TitleLong,
    /// The description is longer than 64 KiB.
    BodyLong,
    /// The wire refused it for a rule this app does not name.
    NotAccepted,
}

/// The request for `session` from `draft`, or why not.
///
/// # Errors
/// The first rule the title or the description breaks.
pub fn open_request(session: SessionId, draft: &OpenDraft) -> Result<OpenPullRequest, OpenProblem> {
    // A line break or a tab typed or pasted into the title is a space: the
    // field is one line, the forge's title too.
    let title: String = draft
        .title
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let title = title.trim();
    if title.is_empty() {
        return Err(OpenProblem::TitleBlank);
    }
    if title.chars().count() > TITLE_CHARS {
        return Err(OpenProblem::TitleLong);
    }
    let body = draft.body.trim_end().replace('\0', "");
    if body.len() > BODY_BYTES {
        return Err(OpenProblem::BodyLong);
    }
    // The wire's own rules decide last; their diagnostic (a rule's name in
    // English) is not a sentence for the form, so only the fact is kept.
    let title: PullTitle = title.parse().ok().ok_or(OpenProblem::NotAccepted)?;
    let body = if body.trim().is_empty() {
        None
    } else {
        Some(
            body.parse::<PullBody>()
                .ok()
                .ok_or(OpenProblem::NotAccepted)?,
        )
    };
    Ok(OpenPullRequest {
        session,
        title,
        body,
        base: draft.base.clone(),
        draft: draft.draft,
    })
}

/// The form's start: `commits` are the branch's new ones, newest first
/// (the log from `HEAD`, as many as its base is behind), `branch` the
/// branch's name and `base` the base the worktree was made from.
#[must_use]
pub fn prefill(commits: &[CommitInfo], branch: &str, base: Option<BranchName>) -> OpenDraft {
    let (title, body) = match commits {
        [one] => (one.subject.clone(), String::new()),
        several => {
            let mut body = String::new();
            for commit in several.iter().rev() {
                body.push_str("- ");
                body.push_str(&commit.subject);
                body.push('\n');
            }
            (humanize(branch), body)
        }
    };
    OpenDraft {
        title,
        body,
        base,
        draft: false,
    }
}

/// `gh`'s `humanize`: a branch name's `-` and `_` read as spaces.
fn humanize(branch: &str) -> String {
    branch
        .chars()
        .map(|character| match character {
            '-' | '_' => ' ',
            other => other,
        })
        .collect()
}

/// What `pr.open` answered, as the form acts on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// Made: the form closes and says so (Paseo's "PR Created").
    Made(Box<PullRequest>),
    /// One was open for that branch already; nothing new was made: the
    /// form closes and shows it.
    Existing(Box<PullRequest>),
    /// Refused: the form stays open with why.
    Refused(OpenRefusal),
}

/// `outcome` read for the form.
#[must_use]
pub fn open_outcome(outcome: OpenOutcome) -> Opened {
    match outcome {
        OpenOutcome::Opened { pull_request, .. } => Opened::Made(pull_request),
        OpenOutcome::Exists { pull_request, .. } => Opened::Existing(pull_request),
        OpenOutcome::Refused { refusal, .. } => Opened::Refused(refusal),
        OpenOutcome::Unknown => Opened::Refused(OpenRefusal::Unknown),
    }
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
