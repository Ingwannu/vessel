//! A modal decision over the current screen: the archive's and the
//! kill's confirms, a delete, a move to another profile. Paseo's adaptive
//! modal (`components/adaptive-modal-sheet.tsx`), cloned: a card on a
//! wide window, a sheet from the bottom on a phone.
//!
//! [Decision Log]
//! - Purpose: one dialog for every screen, so the scrim, Escape and a
//!   click outside behave the same everywhere, with Paseo's look (the
//!   owner's rule after build 0.0.0.0.3).
//! - Values (Paseo's): on a wide window a card at most 520 wide and 85%
//!   of the window tall, on `surface0`, edged with `surface2` at radius
//!   12, over black at 55% with 24 around; its header 24 by 16 with a
//!   rule under it, the title 14 medium, the close button at its right;
//!   the body 24 inside; the footer 24 by 12 with a rule over it. On a
//!   phone the same parts on a sheet as wide as the window, its top
//!   corners at 16, a handle on top, over black at 45%.
//! - Differences from the page's DESIGN.md (the references win): the
//!   page's dialog is 512 wide on the popup surface at radius 10 with no
//!   rules between its parts; the page has no sheet.
//! - Alternatives: `gpui-component`'s dialog layer (opened through its
//!   root, styled by its theme, its own buttons): a screen would then
//!   name its types.
//! - Choice: the dialog is drawn by the screen that keeps it open, over
//!   everything, with a focus handle of the screen's, so Escape reaches it
//!   and nothing behind it takes a click. On a narrow window it takes the
//!   width less 16 pixels each side (ADR 0036: every screen from 360
//!   wide).
//! - Cost: the sheet does not follow a finger down to close (a tap on
//!   the scrim or the close button does); focus is not trapped inside the
//!   dialog the way the page's is
//!   (GPUI has no inert subtree); what is behind it cannot be clicked,
//!   but a Tab could reach it. Focus goes to the dialog on open, not to
//!   its first field: the screen focuses a field itself when it has one.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, FocusHandle, KeyDownEvent, MouseButton, SharedString, Window,
    deferred, div, px, relative, svg,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The inset of a dialog's parts (Paseo's `spacing[6]`).
const INSIDE: f32 = 24.0;

/// A card's widest (Paseo's 520).
const CARD_WIDTH: f32 = 520.0;

/// A wide card's widest, for a grid of cards.
const WIDE_CARD_WIDTH: f32 = 640.0;

/// How wide a dialog is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogSize {
    /// Paseo's 520 points at most.
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
/// call `close`. `narrow`: a phone's width (the lead's breakpoint), where
/// it is a sheet from the bottom.
pub fn dialog<V: 'static>(
    state: &DialogState,
    content: DialogContent,
    tokens: &Tokens,
    narrow: bool,
    close: fn(&mut V, &mut Window, &mut Context<'_, V>),
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let surfaces = Surfaces::of(tokens);
    let width = match content.size {
        DialogSize::Normal => CARD_WIDTH,
        DialogSize::Wide => WIDE_CARD_WIDTH,
    };
    let footer = (!content.footer.is_empty()).then(|| {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap(px(8.0))
            .px(px(INSIDE))
            .py(px(12.0))
            .border_t_1()
            .border_color(surfaces.surface2)
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
        .w_full()
        .overflow_hidden()
        .bg(surfaces.surface0)
        .when(!narrow, |card| {
            card.max_w(px(width))
                .max_h(relative(0.85))
                .border_1()
                .border_color(surfaces.surface2)
                .rounded(px(popup::DIALOG_RADIUS))
        })
        .when(narrow, |sheet| {
            sheet
                .max_h(relative(0.9))
                .rounded_t(px(popup::SHEET_RADIUS))
                .child(handle())
        })
        .child(header(content.title, &surfaces, narrow, close, cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .p(px(INSIDE))
                .type_(Type::Body)
                .text_color(surfaces.foreground_muted)
                .child(content.body),
        )
        .children(footer);
    let scrim = div()
        .id("dialog-scrim")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .justify_center()
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view: &mut V, _event, window, cx| {
                close(view, window, cx);
                cx.notify();
            }),
        );
    let scrim = if narrow {
        scrim.items_end().bg(popup::sheet_scrim())
    } else {
        scrim.items_center().p(px(24.0)).bg(popup::scrim())
    };
    deferred(scrim.child(card))
        .with_priority(2)
        .into_any_element()
}

/// A sheet's drag handle (Gorhom's indicator, as Paseo themes it: zinc
/// 600).
fn handle() -> gpui::Div {
    div().w_full().flex().justify_center().pt(px(8.0)).child(
        div()
            .w(px(32.0))
            .h(px(4.0))
            .rounded(px(2.0))
            .bg(gpui::rgb(0x0052_525b)),
    )
}

/// The title and the close button, over a rule.
fn header<V: 'static>(
    title: SharedString,
    surfaces: &Surfaces,
    narrow: bool,
    close: fn(&mut V, &mut Window, &mut Context<'_, V>),
    cx: &mut Context<'_, V>,
) -> gpui::Div {
    let close_size = if narrow { 44.0 } else { 28.0 };
    let hover = surfaces.surface2;
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(INSIDE))
        .py(px(16.0))
        .border_b_1()
        .border_color(surfaces.surface2)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .type_(Type::Label)
                .text_color(surfaces.foreground)
                .child(title),
        )
        .child(
            div()
                .id("dialog-close")
                .size(px(close_size))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(popup::MENU_RADIUS))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                    close(view, window, cx);
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(Glyph::Close.path())
                        .size(px(10.0))
                        .text_color(surfaces.foreground_muted),
                ),
        )
}
