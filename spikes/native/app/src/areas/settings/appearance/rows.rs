//! Composition only: shared field/row/selector/preview controls, translated
//! labels and typed preference actions. No local input engine or paint table.
use super::super::{List, general::select_for};
use crate::shell::Shell;
use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, SharedString, div, px};
use vessel_app_kit::theme::spacing::{S2, S4};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button_with_kind,
    field::AppearanceProfile, settings,
};
use vessel_app_view::{
    appearance::form::Field,
    prefs::{ContentWidth, PreferenceChange},
};

pub(super) fn fonts(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut card = settings::card(tokens);
    for field in Field::ALL {
        if field != Field::Width {
            card = card.child(row(field, shell, tokens, cx));
        }
    }
    settings::section(shell.words.appearance().fonts(), card, tokens)
}
pub(super) fn layout(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    settings::section(
        shell.words.appearance().layout(),
        settings::card(tokens).child(row(Field::Width, shell, tokens, cx)),
        tokens,
    )
}
pub(super) fn syntax(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.appearance();
    let card = settings::card(tokens).child(settings::row_at(
        words.highlight(),
        Some(words.highlight_hint().into()),
        None,
        Some(select_for(List::Syntax, shell, compact, tokens, cx)),
        true,
        false,
        tokens,
    ));
    let state = &shell.areas.settings.appearance;
    let preview = match (&state.rows, &state.preview) {
        (Ok(rows), Some(typography)) => vessel_app_kit::code_preview::code_preview(
            words.preview_content(),
            rows,
            typography,
            tokens,
        ),
        (Err(_), _) | (Ok(_), None) => div().child(words.preview_unavailable()),
    };
    settings::section(
        words.syntax(),
        div().child(card).child(div().mt(px(S4)).child(preview)),
        tokens,
    )
}
fn row(field: Field, shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let (title, hint) = labels(field, shell);
    let profile = match field {
        Field::InterfaceFamily | Field::MonoFamily => AppearanceProfile::Family,
        Field::InterfaceSize | Field::ContentSize | Field::CodeSize => AppearanceProfile::Number,
        Field::Width => AppearanceProfile::Width,
    };
    let input = shell
        .areas
        .settings
        .appearance
        .fields
        .of(field)
        .appearance_element(profile, SharedString::from(title.to_owned()), tokens, cx);
    let control = match profile {
        AppearanceProfile::Family => input,
        AppearanceProfile::Number | AppearanceProfile::Width => {
            numeric(field, input, shell, tokens, cx)
        }
    };
    settings::row_at(
        title.to_owned(),
        Some(hint.to_owned().into()),
        None,
        Some(control),
        matches!(field, Field::InterfaceFamily | Field::Width),
        false,
        tokens,
    )
}
fn numeric(
    field: Field,
    input: AnyElement,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let mut control = div().flex().items_center().gap(px(S2));
    if field == Field::Width && shell.prefs.settings.appearance.content_width.resettable() {
        control = control.child(
            button_with_kind(
                "appearance-width-reset",
                shell.words.appearance().reset_width(),
                ButtonKind::Ghost,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(|shell, _, window, cx| {
                shell.prefer(
                    &PreferenceChange::ContentWidth(ContentWidth::default()),
                    window,
                    cx,
                );
            })),
        );
    }
    control = control.child(input).child(
        div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(shell.words.appearance().pixels()),
    );
    control.into_any_element()
}
fn labels(field: Field, shell: &Shell) -> (&str, &str) {
    let words = shell.words.appearance();
    match field {
        Field::InterfaceFamily => (words.interface_family(), words.interface_family_hint()),
        Field::InterfaceSize => (words.interface_size(), words.interface_size_hint()),
        Field::ContentSize => (words.content_size(), words.content_size_hint()),
        Field::MonoFamily => (words.code_family(), words.code_family_hint()),
        Field::CodeSize => (words.code_size(), words.code_size_hint()),
        Field::Width => (words.width(), words.width_hint()),
    }
}
