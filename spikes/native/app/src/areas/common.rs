//! Pieces more than one area draws: a frame's title bar and foot, a
//! session's colour.

use vessel_app_kit::ui::{Div, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;

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

/// The frame's foot: the last thing that failed or happened.
pub(crate) fn foot(notice: String, tokens: &Tokens) -> Div {
    div()
        .px(px(16.0))
        .py(px(8.0))
        .border_t_1()
        .border_color(tokens.border_faint)
        .type_(Type::Meta)
        .text_color(tokens.text_secondary)
        .child(notice)
}

/// A session's colour: needs-you first, then working, else at rest.
pub(crate) fn card_color(card: &Card, tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    if card.needs_you {
        tokens.st_needs_you
    } else if card.working {
        tokens.st_building
    } else {
        tokens.text_tertiary
    }
}
