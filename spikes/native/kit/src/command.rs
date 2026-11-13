//! The command palette's pieces, as Paseo's command center
//! (`command-center/command-center.tsx`, its `list-metrics.ts`, and
//! `components/ui/shortcut.tsx` for a key's badge): a panel at the top of
//! the window over a dimmed page, its search in a header over a rule, then
//! its groups, each under a muted heading and after a rule, of rows with a
//! glyph, words and the row's keys.
//!
//! [Decision Log]
//! - Purpose: the page's palette (⌘K) in Paseo's look, cloned closely.
//! - Choice: the kit draws the frame, a group's heading, a row, a key's
//!   badges and the empty line; the screen keeps what is listed, which
//!   row is active and what choosing one does. Paseo's values: the panel
//!   640 by 560, at most 92% by 80% of the window, 48 under its top, a
//!   hairline of `border`, 8 round, `surface0`, the large shadow, over
//!   black at 50%; the header 16 across and 12 down, the search 14 with 4
//!   above and below; a group's heading 12 on 16, `foregroundMuted`, 16
//!   across, 8 above and below, after a rule with 8 on each side but the
//!   first; a row 16 across, 6 above and below (12 on a phone), its glyph
//!   16 wide on a 20 line, 12 from its words, the title 14 (on 18 over a
//!   second line, else on 20) in the foreground, the second line 12 on 16
//!   muted, the active row `surface1`; a key's badge 4 by 2 inside, 6
//!   round, `surface3` at half, 12 muted, one per chord 4 apart; the
//!   search's mark `surface3` behind the foreground; the empty line 14
//!   muted, centred, 16 by 24 inside. On a phone it is a sheet from the
//!   bottom over black at 45%, with its handle.
//! - Differences: no fade (Paseo's fades in); the phone's sheet starts at
//!   Paseo's 60% snap but does not drag to its 90% snap.

mod marks;
mod row;

pub use row::{CommandMatches, CommandRow, command_row, command_row_matched, key_badges};

use gpui::prelude::*;
use gpui::{
    AnyElement, BoxShadow, Context, Div, MouseButton, SharedString, Window, anchored, deferred,
    div, point, px, relative, rgba,
};
use gpui_base::input::{Enter, Escape, MoveDown, MoveUp};

use crate::fonts::{self, Type};
use crate::popup;
use crate::theme::{Scheme, Tokens, spacing};
use crate::widgets::TypeExt as _;

/// Paseo's panel width.
pub const WIDTH: f32 = 640.0;

/// Paseo's panel height.
const HEIGHT: f32 = 560.0;

/// Between the window's top and the panel (Paseo's `spacing[12]`).
const TOP: f32 = spacing::S12;

/// Around a group's heading and its rule (`COMMAND_CENTER_SECTION_GAP`).
const GAP: f32 = spacing::S2;

/// A row's and a heading's inset across.
const ACROSS: f32 = spacing::S4;

/// Paseo's `theme.shadow.lg`.
#[must_use]
pub fn shadow_lg(tokens: &Tokens) -> Vec<BoxShadow> {
    let (alpha, offset) = match tokens.scheme {
        Scheme::Light => (0x14, 8.0),
        Scheme::Dark => (0x66, 12.0),
    };
    vec![BoxShadow {
        color: rgba(alpha).into(),
        offset: point(px(0.0), px(offset)),
        blur_radius: px(24.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

/// What the frame holds.
pub struct CommandFrame {
    /// The search field.
    pub search: AnyElement,
    /// The groups and rows, or the empty line.
    pub list: AnyElement,
    /// A phone's width: a sheet from the bottom.
    pub compact: bool,
}

impl std::fmt::Debug for CommandFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandFrame")
            .field("compact", &self.compact)
            .finish_non_exhaustive()
    }
}

/// A key the palette answers before its search does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKey {
    /// The row above.
    Up,
    /// The row below.
    Down,
    /// Run the active row.
    Enter,
    /// Close.
    Escape,
}

/// The palette over everything: `on_key` hears the arrows, Enter and
/// Escape before the search does (the search's own actions would take
/// them first as raw keys); a press on the dimmed page calls `close`.
pub fn command_frame<V: 'static>(
    frame: CommandFrame,
    on_key: fn(&mut V, CommandKey, &mut Window, &mut Context<'_, V>),
    close: fn(&mut V, &mut Window, &mut Context<'_, V>),
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let mut panel = div()
        .id("command")
        .capture_action(cx.listener(move |view: &mut V, _: &MoveUp, window, cx| {
            cx.stop_propagation();
            on_key(view, CommandKey::Up, window, cx);
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &MoveDown, window, cx| {
            cx.stop_propagation();
            on_key(view, CommandKey::Down, window, cx);
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &Enter, window, cx| {
            cx.stop_propagation();
            on_key(view, CommandKey::Enter, window, cx);
        }))
        .capture_action(cx.listener(move |view: &mut V, _: &Escape, window, cx| {
            cx.stop_propagation();
            on_key(view, CommandKey::Escape, window, cx);
        }))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(neutral.surface0)
        .type_(Type::Body)
        .text_color(neutral.foreground);
    panel = if frame.compact {
        panel
            .w_full()
            .h(relative(0.6))
            .rounded_t(px(popup::SHEET_RADIUS))
            .child(handle(neutral.sheet_handle))
    } else {
        panel
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .max_w(relative(0.92))
            .max_h(relative(0.8))
            .border_1()
            .border_color(neutral.border)
            .rounded(px(popup::MENU_RADIUS))
            .shadow(shadow_lg(tokens))
    };
    let header = div()
        .flex_none()
        .px(px(ACROSS))
        .py(px(spacing::S3))
        .border_b_1()
        .border_color(neutral.border)
        .child(div().py(px(spacing::S1)).child(frame.search));
    let panel = panel.child(header).child(
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(frame.list),
    );
    let size = window.viewport_size();
    let scrim = div()
        .id("command-scrim")
        .w(size.width)
        .h(size.height)
        .flex()
        .flex_col()
        .items_center()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view: &mut V, _event, window, cx| {
                close(view, window, cx);
                cx.notify();
            }),
        );
    let scrim = if frame.compact {
        scrim.justify_end().bg(popup::sheet_scrim())
    } else {
        scrim.justify_start().pt(px(TOP)).bg(rgba(0x0000_0080))
    };
    // Over a dialog (at 2), under a tooltip (at 4).
    deferred(
        anchored()
            .position(point(px(0.0), px(0.0)))
            .child(scrim.child(panel)),
    )
    .with_priority(3)
    .into_any_element()
}

/// A sheet's drag handle (as the kit's dialog draws it).
fn handle(color: gpui::Hsla) -> Div {
    div()
        .w_full()
        .flex_none()
        .flex()
        .justify_center()
        .pt(px(spacing::S2))
        .child(div().w(px(32.0)).h(px(4.0)).rounded(px(2.0)).bg(color))
}

/// A group's heading; after a rule unless it is the `first`.
#[must_use]
pub fn command_heading(label: impl Into<SharedString>, first: bool, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .when(!first, |heading| {
            heading.child(div().my(px(GAP)).h(px(1.0)).bg(neutral.border))
        })
        .child(
            div()
                .px(px(ACROSS))
                .pt(px(GAP))
                .pb(px(GAP))
                .type_(Type::Meta)
                .line_height(px(fonts::ui(16.0)))
                .text_color(neutral.foreground_muted)
                .child(label.into()),
        )
}

/// The line shown when nothing answers the search.
#[must_use]
pub fn command_empty(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .justify_center()
        .px(px(ACROSS))
        .py(px(spacing::S6))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.into())
}
