//! A command retains its exact bytes. Horizontal scrolling leaves long
//! values accessible; the shared copy button uses pointer/touch target sizes.
//!
//! [Decision Log]
//! - Purpose: commands and paths must remain readable and copyable on a phone.
//! - Constraint: the kit's copy control is shared, but no command field exists.
//! - Alternatives: truncate the value, wrap arbitrary shell syntax, or add a
//!   second copy control. Choice: a local scrolling mono value and kit copy.
//! - Cost: this composition stays local until a second area needs it.
use vessel_app_kit::copy::{CopyLook, copy_button};
use vessel_app_kit::ui::{
    App, Div, InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, fonts};
use vessel_app_kit::{breakpoint, popup, theme::spacing};

pub(in crate::areas::settings::agents) fn command_box(
    id: impl Into<gpui::ElementId>,
    text: &str,
    tokens: &Tokens,
    window: &Window,
    cx: &App,
) -> gpui::Stateful<Div> {
    let width = text
        .lines()
        .map(|line| fonts::measure(window, line, Type::Mono))
        .fold(0.0_f32, f32::max);
    div()
        .id(id)
        .w_full()
        .min_w_0()
        .flex()
        .items_start()
        .gap(px(spacing::S2))
        .rounded(px(popup::ROW_RADIUS))
        .bg(tokens.neutral.surface2)
        // The child scrolls first; a horizontal wheel must not also move
        // the settings frame, whose vertical scroller accepts either axis.
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .child(value(text, width, tokens))
        .child(
            copy_button(
                "copy",
                text.to_owned().into(),
                CopyLook {
                    glyph: tokens.neutral.foreground_muted,
                    hover: tokens.neutral.surface3,
                },
                window,
                cx,
            )
            .size(px(if breakpoint::phone(window) {
                breakpoint::TOUCH_TARGET
            } else {
                32.0
            }))
            .self_center(),
        )
}

/// A fact value carries its field identity, so two scrollable facts never
/// share the same element state. The selector also names it in the inspector.
pub(super) fn inline_value(
    id: &'static str,
    text: &str,
    tokens: &Tokens,
    window: &Window,
) -> gpui::Stateful<Div> {
    let width = fonts::measure(window, text, Type::Mono);
    scroll_value(
        id,
        contents(text, width)
            .ml_auto()
            .debug_selector(move || format!("{id}-content")),
        tokens,
    )
    .p(px(spacing::S0))
}
fn value(text: &str, width: f32, tokens: &Tokens) -> gpui::Stateful<Div> {
    scroll_value("value", contents(text, width), tokens)
}
fn contents(text: &str, width: f32) -> Div {
    div().w(px(width)).flex_shrink_0().child(text.to_owned())
}
fn scroll_value(
    id: impl Into<gpui::ElementId>,
    contents: Div,
    tokens: &Tokens,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .min_w_0()
        .overflow_x_scroll()
        .flex()
        .p(px(spacing::S3))
        .type_(Type::Mono)
        .whitespace_nowrap()
        .text_color(tokens.neutral.foreground)
        .child(contents)
}

#[cfg(test)]
mod tests;
