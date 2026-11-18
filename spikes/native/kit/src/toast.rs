//! The toast (Paseo's `components/toast-host.tsx`, its `ToastCard`
//! 230-275 and styles 284-333): one card at the top of the window, centred,
//! as wide as its words up to the boundary, on `surface0` with a hairline,
//! 16 round, 8 by 12 inside, a medium shadow; a leading mark per look, its
//! words 14 in the foreground; an error's hairline `destructive`. No close
//! button: a toast leaves when its life ends. It arrives over the kit's
//! surface length (`popup::appear`).
//!
//! [Decision Log]
//! - Purpose: the page says what an action came to in a toast; the app
//!   says it the same way, in Paseo's look (TRACKER 802).
//! - Choice: one card, drawn by `toast_layer` under the title bar; how long
//!   it stays and what replaces it is the screen's
//!   (`vessel_app_view::toasts`), this module only draws. The marks are
//!   Paseo's: success `CheckCircle2` in the primary colour (the kit's
//!   `accent`), a copy's in the foreground, an error's `AlertTriangle` in
//!   `destructive`; the git guidance keeps its own foreground `Info` at 16
//!   (`git/use-actions.tsx:38-58`).
//! - Cost: the close glyph the app had is gone, as in Paseo: an error can
//!   no longer be kept on screen past its 3.2 s.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, ElementId, SharedString, Window, anchored, deferred, div, point, px, relative,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::glyph;
use crate::popup;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// Paseo's `TOAST_MAX_WIDTH`.
const MAX_WIDTH: f32 = 480.0;

/// Paseo's toast mark size.
const MARK: f32 = 18.0;

/// The card across, besides its words: 12 of padding and a 1 border each
/// side; with a leading mark, its 18 and the 8 gap too.
const CHROME: f32 = 26.0;
const CHROME_MARKED: f32 = 52.0;

/// What a toast says, and so its mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLook {
    /// A plain line (Paseo's default variant): no mark.
    Plain,
    /// Something went as asked (Paseo's `success`).
    Success,
    /// Something was copied (Paseo's `toast.copied`).
    Copied,
    /// Known unavailability guidance, with the reference's foreground Info16.
    Info,
    /// Something failed (Paseo's `error`).
    Error,
}

/// Hover events are semantic across the kit/app boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastHover {
    /// Pause this card's remaining life.
    Enter,
    /// Resume this card's remaining life.
    Leave,
}

/// One card's content.
#[derive(Debug)]
pub struct ToastContent {
    /// Identity also keys the enter animation.
    pub id: ElementId,
    /// Whole localized words supplied by the caller.
    pub text: SharedString,
    /// Its look and mark.
    pub look: ToastLook,
}

/// A card's typed pointer-presence callback.
pub type ToastHovered = Rc<dyn Fn(ToastHover, &mut Window, &mut App)>;

/// One toast card; `id` keys its fade.
#[must_use]
pub fn toast(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    look: ToastLook,
    tokens: &Tokens,
    window: &Window,
) -> AnyElement {
    toast_with(
        ToastContent {
            id: id.into(),
            text: text.into(),
            look,
        },
        None,
        tokens,
        window,
    )
}

/// Paseo's boundary on this window: 92% of its width less 16 each side, at
/// most 480 (`toast_layer`).
fn boundary(window: &Window) -> f32 {
    f32::from(window.viewport_size().width)
        .mul_add(0.92, -29.44)
        .min(MAX_WIDTH)
}

/// The card's leading mark (Paseo's `iconSlot`), if its look has one.
fn mark(look: ToastLook, tokens: &Tokens) -> Option<gpui::Svg> {
    let neutral = tokens.neutral;
    match look {
        ToastLook::Plain => None,
        ToastLook::Success => Some(glyph(Glyph::CircleCheck, MARK, tokens.accent)),
        ToastLook::Copied => Some(glyph(Glyph::CircleCheck, MARK, neutral.foreground)),
        ToastLook::Info => Some(glyph(Glyph::Info, 16.0, neutral.foreground)),
        ToastLook::Error => Some(glyph(Glyph::Alert, MARK, neutral.destructive)),
    }
}

/// A toast card whose pointer presence `hover` hears (the screen holds the
/// toast's life while a pointer rests on it).
#[must_use]
pub fn toast_with(
    content: ToastContent,
    hover: Option<ToastHovered>,
    tokens: &Tokens,
    window: &Window,
) -> AnyElement {
    let ToastContent { id, text, look } = content;
    let neutral = tokens.neutral;
    // GPUI's line wrapper adds each character's advance (no kerning), so
    // words laid out at their own shaped width could wrap their last word
    // ("Copied Session link" on two lines with room to spare). Words that
    // fit the boundary on one line are kept on it; longer ones wrap in it.
    let chrome = if look == ToastLook::Plain {
        CHROME
    } else {
        CHROME_MARKED
    };
    let words = crate::fonts::measure(window, text.as_ref(), Type::Body);
    let one_line = words.mul_add(1.0, chrome) <= boundary(window);
    let border = if look == ToastLook::Error {
        neutral.destructive
    } else {
        neutral.border
    };
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
        .children(mark(look, tokens))
        // The words take their own width and only shrink (Paseo's message:
        // `flexShrink: 1`).
        .child(
            div()
                .flex_shrink_1()
                .min_w_0()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .when(one_line, Styled::whitespace_nowrap)
                .child(text),
        )
        .when_some(hover, |card, hover| {
            card.on_hover(move |inside, window, cx| {
                hover(
                    if *inside {
                        ToastHover::Enter
                    } else {
                        ToastHover::Leave
                    },
                    window,
                    cx,
                );
            })
        });
    // Word 39: the kit's surface arrival (Paseo's toast fades in 140 ms;
    // the tokens give every surface one length).
    popup::appear(
        ElementId::NamedChild(std::sync::Arc::new(id), "appear".into()),
        card,
        crate::motion::STANDARD,
    )
    .into_any_element()
}

/// The toast over the window, `top` under its top edge (under the title
/// bar), in Paseo's boundary: 16 in from each side, 92% of the rest, at
/// most 480 across, the card centred in it. Clicks pass through the room
/// around it.
#[must_use]
pub fn toast_layer(card: AnyElement, top: f32, window: &Window) -> AnyElement {
    let width = window.viewport_size().width;
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
                        .child(popup::layer(div().child(card))),
                ),
        ),
    )
    .with_priority(3)
    .into_any_element()
}
