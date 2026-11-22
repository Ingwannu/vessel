//! Surface the form's admission, target/zone lookup and save outcome.
use super::{Shell, button, hint};
use vessel_app_kit::{
    ButtonKind, Tokens,
    ui::{
        AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
        div,
    },
};
use vessel_app_view::schedules::{
    editor::io::{Ask, Form},
    load::Failure,
    locale,
};

pub(super) fn status(
    shell: &Shell,
    form: &Form,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let editor = &shell.areas.schedules.editor;
    let words = shell.words.schedules();
    if let Some(Err(error)) = form.saved() {
        return Some(hint(locale::failure(error, shell.words), true, tokens).into_any_element());
    }
    if editor.is_retired() {
        return Some(
            hint(
                locale::failure(&Failure::ConnectionChanged, shell.words),
                true,
                tokens,
            )
            .into_any_element(),
        );
    }
    let retry = if form.heartbeat_editable() {
        Ask::HeartbeatFacts
    } else {
        Ask::Targets
    };
    match form.targets()? {
        Err(error) => Some(
            div()
                .child(hint(locale::failure(error, shell.words), true, tokens))
                .child(
                    button(
                        "schedule-form-retry-targets",
                        words.retry(),
                        ButtonKind::Ghost,
                        true,
                        touch,
                        tokens,
                    )
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.schedule_form_issue(retry.clone());
                        cx.notify();
                    })),
                )
                .into_any_element(),
        ),
        Ok(_) if !form.can_operate() => {
            Some(hint(words.read_only(), true, tokens).into_any_element())
        }
        Ok(_) => None,
    }
}
