//! Command-center contribution delegates the menu's projection and activation.
//!
//! [Decision Log]
//! - Purpose: palette callbacks must not become another git policy or RPC path.
//! - Constraint: a captured repository can disappear and later return; its name
//!   alone cannot authorize a callback from the earlier area incarnation.
//! - Choice: use the area's lifetime key issuer and recheck the shown repository,
//!   generation, session and live menu intention before existing owner dispatch.
//! - Source receipt is captured while drawing and rechecked unchanged at press.
//!   Identity equality ignores guard liveness; actual currentness never does.
//! - This is an intent projection, not write authority. The scoped sent/uncertain
//!   owner and original G4 mutation preflight remain pending; origin refuses writes.

use vessel_app_kit::menu::MenuItem;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::workspace::git::{Action, MenuFeedback, RequestKey, menu_feedback, primary};
use vessel_app_view::workspace::pull::{MenuEntry, PullEntry};
use vessel_proto::SessionId;

use super::{Target, menu};
use crate::shell::Shell;

/// Logical intention plus its original drawn source; not mutation authority.
#[derive(Debug, Clone)]
pub(crate) struct GitCommandTarget {
    logical: LogicalTarget,
    source: crate::areas::workspace::ShownChangesContext,
}

/// The menu's pure intention identity remains independent of source admission.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalTarget {
    repository: Target,
    generation: RequestKey,
    session: Option<SessionId>,
    pull: PullEntry,
}

impl PartialEq for GitCommandTarget {
    fn eq(&self, other: &Self) -> bool {
        self.logical == other.logical && self.source.same_source(&other.source)
    }
}
impl Eq for GitCommandTarget {}

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
        if git.panels.owner_problem().is_some() {
            return None;
        }
        // Capture the SAME source now, while constructing this drawn callback.
        // A later callback cannot substitute a newer receipt from the same path.
        let source = self.shown_changes_context()?;
        let repository = git.target.as_ref()?;
        let (machine, root) = self.code_git_context()?;
        if repository.machine != machine
            || repository.root != *root
            || source.connection().machine().as_str() != machine
            || source.root() != *root
        {
            return None;
        }
        let status = git.status.shown()?;
        Some(GitCommands {
            target: GitCommandTarget {
                logical: LogicalTarget {
                    repository: repository.clone(),
                    generation: git.commands_generation?,
                    session: self.pulls_subject().map(|(subject, _)| subject.session),
                    pull: self.pulls_entry(status),
                },
                source,
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
        if !self.shown_changes_is_current(&target.source) {
            return false;
        }
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
        if !self.shown_changes_is_current(&target.source) {
            return false;
        }
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
            MenuEntry::Pull(_) => {
                if self.areas.git.status.ready().is_none() {
                    return false;
                }
                // A live alias/root menu is not the origin of its drawn facts.
                // Keep this unavailable menu open, as Paseo's Info guidance does.
                self.pulls_chosen(window, cx);
            }
            MenuEntry::Merge(method) => {
                if self.areas.git.status.ready().is_none() {
                    return false;
                }
                self.pulls_merge(method, cx);
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
    if !captured.source.same_source(&live.target.source) {
        return None;
    }
    logical_entry(
        &captured.logical,
        &live.target.logical,
        &live.entries,
        entry,
    )
}

fn logical_entry<'a>(
    captured: &LogicalTarget,
    live: &LogicalTarget,
    entries: &'a [MenuEntry],
    entry: &MenuEntry,
) -> Option<&'a MenuEntry> {
    if captured != live {
        return None;
    }
    entries.iter().find(|candidate| match (entry, *candidate) {
        (MenuEntry::Git(old), MenuEntry::Git(new)) => old.action == new.action,
        (MenuEntry::Pull(old), MenuEntry::Pull(new)) => old == new,
        (MenuEntry::Merge(old), MenuEntry::Merge(new)) => old == new,
        (MenuEntry::Git(_), MenuEntry::Pull(_) | MenuEntry::Merge(_))
        | (MenuEntry::Pull(_), MenuEntry::Git(_) | MenuEntry::Merge(_))
        | (MenuEntry::Merge(_), MenuEntry::Git(_) | MenuEntry::Pull(_)) => false,
    })
}

#[cfg(test)]
pub(in crate::areas) use tests::source_projection;

#[cfg(test)]
mod tests;
