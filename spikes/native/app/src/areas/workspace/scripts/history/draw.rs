//! The person acknowledges only the received records drawn with this button.
use super::*;
use vessel_app_kit::ui::{
    Div, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, alert, button_with_kind};

pub(in crate::areas::workspace::scripts) fn render(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let scripts = &shell.areas.workspace.scripts;
    if !scripts.retained() {
        return None;
    }
    let received = scripts.received();
    let mut dismiss = button_with_kind(
        format!("script-receipts-{received:?}"),
        shell.words.scripts().dismiss_received(),
        ButtonKind::Secondary,
        if received.is_empty() {
            ButtonState::Disabled
        } else {
            ButtonState::Enabled
        },
        tokens,
    )
    .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET));
    if !received.is_empty() {
        dismiss = dismiss.on_click(cx.listener(move |shell, _event, window, cx| {
            shell.dismiss_script_results(&received, window, cx);
        }));
    }
    Some(alert::alert(
        alert::Content {
            title: None,
            description: Some(
                div()
                    .child(shell.words.scripts().retained_results().to_owned())
                    .into_any_element(),
            ),
            action: Some(dismiss.into_any_element()),
        },
        alert::Variant::Warning,
        alert::Size::Small,
        tokens,
    ))
}
