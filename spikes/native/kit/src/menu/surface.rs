//! The menu's two surfaces: Paseo's popover at its trigger, and, opted into
//! per menu, a phone's sheet (TRACKER 791; Paseo's `MenuRoot`
//! `compactMode="sheet"` with its `sheetTitle`).
//!
//! [Decision Log]
//! - Purpose: the menus Paseo shows as sheets on a phone (a workspace row's,
//!   the sidebar's Display) come up from the bottom here too, and a page they
//!   open replaces them in place with a back header.
//! - Constraint: one renderer owns keys, leases and dismissal for both; the
//!   window's size is read where the sheet is laid out (a nested deferred
//!   element still takes its parent's layout, as `ModalBoundary` notes).
//! - Choice: below the compact width a menu with a sheet title is a
//!   full-width panel on `surface0` with its top corners at the sheet radius,
//!   the kit's drag handle (Paseo's `SheetHandle`), its title and 44-point
//!   rows, over the 45% scrim that closes it;
//!   while one of its pages is open the menu draws nothing and the screen
//!   draws the page as its own sheet with a back header (the list's sheet),
//!   whose Back returns here.
//! - Cost: a page's sheet is the screen's; the kit draws the root sheet and
//!   the flyouts' wide case.

use super::draw::MAX_WIDTH;
use super::{MenuState, Width};
use crate::dialog::sheet_handle;
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::sheet::snap::host::SheetState;
use crate::sheet::snap::listener::OnRelease;
use crate::sheet::snap::{Origin, Release, Stop};
use crate::submenu::Submenus;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, MouseButton, MouseDownEvent, SharedString, Stateful, Window,
    anchored, div, point, px,
};
use std::rc::Rc;

/// Paseo's popover: content-sized or a caller's width, on `surface1` edged
/// with `borderAccent` under the medium shadow.
pub(super) fn popover(
    panel: Stateful<Div>,
    width: Width,
    surfaces: &Surfaces,
    tokens: &Tokens,
) -> Stateful<Div> {
    panel
        .min_w(width.minimum())
        .when(width.fixed().is_none(), |panel| panel.max_w(MAX_WIDTH))
        .when_some(width.fixed(), Styled::w)
        .bg(surfaces.surface1)
        .border_1()
        .border_color(surfaces.border_accent)
        .rounded(px(popup::MENU_RADIUS))
        .overflow_hidden()
        .shadow(popup::shadow_md(tokens))
}

/// A phone's sheet: the window's width, `surface0`, its top corners
/// rounded, its title first.
pub(super) fn sheet(panel: Stateful<Div>, title: SharedString, tokens: &Tokens) -> Stateful<Div> {
    let surfaces = Surfaces::of(tokens);
    panel
        .w_full()
        .max_h(gpui::relative(0.8))
        .bg(surfaces.surface0)
        .border_1()
        .border_color(tokens.neutral.border)
        .rounded_t(px(popup::SHEET_RADIUS))
        .pb(px(spacing::S4))
        .overflow_hidden()
        // Paseo's `SheetHandle` (menu-surface.tsx), the kit's sheets' one.
        .child(sheet_handle(tokens.neutral.foreground_extra_muted))
        .child(
            // The list's sheet title row (`picker::rows::title_row`), so the
            // menu and the page that replaces it share one header.
            div()
                .flex()
                .items_center()
                .px(px(12.0))
                .py(px(10.0))
                .border_b_1()
                .border_color(tokens.neutral.border)
                .type_(Type::Label)
                .text_color(surfaces.foreground)
                .child(div().min_w_0().truncate().child(title)),
        )
}

/// Whether a phone's sheet has a page open: the page replaces it in place,
/// drawn by the screen as its own sheet, so the menu draws nothing.
pub(super) fn replaced(sheet: Option<&SharedString>, submenus: Option<&Submenus>) -> bool {
    sheet.is_some() && submenus.is_some_and(|pages| !pages.path().pages().is_empty())
}

/// A press on the dimmed page closes the menu (a stale frame's is kept).
pub(super) fn scrim<V: 'static>(
    state: &MenuState,
    get: fn(&mut V) -> &mut MenuState,
    cx: &mut Context<'_, V>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    let lease = state.render_lease();
    cx.listener(move |view: &mut V, _event: &MouseDownEvent, window, cx| {
        if get(view).accepts(&lease) {
            get(view).close_and_restore(window, cx);
            cx.notify();
        } else {
            window.prevent_default();
        }
        cx.stop_propagation();
    })
}

/// A press on the scrim.
pub(super) type ScrimPress = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// A pan down the sheet that ends past its stop closes the menu, as a
/// press on the scrim does (Paseo's `enablePanDownToClose`).
pub(super) fn released<V: 'static>(
    state: &MenuState,
    get: fn(&mut V) -> &mut MenuState,
    cx: &mut Context<'_, V>,
) -> OnRelease {
    let lease = state.render_lease();
    Rc::new(cx.listener(move |view: &mut V, release: &Release, window, cx| {
        if *release == Release::Pan(Stop::Closed) && get(view).accepts(&lease) {
            get(view).close_and_restore(window, cx);
            cx.notify();
        }
    }))
}

/// The sheet at the window's bottom over the scrim, laid out where the
/// window's size is known; its drag (the kit's one, `sheet::snap::host`,
/// content-sized) moves it down and fades the scrim with it. A menu has no
/// text field, so the whole panel takes the pan.
#[derive(gpui::IntoElement)]
pub(super) struct SheetBoundary {
    pub(super) panel: AnyElement,
    pub(super) on_scrim: ScrimPress,
    pub(super) sheet: Rc<SheetState>,
    pub(super) on_release: OnRelease,
}

impl RenderOnce for SheetBoundary {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = window.viewport_size();
        let frame = self.sheet.frame_content(window, cx);
        let mut scrim = popup::sheet_scrim();
        scrim.a = scrim.a.mul_add(self.sheet.backdrop(), 0.0);
        let panel = div()
            .relative()
            .w_full()
            .top(self.sheet.offset())
            .child(self.panel)
            .child(self.sheet.panel_probe())
            .child(frame.listener(Origin::Content, &self.on_release));
        anchored().position(point(px(0.0), px(0.0))).child(
            div()
                .w(size.width)
                .h(size.height)
                .flex()
                .flex_col()
                .justify_end()
                .bg(scrim)
                .on_mouse_down(MouseButton::Left, self.on_scrim)
                .child(panel),
        )
    }
}

// A nested deferred element still participates in its parent's layout. Read
// the actual Window here: size_full inside a dialog would cover only its body,
// leaving the scrim and sibling actions reachable by the same outside press.
#[derive(gpui::IntoElement)]
pub(super) struct ModalBoundary {
    pub(super) surface: AnyElement,
}
impl RenderOnce for ModalBoundary {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let size = window.viewport_size();
        anchored().position(point(px(0.0), px(0.0))).child(
            popup::layer(div())
                .w(size.width)
                .h(size.height)
                .child(self.surface),
        )
    }
}
