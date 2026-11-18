//! The tab menu (TRACKER 797): one builder for a tab's context menu in the
//! wide tab strip and a row's "…" in the compact tab switcher, as Paseo's
//! `buildWorkspaceTabMenuEntries` serves its desktop and mobile surfaces
//! (`workspace-tab-menu.ts`).
//!
//! [Decision Log]
//! - Purpose: Vessel had two builders that drifted: the strip's context
//!   menu (Copy ID or Copy path, Rename on workspace layouts, the closes
//!   when the tab closed locally, no glyphs) and the switcher's (TRACKER
//!   789: Paseo's mobile items, then the session's own actions).
//! - Reference: Paseo, per tab kind and in this order: an agent's Copy
//!   resume command and Copy agent id (its first seven characters as the
//!   hint); a terminal's Copy terminal id (hint); a file's Copy file path;
//!   Rename and a rule on an agent's or a terminal's tab; Close to the left
//!   / above, to the right / below, Close other tabs; Reload agent on an
//!   agent's tab; Close. Each row leads with a 16 muted glyph; the surface
//!   changes only the two directional labels. Both menus are 220 wide.
//! - Choice: `entries(shell, tab, surface)`, drawn by both surfaces, with
//!   one press path per row (`menu::chose`). After Paseo's rows, on the
//!   open session's agent tab, Vessel's own session actions follow a rule
//!   (its marks, Copy link, Agent log, Kill), as 789's row menu had them.
//! - Reload agent (coordinator, 2026-11-18): not in 797. Paseo's refresh
//!   interrupts a running turn and reloads the agent from persistence, which
//!   a client-side park and wake cannot do without racing; it comes with
//!   its own node call as TRACKER 803 (ADR 0110), with the kit's row
//!   tooltip. Its slot is marked below.
//! - Differences kept: Rename shows on the open session's agent tab only
//!   (both rename dialogs read the open session; a terminal has no rename
//!   yet); a file has no "Open with…" (one panel per file); the closes stay
//!   disabled where they would close a tab that does not close locally (an
//!   agent archives, a terminal confirms), where Paseo disables them by
//!   position only.

use super::menu::{Action, Copied, Surface, close_item};
use super::with_layout;
use crate::areas::conversation::{MenuAction, session_menu_item, session_menu_rows};
use crate::shell::Shell;
use vessel_app_kit::MenuItem;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::LeadingSize;
use vessel_app_kit::ui::{ClipboardItem, Context};
use vessel_app_view::layout::{CloseScope, TabId, Target};
use vessel_app_view::session::{MarkAction, ResumeUnavailable, resume_command};
use vessel_app_view::toasts::ToastTone;
use vessel_proto::SessionId;

#[cfg(test)]
mod tests;

/// Paseo's id hint: an id's first seven characters.
const ID_HINT: usize = 7;

/// `item` led by `glyph` at Paseo's 16.
fn led(item: MenuItem, glyph: Glyph) -> MenuItem {
    item.with_leading_at(glyph, LeadingSize::Regular)
}

/// A copy row with an id's hint.
fn hinted(label: &str, id: &impl ToString) -> MenuItem {
    led(MenuItem::new(label.to_owned()), Glyph::Copy)
        .with_value(id.to_string().chars().take(ID_HINT).collect::<String>())
}

/// The nearest kit glyph for a session action.
const fn session_glyph(action: MenuAction) -> Glyph {
    match action {
        MenuAction::Mark(MarkAction::Pin | MarkAction::Unpin) => Glyph::Pin,
        MenuAction::Mark(MarkAction::MarkUnread) => Glyph::CircleDot,
        MenuAction::Mark(MarkAction::MarkRead) => Glyph::Circle,
        MenuAction::Mark(MarkAction::StartAgain) => Glyph::RotateCw,
        MenuAction::Mark(MarkAction::Archive) => Glyph::Archive,
        MenuAction::Mark(MarkAction::Unarchive) => Glyph::ArchiveRestore,
        MenuAction::Mark(MarkAction::Delete) => Glyph::Trash2,
        MenuAction::CopyId | MenuAction::CopyLink => Glyph::Copy,
        MenuAction::Close => Glyph::X,
        MenuAction::AgentLog => Glyph::FileText,
        MenuAction::Kill => Glyph::Stop,
    }
}

/// The glyph of a close row, by its scope.
const fn close_glyph(scope: CloseScope) -> Glyph {
    match scope {
        CloseScope::Before => Glyph::ArrowLeftToLine,
        CloseScope::After => Glyph::ArrowRightToLine,
        CloseScope::Others => Glyph::CopyX,
        CloseScope::Current | CloseScope::Pane => Glyph::X,
    }
}

/// The tab's target, while the tab is in the layout.
fn target_of(shell: &Shell, tab: TabId) -> Option<Target> {
    with_layout(shell, |layout| {
        Ok(layout.tab(tab).map(|tab| tab.target().clone()))
    })
    .ok()
    .flatten()
}

/// Paseo's copies, by the tab's kind.
fn copies(shell: &Shell, target: &Target) -> Vec<(Action, MenuItem)> {
    let words = shell.words.layout();
    let mut items = Vec::new();
    match target {
        Target::Agent(id) => {
            items.push((
                Action::CopyResume,
                led(
                    MenuItem::new(words.copy_resume_command().to_owned()),
                    Glyph::Copy,
                ),
            ));
            items.push((
                Action::Copy(Copied::AgentId),
                hinted(words.copy_agent_id(), id),
            ));
        }
        Target::Terminal(id) => items.push((
            Action::Copy(Copied::TerminalId),
            hinted(words.copy_terminal_id(), id),
        )),
        Target::File { .. } => items.push((
            Action::Copy(Copied::FilePath),
            led(
                MenuItem::new(words.copy_file_path().to_owned()),
                Glyph::Copy,
            ),
        )),
        Target::NewTab
        | Target::Draft(_)
        | Target::ProviderSubagent { .. }
        | Target::Browser(_)
        | Target::Changes
        | Target::Files
        | Target::Scripts
        | Target::PullRequest
        | Target::WorkingDiff
        | Target::Plugin { .. }
        | Target::Setup(_)
        | Target::CommitDiff(_) => {}
    }
    items
}

/// The tab menu's rows, in Paseo's order, then the open session's own.
pub(super) fn entries(shell: &Shell, tab: TabId, surface: Surface) -> Vec<(Action, MenuItem)> {
    let words = shell.words.layout();
    let Some(target) = target_of(shell, tab) else {
        return Vec::new();
    };
    let open = shell.areas.session.open.as_ref().map(|open| open.session);
    let session_tab = matches!(&target, Target::Agent(id) if Some(*id) == open);
    let session_rows = if session_tab {
        session_menu_rows(shell)
    } else {
        Vec::new()
    };
    let mut items = copies(shell, &target);
    let renamable = shell
        .areas
        .conversation
        .info
        .as_deref()
        .is_some_and(vessel_app_view::session::can_rename);
    if session_tab && renamable {
        items.push((
            Action::Rename,
            led(MenuItem::new(words.rename().to_owned()), Glyph::Pencil),
        ));
    }
    let closes = items.len();
    let (before, after) = match surface {
        Surface::Strip => (words.close_left(), words.close_right()),
        Surface::Switcher => (words.close_above(), words.close_below()),
    };
    for (scope, label) in [
        (CloseScope::Before, before),
        (CloseScope::After, after),
        (CloseScope::Others, words.close_others()),
    ] {
        let (action, item) = close_item(shell, tab, scope, label);
        items.push((action, led(item, close_glyph(scope))));
    }
    // Paseo's rule follows an agent's or a terminal's rows (after its
    // Rename); a file's copy runs into its closes.
    if matches!(target, Target::Agent(_) | Target::Terminal(_)) {
        group_from(&mut items, closes);
    }
    // TRACKER 803: Reload agent goes here, on an agent's tab, as Paseo's.
    if session_tab {
        // The agent's own Close follows the session's lifecycle policy (a
        // root archives first), as the bar's Close does.
        if session_rows.contains(&MenuAction::Close) {
            let mut item = session_menu_item(shell, MenuAction::Close);
            item.label = words.close().to_owned().into();
            items.push((Action::Session(MenuAction::Close), led(item, Glyph::X)));
        }
    } else {
        let (action, item) = close_item(shell, tab, CloseScope::Current, words.close());
        items.push((action, led(item, close_glyph(CloseScope::Current))));
    }
    // Vessel's extras, after Paseo's items.
    let extras = items.len();
    items.extend(
        session_rows
            .into_iter()
            .filter(|action| !matches!(action, MenuAction::CopyId | MenuAction::Close))
            .map(|action| {
                let mut item = session_menu_item(shell, action);
                item.starts_group = false;
                (Action::Session(action), led(item, session_glyph(action)))
            }),
    );
    group_from(&mut items, extras);
    items
}

/// A rule above row `at`, when rows come before it.
fn group_from(items: &mut [(Action, MenuItem)], at: usize) {
    if at > 0
        && let Some((_, item)) = items.get_mut(at)
    {
        item.starts_group = true;
    }
}

/// What a copy row puts on the clipboard for a tab.
fn copy_text(target: &Target) -> Option<String> {
    match target {
        Target::Agent(id) => Some(id.to_string()),
        Target::Terminal(id) => Some(id.to_string()),
        Target::File { path, .. } => Some(path.as_str().to_owned()),
        Target::NewTab
        | Target::Draft(_)
        | Target::ProviderSubagent { .. }
        | Target::Browser(_)
        | Target::Changes
        | Target::Files
        | Target::Scripts
        | Target::PullRequest
        | Target::WorkingDiff
        | Target::Plugin { .. }
        | Target::Setup(_)
        | Target::CommitDiff(_) => None,
    }
}

/// The resume command of `session`'s agent: from the open session's own
/// description, else from its card.
fn resume_of(shell: &Shell, session: SessionId) -> Result<String, ResumeUnavailable> {
    if let Some(info) = shell
        .areas
        .conversation
        .info
        .as_deref()
        .filter(|info| info.id == session)
    {
        return resume_command(Some(&info.spec.harness), info.harness_session.as_ref());
    }
    let card = shell
        .fleet
        .machines
        .iter()
        .flat_map(|machine| machine.sessions.iter())
        .find(|card| card.session == session);
    card.map_or(Err(ResumeUnavailable::NoId), |card| {
        resume_command(
            card.detail.harness.as_ref(),
            card.detail.harness_session.as_ref(),
        )
    })
}

impl Shell {
    /// A copy row: the tab's id or path, then Paseo's "Copied …" toast.
    pub(super) fn tab_menu_copy(&mut self, tab: TabId, copied: Copied, cx: &mut Context<'_, Self>) {
        let Some(text) = target_of(self, tab).as_ref().and_then(copy_text) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        // The catalog is the app's for its life: borrowed apart from `self`.
        let translator = self.words;
        let words = translator.layout();
        let label = match copied {
            Copied::AgentId => words.copied_agent_id(),
            Copied::TerminalId => words.copied_terminal_id(),
            Copied::FilePath => words.copied_file_path(),
        };
        self.toast_copied(label, cx);
    }

    /// Copy resume command: the agent's CLI command, or why there is none
    /// (Paseo's notices, the id's first).
    pub(super) fn tab_menu_copy_resume(&mut self, tab: TabId, cx: &mut Context<'_, Self>) {
        let Some(Target::Agent(session)) = target_of(self, tab) else {
            return;
        };
        let translator = self.words;
        let words = translator.layout();
        match resume_of(self, session) {
            Ok(command) => {
                cx.write_to_clipboard(ClipboardItem::new_string(command));
                self.toast_copied(words.copied_resume_command(), cx);
            }
            // Nothing was copied: Paseo's error toasts
            // (workspace-screen.tsx 2132, 2143), gone after 3.2 s.
            Err(ResumeUnavailable::NoId) => {
                self.toast(words.resume_id_unavailable(), ToastTone::Error, cx);
            }
            Err(ResumeUnavailable::NoCommand) => {
                self.toast(words.resume_command_unavailable(), ToastTone::Error, cx);
            }
        }
    }

    /// Paseo's `toast.copied` (workspace-screen.tsx 2090, 2103).
    fn toast_copied(&mut self, label: &str, cx: &mut Context<'_, Self>) {
        let words = self.words.layout();
        let text = words.copied(label).unwrap_or_else(|_| label.to_owned());
        self.toast(text, ToastTone::Copied, cx);
    }
}
