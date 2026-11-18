//! The workspace "…" and its menu, "Workspace actions", at both widths.
//!
//! [Decision Log]
//! - Purpose: TRACKER 795. Paseo has a workspace-level "…" on a wide window
//!   too (`WorkspaceHeaderMenuDesktop`), with the workspace's own actions
//!   only, since tabs are made from the tab strip's "+". Vessel had it only
//!   in the compact header (TRACKER 789).
//! - Reference: Paseo renders the same `WorkspaceHeaderWorkspaceActionItems`
//!   in its desktop and phone menus, "from the same callbacks, so the two
//!   surfaces can't drift" (workspace-header-menu.tsx). Desktop: the trigger
//!   is the large chrome (26 square, radius md; 32 on a phone) with a 16
//!   extra-muted ellipsis, right after the header's title and project row,
//!   8 apart (workspace-screen.tsx 488-530); the menu opens aligned to the
//!   trigger's start, 220 wide.
//! - Choice: one menu state, one item builder and one trigger for both
//!   places. Wide: after the session bar's title group, the row that plays
//!   Paseo's title bar (coordinator, 2026-11-18). Compact: in the header's
//!   island, large and round (789). The tooltip and the accessible name are
//!   "Workspace actions".
//! - Items: Paseo's that Vessel performs, in Paseo's order: Copy workspace
//!   path, then Import session (777's sheet for this host and folder). Left
//!   out, as in 789: New browser tab and the terminal profiles (Paseo leaves
//!   them out where it cannot create them; the desktop menu has neither),
//!   Copy branch name (Vessel knows only the branch a session started on),
//!   Show setup (no page).
//! - The wide bar had a second "…" for the session's own actions until
//!   TRACKER 800 removed it; they are the tab menu's (TRACKER 797).

use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::{Content, LeadingSize, menu_with};
use vessel_app_kit::toolbar_button::{
    Disclosure, ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk,
};
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{
    AnyElement, App, Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonState, ControlSizing, MenuHandlers, MenuItem, MenuState, Tokens, anchor_probe,
};

#[cfg(test)]
mod tests;

/// Paseo's workspace menu `width`.
const WIDTH: f32 = 220.0;

/// The menu and where its trigger was drawn (one trigger shows at a time).
pub(super) struct WorkspaceMenu {
    pub(super) menu: MenuState,
    place: Place,
}

impl WorkspaceMenu {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            place: Rc::new(std::cell::Cell::new(None)),
        }
    }
}

/// Which header the trigger is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Header {
    /// The session bar on a wide window: the large square chrome.
    Bar,
    /// The compact header's island: large and round.
    Island,
}

/// The "…" trigger.
pub(crate) fn trigger(
    shell: &Shell,
    header: Header,
    sizing: ControlSizing,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.layout();
    let area = &shell.areas.layout.workspace_menu;
    let config =
        ToolbarConfig::new(sizing, ButtonState::Enabled).disclosure(if area.menu.is_open() {
            Disclosure::Open
        } else {
            Disclosure::Closed
        });
    let config = match header {
        Header::Bar => config.large(),
        Header::Island => config.header_island(),
    };
    let button = ToolbarButton::new(
        "workspace-actions",
        words.workspace_actions().to_owned(),
        ToolbarIcon::new(Glyph::More, ToolbarInk::ExtraMuted),
        config,
        tokens,
    )
    .tooltip(&shell.areas.layout.tips, &area.place)
    .on_click(cx.listener(
        |shell, event: &vessel_app_kit::ui::ClickEvent, window, cx| {
            shell.open_workspace_menu(event.position(), window, cx);
        },
    ));
    let trigger = div()
        .relative()
        .flex_none()
        .child(button)
        .child(anchor_probe(&area.place));
    #[cfg(test)]
    let trigger = vessel_app_kit::ui::InteractiveElement::debug_selector(trigger, || {
        "workspace-actions-button".to_owned()
    });
    trigger.into_any_element()
}

/// The menu, when open.
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.layout.workspace_menu;
    if !area.menu.is_open() {
        return None;
    }
    let items = items(shell);
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.layout.workspace_menu.menu,
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
    // On a phone, Paseo's sheet titled "Workspace actions" (TRACKER 798,
    // workspace-header-menu.tsx `compactMode="sheet"`, `sheetTitle`).
    let title = shell.words.layout().workspace_actions().to_owned();
    menu_with(
        Content::new("workspace-actions-menu", &items)
            .with_width(px(WIDTH))
            .with_sheet(title),
        &area.menu,
        &handlers,
        tokens,
        vessel_app_kit::breakpoint::phone(window),
        cx,
    )
}

/// The shared items, each led by its 16 glyph, in Paseo's order.
pub(super) fn items(shell: &Shell) -> [MenuItem; 2] {
    let mut copy = MenuItem::new(shell.words.layout().copy_workspace_path().to_owned())
        .with_leading_at(Glyph::Copy, LeadingSize::Regular);
    copy.disabled = workspace_path(shell).is_none();
    [
        copy,
        MenuItem::new(shell.words.history().title().to_owned())
            .with_leading_at(Glyph::Import, LeadingSize::Regular),
    ]
}

/// The open session's folder, once the node described it.
pub(super) fn workspace_path(shell: &Shell) -> Option<String> {
    shell
        .areas
        .conversation
        .info
        .as_deref()
        .and_then(|info| info.spec.cwd.as_ref())
        .map(|cwd| cwd.display().to_string())
}

impl Shell {
    /// "…": Workspace actions, under it (Paseo's `align="start"`).
    fn open_workspace_menu(
        &mut self,
        at: vessel_app_kit::ui::Point<vessel_app_kit::ui::Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.layout.workspace_menu;
        match area.place.get() {
            Some(bounds) => area.menu.open_start(bounds, window, cx),
            None => area.menu.open(at, window, cx),
        }
        cx.notify();
    }
}
