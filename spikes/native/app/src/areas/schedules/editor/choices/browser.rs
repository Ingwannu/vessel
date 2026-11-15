//! Provider browsing changes only the catalog view; offered model rows own picks.
use super::{Context, HarnessRef, Picker, Rc, Shell, Visit, Window, kit, label};
use kit::ui::SharedString;
use vessel_app_link::Command;
pub(super) fn initial(
    shell: &mut Shell,
    picker: Picker,
    visit: Option<Visit>,
) -> Option<HarnessRef> {
    let initial = (picker == Picker::Model)
        .then(|| {
            shell
                .areas
                .schedules
                .data
                .editor
                .initial_browser_harness()
                .cloned()
        })
        .flatten();
    if let (Some(visit), Some(harness)) = (visit, initial.as_ref())
        && let Some(request) = shell
            .areas
            .schedules
            .data
            .editor
            .browse_agent(visit, harness.clone())
    {
        shell.command(Command::Schedules(request.into()));
    }
    initial
}
pub(super) fn back(shell: &Shell, picker: Picker) -> Option<kit::picker::Back<Shell>> {
    let ui = &shell.areas.schedules.ui.editor;
    let back: Option<kit::picker::Back<Shell>> =
        if picker == Picker::Model && ui.browsing && has_multiple_providers(shell) {
            Some(Rc::new(|shell, window, cx| {
                shell.areas.schedules.ui.editor.browsing = false;
                shell
                    .areas
                    .schedules
                    .ui
                    .editor
                    .picker
                    .set_header_leading(None);
                shell
                    .areas
                    .schedules
                    .ui
                    .editor
                    .picker
                    .reset_query("", window, cx);
                cx.notify();
            }))
        } else {
            None
        };
    back
}
pub(super) fn title(shell: &Shell, picker: Picker) -> SharedString {
    if picker == Picker::Model && shell.areas.schedules.ui.editor.browsing {
        shell
            .areas
            .schedules
            .data
            .editor
            .catalog_harness()
            .map_or_else(
                || label(shell, picker).to_owned(),
                vessel_app_view::agent_name,
            )
            .into()
    } else {
        label(shell, picker).to_owned().into()
    }
}
pub(super) fn drill(
    shell: &mut Shell,
    visit: Visit,
    harness: HarnessRef,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if let Some(request) = shell
        .areas
        .schedules
        .data
        .editor
        .browse_agent(visit, harness.clone())
    {
        shell.command(Command::Schedules(request.into()));
    }
    let ui = &mut shell.areas.schedules.ui.editor;
    ui.browsing = true;
    ui.picker
        .set_header_leading(Some(crate::agent_mark::leading(&harness)));
    ui.picker.reset_query("", window, cx);
}

fn has_multiple_providers(shell: &Shell) -> bool {
    shell
        .areas
        .schedules
        .data
        .editor
        .targets()
        .is_some_and(|targets| {
            targets
                .harnesses
                .iter()
                .filter(|entry| {
                    matches!(entry.structured, vessel_proto::StructuredSupport::Available)
                })
                .count()
                > 1
        })
}
