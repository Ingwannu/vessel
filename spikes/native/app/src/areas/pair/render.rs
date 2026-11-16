//! The two pairing dialogs; layout and control order follow Paseo.

use crate::shell::Shell;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button, button_with_kind, settings,
};
use vessel_app_link::Offer;

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    // Paseo pairs in a modal over the screen it was asked from: here the
    // new chat, where pairing leads.
    let narrow = crate::areas::draft::compact(window);
    let modal = match &shell.areas.pair.offer {
        None => link_step(shell, tokens, window, narrow, cx),
        Some(offer) => confirm_step(shell, offer, tokens, window, narrow, cx),
    };
    crate::areas::draft::render(shell, tokens, window, cx).child(modal)
}

/// The first step, Paseo's paste-link modal: what to paste, the field,
/// why it is a secret, Cancel and Pair.
fn link_step(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.pairing();
    let pair = &shell.areas.pair;
    let neutral = tokens.neutral;
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(words.helper()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(field_label(words.label(), tokens))
                .child(pair.link.element(tokens, narrow, cx))
                .children(
                    pair.link_error
                        .map(|error| beside(error.words(shell.words), tokens)),
                ),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(words.security_warning()),
        )
        .child(actions(
            words.cancel(),
            button("pair-link", words.pair(), true, tokens).on_click(cx.listener(
                |this, _event, window, cx| {
                    let text = this.areas.pair.link.text(cx);
                    this.read_offer(&text, window, cx);
                },
            )),
            true,
            tokens,
            cx,
        ));
    modal(shell, words.title(), body, tokens, window, narrow, cx)
}

/// The second step, Paseo's host confirmation: what connecting allows, the
/// host, its key's fingerprint (the machine printed the same with the
/// link) and its relay in a settings card; then the name the book will
/// call it (Vessel's), Cancel and Connect.
fn confirm_step(
    shell: &Shell,
    offer: &Offer,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.pairing();
    let pair = &shell.areas.pair;
    let neutral = tokens.neutral;
    let card = settings::card(tokens)
        .child(settings::code_row(
            words.machine(),
            offer.node.clone(),
            true,
            tokens,
        ))
        .child(settings::code_row(
            words.fingerprint(),
            offer.key.clone(),
            false,
            tokens,
        ))
        .children(offer.routes.rows().iter().map(|route| {
            settings::row(route.words(shell.t.lang).clone(), None, None, false, tokens)
        }));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(words.confirm_description()),
        )
        .child(card)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(field_label(words.name_label(), tokens))
                .child(pair.name.element(tokens, narrow, cx))
                .child(match pair.name_problem {
                    Some(problem) => beside(problem.words(shell.words), tokens),
                    None => div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(words.name_hint()),
                }),
        )
        .children(
            pair.pair_error
                .as_ref()
                .map(|error| beside(error.shared_words(), tokens)),
        )
        .child(actions(
            words.cancel(),
            button_with_kind(
                "pair-connect",
                if pair.busy {
                    words.busy()
                } else {
                    words.connect()
                },
                ButtonKind::Primary,
                if pair.busy {
                    ButtonState::Disabled
                } else {
                    ButtonState::Enabled
                },
                tokens,
            )
            .when(!pair.busy, |button| {
                button.on_click(cx.listener(|this, _event, _window, cx| this.pair_from_field(cx)))
            }),
            !pair.busy,
            tokens,
            cx,
        ));
    modal(
        shell,
        words.confirm_title(),
        body,
        tokens,
        window,
        narrow,
        cx,
    )
}

/// The modal (the kit's dialog: Paseo's adaptive modal sheet).
fn modal(
    shell: &Shell,
    title: &'static str,
    body: Div,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    dialog(
        &shell.areas.pair.dialog,
        DialogContent {
            title: title.into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        |shell: &mut Shell, window, cx| shell.close_pairing(window, cx),
        cx,
    )
}

/// A field's label (Paseo's: 14, medium, muted).
fn field_label(text: &'static str, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Label)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
}

/// Cancel and the step's own button, side by side and as wide (Paseo's
/// `actions`: 12 between, 8 above).
fn actions(
    cancel: &'static str,
    go: vessel_app_kit::ui::Stateful<Div>,
    enabled: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .flex()
        .flex_row()
        .gap(px(12.0))
        .mt(px(8.0))
        .child(
            button_with_kind(
                "pair-cancel",
                cancel,
                ButtonKind::Secondary,
                if enabled {
                    ButtonState::Enabled
                } else {
                    ButtonState::Disabled
                },
                tokens,
            )
            .flex_1()
            .when(enabled, |button| {
                button.on_click(
                    cx.listener(|this, _event, window, cx| this.close_pairing(window, cx)),
                )
            }),
        )
        .child(go.flex_1())
}

/// An error, beside the control it is about (Paseo's: 14, destructive).
fn beside(text: impl Into<vessel_app_kit::ui::SharedString>, tokens: &Tokens) -> Div {
    let text: vessel_app_kit::ui::SharedString = text.into();
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.destructive)
        .child(text)
}
