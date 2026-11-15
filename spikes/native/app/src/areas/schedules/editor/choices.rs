//! One shared combobox for field choices. Retained rows carry the actual form
//! visit and, for agent options, the exact catalog request that offered them.
use super::{
    AnyElement, ConnectionContext, Context, Picker, Rc, Shell, TimeZoneName, Tokens, Visit, Weak,
    Window, cadence, choose_host, inputs, install, io, kit, queue_preview,
};
use kit::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxContent, ComboboxWidth, Leading, ui::SharedString,
};
use vessel_app_view::schedules::editor::{IntervalUnit, cadence::Choice as CadenceChoice};
use vessel_proto::{ChoiceValue, HarnessRef, OptionChoice, ProjectId};
const NEW_INTERVAL_COUNT: &str = "1";
mod words;
use words::{empty_label, search_placeholder, searchable};
#[derive(Debug, Clone)]
enum Choice {
    Host(ConnectionContext),
    Project(ProjectId),
    Agent(HarnessRef),
    Option {
        request: io::Request,
        choice: OptionChoice,
    },
    Cadence(CadenceChoice),
    Unit(IntervalUnit),
    Zone(TimeZoneName),
}
#[derive(Debug)]
struct Offered {
    rows: Vec<ComboRow>,
    values: Vec<Choice>,
}
impl Offered {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            values: Vec::new(),
        }
    }
    fn push(&mut self, row: ComboRow, value: Choice) {
        self.rows.push(row);
        self.values.push(value);
    }
    fn filtered(self, query: &str) -> Self {
        let query = query.trim().to_lowercase();
        let mut selected = Self::new();
        for (row, value) in self.rows.into_iter().zip(self.values) {
            if row.label.to_lowercase().contains(&query) {
                selected.push(row, value);
            }
        }
        selected
    }
}
pub(super) fn label(shell: &Shell, picker: Picker) -> &str {
    let words = shell.words.schedules();
    match picker {
        Picker::Host => words.form_host(),
        Picker::Project => words.form_project(),
        Picker::Model => words.form_model(),
        Picker::Thinking => words.form_thinking(),
        Picker::Mode => words.form_mode(),
        Picker::Cadence => words.form_cadence(),
        Picker::Unit => words.form_interval(),
        Picker::Zone => words.form_zone(),
    }
}
pub(super) fn open(
    shell: &mut Shell,
    picker: Picker,
    visit: Option<Visit>,
    bootstrap: Option<Weak<()>>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !current(shell, visit, bootstrap.as_ref()) {
        return;
    }
    if bootstrap.is_none() && shell.areas.schedules.data.editor.blocked() {
        return;
    }
    let Some(bounds) = shell.areas.schedules.ui.editor.anchors.get(picker).get() else {
        return;
    };
    let search = searchable(shell, picker);
    let placeholder = search_placeholder(shell, picker).to_owned();
    let ui = &mut shell.areas.schedules.ui.editor;
    ui.picking = Some(picker);
    ui.browsing = false;
    ui.picker.set_anchor(Some(bounds));
    ui.picker
        .open(search.then_some(placeholder.as_str()), window, cx);
    cx.notify();
}
fn current(shell: &Shell, visit: Option<Visit>, bootstrap: Option<&Weak<()>>) -> bool {
    let ui = &shell.areas.schedules.ui.editor;
    if ui.visit() != visit {
        return false;
    }
    if let Some(expected) = bootstrap {
        ui.bootstrap
            .as_ref()
            .is_some_and(|value| Weak::ptr_eq(&Rc::downgrade(&value.cycle), expected))
    } else {
        ui.bootstrap.is_none() && shell.areas.schedules.data.editor.visit() == visit
    }
}
fn offered(shell: &Shell, picker: Picker, tokens: &Tokens) -> Offered {
    let mut out = Offered::new();
    let model = &shell.areas.schedules.data.editor;
    match picker {
        Picker::Host => {
            for machine in &shell.fleet.machines {
                if let Some(context) = machine.connection_context() {
                    out.push(
                        ComboRow {
                            checked: model.connection() == Some(&context),
                            leading: Some(Leading::Dot(super::super::filters::status_color(
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
                            leading: Some(Leading::Initial(kit::picker::initial_of(
                                project.name.as_str(),
                            ))),
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
        Picker::Model | Picker::Thinking | Picker::Mode => {
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
                    Picker::Host
                    | Picker::Project
                    | Picker::Cadence
                    | Picker::Unit
                    | Picker::Zone => None,
                };
                if let Some(control) = control {
                    for value in &control.values {
                        out.push(
                            ComboRow {
                                checked: value.id == control.current,
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
        Picker::Cadence => {
            for preset in cadence::Preset::ALL {
                out.push(
                    ComboRow {
                        checked: model.draft().is_some_and(|draft| {
                            draft.cadence.choice() == CadenceChoice::Preset(preset)
                        }),
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
pub(super) fn popup(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let ui = &shell.areas.schedules.ui.editor;
    if !ui.picker.is_open() {
        return None;
    }
    let picker = ui.picking?;
    let visit = ui.visit();
    let bootstrap = ui
        .bootstrap
        .as_ref()
        .map(|value| Rc::downgrade(&value.cycle));
    if !current(shell, visit, bootstrap.as_ref()) {
        return None;
    }
    let offered = offered(shell, picker, tokens).filtered(&ui.picker.query(cx));
    let values = offered.values;
    let back: Option<kit::picker::Back<Shell>> = if picker == Picker::Model && ui.browsing {
        Some(Rc::new(|shell, window, cx| {
            shell.areas.schedules.ui.editor.browsing = false;
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
    let header = ComboHeader {
        title: Some(label(shell, picker).to_owned().into()),
        back: back.is_some(),
        search: searchable(shell, picker),
        empty: empty_label(shell, picker).into(),
        note: (picker == Picker::Zone && ui.zones.is_err())
            .then(|| SharedString::from(shell.words.schedules().clock_zone())),
        side: kit::picker::Side::Below,
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.schedules.ui.editor.picker,
        chose: Rc::new(move |shell, index, window, cx| {
            if !current(shell, visit, bootstrap.as_ref()) {
                return;
            }
            let Some(choice) = values.get(index).cloned() else {
                return;
            };
            choose(shell, visit, bootstrap.as_ref(), choice, window, cx);
        }),
        back,
    };
    kit::combobox_with(
        ComboboxContent::new(
            "schedule-form-picker",
            &header,
            &offered.rows,
            ComboboxWidth::Content,
        ),
        &ui.picker,
        &handlers,
        tokens,
        window,
        cx,
    )
}
fn choose(
    shell: &mut Shell,
    visit: Option<Visit>,
    bootstrap: Option<&Weak<()>>,
    choice: Choice,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if let Some(expected) = bootstrap {
        if let Choice::Zone(zone) = choice {
            let Some(value) = shell.areas.schedules.ui.editor.bootstrap.take() else {
                return;
            };
            if !Weak::ptr_eq(&Rc::downgrade(&value.cycle), expected) {
                shell.areas.schedules.ui.editor.bootstrap = Some(value);
                return;
            }
            if !install(shell, value.opening, zone, window, cx) {
                let ui = &mut shell.areas.schedules.ui.editor;
                ui.inputs = None;
                ui.cancel_preview();
                ui.picking = None;
                ui.picker.close_and_restore(window, cx);
                ui.dialog.dismissed(window, cx);
                ui.mounted = false;
            }
        }
    } else if let Some(visit) = visit {
        if shell.areas.schedules.data.editor.blocked() {
            return;
        }
        let mut keep_open = false;
        let mut cadence_changed = false;
        match choice {
            Choice::Host(context) => choose_host(shell, visit, context, window, cx),
            Choice::Project(project) => {
                let _changed = shell.areas.schedules.data.editor.project(visit, project);
            }
            Choice::Agent(harness) => {
                if let Some(request) = shell.areas.schedules.data.editor.agent(visit, harness) {
                    shell.command(Command::Schedules(request.into()));
                    shell.areas.schedules.ui.editor.browsing = true;
                    shell
                        .areas
                        .schedules
                        .ui
                        .editor
                        .picker
                        .reset_query("", window, cx);
                    keep_open = true;
                }
            }
            Choice::Option { request, choice } => {
                let model = &mut shell.areas.schedules.data.editor;
                let valid = model
                    .operations()
                    .any(|slot| slot.request() == Some(&request) && slot.value().is_some());
                if valid {
                    let _changed = model.option(visit, choice);
                }
            }
            Choice::Cadence(choice) => {
                cadence_changed =
                    shell
                        .areas
                        .schedules
                        .data
                        .editor
                        .cadence(visit, |draft| match choice {
                            CadenceChoice::Preset(preset) => draft.preset(preset),
                            CadenceChoice::Custom => draft.custom(),
                            CadenceChoice::Interval => {
                                let count = if draft.unit().is_some() {
                                    draft.text().to_owned()
                                } else {
                                    NEW_INTERVAL_COUNT.to_owned()
                                };
                                let unit = draft.unit().unwrap_or(IntervalUnit::Hours);
                                draft.interval(count, unit);
                            }
                        });
                inputs::reset_rule(shell, window, cx);
            }
            Choice::Unit(unit) => {
                cadence_changed = shell
                    .areas
                    .schedules
                    .data
                    .editor
                    .cadence(visit, |draft| draft.interval(draft.text().to_owned(), unit));
                inputs::reset_rule(shell, window, cx);
            }
            Choice::Zone(zone) => {
                cadence_changed = shell
                    .areas
                    .schedules
                    .data
                    .editor
                    .cadence(visit, |draft| draft.set_zone(zone));
            }
        }
        if cadence_changed {
            queue_preview(shell, visit, cx);
        }
        if !keep_open {
            shell.areas.schedules.ui.editor.picker.close();
        }
    }
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
