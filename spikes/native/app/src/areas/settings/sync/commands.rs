//! Existing local CLI operations are guidance, never app-executed actions.
use super::{Shell, frame};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{
    Tokens, Type, TypeExt as _, breakpoint::TOUCH_TARGET, settings, theme::spacing,
};
use vessel_app_view::sync::LocalCommand;

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.sync();
    let mut card = settings::card(tokens).child(frame::message(words.commands_hint, tokens));
    for command in LocalCommand::ALL {
        let copy = vessel_app_kit::copy::copy_button(
            format!("sync-copy-{}", command.text()),
            command.text().into(),
            vessel_app_kit::copy::CopyLook {
                glyph: tokens.neutral.foreground_muted,
                hover: tokens.neutral.surface2,
            },
            window,
            cx,
        )
        .when(compact, |copy| {
            copy.min_w(px(TOUCH_TARGET)).min_h(px(TOUCH_TARGET))
        });
        let command_line = div()
            .flex()
            .items_center()
            .min_w_0()
            .max_w_full()
            .gap(px(spacing::S2))
            .child(
                div()
                    .min_w_0()
                    .type_(Type::Mono)
                    .whitespace_normal()
                    .child(command.text()),
            )
            .child(copy)
            .into_any_element();
        card = card.child(settings::row_at(
            words.command(command),
            None,
            None,
            Some(command_line),
            false,
            compact,
            tokens,
        ));
    }
    settings::section(words.commands, card, tokens).into_any_element()
}
