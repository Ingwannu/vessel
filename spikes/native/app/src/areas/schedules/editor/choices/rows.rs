//! Build only the current form's offered rows; all frame/input chrome is shared.
use super::{Choice, Offered, Picker, Shell, Tokens, io, kit};
use kit::{ComboRow, Leading};
use vessel_app_view::schedules::editor::{cadence, cadence::Choice as CadenceChoice};
use vessel_proto::{ChoiceValue, OptionChoice};
pub(super) fn offered(shell: &Shell, picker: Picker, tokens: &Tokens) -> Offered {
    let mut out = Offered::new();
    let model = &shell.areas.schedules.data.editor;
    match picker {
        Picker::Host => host_rows(shell, tokens, &mut out),
        Picker::Project => {
            if let Some(targets) = model.targets() {
                for project in &targets.projects {
                    out.push(
                        ComboRow {
                            disabled: !matches!(
                                project.state,
                                vessel_proto::workspace::ProjectState::Ready
                            ),
                            checked: model.project_id() == Some(project.id),
                            leading: Some(Leading::FormGlyph(kit::assets::Glyph::Folder)),
                            ..ComboRow::new(project.name.as_str().to_owned())
                        },
                        Choice::Project(project.id),
                    );
                }
            }
        }
        Picker::Model if !shell.areas.schedules.ui.editor.browsing => {
            if let Some(targets) = model.targets() {
                for harness in &targets.harnesses {
                    out.push(
                        ComboRow {
                            disabled: !matches!(
                                harness.structured,
                                vessel_proto::StructuredSupport::Available
                            ),
                            opens: true,
                            leading: Some(crate::agent_mark::leading(&harness.kind)),
                            ..ComboRow::new(vessel_app_view::agent_name(&harness.kind))
                        },
                        Choice::Agent(harness.kind.clone()),
                    );
                }
            }
        }
        Picker::Model | Picker::Thinking | Picker::Mode => option_rows(shell, picker, &mut out),
        Picker::Cadence => cadence_rows(shell, &mut out),
        Picker::Unit => {
            if let Some(draft) = model.draft() {
                for unit in draft.cadence.units() {
                    out.push(
                        ComboRow {
                            checked: draft.cadence.unit() == Some(*unit),
                            ..ComboRow::new(shell.t.lang.interval_unit(*unit))
                        },
                        Choice::Unit(*unit),
                    );
                }
            }
        }
        Picker::Zone => {
            if let Ok(zones) = &shell.areas.schedules.ui.editor.zones {
                for zone in zones {
                    out.push(
                        ComboRow {
                            checked: model
                                .draft()
                                .is_some_and(|draft| draft.cadence.zone() == zone),
                            ..ComboRow::new(zone.as_str().to_owned())
                        },
                        Choice::Zone(zone.clone()),
                    );
                }
            }
        }
    }
    out
}
fn host_rows(shell: &Shell, tokens: &Tokens, out: &mut Offered) {
    let model = &shell.areas.schedules.data.editor;

    for machine in &shell.fleet.machines {
        if let Some(context) = machine.connection_context() {
            out.push(
                ComboRow {
                    checked: model.connection() == Some(&context),
                    leading: Some(Leading::Dot(super::super::super::filters::status_color(
                        match &machine.reach {
                            vessel_app_view::Reach::Online => {
                                vessel_app_view::badge::Variant::Success
                            }
                            vessel_app_view::Reach::Unreachable(_) => {
                                vessel_app_view::badge::Variant::Error
                            }
                        },
                        tokens,
                    ))),
                    ..ComboRow::new(machine.name.clone())
                },
                Choice::Host(context),
            );
        }
    }
}

fn option_rows(shell: &Shell, picker: Picker, out: &mut Offered) {
    let model = &shell.areas.schedules.data.editor;

    let request = model
        .operations()
        .filter_map(io::Mailbox::request)
        .find(|request| matches!(request.operation(), io::Operation::Options(_)));
    let controls = shell
        .areas
        .schedules
        .data
        .editor_cache
        .as_ref()
        .ok()
        .and_then(Option::as_ref)
        .and_then(|form| form.controls.as_ref());
    if let (Some(controls), Some(request)) = (controls, request) {
        let control = match picker {
            Picker::Model => controls.model.as_ref(),
            Picker::Thinking => controls.effort.as_ref(),
            Picker::Mode => controls.permissions.as_ref(),
            Picker::Host | Picker::Project | Picker::Cadence | Picker::Unit | Picker::Zone => None,
        };
        if let Some(control) = control {
            for value in &control.values {
                out.push(
                    ComboRow {
                        checked: value.id == control.current,
                        leading: (picker == Picker::Thinking)
                            .then_some(Leading::FormGlyph(kit::assets::Glyph::Brain)),
                        ..ComboRow::new(value.name.clone())
                    },
                    Choice::Option {
                        request: request.clone(),
                        choice: OptionChoice {
                            id: control.id.clone(),
                            value: ChoiceValue::Select {
                                value: value.id.clone(),
                            },
                        },
                    },
                );
            }
        }
    }
}

fn cadence_rows(shell: &Shell, out: &mut Offered) {
    let model = &shell.areas.schedules.data.editor;

    for preset in cadence::Preset::ALL {
        out.push(
            ComboRow {
                checked: model
                    .draft()
                    .is_some_and(|draft| draft.cadence.choice() == CadenceChoice::Preset(preset)),
                ..ComboRow::new(preset.label(shell.words))
            },
            Choice::Cadence(CadenceChoice::Preset(preset)),
        );
    }
    for (choice, label) in [
        (
            CadenceChoice::Interval,
            shell.words.schedules().form_interval(),
        ),
        (CadenceChoice::Custom, shell.words.schedules().form_custom()),
    ] {
        out.push(
            ComboRow {
                checked: model
                    .draft()
                    .is_some_and(|draft| draft.cadence.choice() == choice),
                ..ComboRow::new(label)
            },
            Choice::Cadence(choice),
        );
    }
}
