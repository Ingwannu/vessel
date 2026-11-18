//! A switcher row's menu (TRACKER 789): Paseo's tab items
//! (`workspace-tab-menu.ts` `buildWorkspaceTabMenuEntries`, surface
//! "mobile"), then, on the open session's agent tab, the session's own
//! actions that Paseo's tab menu lacks (its marks, Copy link, Agent log,
//! Kill), as the coordinator ruled (Q1).
//!
//! [Decision Log]
//! - Values (Paseo's): every row leads with a 16 muted glyph: Copy agent id
//!   `copy`, Rename `pencil`, Close tabs above `arrow-left-to-line`, Close
//!   tabs below `arrow-right-to-line`, Close other tabs `copy-x`, Close `x`.
//!   Copy agent id carries the id's first seven characters as its muted
//!   hint.
//! - Vessel's extras take the nearest kit glyph (Pin, CircleDot, Circle,
//!   RotateCw, Archive, ArchiveRestore, Trash2, Copy, FileText, Stop), so
//!   every label starts at one x without a gutter for glyphless rows.
//! - Not here (TRACKER 797): Copy resume command and Reload agent, the
//!   terminal and file rows' hints, and one builder shared with the wide
//!   tab strip's context menu.

use super::super::menu::{Action, close_item};
use super::super::with_layout;
use crate::areas::conversation::{MenuAction, session_menu_item, session_menu_rows};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::LeadingSize;
use vessel_app_kit::MenuItem;
use vessel_app_view::layout::{CloseScope, TabId, Target};
use vessel_app_view::session::MarkAction;

/// Paseo's agent id hint: its first seven characters.
const ID_HINT: usize = 7;

/// `item` led by `glyph` at Paseo's 16.
fn led(item: MenuItem, glyph: Glyph) -> MenuItem {
    item.with_leading_at(glyph, LeadingSize::Regular)
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

/// A switcher row's menu: Paseo's items (its copy, Rename, the closes),
/// then the open session's own actions.
pub(in crate::areas::layout) fn row_actions(shell: &Shell, tab: TabId) -> Vec<(Action, MenuItem)> {
    let words = shell.words.layout();
    let Some(target) = with_layout(shell, |layout| {
        Ok(layout.tab(tab).map(|tab| tab.target().clone()))
    })
    .ok()
    .flatten() else {
        return Vec::new();
    };
    let open = shell.areas.session.open.as_ref().map(|open| open.session);
    let session_tab = matches!(&target, Target::Agent(id) if Some(*id) == open);
    let session_rows = if session_tab {
        session_menu_rows(shell)
    } else {
        Vec::new()
    };
    let mut items = Vec::new();
    match target {
        Target::Agent(id) => {
            let hint: String = id.to_string().chars().take(ID_HINT).collect();
            let item = led(MenuItem::new(words.copy_agent_id().to_owned()), Glyph::Copy)
                .with_value(hint);
            let action = if session_tab {
                Action::Session(MenuAction::CopyId)
            } else {
                Action::Copy
            };
            items.push((action, item));
        }
        Target::Terminal(_) => items.push((
            Action::Copy,
            led(MenuItem::new(words.copy_id().to_owned()), Glyph::Copy),
        )),
        Target::File { .. } => items.push((
            Action::Copy,
            led(MenuItem::new(words.copy_path().to_owned()), Glyph::Copy),
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
    let renamable = shell
        .areas
        .conversation
        .info
        .as_deref()
        .is_some_and(vessel_app_view::session::can_rename);
    if session_tab && renamable {
        items.push((
            Action::SessionRename,
            led(MenuItem::new(words.rename().to_owned()), Glyph::Pencil),
        ));
    }
    // Paseo's separator, then the closes: above, below, the others, this.
    let closes = items.len();
    for (scope, label) in [
        (CloseScope::Before, words.close_above()),
        (CloseScope::After, words.close_below()),
        (CloseScope::Others, words.close_others()),
    ] {
        let (action, item) = close_item(shell, tab, scope, label);
        items.push((action, led(item, close_glyph(scope))));
    }
    if session_tab {
        // The agent's own Close follows the session's lifecycle policy (a
        // root archives first), as the bar's Close did.
        if session_rows.contains(&MenuAction::Close) {
            let mut item = session_menu_item(shell, MenuAction::Close);
            item.label = words.close().to_owned().into();
            items.push((Action::Session(MenuAction::Close), led(item, Glyph::X)));
        }
    } else {
        let (action, item) = close_item(shell, tab, CloseScope::Current, words.close());
        items.push((action, led(item, close_glyph(CloseScope::Current))));
    }
    group_from(&mut items, closes);
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
