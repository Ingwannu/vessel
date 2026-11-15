//! The answer button of Paseo's cards (`agent-stream/view.tsx`,
//! `PermissionActionButton` and `permissionStyles.optionButton`; the
//! question card's Dismiss and Submit, `question-form-card.tsx`): a
//! bordered button, not a filled pill.
//!
//! [Decision Log]
//! - Purpose: a permission's and a question's answers drawn alike, as
//!   Paseo's, by one control rather than a look per screen.
//! - Constraint: the kit's `outline_button` is Paseo's general outline
//!   button (a pill, 16 across, no fill); the cards' button is another:
//!   8 by 12 inside, radius 6, a hairline of `borderAccent` on `surface1`,
//!   `surface2` under the pointer, 90% while pressed; a glyph of 14 before
//!   words at 14 normal, in the foreground for the primary answer and
//!   muted for the rest. The question card's Submit is the same button
//!   on the accent.
//! - Choice: one control with its look ([`CardLook`]) and its state
//!   ([`CardState`]): while an answer is sent, its button shows the kit's
//!   spinner in place of its content and every button stops answering
//!   (Paseo disables them, without fading them); Submit waiting for an
//!   answer fades to half (Paseo's `opacity: 0.5`).

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::glyph;
use crate::slider::spinner;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// What a card button looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardLook {
    /// The primary answer: its words and glyph in the foreground.
    Primary,
    /// Any other answer: muted.
    Muted,
    /// The question card's Submit: on the accent.
    Accent,
}

/// What a card button does now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    /// It answers a press.
    Ready,
    /// Another answer is being sent: it does not answer, as it is.
    Waiting,
    /// It cannot be pressed yet: at half strength.
    Faded,
    /// Its answer is being sent: the spinner in place of its content.
    Sending,
}

/// A card's answer button: `mark` and `label` as `look` says, in `state`.
/// The screen attaches its click only when the state is
/// [`CardState::Ready`].
#[must_use]
pub fn card_button(
    id: impl Into<ElementId>,
    mark: Glyph,
    label: impl Into<SharedString>,
    look: CardLook,
    state: CardState,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let (fill, edge, ink, hover) = match look {
        CardLook::Primary => (
            neutral.surface1,
            neutral.border_accent,
            neutral.foreground,
            neutral.surface2,
        ),
        CardLook::Muted => (
            neutral.surface1,
            neutral.border_accent,
            neutral.foreground_muted,
            neutral.surface2,
        ),
        CardLook::Accent => (
            neutral.accent,
            neutral.accent,
            neutral.accent_foreground,
            neutral.accent,
        ),
    };
    let id = id.into();
    let content = match state {
        CardState::Sending => spinner(id.clone(), 14.0, ink),
        // A long answer (an agent's "always" scope) wraps inside the
        // button, its glyph kept at the first line's left.
        CardState::Ready | CardState::Waiting | CardState::Faded => div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .min_w(px(0.0))
            .child(glyph(mark, 14.0, ink))
            .child(
                div()
                    .min_w(px(0.0))
                    .whitespace_normal()
                    .text_color(ink)
                    .child(label.into()),
            )
            .into_any_element(),
    };
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .min_w(px(0.0))
        .max_w_full()
        .py(px(8.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(edge)
        .bg(fill)
        .type_(Type::Body)
        .text_size(px(14.0))
        .when(state == CardState::Ready, |button| {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(|style| style.opacity(0.9))
        })
        .when(state == CardState::Faded, |button| button.opacity(0.5))
        .child(content)
}
