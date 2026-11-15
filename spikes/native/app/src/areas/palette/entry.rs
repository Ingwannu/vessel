//! Where the palette is opened from, besides its shortcut: the sidebar's
//! Search row and a search mark in the title bar.
//!
//! [Decision Log]
//! - Purpose: the palette is the one place to search (machines, sessions,
//!   commands, and every session's messages once track C's search lands),
//!   so it must be one press away on every surface.
//! - Reference: Paseo opens its command center from the sidebar's Search
//!   row (`components/sidebar/sidebar-nav-rows.tsx` `SidebarSearchRow`, the
//!   third of the header rows in `sidebar-nav/model.ts`: new, history,
//!   search, schedules). The sidebar has no search field of its own.
//! - Difference, named: the title bar also holds a search mark (the
//!   coordinator's brief). At a phone's width the sidebar is a sheet
//!   behind the menu button, so without it the palette would be two
//!   presses away on a phone and one on a desktop (ADR 0036: one
//!   experience). Paseo's compact header has no such button.
//! - The mark is drawn as the sidebar's toggle beside it is (32 by 28, the
//!   kit's second radius, the secondary text's colour) so the two read as
//!   one group; it fills while the palette is open, as the toggle fills
//!   while the sidebar is.

use vessel_app_kit::Tokens;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, StatefulInteractiveElement as _, Styled as _, div, px, svg,
};

use crate::shell::Shell;

/// The search mark in the title bar.
pub(crate) fn title_trigger(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .id("title-palette")
        .w(px(spacing::S8))
        .h(px(spacing::S7))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .when(shell.areas.palette.is_open(), |button| {
            button.bg(tokens.bg_active)
        })
        .hover(|button| button.bg(tokens.bg_hover))
        // Above the bar's drag area, so a press is the button's.
        .occlude()
        .cursor_pointer()
        .child(
            svg()
                .path(Glyph::Search.path())
                .size(px(spacing::S3_5))
                .text_color(tokens.text_secondary),
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.open_palette(window, cx)))
        .into_any_element()
}

/// The sidebar's Search row, after its routes (Paseo's header rows: new,
/// history, search, schedules); `touch`: drawn in the phone's sheet.
pub(crate) fn sidebar_row(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Stateful<Div> {
    crate::areas::sidebar::nav_row(
        "nav-search",
        shell.t.lang.palette().search,
        Some(Glyph::Search),
        shell.areas.palette.is_open(),
        touch,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| shell.open_palette(window, cx)))
}
