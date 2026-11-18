//! Menus use the same typed operations and close planning as keyboard commands.
//!
//! One tab menu serves two surfaces (TRACKER 797, `tab_menu::entries`): a
//! tab's context menu in the tab strip, and, below the compact width, the
//! "…" of a row in the tab switcher (TRACKER 789), which opens over the
//! switcher's sheet. Both are Paseo's 220 wide.
use super::with_layout;
use crate::areas::conversation::MenuAction;
use crate::shell::Shell;
use vessel_app_kit::menu::{Content, Layer, menu_with};
use vessel_app_kit::ui::{AnyElement, Context, Window, px};
use vessel_app_kit::{MenuHandlers, MenuItem, Tokens};
use vessel_app_view::layout::{CloseEffect, CloseScope, LayoutAction, TabId};

/// Paseo's tab menu `width` (`DROPDOWN_WIDTH`, workspace-desktop-tabs-row.tsx;
/// workspace-tab-trailing-accessory.tsx).
const WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    /// Put the tab's id or path on the clipboard.
    Copy(Copied),
    /// Put the agent's resume command on the clipboard.
    CopyResume,
    /// Rename the open session's agent.
    Rename,
    Close(CloseScope),
    /// A Session action (the bar's menu), on the open session's agent tab.
    Session(MenuAction),
}

/// What a copy row copies, for its toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Copied {
    AgentId,
    TerminalId,
    FilePath,
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
    let actions = super::tab_menu::entries(shell, tab, shell.areas.layout.menu_surface);
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
    // On a phone, Paseo's sheet titled with the tab's label (TRACKER 798,
    // workspace-tab-trailing-accessory.tsx `sheetTitle={presentationLabel}`).
    let title = with_layout(shell, |layout| {
        Ok(layout
            .tab(tab)
            .map(|tab| super::draw::tab_label(shell, tab.target())))
    })
    .ok()
    .flatten()
    .unwrap_or_default();
    let content = Content::new("layout-tab-menu", &items)
        .with_width(px(WIDTH))
        .with_sheet(title);
    let content = match shell.areas.layout.menu_surface {
        Surface::Strip => content,
        // Over the switcher's sheet (a dialog).
        Surface::Switcher => content.with_layer(Layer::AboveDialog),
    };
    menu_with(
        content,
        &shell.areas.layout.menu,
        &handlers,
        tokens,
        vessel_app_kit::breakpoint::phone(window),
        cx,
    )
}

pub(super) fn chose(
    shell: &mut Shell,
    tab: TabId,
    action: Action,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    match action {
        Action::Copy(copied) => shell.tab_menu_copy(tab, copied, cx),
        Action::CopyResume => shell.tab_menu_copy_resume(tab, cx),
        Action::Rename => {
            shell.compact_close_switcher(window, cx);
            // The one rename dialog (TRACKER 800), as the bar's title opens
            // it; it picks the workspace layout's route itself.
            shell.open_rename(window, cx);
        }
        Action::Close(scope) => {
            shell.layout_select(tab, cx);
            let _handled = shell.run_layout_action(LayoutAction::Close(scope), window, cx);
        }
        Action::Session(action) => {
            // Copies leave the switcher open; anything that opens a
            // confirm, a log or the page's next screen closes it first.
            if action != MenuAction::CopyLink {
                shell.compact_close_switcher(window, cx);
            }
            shell.menu_chose(action, window, cx);
        }
    }
}
