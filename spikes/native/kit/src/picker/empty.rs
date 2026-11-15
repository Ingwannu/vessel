//! Named model-browser feedback inside the SAME leased combobox surface.
//! No arbitrary body, caller geometry or domain retry policy enters this control.
use super::{Back, ComboHandlers, ComboboxState};
use crate::{
    ButtonKind, ButtonState, ControlSizing, Tokens, Type, TypeExt as _, assets::Glyph,
    theme::spacing,
};
use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, SharedString, div, px};
/// Existing callback shape, with a shared primary button and explicit availability.
pub enum Retry<V> {
    /// Ready read callback; the owner also rechecks request/context/visit.
    Ready {
        /// Localized ordinary Retry caption.
        label: SharedString,
        /// Existing caller callback; the kit admits its captured opening first.
        action: Back<V>,
    },
    /// Explicit retry is pending; same primary button, disabled, no callback.
    Pending(SharedString),
}
/// Optional source feedback used only when the offered list is empty.
pub enum Empty<V> {
    /// Numeric14 loader painted at0.7, plus the source's short loading label.
    Loading(SharedString),
    /// Source alert glyph/message, with an optional explicit retry.
    Failed {
        /// Complete localized error; no raw error diagnostic.
        message: SharedString,
        /// Absent when retry would be unavailable, unsafe or unsupported.
        retry: Option<Retry<V>>,
    },
    /// Search glyph with one localized no-matching-models label.
    NoMatches(SharedString),
}
fn frame(tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .p(px(spacing::S4))
        .gap(px(spacing::S2))
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .text_center()
}
pub(super) fn render<V: 'static>(
    empty: &Empty<V>,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let mut body = frame(tokens);
    match empty {
        Empty::Loading(label) => {
            body = body
                .child(crate::loading::model_browser_indicator(
                    "model-browser-loading",
                    tokens.neutral.foreground_muted,
                ))
                .child(label.clone())
        }
        Empty::NoMatches(label) => {
            body = body
                .child(super::glyph(
                    Glyph::Search,
                    crate::theme::icon::MD,
                    tokens.neutral.foreground_muted,
                ))
                .child(label.clone())
        }
        Empty::Failed { message, retry } => {
            body = body
                .child(super::glyph(
                    Glyph::Alert,
                    crate::theme::icon::MD,
                    tokens.neutral.foreground_muted,
                ))
                .child(message.clone());
            if let Some(retry) = retry {
                body = body.child(retry_button(retry, state, handlers, compact, tokens, cx));
            }
        }
    }
    body.into_any_element()
}
fn retry_button<V: 'static>(
    retry: &Retry<V>,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> gpui::Stateful<Div> {
    let (label, availability) = match retry {
        Retry::Ready { label, .. } => (label, ButtonState::Enabled),
        Retry::Pending(label) => (label, ButtonState::Disabled),
    };
    let mut button = crate::button_with_kind_at(
        "model-browser-retry",
        label.clone(),
        ButtonKind::Primary,
        availability,
        ControlSizing::from_compact(compact),
        tokens,
    );
    if let Retry::Ready { action, .. } = retry {
        let lease = state.render_lease();
        let get = handlers.state;
        let action = std::rc::Rc::clone(action);
        button = button.on_click(cx.listener(move |view: &mut V, _, window, cx| {
            cx.stop_propagation();
            if !get(view).accepts(&lease) {
                window.prevent_default();
                return;
            }
            action(view, window, cx);
        }));
    }
    button
}

impl<V> std::fmt::Debug for Retry<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready { label, .. } => {
                f.debug_tuple("Ready").field(label).finish_non_exhaustive()
            }
            Self::Pending(label) => f.debug_tuple("Pending").field(label).finish(),
        }
    }
}
impl<V> std::fmt::Debug for Empty<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading(label) => f.debug_tuple("Loading").field(label).finish(),
            Self::NoMatches(label) => f.debug_tuple("NoMatches").field(label).finish(),
            Self::Failed { message, retry } => f
                .debug_struct("Failed")
                .field("message", message)
                .field("retry", retry)
                .finish(),
        }
    }
}
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod tests;
