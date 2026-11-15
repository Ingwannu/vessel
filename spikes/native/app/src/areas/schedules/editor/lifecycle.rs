//! Actual mount/dismiss and read preparation; no second field or focus engine.
use super::{Command, Context, OpeningError, Shell, Window, io};
pub(in crate::areas::schedules) fn ready_choices(shell: &mut Shell) {
    let model = &shell.areas.schedules.data.editor;
    if model.targets().is_none()
        || model.operations().any(|slot| {
            slot.request()
                .is_some_and(|request| matches!(request.operation(), io::Operation::Options(_)))
        })
    {
        return;
    }
    let (Some(visit), Some(harness)) = (
        model.visit(),
        model
            .draft()
            .and_then(|draft| draft.target())
            .map(|target| target.spec().harness.clone()),
    ) else {
        return;
    };
    if let Some(request) = shell.areas.schedules.data.editor.agent(visit, harness) {
        shell.command(Command::Schedules(request.into()));
    }
}
pub(in crate::areas::schedules) fn hold(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    use vessel_app_view::schedules::editor::definition::Issue;
    let catalog = shell.words;
    let error_words = shell
        .areas
        .schedules
        .ui
        .editor
        .error
        .as_ref()
        .map(|error| match error {
            OpeningError::Cadence(error) => Issue::Cadence(error).notice(catalog),
            OpeningError::Definition(error) => Issue::Edit(*error).notice(catalog),
            OpeningError::PreviewCount(error) => {
                // The typed configuration failure stays available to diagnostics;
                // its English Display is never a customer-facing sentence.
                tracing::error!(error = %error, "schedule preview count cannot be prepared");
                vessel_app_view::schedules::presentation::Notice {
                    label: catalog.schedules().form_preview_configuration().into(),
                    variant: vessel_app_view::alert::Variant::Error,
                }
            }
            OpeningError::Changed => {
                let failure = vessel_app_view::schedules::load::Failure::ConnectionChanged;
                vessel_app_view::schedules::presentation::Notice {
                    label: vessel_app_view::schedules::locale::failure(&failure, catalog).into(),
                    variant: failure.variant(),
                }
            }
        });
    shell.areas.schedules.ui.editor.error_words = error_words;
    let blocked = shell.areas.schedules.data.editor.blocked()
        || shell.route != crate::routes::Route::Schedules
        || !shell.areas.schedules.ui.editor.mounted;
    if let Some(inputs) = &mut shell.areas.schedules.ui.editor.inputs {
        inputs.hold(blocked, cx);
    }
}
pub(in crate::areas::schedules) fn needs_frame(shell: &Shell) -> bool {
    let ui = &shell.areas.schedules.ui.editor;
    ui.inputs.as_ref().is_some_and(|inputs| {
        Some(inputs.visit) != shell.areas.schedules.data.editor.visit()
            || ui.lang != Some(shell.t.lang)
    })
}
pub(in crate::areas::schedules) fn frame(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let words = shell.words.schedules();
    let ui = &mut shell.areas.schedules.ui.editor;
    if ui.lang != Some(shell.t.lang) {
        if let Some(inputs) = &ui.inputs {
            inputs
                .name
                .set_placeholder(words.form_optional(), window, cx);
            inputs
                .prompt
                .set_placeholder(words.form_prompt_placeholder(), window, cx);
            inputs
                .limit
                .set_placeholder(words.form_unlimited(), window, cx);
        }
        ui.lang = Some(shell.t.lang);
        cx.notify();
    }
    if ui.visit().is_some() && ui.visit() != shell.areas.schedules.data.editor.visit() {
        ui.picker.close_and_restore(window, cx);
        ui.inputs = None;
        ui.cancel_preview();
        ui.dialog.dismissed(window, cx);
        ui.mounted = false;
        cx.notify();
    }
}
pub(in crate::areas::schedules) fn leaving(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let ui = &mut shell.areas.schedules.ui.editor;
    ui.picker.close_and_restore(window, cx);
    ui.cancel_preview();
    if let Some(inputs) = &mut ui.inputs {
        inputs.hold(true, cx);
    }
    if ui.mounted {
        ui.dialog.dismissed(window, cx);
        ui.mounted = false;
    }
}
pub(in crate::areas::schedules) fn arrive(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let current = shell.areas.schedules.data.editor.visit();
    let blocked = shell.areas.schedules.data.editor.blocked();
    let ui = &mut shell.areas.schedules.ui.editor;
    // A completed save can settle while this route is away. Never capture
    // focus for its retired form on a later route arrival.
    if ui.visit().is_some() && ui.visit() != current {
        ui.inputs = None;
        ui.cancel_preview();
    }
    if let Some(inputs) = &mut ui.inputs {
        inputs.hold(blocked, cx);
    }
    if (ui.inputs.is_some() || ui.bootstrap.is_some()) && !ui.mounted {
        ui.dialog.focus(window, cx);
        ui.mounted = true;
    }
}
