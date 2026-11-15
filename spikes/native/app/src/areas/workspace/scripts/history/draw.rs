//! The person acknowledges only the received records drawn with this button.
use super::{Context, Shell};
use vessel_app_kit::ui::{
    Div, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, alert, button_with_kind, theme::spacing,
};
use vessel_app_view::workspace::scripts::{Action, RetiredWrite, words};
use vessel_proto::scripts::ScriptPlace;

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
    let mut details = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(shell.words.scripts().retained_results().to_owned());
    for write in &scripts.retired_projects {
        details = details.child(receipt(shell, write));
    }
    for controls in scripts.panels.borrow().values() {
        if let Some(write) = controls
            .pending_write()
            .or_else(|| controls.returned_write())
        {
            details = details.child(receipt(shell, write));
        }
    }
    Some(alert::alert(
        alert::Content {
            title: None,
            description: Some(details.into_any_element()),
            action: Some(dismiss.into_any_element()),
        },
        alert::Variant::Warning,
        alert::Size::Small,
        tokens,
    ))
}

/// Original safe labels; raw diagnostic messages and remote configuration stay private.
pub(in crate::areas::workspace::scripts) fn receipt(shell: &Shell, write: &RetiredWrite) -> Div {
    let labels = shell.words.scripts();
    let (action, resource) = match write.action() {
        Action::Run(script) => (labels.run(), script.to_string()),
        Action::Stop(terminal) => (labels.stop(), terminal.to_string()),
        Action::Restart(terminal) => (labels.restart(), terminal.to_string()),
    };
    let place = match write.target().place {
        ScriptPlace::Project { project } => project.to_string(),
        ScriptPlace::Session { session } => session.to_string(),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(spacing::S2))
                .child(write.target().machine().to_owned())
                .child(place),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(spacing::S2))
                .child(action.to_owned())
                .child(resource),
        )
        .child(
            div()
                .type_(Type::Meta)
                .child(words::receipt(shell.words, write).to_owned()),
        )
}
