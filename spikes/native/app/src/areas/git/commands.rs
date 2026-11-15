//! Command-center contribution delegates the menu's projection and activation.
//!
//! [Decision Log]
//! - Purpose: palette callbacks must not become another git policy or RPC path.
//! - Constraint: a captured repository can disappear and later return; its name
//!   alone cannot authorize a callback from the earlier area incarnation.
//! - Choice: use the area's lifetime key issuer and recheck the shown repository,
//!   generation, session and live menu intention before existing owner dispatch.
//! - Tradeoff: this adds no connection resolver. The shared connection epoch
//!   preflight remains G4's boundary and is still pending for the C3 link.

use vessel_app_kit::menu::MenuItem;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::git::{Action, MenuFeedback, RequestKey, menu_feedback, primary};
use vessel_app_view::workspace::pull::{MenuEntry, PullEntry};
use vessel_proto::SessionId;

use super::{Target, menu};
use crate::shell::Shell;

/// Opaque callback authority; consumers retain it without rebuilding its fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitCommandTarget {
    repository: Target,
    generation: RequestKey,
    session: Option<SessionId>,
    pull: PullEntry,
}

/// The same primary choice and menu intentions the repository toolbar offers.
pub(crate) struct GitCommands {
    pub(crate) target: GitCommandTarget,
    /// A primary may explain unavailability; inspect its live menu item before
    /// promoting it as an enabled command-center default.
    pub(crate) primary: Option<Action>,
    pub(crate) entries: Vec<MenuEntry>,
}

impl Shell {
    /// One owner projection, restricted to the repository currently shown.
    #[must_use]
    pub(crate) fn git_command_entries(&self) -> Option<GitCommands> {
        let git = &self.areas.git;
        let repository = git.target.as_ref()?;
        let (machine, root) = self.code_git_context()?;
        if repository.machine != machine || repository.root != *root {
            return None;
        }
        let status = git.status.shown()?;
        Some(GitCommands {
            target: GitCommandTarget {
                repository: repository.clone(),
                generation: git.commands_generation?,
                session: self.pulls_subject().map(|(subject, _)| subject.session),
                pull: self.pulls_entry(status),
            },
            primary: primary(&git.status),
            entries: menu::entries(self, status),
        })
    }

    /// Existing localized menu presentation, including its busy gate.
    #[must_use]
    pub(crate) fn git_command_item(&self, entry: &MenuEntry) -> Option<MenuItem> {
        let status = self.areas.git.status.shown()?;
        Some(menu::item(self, entry, status))
    }

    /// A primary keeps its established explain-on-busy behaviour, while using
    /// the same captured authority. A menu acknowledgement remains inert.
    #[must_use]
    pub(super) fn run_git_primary(
        &mut self,
        target: &GitCommandTarget,
        entry: &MenuEntry,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(live) = self.git_command_entries() else {
            return false;
        };
        let Some(MenuEntry::Git(row)) = current_entry(target, &live, entry) else {
            return false;
        };
        let action = row.action;
        if matches!(
            menu_feedback(action, &self.areas.git.writing),
            MenuFeedback::Success(_)
        ) {
            return false;
        }
        self.git_choose(action, window, cx);
        true
    }

    /// Revalidate the captured contribution and delegate the current owner path.
    /// False means no callback was dispatched, so a palette may retain itself.
    #[must_use]
    pub(crate) fn run_git_command(
        &mut self,
        target: &GitCommandTarget,
        entry: &MenuEntry,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(live) = self.git_command_entries() else {
            return false;
        };
        let Some(entry) = current_entry(target, &live, entry).cloned() else {
            return false;
        };
        let Some(item) = self.git_command_item(&entry) else {
            return false;
        };
        if !item.selectable() {
            return false;
        }
        match entry {
            MenuEntry::Git(row) => self.git_choose(row.action, window, cx),
            MenuEntry::Pull(_) | MenuEntry::Merge(_) => {
                if self.areas.git.status.ready().is_none() {
                    return false;
                }
                // A live alias/root menu is not the origin of its drawn facts.
                // Keep this unavailable menu open, as Paseo's Info guidance does.
                self.git_unverified_view(cx);
            }
        }
        true
    }
}

/// A changed availability is re-evaluated, not trusted from a captured frame.
fn current_entry<'a>(
    captured: &GitCommandTarget,
    live: &'a GitCommands,
    entry: &MenuEntry,
) -> Option<&'a MenuEntry> {
    if captured != &live.target {
        return None;
    }
    live.entries
        .iter()
        .find(|candidate| match (entry, *candidate) {
            (MenuEntry::Git(old), MenuEntry::Git(new)) => old.action == new.action,
            (MenuEntry::Pull(old), MenuEntry::Pull(new)) => old == new,
            (MenuEntry::Merge(old), MenuEntry::Merge(new)) => old == new,
            (MenuEntry::Git(_), MenuEntry::Pull(_) | MenuEntry::Merge(_))
            | (MenuEntry::Pull(_), MenuEntry::Git(_) | MenuEntry::Merge(_))
            | (MenuEntry::Merge(_), MenuEntry::Git(_) | MenuEntry::Pull(_)) => false,
        })
}

#[cfg(test)]
mod tests;
