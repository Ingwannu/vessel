//! Existing local CLI operations are guidance, never app-executed actions.
use super::{Shell, frame};
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, IntoElement as _, ParentElement as _, Styled as _,
    Window, px,
};
use vessel_app_kit::{Tokens, breakpoint::TOUCH_TARGET, settings};
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
        card = card.child(
            settings::code_row(words.command(command), command.text(), false, tokens).child(copy),
        );
    }
    settings::section(words.commands, card, tokens).into_any_element()
}
