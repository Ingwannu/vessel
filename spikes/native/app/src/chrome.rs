//! The app's title bar (the kit draws the bar and the window's controls;
//! this says what is in it): the mark, the app's name and the build's
//! label.

use vessel_app_kit::ui::{
    Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

use crate::shell::Shell;

pub(crate) fn title_bar(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) -> impl vessel_app_kit::ui::IntoElement {
    let content = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(7.0))
        // The mark in the text's colour at the text's size, beside the name
        // (docs/design/brand/README.md: "In the interface").
        .child(vessel_app_kit::mark_mono(14.0, tokens.text_primary))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.text_primary)
                .child(shell.t.app),
        )
        .child(
            div()
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(crate::LABEL),
        )
        .into_any_element();
    vessel_app_kit::chrome::title_bar(content, tokens, window)
}
