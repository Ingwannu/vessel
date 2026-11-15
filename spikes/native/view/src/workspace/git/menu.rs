//! The toolbar's available actions, shared by the primary half and menu.
//! Decisions stay here so a stale drawing cannot bypass a busy/read gate.
//!
//! [Decision Log]
//! - Purpose: one action policy for the primary half, menu and callbacks.
//! - Constraints: a refreshing status may be drawn but cannot authorize
//!   a write; a fixed-height label must never wrap inside the control.
//! - Alternatives: promote Fetch as the idle primary as the page does,
//!   or let each callback interpret repository state independently.
//! - Choice: reuse the tested quick-action rules, keep idle Fetch in the
//!   menu-only shape after Paseo, and choose a typed density from width.
//! - Why: the references' policy fixtures stay useful while all actions
//!   remain reachable and no narrow control cuts a second line in half.
//! - Cost: callers supply their actual allocated panel width and recheck
//!   availability when selecting an action.

use vessel_proto::workspace::{Branch, GitStatus};

use crate::workspace::git_quick::{Blocked, GitAction, action_blocked, quick_action};

use super::{CommitForm, Reading, Writing};

/// What a saved stash includes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashScope {
    /// Tracked paths only.
    Tracked,
    /// Also save untracked paths.
    WithUntracked,
}

/// A toolbar/menu action; form opening is distinct from a node write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Commit, push, pull or fetch.
    Quick(GitAction),
    /// Choose a listed branch.
    SwitchBranch,
    /// Create and check out a branch.
    CreateBranch,
    /// Rename the current branch.
    RenameBranch,
    /// Save local changes.
    SaveStash(StashScope),
    /// Inspect saved entries.
    Stashes,
}

/// Why a drawn action cannot run yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// Another write owns the repository's action slot.
    Writing,
    /// The displayed facts are from a previous read.
    Reading,
    /// No status request has supplied facts yet.
    StatusMissing,
    /// Reading failed and automatic polling is stopped until retry.
    StatusFailed,
    /// A quick action's state-specific reason.
    Git(Blocked),
    /// Renaming needs a checked-out branch.
    NoBranch,
    /// Stashing needs local edits.
    NoChanges,
    /// A repository without a first commit cannot push or stash.
    NoCommits,
}

/// One menu row; a reason also makes the row unselectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionRow {
    /// Its typed intention.
    pub action: Action,
    /// Its reason, when unavailable.
    pub unavailable: Option<Unavailable>,
    /// A rule separates this row from the previous group.
    pub starts_group: bool,
}

/// One action's gate, used again on selection rather than trusting an
/// earlier frame. Refreshing facts remain visible but cannot authorize a
/// write; the node rechecks once the request arrives too.
#[must_use]
pub fn availability(
    action: Action,
    status: &Reading<GitStatus>,
    writing: &Writing,
) -> Option<Unavailable> {
    if writing.busy() {
        return Some(Unavailable::Writing);
    }
    let status = match status {
        Reading::Empty => return Some(Unavailable::StatusMissing),
        Reading::Failed(_) => return Some(Unavailable::StatusFailed),
        Reading::Loading { .. } => return Some(Unavailable::Reading),
        Reading::Ready(status) => status,
    };
    if status.head.is_none()
        && matches!(
            action,
            Action::Quick(GitAction::Push | GitAction::Pull)
                | Action::CreateBranch
                | Action::SaveStash(_)
        )
    {
        return Some(Unavailable::NoCommits);
    }
    match action {
        Action::Quick(GitAction::Commit)
            if CommitForm::of(status)
                .has_changes(vessel_proto::workspace::CommitStage::Everything) =>
        {
            None
        }
        Action::Quick(action) => action_blocked(action, status).map(Unavailable::Git),
        Action::RenameBranch => status.branch.is_none().then_some(Unavailable::NoBranch),
        Action::SaveStash(_) => status.files.is_empty().then_some(Unavailable::NoChanges),
        Action::SwitchBranch | Action::CreateBranch | Action::Stashes => None,
    }
}

/// The primary action chosen from the shown facts. Availability is kept
/// separate so refresh does not replace a useful label with another one.
#[must_use]
pub fn primary(status: &Reading<GitStatus>) -> Option<Action> {
    let status = status.shown()?;
    let action = quick_action(status);
    if status.head.is_none() && action != GitAction::Commit {
        return None;
    }
    // Paseo's policy.test.ts keeps a clean, synchronized base branch
    // menu-only. Fetch remains reachable there rather than pretending a
    // commit, push or pull is needed.
    match action {
        GitAction::Fetch => None,
        GitAction::Commit | GitAction::Push | GitAction::Pull => Some(Action::Quick(action)),
    }
}

/// The split button's primary half as it is drawn now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryControl {
    /// Its action; none when nothing is needed (the half is the menu's
    /// glyph and opens the menu, Paseo's menu-only trigger).
    pub action: Option<Action>,
    /// Its action is the write out now: the spinner replaces it. Another
    /// write out does not dim it; choosing it then says why it waits
    /// (Paseo's runner keeps the button live and explains).
    pub running: bool,
    /// The panel has room for its words beside its glyph.
    pub labelled: bool,
}

/// The primary half from the shown facts, the write out and the panel's width.
#[must_use]
pub fn primary_control(
    status: &Reading<GitStatus>,
    writing: &Writing,
    width: PanelWidth,
) -> PrimaryControl {
    let action = primary(status);
    let running =
        action.is_some() && writing.intent().and_then(super::WriteIntent::action) == action;
    PrimaryControl {
        action,
        running,
        labelled: action.is_some() && width.density() == ControlDensity::Labelled,
    }
}

/// The page's action order, with no primary action hidden from its menu.
#[must_use]
pub fn rows(status: &Reading<GitStatus>, writing: &Writing) -> Vec<ActionRow> {
    [
        Action::Quick(GitAction::Commit),
        Action::Quick(GitAction::Push),
        Action::Quick(GitAction::Pull),
        Action::Quick(GitAction::Fetch),
        Action::SwitchBranch,
        Action::CreateBranch,
        Action::RenameBranch,
        Action::SaveStash(StashScope::Tracked),
        Action::SaveStash(StashScope::WithUntracked),
        Action::Stashes,
    ]
    .into_iter()
    .map(|action| ActionRow {
        action,
        unavailable: availability(action, status, writing),
        starts_group: matches!(
            action,
            Action::SwitchBranch | Action::SaveStash(StashScope::Tracked)
        ),
    })
    .collect()
}

/// The page's case-insensitive branch filter. The native measured list
/// can show all of the node's bounded result, so none are silently sliced
/// away after 200 as in the page's DOM list.
#[must_use]
pub fn filter_branches(branches: &[Branch], filter: &str) -> Vec<Branch> {
    let needle = filter.trim().to_lowercase();
    branches
        .iter()
        .filter(|branch| branch.name.as_str().to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

/// The width the code panel allocates to its toolbar, in logical points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelWidth(pub f32);

/// A control's density, not a boolean whose meaning a caller must guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlDensity {
    /// Keep the action's one-line label beside its glyph.
    Labelled,
    /// Preserve the action and its tooltip without squeezing its label.
    IconOnly,
}

impl PanelWidth {
    /// The split action shares the toolbar with a comparison pill and
    /// counts. Below 480 points the label yields first, as Paseo's
    /// hideLabels does. The 440-point side panel therefore never wraps a
    /// label inside a fixed-height control.
    #[must_use]
    pub const fn density(self) -> ControlDensity {
        if self.0 >= 480.0 {
            ControlDensity::Labelled
        } else {
            ControlDensity::IconOnly
        }
    }
}
