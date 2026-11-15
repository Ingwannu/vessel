//! Menu feedback is chosen from the keyed write, not from a refreshed status.
//! A direct action keeps its menu; opening a form deliberately dismisses it.

use super::{Action, ActionRow, SuccessReceipt, WriteIntent, Writing};
use crate::workspace::git_quick::GitAction;

/// Rendering maps this phase to the shared kit; it owns no timer or write slot.
#[derive(Debug)]
pub enum MenuFeedback<'a> {
    /// No matching write owns this row.
    Idle,
    /// The exact pending intention supplies its localized label.
    Pending(&'a WriteIntent),
    /// The matching receipt retains its original intention and outcome.
    Success(&'a SuccessReceipt),
}

/// Whether the action opens another surface or stays beside its feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuClose {
    /// Direct writes and unavailable explanations keep the current menu.
    KeepOpen,
    /// Vessel's additional manual forms/choosers replace the menu.
    OpenSurface,
}

/// Only the owning action can acquire another request's spinner or check.
#[must_use]
pub fn menu_feedback(action: Action, writing: &Writing) -> MenuFeedback<'_> {
    if let Some(intent) = writing.intent()
        && intent.action() == Some(action)
    {
        return MenuFeedback::Pending(intent);
    }
    if let Some(receipt) = writing.success()
        && receipt.intent().action() == Some(action)
    {
        return MenuFeedback::Success(receipt);
    }
    MenuFeedback::Idle
}

/// Known unavailability is explained in place, with no node write. Otherwise
/// the legacy manual form choices close; direct writes keep pending feedback.
#[must_use]
pub const fn menu_close(row: ActionRow) -> MenuClose {
    if row.unavailable.is_some() {
        return MenuClose::KeepOpen;
    }
    match row.action {
        Action::Quick(GitAction::Push | GitAction::Pull | GitAction::Fetch)
        | Action::SaveStash(_) => MenuClose::KeepOpen,
        Action::Quick(GitAction::Commit)
        | Action::SwitchBranch
        | Action::CreateBranch
        | Action::RenameBranch
        | Action::Stashes => MenuClose::OpenSurface,
    }
}

#[cfg(test)]
mod tests;
