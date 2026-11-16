//! One shared combobox for field choices. Retained rows carry the actual form
//! visit and, for agent options, the exact catalog request that offered them.
use super::{
    AnyElement, Command, ConnectionContext, Context, Picker, Rc, Shell, TimeZoneName, Tokens,
    Visit, Weak, Window, choose_host, inputs, install, io, kit, queue_preview,
};
use kit::{ComboHandlers, ComboHeader, ComboRow, ComboboxContent, ComboboxWidth, ui::SharedString};
use vessel_app_view::schedules::editor::{IntervalUnit, cadence::Choice as CadenceChoice};
use vessel_proto::{HarnessRef, OptionChoice, ProjectId};
const NEW_INTERVAL_COUNT: &str = "1";
mod browser;
mod rows;
mod words;
use rows::offered;
use words::{empty_label, search_placeholder, searchable};
#[derive(Debug, Clone)]
enum Choice {
    Host(ConnectionContext),
    Project(ProjectId),
    Agent(HarnessRef),
    Model {
        request: io::Request,
        choice: OptionChoice,
    },
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
    bootstrap: Option<&Weak<()>>,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if !current(shell, visit, bootstrap) {
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
    let initial = browser::initial(shell, picker, visit);
    let ui = &mut shell.areas.schedules.ui.editor;
    ui.picking = Some(picker);
    ui.browsing = initial.is_some();
    ui.picker
        .set_header_leading(initial.as_ref().map(crate::agent_mark::leading));
    ui.picker.set_anchor(Some(bounds));
    ui.picker
        .open(search.then_some(placeholder.as_str()), window, cx);
    // A previous browse may have read another provider. The displayed header
    // must name the same catalog request as these rows; never reuse it by index.
    if let (Some(visit), Some(harness)) = (visit, initial)
        && let Some(request) = shell
            .areas
            .schedules
            .data
            .editor
            .browse_agent(visit, harness)
    {
        shell.command(Command::Schedules(request.into()));
    }
    let _changed = shell.schedules_changed(cx);
    cx.notify();
}
fn current(shell: &Shell, visit: Option<Visit>, bootstrap: Option<&Weak<()>>) -> bool {
    let ui = &shell.areas.schedules.ui.editor;
    if shell.route != crate::routes::Route::Schedules || !ui.mounted {
        return false;
    }
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

pub(in crate::areas::schedules) fn popup(
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
    let back = browser::back(shell, picker);
    let header = ComboHeader {
        title: Some(browser::title(shell, picker)),
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
    let content = ComboboxContent::new(
        "schedule-form-picker",
        &header,
        &offered.rows,
        ComboboxWidth::Content,
    );
    if picker == Picker::Model && ui.browsing {
        let empty = browser::empty(shell, visit);
        kit::combobox_with_empty(content, &ui.picker, &handlers, &empty, tokens, window, cx)
    } else {
        kit::combobox_with(content, &ui.picker, &handlers, tokens, window, cx)
    }
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
        if !choose_bootstrap(shell, expected, choice, window, cx) {
            return;
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
                if shell.areas.schedules.data.editor.project(visit, project) {
                    super::preferences::after_project(shell, visit);
                }
            }
            Choice::Agent(harness) => {
                browser::drill(shell, visit, harness, window, cx);
                keep_open = true;
            }
            Choice::Model { request, choice } => {
                let _changed = shell
                    .areas
                    .schedules
                    .data
                    .editor
                    .choose_model(visit, &request, choice);
            }
            Choice::Option { request, choice } => {
                let model = &mut shell.areas.schedules.data.editor;
                let valid = model.selected_options_request() == Some(&request);
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

fn choose_bootstrap(
    shell: &mut Shell,
    expected: &Weak<()>,
    choice: Choice,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> bool {
    if let Choice::Zone(zone) = choice {
        let Some(value) = shell.areas.schedules.ui.editor.bootstrap.take() else {
            return false;
        };
        if !Weak::ptr_eq(&Rc::downgrade(&value.cycle), expected) {
            shell.areas.schedules.ui.editor.bootstrap = Some(value);
            return false;
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
    true
}
