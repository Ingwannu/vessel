//! What choosing a git action does, decided again at the moment it is
//! chosen: Paseo's `useGitActionRunner` (`git/use-actions.tsx`) explains an
//! unavailable action in an Info toast and sends nothing; an available one
//! runs its handler. Here a handler either opens a form or prepares the
//! one write the node is sent.
//!
//! [Decision Log]
//! - Purpose: the menu, the primary half and their keys share one rule,
//!   tested without drawing, so a stale frame cannot send a write.
//! - Constraints: the node validates again; the wire's request types are
//!   built here once (the page's `git-bar.tsx` sets the upstream when the
//!   branch has none); forms (commit, branch name, saved entries) open
//!   instead of writing.
//! - Alternatives: let each GPUI callback match on the action and build
//!   its request; or disable unavailable rows (the page's way) so nothing
//!   can be explained when one is chosen.
//! - Choice: one pure function over the action, the shown root and the
//!   current reads, returning a typed step.
//! - Why: Paseo keeps unavailable rows selectable and says why on choice;
//!   the request's shape is checked by tests rather than in a drawing.
//! - Cost: the app wraps the prepared request in its link's write type.

use vessel_proto::workspace::{
    Fetch, GitStatus, Pull, Push, StashAction, StashChanges, WorkspaceRoot,
};

use super::{Action, BranchMode, Reading, StashScope, Unavailable, WriteIntent, Writing};
use crate::workspace::git_quick::GitAction;

/// A write the menu sends directly, without a form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectWrite {
    /// `git.push`.
    Push(Push),
    /// `git.pull`.
    Pull(Pull),
    /// `git.fetch`.
    Fetch(Fetch),
    /// `git.stash` (save).
    Stash(StashChanges),
}

/// What choosing an action does now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Say why it cannot run (Paseo's Info toast); nothing is sent.
    Explain(Unavailable),
    /// Open the commit form.
    Commit,
    /// Open the branch list.
    Switch,
    /// Open the branch name form.
    Branch(BranchMode),
    /// Open the saved entries.
    Stashes,
    /// Send this write, shown pending as this intention.
    Write(DirectWrite, WriteIntent),
}

/// The step for `action` on `root`, from the reads as they are now.
#[must_use]
pub fn step(
    action: Action,
    root: WorkspaceRoot,
    status: &Reading<GitStatus>,
    writing: &Writing,
) -> Step {
    // A form opens on the facts shown while a refresh is out (the form's
    // own submit waits for the newer read); a write never runs on them.
    // Without this, the five-second refresh turned "Commit…" into "wait"
    // for the length of each read.
    let on_shown;
    let current = match (status, status.shown()) {
        (Reading::Loading { .. }, Some(facts)) if opens_form(action) => {
            on_shown = Reading::Ready(facts.clone());
            &on_shown
        }
        _ => status,
    };
    if let Some(reason) = super::availability(action, current, writing) {
        return Step::Explain(reason);
    }
    let Some(facts) = current.ready() else {
        // `availability` returns a reason for every read that is not ready.
        return Step::Explain(Unavailable::StatusMissing);
    };
    match action {
        Action::Quick(GitAction::Commit) => Step::Commit,
        Action::Quick(GitAction::Push) => Step::Write(
            DirectWrite::Push(Push {
                root,
                remote: None,
                // The page's `git-bar.tsx`: a branch without an upstream is
                // pushed as its own and set as the upstream.
                set_upstream: facts.upstream.is_none() && facts.branch.is_some(),
            }),
            WriteIntent::Quick(GitAction::Push),
        ),
        Action::Quick(GitAction::Pull) => Step::Write(
            DirectWrite::Pull(Pull { root }),
            WriteIntent::Quick(GitAction::Pull),
        ),
        Action::Quick(GitAction::Fetch) => Step::Write(
            DirectWrite::Fetch(Fetch { root, remote: None }),
            WriteIntent::Quick(GitAction::Fetch),
        ),
        Action::SwitchBranch => Step::Switch,
        Action::CreateBranch => Step::Branch(BranchMode::Create),
        Action::RenameBranch => Step::Branch(BranchMode::Rename),
        Action::SaveStash(scope) => Step::Write(
            DirectWrite::Stash(StashChanges {
                root,
                action: StashAction::Save {
                    message: None,
                    untracked: scope == StashScope::WithUntracked,
                },
            }),
            WriteIntent::SaveStash(scope),
        ),
        Action::Stashes => Step::Stashes,
    }
}

/// Whether choosing `action` opens a form rather than writing.
const fn opens_form(action: Action) -> bool {
    match action {
        Action::Quick(GitAction::Commit)
        | Action::SwitchBranch
        | Action::CreateBranch
        | Action::RenameBranch
        | Action::Stashes => true,
        Action::Quick(GitAction::Push | GitAction::Pull | GitAction::Fetch)
        | Action::SaveStash(_) => false,
    }
}

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
