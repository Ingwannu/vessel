//! Adding a machine by pairing (ADR 0010): paste the link `vessel node
//! pair` printed on it, check the node's key fingerprint against what the
//! node shows, name it, pair. The app then reaches it as the CLI does,
//! directly or through its relay, with the device credential it stored.

use vessel_app_kit::ui::{Context, Div, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};

use super::title_bar;
use crate::shell::Shell;

pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut form = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(24.0))
        .max_w(px(640.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.text_secondary)
                .child(
                    "On the machine to add, run `vessel node pair`, then paste the link it \
                     prints here and press Enter.",
                ),
        )
        .child(shell.pair_link.element());
    if let Some(offer) = &shell.offer {
        form = form
            .child(
                div()
                    .p(px(10.0))
                    .bg(tokens.bg_well)
                    .rounded(px(vessel_app_kit::theme::radius::R2))
                    .type_(Type::Mono)
                    .text_color(tokens.text_secondary)
                    .child(offer.summary.clone()),
            )
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.text_tertiary)
                    .child(
                        "Check that the key matches what `vessel node status` shows on that \
                         machine, then name it and pair.",
                    ),
            )
            .child(shell.pair_name.element())
            .child(
                div().flex().flex_row().child(
                    button("pair", "Pair", true, tokens).on_click(
                        cx.listener(|this, _event, _window, cx| this.pair_from_field(cx)),
                    ),
                ),
            );
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar("Add a machine".to_owned(), None, tokens))
        .child(form)
}

use vessel_app_kit::ui::StatefulInteractiveElement as _;
