//! The toast (Paseo's `components/toast-host.tsx`): a card at the top of
//! the window, centred, on `surface0` with a hairline, 16 round, 8 by 12
//! inside, a medium shadow; its words 14 in the foreground; an error's
//! hairline `destructive` with Paseo's warning glyph beside the words. It
//! arrives over the kit's surface length (`popup::appear`).
//!
//! [Decision Log]
//! - Purpose: the page says what an action came to in a toast (a kill, a
//!   refused restore); the app needs the same, in Paseo's look.
//! - Choice: the cards stack, oldest first, each with its own dismiss
//!   (the page's `ui/toasts.tsx`: an error stays until dismissed), where
//!   Paseo shows one at a time and lets an error go after 3.2 s. How long
//!   one stays is the screen's (`vessel_app_view::toasts`); this module
//!   only draws.
//! - Cost: two toasts at once take two rows of the window's top, where
//!   Paseo's second replaces the first.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, ClickEvent, ElementId, SharedString, Window, anchored, deferred, div, point,
    px, relative,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::glyph;
use crate::popup;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// Paseo's `TOAST_MAX_WIDTH`.
const MAX_WIDTH: f32 = 480.0;

/// What a toast says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLook {
    /// A plain line (Paseo's default variant).
    Plain,
    /// Something failed (Paseo's `error`).
    Error,
}

/// One toast card; `id` keys its fade, `on_dismiss` runs from its close
/// glyph.
#[must_use]
pub fn toast(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    look: ToastLook,
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    tokens: &Tokens,
) -> AnyElement {
    let neutral = tokens.neutral;
    let id = id.into();
    let border = match look {
        ToastLook::Plain => neutral.border,
        ToastLook::Error => neutral.destructive,
    };
    let close = div()
        .id(ElementId::Name(format!("{id}-dismiss").into()))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(20.0))
        .rounded(px(popup::ROW_RADIUS))
        .cursor_pointer()
        .hover(move |style| style.bg(neutral.surface2))
        .on_click(on_dismiss)
        .child(glyph(Glyph::Close, 14.0, neutral.foreground_muted));
    let card = div()
        .id(id.clone())
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .max_w_full()
        .px(px(12.0))
        .py(px(8.0))
        .bg(neutral.surface0)
        .border_1()
        .border_color(border)
        .rounded(px(16.0))
        .shadow(popup::shadow_md(tokens))
        .when(look == ToastLook::Error, |card| {
            card.child(glyph(Glyph::Alert, 18.0, neutral.destructive))
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .child(text.into()),
        )
        .child(close);
    // Word 39: the kit's surface arrival (Paseo's toast fades in 140 ms;
    // the tokens give every surface one length).
    popup::appear(
        ElementId::NamedChild(std::sync::Arc::new(id), "appear".into()),
        card,
        crate::motion::STANDARD,
    )
    .into_any_element()
}

/// The toasts over the window, oldest first, `top` under its top edge
/// (under the title bar), 16 in from each side and at most 480 across.
/// Clicks pass through the room around them.
#[must_use]
pub fn toasts(cards: Vec<AnyElement>, top: f32, window: &Window) -> Option<AnyElement> {
    if cards.is_empty() {
        return None;
    }
    let width = window.viewport_size().width;
    Some(
        deferred(
            anchored().position(point(px(0.0), px(0.0))).child(
                div()
                    .w(width)
                    .flex()
                    .flex_col()
                    .items_center()
                    .pt(px(top))
                    .px(px(16.0))
                    .child(
                        div()
                            .w(relative(0.92))
                            .max_w(px(MAX_WIDTH))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(8.0))
                            .children(
                                cards
                                    .into_iter()
                                    .map(|card| popup::layer(div().child(card))),
                            ),
                    ),
            ),
        )
        .with_priority(3)
        .into_any_element(),
    )
}
