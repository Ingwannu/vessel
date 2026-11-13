//! Settings values use the same row frame as other facts. Copy stays quiet
//! until hover on pointer widths and remains visible on touch.
use vessel_app_kit::copy::{CopyLook, copy_button};
use vessel_app_kit::ui::{
    App, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, settings, theme::spacing};
#[derive(Debug, Clone, Copy)]
pub(super) enum Value<'a> {
    Code(&'a str),
    Text(&'a str),
}
pub(super) struct Fact<'a> {
    pub(super) id: &'static str,
    pub(super) label: &'a str,
    pub(super) value: Value<'a>,
    pub(super) first: bool,
}
pub(super) fn row(fact: &Fact<'_>, tokens: &Tokens, window: &Window, cx: &App) -> Div {
    let compact = vessel_app_kit::breakpoint::phone(window);
    let id = fact.id;
    let control = match fact.value {
        Value::Code(text) => div()
            .id(fact.id)
            .group(fact.id)
            .on_scroll(|_, _, cx| cx.stop_propagation())
            .max_w(px(420.0))
            .flex()
            .items_center()
            .gap(px(spacing::S2))
            .child(super::widgets::inline_value(fact.id, text, tokens, window))
            .child(
                copy_button(
                    "copy",
                    text.to_owned().into(),
                    CopyLook {
                        glyph: tokens.neutral.foreground_muted,
                        hover: tokens.neutral.surface2,
                    },
                    window,
                    cx,
                )
                .size(px(if compact {
                    vessel_app_kit::breakpoint::TOUCH_TARGET
                } else {
                    32.0
                }))
                .debug_selector(move || format!("{id}-copy"))
                .when(!compact, |copy| {
                    copy.opacity(0.0)
                        .group_hover(fact.id, |style| style.opacity(1.0))
                }),
            ),
        Value::Text(text) => div()
            .id(fact.id)
            .max_w(px(420.0))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(text.to_owned()),
    }
    // A wrapped compact control still has to fit its row's content box.
    // A fixed desktop cap alone lets a long path push Copy past the clip.
    .when(compact, |control| control.min_w_0().max_w_full());
    settings::row_at(
        fact.label.to_owned(),
        None,
        None,
        Some(control.into_any_element()),
        fact.first,
        compact,
        tokens,
    )
}

#[cfg(test)]
mod tests;
