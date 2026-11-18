//! The tab menu from the keyboard (TRACKER 800): Shift+F10 or the Menu key
//! opens the focused tab's menu under it.
//!
//! [Decision Log]
//! - Purpose: the bar's session "…" went (Paseo's header has no second
//!   "…"), so the session's actions live only in the tab menu; the
//!   coordinator asked that the menu open from the keyboard first.
//! - Reference: Paseo's tabs are web elements; a browser opens a focused
//!   element's context menu on Shift+F10 or the Menu key, and Paseo's
//!   `ContextMenu` takes that like a right press.
//! - Choice: those two platform keys, outside the shortcut registry (they
//!   are the platform's context-menu keys, not the app's shortcuts, so they
//!   are neither listed nor rebound), open the menu of the focused pane's
//!   focused tab on a wide window, under the tab and aligned to its start
//!   (Paseo's `align="start"`), as a right press opens it at the pointer.
//!   Below the compact width the strip is not drawn; the switcher's row
//!   "…" covers it.
//! - Alternatives: a registry action (a new rebindable shortcut and a help
//!   row Paseo does not have); focusable tab elements (no tab is a focus
//!   stop in Vessel's strip yet).

use super::menu::Surface;
use super::{key, mode, with_layout};
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::layout::{LayoutMode, Pane};

/// Whether a key is the platform's context-menu key: Shift+F10, or the Menu
/// key alone.
pub(crate) fn is_context_menu_key(key: &str, shift: bool, other_modifier: bool) -> bool {
    !other_modifier && ((key == "f10" && shift) || (key == "menu" && !shift))
}

impl Shell {
    /// The focused tab's menu, under it, on a wide session page. Whether it
    /// opened (else the key goes on).
    pub(crate) fn layout_tab_menu_from_keyboard(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.route != Route::Session || mode(window) == LayoutMode::Compact {
            return false;
        }
        let Ok(workspace) = key(self) else {
            return false;
        };
        let focused = with_layout(self, |layout| {
            Ok(layout
                .root()
                .panes()
                .into_iter()
                .find(|pane| pane.id() == layout.focused_pane())
                .and_then(Pane::focused))
        })
        .ok()
        .flatten();
        let Some(tab) = focused else {
            return false;
        };
        let Some(bounds) = self.areas.layout.tab_place(tab).get() else {
            return false;
        };
        let area = &mut self.areas.layout;
        area.menu_surface = Surface::Strip;
        area.menu_tab = Some((workspace, tab));
        area.menu.open_start(bounds, window, cx);
        cx.notify();
        true
    }
}

#[cfg(test)]
mod tests;
