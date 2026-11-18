//! The compact workspace header (TRACKER 789): below the compact width one
//! header row replaces both the pane's tab head and the session bar.
//!
//! [Decision Log]
//! - Purpose: Paseo draws no tab strip on a phone. Its workspace header row
//!   holds the title, a line under it, and an island of actions: the tab
//!   switcher, "…" (Workspace actions) and the explorer toggle. At 390,
//!   Vessel drew two rows instead: the tab head (tabs, "+") and the session
//!   bar (title, state, where, the code panel toggle, the session's "…").
//! - Reference: pinned Paseo 0.15.4. `CompactWorkspaceHeader` and
//!   `CompactHeader` (a 56 row, 12 across, 8 between, the title 14 medium
//!   over its subtitle, the actions in an island). `MobileWorkspaceTabSwitcher`
//!   (the open-tab count; a sheet "Workspace tabs"; rows with the current
//!   one checked and a trailing "…"; a footer with New agent and New
//!   terminal). `WorkspaceHeaderMenuMobile` ("Workspace actions").
//! - Rulings (coordinator, 2026-11-17): compact only, the wide "…" is
//!   TRACKER 795. The session's actions move to each switcher row's menu,
//!   after Paseo's tab items. No second "…" in the header. Import session
//!   joins "…" once 777 is in a package base (base 14.32 has it).
//! - Alternatives: keep the tab head at compact and drop only the bar (not
//!   Paseo's row); keep the session's "…" beside the workspace's (two "…").
//! - Choice: the kit's compact header frame (`screen_header::frame`, now
//!   with a subtitle) draws the session's heading (`conversation::heading`)
//!   and one island: the switcher trigger (`ToolbarButton::count`), "…" and
//!   the code panel toggle, all in the toolbar's touch chrome. The
//!   switcher is the kit dialog's sheet; each row's "…" opens the tab menu
//!   (`menu`, surface `Switcher`) over it.
//! - Cost and differences: Vessel's titlebar keeps the sidebar menu, so the
//!   header has no menu island of its own. Paseo's menus are sheets on a
//!   phone; the kit's are popovers with touch rows, as the bar's was. The
//!   dialog's sheet keeps its close button, which Paseo's combobox sheet
//!   has none of. The header's glyphs are the toolbar's 14, not Paseo's 16.

mod switcher;
#[cfg(test)]
mod tests;

use super::menu::{Action, close_item};
use super::{key, with_layout};
use crate::areas::conversation::{MenuAction, heading, session_menu_item, session_menu_rows};
use crate::shell::Shell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, menu_with};
use vessel_app_kit::screen_header::{Content as HeaderContent, Density, frame};
use vessel_app_kit::toolbar_button::{
    Disclosure, Selection, ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk,
};
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, IntoElement as _, ParentElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{
    ButtonState, ControlSizing, DialogState, MenuHandlers, MenuItem, MenuState, Tokens,
    anchor_probe,
};
use vessel_app_view::layout::{CloseScope, Layout, Pane, PaneId, TabId, Target};

/// Paseo's workspace menu `width`.
const ACTIONS_WIDTH: f32 = 220.0;

/// The header's state, kept by the layout area.
pub(super) struct Compact {
    /// The tab switcher's sheet is open.
    switcher_open: bool,
    switcher: DialogState,
    /// Where each row's "…" was drawn: its menu opens under it.
    row_places: RefCell<BTreeMap<TabId, Place>>,
    /// Where "…" was drawn, and its menu.
    actions_place: Place,
    actions: MenuState,
}

impl Compact {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            switcher_open: false,
            switcher: DialogState::new(cx),
            row_places: RefCell::new(BTreeMap::new()),
            actions_place: Rc::new(std::cell::Cell::new(None)),
            actions: MenuState::new(cx),
        }
    }

    fn row_place(&self, tab: TabId) -> Place {
        Rc::clone(
            self.row_places
                .borrow_mut()
                .entry(tab)
                .or_insert_with(|| Rc::new(std::cell::Cell::new(None))),
        )
    }
}

/// The workspace's tabs in the switcher's order: pane by pane, each pane's
/// tabs in its strip's order (Paseo lists every tab of the workspace).
fn tabs(layout: &Layout) -> Vec<TabId> {
    layout
        .root()
        .panes()
        .into_iter()
        .filter(|pane| pane.id() != PaneId::EXPLORER)
        .flat_map(|pane| pane.tabs().to_vec())
        .collect()
}

/// The tab the compact layout shows: the focused pane's.
fn current(layout: &Layout) -> Option<TabId> {
    layout
        .root()
        .panes()
        .into_iter()
        .find(|pane| pane.id() == layout.focused_pane())
        .and_then(Pane::focused)
}

/// The header row, in place of the tab head and the session bar.
pub(super) fn header(
    shell: &Shell,
    layout: &Layout,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.layout();
    let area = &shell.areas.layout.compact;
    let count = tabs(layout).len();
    let config = ToolbarConfig::new(ControlSizing::Touch, ButtonState::Enabled);
    let switch_label = words
        .switch_tabs(&count.to_string())
        .unwrap_or_else(|_| words.workspace_tabs().to_owned());
    let switch = ToolbarButton::count(
        "compact-tab-switcher",
        switch_label,
        count,
        config.disclosure(if area.switcher_open {
            Disclosure::Open
        } else {
            Disclosure::Closed
        }),
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.compact_open_switcher(window, cx);
    }));
    let more = ToolbarButton::new(
        "compact-workspace-actions",
        words.workspace_actions().to_owned(),
        ToolbarIcon::new(Glyph::More, ToolbarInk::Muted),
        config.disclosure(if area.actions.is_open() {
            Disclosure::Open
        } else {
            Disclosure::Closed
        }),
        tokens,
    )
    .on_click(cx.listener(
        |shell, event: &vessel_app_kit::ui::ClickEvent, window, cx| {
            shell.compact_open_actions(event.position(), window, cx);
        },
    ));
    let explorer_open = shell.areas.workspace.code.open;
    let toggle = ToolbarButton::new(
        "compact-explorer-toggle",
        if explorer_open {
            words.explorer_close()
        } else {
            words.explorer_open()
        }
        .to_owned(),
        ToolbarIcon::new(Glyph::PanelRight, ToolbarInk::Muted),
        config.selected(if explorer_open {
            Selection::Selected
        } else {
            Selection::Unselected
        }),
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| shell.toggle_code(window, cx)));
    let more = div()
        .relative()
        .flex_none()
        .child(more)
        .child(anchor_probe(&area.actions_place));
    #[cfg(test)]
    let more = vessel_app_kit::ui::InteractiveElement::debug_selector(more, || {
        "compact-workspace-actions-button".to_owned()
    });
    let island = div()
        .flex()
        .flex_row()
        .items_center()
        .flex_none()
        .child(switch)
        .child(more)
        .child(toggle);
    let (title, subtitle, editing) = match heading(shell, tokens, cx) {
        // A rename's touch field is as tall as the title and its line
        // together; the line comes back when the rename ends.
        Some(session) if session.editing.is_some() => (session.title, None, session.editing),
        Some(session) => (session.title, Some(session.line), None),
        None => (String::new(), None, None),
    };
    let row = frame(
        HeaderContent {
            title: title.into(),
            leading: None,
            actions: Some(island.into_any_element()),
            subtitle,
            editing,
        },
        Density::Compact,
        tokens,
    );
    #[cfg(test)]
    let row = vessel_app_kit::ui::InteractiveElement::debug_selector(row, || {
        "compact-workspace-header".to_owned()
    });
    row
}

/// The switcher's sheet and the Workspace actions menu, when open.
pub(super) fn overlay(
    shell: &Shell,
    layout: &Layout,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let sheet = switcher::sheet(shell, layout, tokens, window, cx);
    let actions = actions_menu(shell, tokens, window, cx);
    (sheet.is_some() || actions.is_some())
        .then(|| div().children(sheet).children(actions).into_any_element())
}

/// What "…" offers: Paseo's workspace actions that Vessel performs (Copy
/// workspace path, Import session). New browser tab and the terminal
/// profiles are left out, as Paseo leaves them out where it cannot create
/// them; Copy branch name too, since Vessel knows only the branch a session
/// started on, and Show setup, which Vessel has no page for.
fn actions_menu(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.layout.compact;
    if !area.actions.is_open() {
        return None;
    }
    let words = shell.words.layout();
    let mut copy = MenuItem::new(words.copy_workspace_path().to_owned());
    copy.disabled = workspace_path(shell).is_none();
    // Paseo's order: Copy workspace path, then Import session.
    let items = [
        copy,
        MenuItem::new(shell.words.history().title().to_owned()),
    ];
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.layout.compact.actions,
        chose: Rc::new(|shell: &mut Shell, index, window, cx| {
            if index == 0 {
                if let Some(path) = workspace_path(shell) {
                    cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(path));
                }
            } else {
                shell.open_import_for_workspace(window, cx);
            }
        }),
    };
    menu_with(
        Content::new("compact-workspace-actions-menu", &items).with_width(px(ACTIONS_WIDTH)),
        &area.actions,
        &handlers,
        tokens,
        vessel_app_kit::breakpoint::phone(window),
        cx,
    )
}

/// The open session's folder, once the node described it.
fn workspace_path(shell: &Shell) -> Option<String> {
    shell
        .areas
        .conversation
        .info
        .as_deref()
        .and_then(|info| info.spec.cwd.as_ref())
        .map(|cwd| cwd.display().to_string())
}

/// A switcher row's menu: Paseo's tab items (its copy, Rename, the closes),
/// then, on the open session's agent tab, the session's own actions that
/// Paseo's tab menu lacks (its marks, Copy link, Agent log, Kill).
pub(super) fn row_actions(shell: &Shell, tab: TabId) -> Vec<(Action, MenuItem)> {
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
        Target::Agent(_) if session_tab => items.push((
            Action::Session(MenuAction::CopyId),
            MenuItem::new(words.copy_agent_id().to_owned()),
        )),
        Target::Agent(_) => items.push((
            Action::Copy,
            MenuItem::new(words.copy_agent_id().to_owned()),
        )),
        Target::Terminal(_) => {
            items.push((Action::Copy, MenuItem::new(words.copy_id().to_owned())));
        }
        Target::File { .. } => {
            items.push((Action::Copy, MenuItem::new(words.copy_path().to_owned())));
        }
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
            MenuItem::new(words.rename().to_owned()),
        ));
    }
    // Paseo's separator, then the closes: above, below, the others, this.
    let closes = items.len();
    for (scope, label) in [
        (CloseScope::Before, words.close_above()),
        (CloseScope::After, words.close_below()),
        (CloseScope::Others, words.close_others()),
    ] {
        items.push(close_item(shell, tab, scope, label));
    }
    if session_tab {
        // The agent's own Close follows the session's lifecycle policy (a
        // root archives first), as the bar's Close did.
        if session_rows.contains(&MenuAction::Close) {
            let mut item = session_menu_item(shell, MenuAction::Close);
            item.label = words.close().to_owned().into();
            items.push((Action::Session(MenuAction::Close), item));
        }
    } else {
        items.push(close_item(shell, tab, CloseScope::Current, words.close()));
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
                (Action::Session(action), item)
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

impl Shell {
    fn compact_open_switcher(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.layout.compact;
        area.switcher_open = true;
        area.switcher.focus(window, cx);
        cx.notify();
    }

    /// Close the switcher's sheet, its focus back where it was.
    pub(super) fn compact_close_switcher(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.layout.compact;
        if area.switcher_open {
            area.switcher_open = false;
            area.switcher.dismissed(window, cx);
            cx.notify();
        }
    }

    /// A row's press: show that tab and close the sheet (Paseo's
    /// `onSelectSwitcherTab`).
    fn compact_select(&mut self, tab: TabId, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.layout_select(tab, cx);
        self.compact_close_switcher(window, cx);
    }

    /// A row's "…": the tab menu, under it, over the sheet.
    fn compact_row_menu(&mut self, tab: TabId, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Ok(workspace) = key(self) else {
            return;
        };
        // Not drawn yet: nothing to open under.
        let Some(bounds) = self.areas.layout.compact.row_place(tab).get() else {
            return;
        };
        let area = &mut self.areas.layout;
        area.menu_surface = super::menu::Surface::Switcher;
        area.menu_tab = Some((workspace, tab));
        area.menu.open_end(bounds, window, cx);
        cx.notify();
    }

    /// "…": Workspace actions, under it (Paseo's `align="start"`).
    fn compact_open_actions(
        &mut self,
        at: vessel_app_kit::ui::Point<vessel_app_kit::ui::Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.layout.compact;
        match area.actions_place.get() {
            Some(bounds) => area.actions.open_start(bounds, window, cx),
            None => area.actions.open(at, window, cx),
        }
        cx.notify();
    }
}
