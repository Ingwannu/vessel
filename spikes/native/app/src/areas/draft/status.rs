//! Creation status is rendered from exhaustive typed state and Fluent words.

use vessel_app_kit::Tokens;
use vessel_app_kit::alert::{Content, Size, Variant, alert};
use vessel_app_kit::ui::{Div, IntoElement as _, div};
use vessel_app_view::creation::{Failure, Flow, Stage};

use crate::shell::Shell;

pub(super) fn render(shell: &Shell, tokens: &Tokens) -> Option<Div> {
    let words = shell.words.creation();
    let (description, variant) = match &shell.areas.draft.creation {
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
    Some(alert(
        Content {
            title: None,
            description: Some(div().child(description.to_owned()).into_any_element()),
            action: None,
        },
        variant,
        Size::Small,
        tokens,
    ))
}
