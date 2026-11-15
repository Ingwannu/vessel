//! Pieces more than one area draws: a frame's title bar and foot.

use vessel_app_kit::ui::{Div, IntoElement, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

/// The frame's title bar: a heading and a line under it.
pub(crate) fn title_bar(title: String, detail: Option<String>, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .h(px(48.0))
        .px(px(16.0))
        .border_b_1()
        .border_color(tokens.border_faint)
        .child(div().type_(Type::Title).child(title))
        .children(detail.map(|detail| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(detail)
        }))
}

/// The frame's foot: the last thing that failed or happened (`notice`:
/// its words, or an element holding them).
pub(crate) fn foot(notice: impl IntoElement, tokens: &Tokens) -> Div {
    div()
        .px(px(16.0))
        .py(px(8.0))
        .border_t_1()
        .border_color(tokens.border_faint)
        .type_(Type::Meta)
        .text_color(tokens.text_secondary)
        .child(notice)
}
