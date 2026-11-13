//! A modal decision over the current screen (DESIGN.md section 6.9; the
//! page's `ui/kit/dialog.tsx`): the archive's and the kill's confirms, a
//! delete, a move to another profile.
//!
//! [Decision Log]
//! - Purpose: one dialog for every screen, so the scrim, Escape and a
//!   click outside behave the same everywhere.
//! - Values (the reference's base dialog): centred, 512 pixels wide (640
//!   for `Wide`), the popup surface at radius 10, 16 pixels inside, a
//!   15-pixel semibold title, a close button at the top right, the
//!   footer's actions at the right (a quiet Cancel, then one primary);
//!   the scrim is `--overlay`, no blur.
//! - Alternatives: `gpui-component`'s dialog layer (opened through its
//!   root, styled by its theme, its own buttons): a screen would then
//!   name its types.
//! - Choice: the dialog is drawn by the screen that keeps it open, over
//!   everything, with a focus handle of the screen's, so Escape reaches it
//!   and nothing behind it takes a click. On a narrow window it takes the
//!   width less 16 pixels each side (ADR 0036: every screen from 360
//!   wide).
//! - Cost: focus is not trapped inside the dialog the way the page's is
//!   (GPUI has no inert subtree); what is behind it cannot be clicked,
//!   but a Tab could reach it. Focus goes to the dialog on open, not to
//!   its first field: the screen focuses a field itself when it has one.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, FocusHandle, KeyDownEvent, MouseButton, SharedString, Window,
    deferred, div, px, svg,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::popup;
use crate::theme::{Tokens, radius};
use crate::widgets::TypeExt as _;

/// How wide a dialog is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogSize {
    /// The reference's 512 pixels.
    Normal,
    /// 640 pixels, for a grid of cards.
    Wide,
}

/// A dialog's focus, kept by the screen that shows it.
#[derive(Debug)]
pub struct DialogState {
    focus: FocusHandle,
}

impl DialogState {
    /// A dialog's state.
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
        }
    }

    /// Take the keys (when the dialog opens).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }
}

/// What a dialog shows.
pub struct DialogContent {
    /// Its title.
    pub title: SharedString,
    /// What it says or asks.
    pub body: AnyElement,
    /// Its actions, right-aligned: a quiet Cancel, then one primary.
    pub footer: Vec<AnyElement>,
    /// How wide.
    pub size: DialogSize,
}

impl std::fmt::Debug for DialogContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogContent")
            .field("title", &self.title)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// The dialog, over everything: Escape, the scrim and the close button
/// call `close`. `narrow`: a phone-sized window.
pub fn dialog<V: 'static>(
    state: &DialogState,
    content: DialogContent,
    tokens: &Tokens,
    narrow: bool,
    close: fn(&mut V, &mut Window, &mut Context<'_, V>),
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let width = match (narrow, content.size) {
        (true, _) => None,
        (false, DialogSize::Normal) => Some(512.0),
        (false, DialogSize::Wide) => Some(640.0),
    };
    let header = header(content.title, tokens, narrow, close, cx);
    let footer = (!content.footer.is_empty()).then(|| {
        div()
            .flex()
            .flex_wrap()
            .justify_end()
            .gap(px(8.0))
            .children(content.footer)
    });
    let card = div()
        .id("dialog")
        .track_focus(&state.focus)
        .on_key_down(
            cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    close(view, window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }),
        )
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .flex()
        .flex_col()
        .gap(px(12.0))
        .p(px(16.0))
        .when_some(width, |card, width| card.w(px(width)))
        .when(narrow, |card| card.mx(px(16.0)).w_full())
        .max_h_full()
        .bg(popup::fill(tokens))
        .border_1()
        .border_color(popup::edge(tokens))
        .rounded(px(radius::R4))
        .shadow(popup::shadow(tokens))
        .child(header)
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.text_secondary)
                .child(content.body),
        )
        .children(footer);
    deferred(
        div()
            .id("dialog-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(popup::scrim(tokens))
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view: &mut V, _event, window, cx| {
                    close(view, window, cx);
                    cx.notify();
                }),
            )
            .child(card),
    )
    .with_priority(2)
    .into_any_element()
}

/// The title and the close button.
fn header<V: 'static>(
    title: SharedString,
    tokens: &Tokens,
    narrow: bool,
    close: fn(&mut V, &mut Window, &mut Context<'_, V>),
    cx: &mut Context<'_, V>,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .type_(Type::Title)
                .text_color(tokens.text_primary)
                .child(title),
        )
        .child(
            div()
                .id("dialog-close")
                .size(px(if narrow { 44.0 } else { 28.0 }))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(radius::R2))
                .cursor_pointer()
                .hover(|style| style.bg(tokens.bg_hover))
                .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                    close(view, window, cx);
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(Glyph::Close.path())
                        .size(px(10.0))
                        .text_color(tokens.text_secondary),
                ),
        )
}
