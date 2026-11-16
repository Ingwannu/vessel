//! One existing preference store; the actor orders persistence before node writes.
use super::{Command, Context, Shell, Visit, io};
use vessel_app_link::schedules::{FormAnswer, Message, Request, ScheduleError};
use vessel_app_view::schedules::editor::definition::Save;
use vessel_proto::{HarnessRef, OptionChoice, SessionSpec, automation::ScheduleTarget};
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
    let model = &shell.areas.schedules.data.editor;
    let choices = model
        .mailbox(request.operation())
        .filter(|slot| slot.pending() && slot.request() == Some(&request))
        .and_then(|_| model.remembered_choices());
    let snapshot = choices.as_deref().and_then(|choices| {
        kept_snapshot(
            &shell.prefs,
            request.connection().machine().as_str(),
            &spec.harness,
            choices,
        )
    });
    let Some(snapshot) = snapshot else {
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
    shell.prefs = snapshot.clone();
    shell.command(Command::Schedules(Request::remembering(request, snapshot)));
}
fn kept_snapshot(
    prefs: &vessel_app_link::Prefs,
    machine: &str,
    harness: &HarnessRef,
    choices: &[OptionChoice],
) -> Option<vessel_app_link::Prefs> {
    let mut snapshot = prefs.clone();
    snapshot.agent = Some(harness.clone());
    snapshot.keep_choices(machine, harness, choices);
    // Reuse the canonical bounds; never claim persistence of a silently dropped choice.
    if snapshot.choices(machine, harness) != Some(choices) {
        return None;
    }
    Some(snapshot)
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

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::OptionId;
    const UNKEEPABLE_MODEL_BYTES: usize = 65;
    #[test]
    fn canonical_snapshot_bounds_refuse_loss_without_changing_unrelated_preferences() {
        let harness: HarnessRef = "claude-code".parse().unwrap();
        let before = vessel_app_link::Prefs {
            machine: Some("untouched-book-entry".into()),
            ..Default::default()
        };
        let choice = |value: String| OptionChoice {
            id: OptionId::new("model"),
            value: vessel_proto::ChoiceValue::Select {
                value: vessel_proto::OptionValueId::new(value),
            },
        };
        let kept = choice("offered-model".into());
        let snapshot = kept_snapshot(
            &before,
            "selected-book-entry",
            &harness,
            std::slice::from_ref(&kept),
        )
        .unwrap();
        assert_eq!(snapshot.machine, before.machine);
        assert_eq!(snapshot.settings, before.settings);
        assert_eq!(snapshot.folders, before.folders);
        assert_eq!(snapshot.agent.as_ref(), Some(&harness));
        assert_eq!(
            snapshot.choices("selected-book-entry", &harness),
            Some([kept].as_slice())
        );
        assert!(
            kept_snapshot(
                &before,
                "selected-book-entry",
                &harness,
                &[choice("m".repeat(UNKEEPABLE_MODEL_BYTES))]
            )
            .is_none()
        );
        assert!(before.choices("selected-book-entry", &harness).is_none());
        assert!(before.agent.is_none());
    }
}
