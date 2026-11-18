//! Menus use the same typed operations and close planning as keyboard commands.
//!
//! One tab menu serves two surfaces: a tab's context menu in the tab strip,
//! and, below the compact width, the "…" of a row in the tab switcher
//! (TRACKER 789, `compact::row_actions`), which also carries the session's
//! own actions and opens over the switcher's sheet.
use super::with_layout;
use crate::areas::conversation::MenuAction;
use crate::shell::Shell;
use vessel_app_kit::menu::{Content, Layer, menu_with};
use vessel_app_kit::ui::{AnyElement, Context, Window, px};
use vessel_app_kit::{MenuHandlers, MenuItem, Tokens};
use vessel_app_view::layout::{CloseEffect, CloseScope, LayoutAction, TabId, Target};

/// Paseo's tab menu `width` (workspace-tab-trailing-accessory.tsx).
const SWITCHER_WIDTH: f32 = 220.0;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Copy,
    Rename,
    Close(CloseScope),
    /// A Session action (the bar's menu), on the open session's agent tab.
    Session(MenuAction),
    /// The open session's rename, in the compact header's title.
    SessionRename,
}

/// Where the tab menu was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Surface {
    /// A tab's context menu in the tab strip.
    #[default]
    Strip,
    /// A row's "…" in the compact tab switcher.
    Switcher,
}

fn actions(shell: &Shell, tab: TabId) -> Vec<(Action, MenuItem)> {
    if shell.areas.layout.menu_surface == Surface::Switcher {
        return super::compact::row_actions(shell, tab);
    }
    let words = shell.words.layout();
    let target = with_layout(shell, |layout| {
        Ok(layout.tab(tab).map(|tab| tab.target().clone()))
    })
    .ok()
    .flatten();
    let Some(target) = target else {
        return Vec::new();
    };
    let mut items = Vec::new();
    if matches!(
        target,
        Target::Agent(_) | Target::Terminal(_) | Target::File { .. }
    ) {
        let label = if matches!(target, Target::File { .. }) {
            words.copy_path()
        } else {
            words.copy_id()
        };
        items.push((Action::Copy, MenuItem::new(label.to_owned())));
    }
    if matches!(target, Target::Agent(_))
        && matches!(
            super::key(shell),
            Ok(vessel_app_view::layout::LayoutKey::Workspace { .. })
        )
    {
        items.push((Action::Rename, MenuItem::new(words.rename().to_owned())));
    }
    let local = with_layout(shell, |layout| {
        layout
            .request_close(tab)
            .map(|plan| plan.effect() == CloseEffect::Local)
    })
    .unwrap_or(false);
    if local {
        for (scope, label) in [
            (CloseScope::Before, words.close_left()),
            (CloseScope::After, words.close_right()),
            (CloseScope::Others, words.close_others()),
            (CloseScope::Current, words.close()),
        ] {
            items.push(close_item(shell, tab, scope, label));
        }
    }
    items
}

/// A close row, disabled when it would close nothing or anything that is
/// not local (an agent archives, a terminal confirms).
pub(super) fn close_item(
    shell: &Shell,
    tab: TabId,
    scope: CloseScope,
    label: &str,
) -> (Action, MenuItem) {
    let disabled = with_layout(shell, |layout| {
        layout.close_plans(tab, scope).map(|plans| {
            plans.is_empty() || plans.iter().any(|plan| plan.effect() != CloseEffect::Local)
        })
    })
    .unwrap_or(true);
    let mut item = MenuItem::new(label.to_owned());
    item.disabled = disabled;
    (Action::Close(scope), item)
}
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let (workspace, tab) = shell.areas.layout.menu_tab.as_ref()?;
    if super::key(shell).as_ref() != Ok(workspace) {
        return None;
    }
    let workspace = workspace.clone();
    let tab = *tab;
    let actions = actions(shell, tab);
    let items: Vec<_> = actions.iter().map(|(_, item)| item.clone()).collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.layout.menu,
        chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
            if super::key(shell).as_ref() != Ok(&workspace) {
                return;
            }
            let Some((action, _)) = actions.get(index) else {
                return;
            };
            chose(shell, tab, *action, window, cx);
        }),
    };
    let narrow = vessel_app_kit::breakpoint::phone(window);
    match shell.areas.layout.menu_surface {
        Surface::Strip => vessel_app_kit::menu(
            "layout-tab-menu",
            &shell.areas.layout.menu,
            &items,
            &handlers,
            tokens,
            narrow,
            cx,
        ),
        // Over the switcher's sheet (a dialog), at Paseo's width.
        Surface::Switcher => menu_with(
            Content::new("layout-tab-menu", &items)
                .with_layer(Layer::AboveDialog)
                .with_width(px(SWITCHER_WIDTH)),
            &shell.areas.layout.menu,
            &handlers,
            tokens,
            narrow,
            cx,
        ),
    }
}

pub(super) fn chose(
    shell: &mut Shell,
    tab: TabId,
    action: Action,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    match action {
        Action::Rename => shell.layout_start_rename(tab, window, cx),
        Action::Copy => {
            let target = with_layout(shell, |layout| {
                Ok(layout.tab(tab).map(|tab| tab.target().clone()))
            })
            .ok()
            .flatten();
            if let Some(text) = target.as_ref().and_then(copy_text) {
                cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(text));
            }
        }
        Action::Close(scope) => {
            shell.layout_select(tab, cx);
            let _handled = shell.run_layout_action(LayoutAction::Close(scope), window, cx);
        }
        Action::Session(action) => {
            // Copies leave the switcher open; anything that opens a
            // confirm, a log or the page's next screen closes it first.
            if !matches!(action, MenuAction::CopyId | MenuAction::CopyLink) {
                shell.compact_close_switcher(window, cx);
            }
            shell.menu_chose(action, window, cx);
        }
        Action::SessionRename => {
            shell.compact_close_switcher(window, cx);
            shell.rename_from_header(window, cx);
        }
    }
}

/// What Copy puts on the clipboard for a tab.
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
