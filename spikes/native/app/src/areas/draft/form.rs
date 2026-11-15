//! Workspace-only controls reuse the kit and retain payload when the target changes.
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};
use vessel_app_view::{composer::drafts::DraftKey, creation::Launch};

pub(super) fn launch(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Option<Div> {
    if shell.route != Route::NewWorkspace {
        return None;
    }
    let form = shell.areas.draft.creation.workspace.as_ref()?;
    let enabled = shell.areas.draft.creation.flow.may_submit()
        && !shell.composer_attachments_blocked(DraftKey::New);
    let size = SegmentSize::Medium;
    let words = shell.words.creation();
    Some(
        segmented(size)
            .child(
                segment(
                    "workspace-chat",
                    words.launch_chat().to_owned(),
                    form.launch == Launch::Chat,
                    enabled,
                    size,
                    tokens,
                )
                .when(enabled, |button| {
                    button.on_click(cx.listener(|shell, _event, _window, cx| {
                        shell.workspace_launch(Launch::Chat, cx)
                    }))
                }),
            )
            .child(
                segment(
                    "workspace-shell",
                    words.launch_shell().to_owned(),
                    form.launch == Launch::BlankTerminal,
                    enabled,
                    size,
                    tokens,
                )
                .when(enabled, |button| {
                    button.on_click(cx.listener(|shell, _event, _window, cx| {
                        shell.workspace_launch(Launch::BlankTerminal, cx)
                    }))
                }),
            ),
    )
}

pub(super) fn submit(
    shell: &Shell,
    enabled: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    button_with_kind(
        "create-workspace",
        shell.words.creation().create_workspace().to_owned(),
        ButtonKind::Primary,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        tokens,
    )
    .when(enabled, |button| {
        button.on_click(cx.listener(|shell, _event, window, cx| {
            let text = shell.areas.draft.composer.text(cx);
            shell.send_draft(text, window, cx);
        }))
    })
    .into_any_element()
}
impl Shell {
    /// A deliberate host choice replaces placement, never the retained input.
    pub(super) fn workspace_machine_choice(&mut self, name: &str) -> bool {
        if self.route != Route::NewWorkspace {
            return true;
        }
        if !self.areas.draft.creation.flow.may_submit()
            || self.composer_attachments_blocked(DraftKey::New)
        {
            return false;
        }
        let Some(machine) = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == name)
        else {
            return false;
        };
        let (Some(node), Ok(machine)) = (
            machine.node,
            vessel_app_view::machine_book::Name::parse(name),
        ) else {
            return false;
        };
        let Some(form) = &mut self.areas.draft.creation.workspace else {
            return false;
        };
        let destination = vessel_app_view::creation::DraftDestination { machine, node };
        if form.destination != destination {
            form.destination = destination;
            // A project id/root belongs to the prior node. Only the project
            // owner may populate placement for the newly selected destination.
            form.project = None;
            self.areas.draft.creation.plan = None;
            self.forget_place();
        }
        true
    }
    fn workspace_launch(&mut self, launch: Launch, cx: &mut Context<'_, Self>) {
        if !self.areas.draft.creation.flow.may_submit()
            || self.composer_attachments_blocked(DraftKey::New)
        {
            return;
        }
        if let Some(form) = &mut self.areas.draft.creation.workspace {
            form.launch = launch;
            self.areas.draft.creation.plan = None;
            self.areas.draft.error = None;
            cx.notify();
        }
    }
}
