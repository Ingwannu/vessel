//! One existing preference store; the actor orders persistence before node writes.
use super::{Command, Context, Shell, Visit, io};
use vessel_app_link::schedules::{FormAnswer, Message, Request, ScheduleError};
use vessel_app_view::schedules::editor::definition::Save;
use vessel_proto::{SessionSpec, automation::ScheduleTarget};
fn new_session(request: &io::Request) -> Option<&SessionSpec> {
    let io::Operation::Save(save) = request.operation() else {
        return None;
    };
    let definition = match save.as_ref() {
        Save::Create(value) => &value.definition,
        Save::Update(value) => &value.definition,
    };
    match &definition.target {
        ScheduleTarget::NewSession { spec, .. } => Some(spec),
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => None,
    }
}
pub(super) fn dispatch(shell: &mut Shell, request: io::Request, cx: &mut Context<'_, Shell>) {
    let Some(spec) = new_session(&request) else {
        // Heartbeat saves do not change new-agent preferences in the source.
        shell.command(Command::Schedules(request.into()));
        return;
    };
    let Some(choices) = shell.areas.schedules.data.editor.remembered_choices() else {
        let _changed = shell.schedules_message(
            Message::FormReplied(FormAnswer {
                request,
                result: Err(ScheduleError::Preferences),
            }),
            cx,
        );
        return;
    };
    // Follow the native composer's current-launch cache convention, preserving
    // every unrelated setting/folder/host. The actor, not this cache, gates RPCs.
    shell.prefs.agent = Some(spec.harness.clone());
    shell.prefs.keep_choices(
        request.connection().machine().as_str(),
        &spec.harness,
        &choices,
    );
    shell.command(Command::Schedules(Request::remembering(
        request,
        shell.prefs.clone(),
    )));
}
pub(super) fn after_project(shell: &mut Shell, visit: Visit) {
    let model = &shell.areas.schedules.data.editor;
    let Some(harness) = model.initial_agent(shell.prefs.agent.as_ref()).cloned() else {
        return;
    };
    if let Some(request) = shell.areas.schedules.data.editor.agent(visit, harness) {
        shell.command(Command::Schedules(request.into()));
    }
}
pub(super) fn restore(shell: &mut Shell) {
    let model = &shell.areas.schedules.data.editor;
    let (Some(visit), Some(connection), Some(target)) = (
        model.visit(),
        model.connection(),
        model.draft().and_then(|draft| draft.target()),
    ) else {
        return;
    };
    let Some(choices) = shell
        .prefs
        .choices(connection.machine().as_str(), &target.spec().harness)
    else {
        return;
    };
    let _changed = shell
        .areas
        .schedules
        .data
        .editor
        .restore_choices(visit, choices);
}
