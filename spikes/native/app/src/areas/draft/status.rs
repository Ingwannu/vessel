//! Creation status is rendered from exhaustive typed state and Fluent words.

use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::ui::{
    Context, Div, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, div,
};
use vessel_app_kit::{Tokens, button};
use vessel_app_view::creation::{Failure, Flow, Stage};

use crate::shell::Shell;

pub(super) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Option<Div> {
    let words = shell.words.creation();
    let (description, variant) = match &shell.areas.draft.creation.flow {
        Flow::Editing | Flow::Complete { .. } => return None,
        Flow::Creating { stage, .. } => (
            match stage {
                Stage::Workspace => words.preparing_workspace(),
                Stage::Resource => words.creating_resource(),
                Stage::InitialInput => words.sending_first_message(),
            },
            Variant::Info,
        ),
        Flow::Failed { failure, .. } => (
            match failure {
                Failure::QueueFull => words.queue_full(),
                Failure::Unsupported => words.unsupported(),
                Failure::Offline => words.offline(),
                Failure::Refused(_) => words.refused(),
                Failure::OutcomeUnknown => words.outcome_unknown(),
            },
            Variant::Error,
        ),
    };
    let retry = shell.areas.draft.creation.registered.is_some()
        && matches!(shell.areas.draft.creation.flow, Flow::Failed { .. })
        && shell.areas.draft.creation.flow.may_submit();
    let description = if retry {
        words.registered_retry()
    } else {
        description
    };
    let action = retry.then(|| {
        button(
            "creation-retry-input",
            words.retry_first_message(),
            true,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| {
            let text = shell.areas.draft.composer.text(cx);
            shell.send_draft(text, window, cx);
        }))
        .into_any_element()
    });
    Some(alert(
        Content {
            title: None,
            description: Some(div().child(description.to_owned()).into_any_element()),
            action,
        },
        variant,
        Size::Small,
        tokens,
    ))
}
