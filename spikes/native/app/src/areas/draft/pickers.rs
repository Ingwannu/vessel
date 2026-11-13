//! The new chat's pickers, as Paseo's composer has them
//! (`composer/agent-controls/`, `components/combined-model-selector.tsx`,
//! `components/model-browser.tsx`): the model selector, the thinking
//! control, the mode control and the features control in the card; the
//! machine and the folder under it. Each is the kit's pill and opens the
//! kit's list; the rows are built here from what the machine's agents
//! offer, and a chosen row is matched back to what it means by index.

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{ComboHandlers, ComboHeader, ComboRow, ComboboxState, Leading, Tokens};
use vessel_app_view::{Agent, Controls, Machine, Reach, SelectControl, agent_name, controls_of};
use vessel_proto::{HarnessRef, OptionChoice, OptionId};

use super::{Probe, Shown, pill_with};
use crate::shell::Shell;

/// Which picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picker {
    Model,
    Thinking,
    Mode,
    Features,
    Machine,
    Folder,
}

/// Paseo shows a list's search row past this many rows (its composer's
/// threshold).
const SEARCH_PAST: usize = 6;

/// Paseo's host picker searches past ten hosts.
const SEARCH_HOSTS_PAST: usize = 10;

/// The model selector's width (Paseo's, fixed).
const MODEL_WIDTH: f32 = 360.0;

/// The other lists' width.
const LIST_WIDTH: f32 = 300.0;

fn list_of(shell: &mut Shell, which: Picker) -> &mut ComboboxState {
    let lists = &mut shell.areas.draft.lists;
    match which {
        Picker::Model => &mut lists.model,
        Picker::Thinking => &mut lists.thinking,
        Picker::Mode => &mut lists.mode,
        Picker::Features => &mut lists.features,
        Picker::Machine => &mut lists.machine,
        Picker::Folder => &mut lists.folder,
    }
}

fn list_ref(shell: &Shell, which: Picker) -> &ComboboxState {
    let lists = &shell.areas.draft.lists;
    match which {
        Picker::Model => &lists.model,
        Picker::Thinking => &lists.thinking,
        Picker::Mode => &lists.mode,
        Picker::Features => &lists.features,
        Picker::Machine => &lists.machine,
        Picker::Folder => &lists.folder,
    }
}

const ALL: [Picker; 6] = [
    Picker::Model,
    Picker::Thinking,
    Picker::Mode,
    Picker::Features,
    Picker::Machine,
    Picker::Folder,
];

/// The machine the draft is on, when online.
fn machine<'a>(shell: &'a Shell, online: &[&'a Machine]) -> Option<&'a Machine> {
    let name = shell.areas.draft.machine.as_deref()?;
    online.iter().find(|machine| machine.name == name).copied()
}

/// What a machine's agent offers, laid out, with the draft's choices on
/// it when it is the draft's agent.
fn controls_for(shell: &Shell, machine: &str, agent: &HarnessRef) -> Option<Controls> {
    let key = (machine.to_owned(), agent.clone());
    let Some(Probe::Ready(options)) = shell.areas.draft.probes.get(&key) else {
        return None;
    };
    let current = shell.areas.draft.agent.as_ref() == Some(agent);
    let options = if current {
        vessel_app_view::with_picks(options, &shell.effective_picks())
    } else {
        options.clone()
    };
    Some(controls_of(&options, &agent_name(agent), shell.t.lang))
}

impl Shell {
    /// Open `which`'s list (closing any other): the model selector asks
    /// every agent of the machine what it offers, and starts on the agent's
    /// models when the machine has one agent.
    pub(crate) fn open_picker(
        &mut self,
        which: Picker,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let was_open = list_ref(self, which).is_open();
        for other in ALL {
            list_of(self, other).close();
        }
        if was_open {
            cx.notify();
            return;
        }
        let words = self.t.lang.options();
        let search = match which {
            Picker::Model => {
                let online: Vec<&Machine> = self.fleet.online().collect();
                let agents: Vec<HarnessRef> = machine(self, &online)
                    .map(|machine| {
                        machine
                            .agents
                            .iter()
                            .filter(|agent| agent.available())
                            .map(|agent| agent.harness.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let name = self.areas.draft.machine.clone();
                let all_agents = machine(self, &online).map_or(0, |machine| machine.agents.len());
                self.areas.draft.browsing =
                    (all_agents == 1).then(|| agents.first().cloned()).flatten();
                if let Some(name) = name {
                    for agent in agents {
                        self.probe_agent((name.clone(), agent), false);
                    }
                }
                Some(if self.areas.draft.browsing.is_some() {
                    words.search_models
                } else {
                    words.search_all_models
                })
            }
            Picker::Mode => self
                .shown()
                .and_then(|shown| match shown {
                    Shown::Ready(controls) => controls.permissions.map(|mode| mode.values.len()),
                    Shown::Loading | Shown::Failed(_) => None,
                })
                .filter(|count| *count > SEARCH_PAST)
                .map(|_| words.search_modes),
            Picker::Machine => {
                (self.fleet.machines.len() > SEARCH_HOSTS_PAST).then_some(words.search_hosts)
            }
            Picker::Folder => Some(words.search_folders),
            Picker::Features => {
                self.areas.draft.feature = None;
                None
            }
            Picker::Thinking => None,
        };
        list_of(self, which).open(search, window, cx);
        cx.notify();
    }

    fn close_picker(&mut self, which: Picker, window: &mut Window, cx: &mut Context<'_, Self>) {
        list_of(self, which).close();
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }
}

/// The controls in the card: the model selector, then thinking, mode and
/// features when the agent offers them (Paseo's order).
pub(super) fn controls(
    shell: &Shell,
    shown: Option<&Shown>,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let words = t.lang.options();
    let lists = &shell.areas.draft.lists;
    let mut row = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(4.0));
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let several = machine(shell, &online).is_some_and(|machine| machine.agents.len() > 1);
    let (label, quiet) = match (&shell.areas.draft.agent, shown) {
        (None, _) => (t.agent.to_owned(), None),
        (Some(_), Some(Shown::Loading) | None) => (words.loading.to_owned(), None),
        (Some(agent), Some(Shown::Failed(_))) => (agent_name(agent), None),
        (Some(agent), Some(Shown::Ready(controls))) => match &controls.model {
            Some(model) => (
                model.current_name().to_owned(),
                several.then(|| agent_name(agent)),
            ),
            None => (agent_name(agent), None),
        },
    };
    row = row.child(
        pill_with(
            "pick-model",
            &lists.model,
            Some(Glyph::Bot),
            label,
            quiet,
            touch,
            tokens,
        )
        .on_click(
            cx.listener(|this, _event, window, cx| this.open_picker(Picker::Model, window, cx)),
        ),
    );
    let Some(Shown::Ready(controls)) = shown else {
        return row;
    };
    if let Some(effort) = &controls.effort {
        row = row.child(
            pill_with(
                "pick-thinking",
                &lists.thinking,
                Some(Glyph::Brain),
                effort.current_name().to_owned(),
                None,
                touch,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_picker(Picker::Thinking, window, cx);
            })),
        );
    }
    if let Some(mode) = &controls.permissions {
        row = row.child(
            pill_with(
                "pick-mode",
                &lists.mode,
                Some(Glyph::Shield),
                mode.current_name().to_owned(),
                None,
                touch,
                tokens,
            )
            .on_click(
                cx.listener(|this, _event, window, cx| this.open_picker(Picker::Mode, window, cx)),
            ),
        );
    }
    if !controls.toggles.is_empty() || !controls.selects.is_empty() {
        row = row.child(
            pill_with(
                "pick-features",
                &lists.features,
                None,
                words.features.to_owned(),
                None,
                touch,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_picker(Picker::Features, window, cx);
            })),
        );
    }
    row
}

/// Under the card: the machine and the folder.
pub(super) fn place(
    shell: &Shell,
    online: &[&Machine],
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let draft = &shell.areas.draft;
    let machine_label =
        machine(shell, online).map_or_else(|| t.machine.to_owned(), |machine| machine.name.clone());
    let folder_label = if draft.folder.trim().is_empty() {
        t.home_folder.to_owned()
    } else {
        draft.folder.clone()
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(4.0))
        .px(px(10.0))
        .child(
            pill_with(
                "pick-machine",
                &draft.lists.machine,
                Some(Glyph::Machine),
                machine_label,
                None,
                touch,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_picker(Picker::Machine, window, cx);
            })),
        )
        .child(
            pill_with(
                "pick-folder",
                &draft.lists.folder,
                Some(Glyph::Folder),
                folder_label,
                None,
                touch,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_picker(Picker::Folder, window, cx);
            })),
        )
}

/// What a row of a list means.
#[derive(Debug, Clone)]
enum Meaning {
    /// Open an agent's models.
    Browse(HarnessRef),
    /// Start on this agent, with this choice (none: its default model).
    Agent(HarnessRef, Option<OptionChoice>),
    /// One of the draft agent's options; `close` the list after it (else
    /// back to the features).
    Choose(OptionChoice, bool),
    /// Open a feature's own list.
    Feature(OptionId),
    Machine(String),
    Folder(String),
}

/// A list's header and rows, and what each row means.
struct Built {
    header: ComboHeader,
    rows: Vec<ComboRow>,
    meanings: Vec<Meaning>,
    width: f32,
}

fn build(shell: &Shell, which: Picker, query: &str, cx: &Context<'_, Shell>) -> Built {
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let words = shell.t.lang.options();
    let mut built = Built {
        header: ComboHeader {
            title: None,
            back: false,
            search: false,
            empty: words.no_matches.into(),
        },
        rows: Vec::new(),
        meanings: Vec::new(),
        width: LIST_WIDTH,
    };
    let query = query.trim().to_lowercase();
    match which {
        Picker::Model => model_rows(shell, &online, &query, &mut built),
        Picker::Thinking | Picker::Mode | Picker::Features => {
            option_rows(shell, which, &query, &mut built);
        }
        Picker::Machine => machine_rows(shell, &query, &vessel_app_kit::tokens(cx), &mut built),
        Picker::Folder => folder_rows(shell, &query, &mut built),
    }
    built
}

fn matches(text: &str, query: &str) -> bool {
    query.is_empty() || text.to_lowercase().contains(query)
}

/// A select's values as rows, under their groups' names, the one in
/// effect checked.
fn value_rows(
    control: &SelectControl,
    query: &str,
    detail: Option<&str>,
    meaning: &dyn Fn(OptionChoice) -> Meaning,
    built: &mut Built,
) {
    let mut group: Option<&str> = None;
    for value in &control.values {
        if !matches(&value.name, query) && !matches(value.id.as_str(), query) {
            continue;
        }
        let mut row = ComboRow::new(value.name.clone());
        row.checked = value.id == control.current;
        row.detail = value
            .description
            .clone()
            .or_else(|| detail.map(str::to_owned))
            .map(Into::into);
        let this_group = value.group.as_deref();
        if this_group != group {
            row.heading = this_group.map(|name| name.to_owned().into());
            group = this_group;
        }
        built.rows.push(row);
        built
            .meanings
            .push(meaning(control.choose(value.id.clone())));
    }
}

fn model_rows(shell: &Shell, online: &[&Machine], query: &str, built: &mut Built) {
    let words = shell.t.lang.options();
    built.width = MODEL_WIDTH;
    built.header.search = true;
    let Some(here) = machine(shell, online) else {
        built.header.empty = words.no_hosts.into();
        return;
    };
    let single = here.agents.len() == 1;
    if let Some(agent) = shell.areas.draft.browsing.clone() {
        // One agent's models (Paseo's provider view).
        built.header.title = Some(agent_name(&agent).into());
        built.header.back = !single;
        let key = (here.name.clone(), agent.clone());
        match shell.areas.draft.probes.get(&key) {
            None | Some(Probe::Loading) => built.header.empty = words.loading.into(),
            Some(Probe::Failed(why)) => {
                built.header.empty = format!("{} — {why}", words.error).into();
            }
            Some(Probe::Ready(_)) => {
                let model =
                    controls_for(shell, &here.name, &agent).and_then(|controls| controls.model);
                if let Some(model) = model {
                    let picked = agent.clone();
                    value_rows(
                        &model,
                        query,
                        None,
                        &|choice| Meaning::Agent(picked.clone(), Some(choice)),
                        built,
                    );
                } else {
                    let mut row = ComboRow::new(words.default_model);
                    row.checked = shell.areas.draft.agent.as_ref() == Some(&agent);
                    built.rows.push(row);
                    built.meanings.push(Meaning::Agent(agent, None));
                }
            }
        }
        return;
    }
    if !query.is_empty() {
        // A search over every agent's models (Paseo's "all" search).
        for agent in here.agents.iter().filter(|agent| agent.available()) {
            if let Some(model) =
                controls_for(shell, &here.name, &agent.harness).and_then(|controls| controls.model)
            {
                let picked = agent.harness.clone();
                let mut rows = Built {
                    header: built.header.clone(),
                    rows: Vec::new(),
                    meanings: Vec::new(),
                    width: MODEL_WIDTH,
                };
                value_rows(
                    &model,
                    query,
                    Some(&agent.name),
                    &|choice| Meaning::Agent(picked.clone(), Some(choice)),
                    &mut rows,
                );
                for row in &mut rows.rows {
                    row.heading = None;
                    row.checked =
                        row.checked && shell.areas.draft.agent.as_ref() == Some(&agent.harness);
                }
                built.rows.extend(rows.rows);
                built.meanings.extend(rows.meanings);
            }
        }
        return;
    }
    // Every agent, as Paseo lists its providers.
    built.header.title = None;
    for agent in &here.agents {
        built.rows.push(agent_row(shell, &here.name, agent));
        built.meanings.push(Meaning::Browse(agent.harness.clone()));
    }
}

/// An agent's row in the model selector: its count of models, "Loading",
/// "Error", or why it cannot start (disabled).
fn agent_row(shell: &Shell, machine: &str, agent: &Agent) -> ComboRow {
    let words = shell.t.lang.options();
    let mut row = ComboRow::new(agent.name.clone());
    row.leading = Some(Leading::Glyph(Glyph::Bot));
    row.checked = shell.areas.draft.agent.as_ref() == Some(&agent.harness);
    if let Some(why) = &agent.unavailable {
        row.disabled = true;
        row.trailing = Some(words.unavailable.into());
        row.detail = Some(why.clone().into());
        return row;
    }
    row.opens = true;
    let key = (machine.to_owned(), agent.harness.clone());
    row.trailing = Some(
        match shell.areas.draft.probes.get(&key) {
            None | Some(Probe::Loading) => words.loading_short.to_owned(),
            Some(Probe::Failed(_)) => words.error.to_owned(),
            Some(Probe::Ready(_)) => controls_for(shell, machine, &agent.harness)
                .and_then(|controls| controls.model)
                .map_or_else(
                    || words.default_model.to_owned(),
                    |model| words.model_count(model.values.len()),
                ),
        }
        .into(),
    );
    row
}

fn option_rows(shell: &Shell, which: Picker, query: &str, built: &mut Built) {
    let words = shell.t.lang.options();
    let Some(Shown::Ready(controls)) = shell.shown() else {
        built.header.empty = words.loading.into();
        return;
    };
    match which {
        Picker::Thinking => {
            if let Some(effort) = &controls.effort {
                value_rows(
                    effort,
                    query,
                    None,
                    &|choice| Meaning::Choose(choice, true),
                    built,
                );
            }
        }
        Picker::Mode => {
            if let Some(mode) = &controls.permissions {
                built.header.search = mode.values.len() > SEARCH_PAST;
                value_rows(
                    mode,
                    query,
                    None,
                    &|choice| Meaning::Choose(choice, true),
                    built,
                );
            }
        }
        Picker::Features | Picker::Model | Picker::Machine | Picker::Folder => {
            match &shell.areas.draft.feature {
                Some(id) => feature_list(shell, &controls, id, built),
                None => feature_rows(shell, &controls, built),
            }
        }
    }
}

/// A feature's name, its id when the agent gave it none.
fn feature_name<'a>(name: &'a str, id: &'a OptionId) -> &'a str {
    if name.trim().is_empty() {
        id.as_str()
    } else {
        name
    }
}

/// The features (Paseo's features sheet, `SheetFeatureItem`): a row per
/// feature with its name and its value ("켜짐", "꺼짐", or the chosen
/// option), each opening its own list. Paseo shows no description there,
/// and an on/off feature is a row like the others, not a switch: it
/// opens "켜짐" and "꺼짐".
fn feature_rows(shell: &Shell, controls: &Controls, built: &mut Built) {
    let words = shell.t.lang.options();
    for toggle in &controls.toggles {
        let mut row = ComboRow::new(feature_name(&toggle.name, &toggle.id).to_owned());
        row.trailing = Some(
            if toggle.on {
                words.setting_on
            } else {
                words.setting_off
            }
            .into(),
        );
        built.rows.push(row);
        built.meanings.push(Meaning::Feature(toggle.id.clone()));
    }
    for select in &controls.selects {
        let mut row = ComboRow::new(feature_name(&select.name, &select.id).to_owned());
        row.trailing = Some(select.current_name().to_owned().into());
        built.rows.push(row);
        built.meanings.push(Meaning::Feature(select.id.clone()));
    }
}

/// One feature's own list (Paseo's pushed combobox): its name above with
/// a way back, no search; "켜짐" and "꺼짐" for an on/off feature, the
/// option names alone for a choice, the one in effect checked.
fn feature_list(shell: &Shell, controls: &Controls, id: &OptionId, built: &mut Built) {
    let words = shell.t.lang.options();
    built.header.back = true;
    if let Some(toggle) = controls.toggles.iter().find(|toggle| &toggle.id == id) {
        built.header.title = Some(feature_name(&toggle.name, &toggle.id).to_owned().into());
        for (on, label) in [(true, words.setting_on), (false, words.setting_off)] {
            let mut row = ComboRow::new(label);
            row.checked = toggle.on == on;
            built.rows.push(row);
            built
                .meanings
                .push(Meaning::Choose(toggle.choose(on), false));
        }
    } else if let Some(select) = controls.selects.iter().find(|select| &select.id == id) {
        built.header.title = Some(feature_name(&select.name, &select.id).to_owned().into());
        for value in &select.values {
            let mut row = ComboRow::new(value.name.clone());
            row.checked = value.id == select.current;
            built.rows.push(row);
            built
                .meanings
                .push(Meaning::Choose(select.choose(value.id.clone()), false));
        }
    }
}

fn machine_rows(shell: &Shell, query: &str, tokens: &Tokens, built: &mut Built) {
    let words = shell.t.lang.options();
    built.header.search = shell.fleet.machines.len() > SEARCH_HOSTS_PAST;
    built.header.empty = words.no_hosts.into();
    for machine in &shell.fleet.machines {
        if !matches(&machine.name, query) {
            continue;
        }
        let mut row = ComboRow::new(machine.name.clone());
        row.leading = Some(Leading::Dot(match &machine.reach {
            Reach::Online => tokens.st_ready,
            Reach::Unreachable(_) => tokens.danger,
        }));
        if let Reach::Unreachable(why) = &machine.reach {
            row.disabled = true;
            row.detail = Some(why.clone().into());
        }
        row.checked = shell.areas.draft.machine.as_deref() == Some(machine.name.as_str());
        built.rows.push(row);
        built.meanings.push(Meaning::Machine(machine.name.clone()));
    }
}

fn folder_rows(shell: &Shell, query: &str, built: &mut Built) {
    let t = shell.t;
    let words = t.lang.options();
    built.header.search = true;
    built.header.empty = words.no_folders.into();
    let current = shell.areas.draft.folder.trim();
    let typed = query.trim();
    let mut used: Vec<&String> = shell.prefs.folders.values().collect();
    used.sort();
    used.dedup();
    if !typed.is_empty() && !used.iter().any(|folder| folder.as_str() == typed) {
        let mut row = ComboRow::new(words.use_folder(typed));
        row.leading = Some(Leading::Glyph(Glyph::Folder));
        built.rows.push(row);
        built.meanings.push(Meaning::Folder(typed.to_owned()));
    }
    if matches(t.home_folder, query) {
        let mut row = ComboRow::new(t.home_folder);
        row.leading = Some(Leading::Glyph(Glyph::Folder));
        row.checked = current.is_empty();
        built.rows.push(row);
        built.meanings.push(Meaning::Folder(String::new()));
    }
    for folder in used {
        if !matches(folder, query) {
            continue;
        }
        let mut row = ComboRow::new(folder.clone());
        row.leading = Some(Leading::Glyph(Glyph::Folder));
        row.checked = current == folder.as_str();
        built.rows.push(row);
        built.meanings.push(Meaning::Folder(folder.clone()));
    }
}

/// What choosing a row of `which`'s list does.
fn chose(
    shell: &mut Shell,
    which: Picker,
    index: usize,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let query = list_ref(shell, which).query(cx);
    let built = build(shell, which, &query, cx);
    let Some(meaning) = built.meanings.get(index).cloned() else {
        return;
    };
    match meaning {
        Meaning::Browse(agent) => {
            if let Some(machine) = shell.areas.draft.machine.clone() {
                shell.probe_agent((machine, agent.clone()), false);
            }
            shell.areas.draft.browsing = Some(agent);
            let words = shell.t.lang.options();
            list_of(shell, which).reset_query(words.search_models, window, cx);
        }
        Meaning::Agent(agent, choice) => {
            if shell.areas.draft.agent.as_ref() != Some(&agent) {
                shell.choose_agent(agent);
            }
            if let Some(choice) = choice {
                shell.choose_option(choice);
            }
            shell.close_picker(which, window, cx);
        }
        Meaning::Choose(choice, close) => {
            shell.choose_option(choice);
            if close {
                shell.close_picker(which, window, cx);
            } else {
                // Paseo closes a feature's list on a choice, back on the
                // features.
                shell.areas.draft.feature = None;
            }
        }
        Meaning::Feature(id) => shell.areas.draft.feature = Some(id),
        Meaning::Machine(name) => {
            list_of(shell, which).close();
            shell.choose_machine(name, window, cx);
        }
        Meaning::Folder(folder) => {
            shell.choose_folder(folder);
            shell.close_picker(which, window, cx);
        }
    }
    cx.notify();
}

/// The open list, if any, over everything.
pub(super) fn open_list(
    shell: &Shell,
    _online: &[&Machine],
    _shown: Option<&Shown>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let which = ALL
        .into_iter()
        .find(|which| list_ref(shell, *which).is_open())?;
    let list = list_ref(shell, which);
    let query = list.query(cx);
    let built = build(shell, which, &query, cx);
    let handlers = ComboHandlers {
        state: match which {
            Picker::Model => |shell: &mut Shell| list_of(shell, Picker::Model),
            Picker::Thinking => |shell: &mut Shell| list_of(shell, Picker::Thinking),
            Picker::Mode => |shell: &mut Shell| list_of(shell, Picker::Mode),
            Picker::Features => |shell: &mut Shell| list_of(shell, Picker::Features),
            Picker::Machine => |shell: &mut Shell| list_of(shell, Picker::Machine),
            Picker::Folder => |shell: &mut Shell| list_of(shell, Picker::Folder),
        },
        chose: Rc::new(
            move |shell: &mut Shell, index, window: &mut Window, cx: &mut Context<'_, Shell>| {
                chose(shell, which, index, window, cx);
            },
        ),
        back: match which {
            Picker::Model => {
                let back: vessel_app_kit::picker::Back<Shell> = Rc::new(
                    |shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>| {
                        if shell.areas.draft.browsing.take().is_some() {
                            let words = shell.t.lang.options();
                            list_of(shell, Picker::Model).reset_query(
                                words.search_all_models,
                                window,
                                cx,
                            );
                        }
                    },
                );
                Some(back)
            }
            Picker::Features => {
                let back: vessel_app_kit::picker::Back<Shell> = Rc::new(
                    |shell: &mut Shell, _window: &mut Window, _cx: &mut Context<'_, Shell>| {
                        shell.areas.draft.feature = None;
                    },
                );
                Some(back)
            }
            Picker::Thinking | Picker::Mode | Picker::Machine | Picker::Folder => None,
        },
    };
    vessel_app_kit::combobox(
        "draft-list",
        list,
        &built.header,
        &built.rows,
        &handlers,
        built.width,
        tokens,
        window,
        cx,
    )
}
