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
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Leading, PillParts, Side, Tokens, Type,
};
use vessel_app_view::{Agent, Controls, Machine, Reach, SelectControl, agent_name, controls_of};
use vessel_proto::{HarnessRef, OptionChoice, OptionId};

use super::density::Density;
use super::features::feature_name;
use super::{Probe, Shown, pill_with};
use crate::areas::workspace::{PlacePick, PlaceRows};
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
    /// The chosen project's isolation (track F's `workspace::place`).
    Isolation,
    /// A new worktree's starting branch (track F's).
    Base,
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

pub(super) fn list_of(shell: &mut Shell, which: Picker) -> &mut ComboboxState {
    let lists = &mut shell.areas.draft.lists;
    match which {
        Picker::Model => &mut lists.model,
        Picker::Thinking => &mut lists.thinking,
        Picker::Mode => &mut lists.mode,
        Picker::Features => &mut lists.features,
        Picker::Machine => &mut lists.machine,
        Picker::Folder => &mut lists.folder,
        Picker::Isolation => &mut shell.areas.workspace.place.isolation,
        Picker::Base => &mut shell.areas.workspace.place.base,
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
        Picker::Isolation => &shell.areas.workspace.place.isolation,
        Picker::Base => &shell.areas.workspace.place.base,
    }
}

pub(super) const ALL: [Picker; 8] = [
    Picker::Model,
    Picker::Thinking,
    Picker::Mode,
    Picker::Features,
    Picker::Machine,
    Picker::Folder,
    Picker::Isolation,
    Picker::Base,
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
            // With projects in the list, Paseo's project picker's search.
            Picker::Folder => Some(if self.project_rows("").is_empty() {
                words.search_folders
            } else {
                self.words().labels().search_places
            }),
            Picker::Base => Some(self.words().labels().search_branches),
            // A feature's list opens through `open_feature`, which says
            // where and with what.
            // Isolation's two rows need no search (Paseo's has none).
            Picker::Thinking | Picker::Features | Picker::Isolation => None,
        };
        list_of(self, which).open(search, window, cx);
        cx.notify();
    }

    fn close_picker(&mut self, which: Picker, window: &mut Window, cx: &mut Context<'_, Self>) {
        list_of(self, which).close();
        // Back to the features sheet when the list was opened from it.
        if !self.areas.draft.features.sheet {
            self.areas.draft.composer.focus(window, cx);
        }
        cx.notify();
    }
}

/// The controls in the card: the model selector, then thinking, mode and
/// features when the agent offers them (Paseo's order), at the step of
/// Paseo's ladder the card's width allows (`density.rs`).
pub(super) fn controls(
    shell: &Shell,
    shown: Option<&Shown>,
    touch: bool,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let draft = &shell.areas.draft;
    let lists = &draft.lists;
    let ready = match shown {
        Some(Shown::Ready(controls)) => Some(controls.as_ref()),
        Some(Shown::Loading | Shown::Failed(_)) | None => None,
    };
    let density = ready.map_or(Density::Full, |controls| {
        let (model, agent) = model_words(shell, shown);
        let measure = |text: &str| vessel_app_kit::fonts::measure(window, text, Type::Body);
        // The model, and the gap before the agent with the agent.
        let agent = agent
            .as_deref()
            .map_or(0.0, |agent| [WORD_GAP, measure(agent)].iter().sum());
        let width = [measure(&model), agent].iter().sum();
        draft.features.density(controls, width, touch, window)
    });
    let caret = density.carets();
    // The row takes the card's width less the send button, and measures
    // it for the next frame (Paseo's `onLayout`).
    let mut row = div()
        .relative()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        // One line, as Paseo's. The ladder drops a step before any
        // control's words would be cut (`density`); only its last step
        // cuts the model's.
        .overflow_hidden()
        .child(draft.features.measure());
    row = row.child(model_pill(shell, shown, caret, touch, tokens, cx));
    let Some(controls) = ready else {
        return row;
    };
    if let Some(effort) = &controls.effort {
        let pill = pill_with(
            "pick-thinking",
            &lists.thinking,
            PillParts {
                leading: Some(Glyph::Brain),
                label: density
                    .thinking_label()
                    .then(|| effort.current_name().to_owned().into()),
                quiet: None,
                caret,
                ..PillParts::default()
            },
            touch,
            tokens,
        )
        .on_click(cx.listener(|this, _event, window, cx| {
            this.open_picker(Picker::Thinking, window, cx);
        }));
        row = row.child(draft.features.tips.on(
            pill,
            &draft.features.places[1],
            shell.t.lang.options().hint_thinking,
            touch,
        ));
    }
    if let Some(mode) = &controls.permissions {
        let pill = pill_with(
            "pick-mode",
            &lists.mode,
            PillParts {
                leading: Some(Glyph::Shield),
                label: density
                    .mode_label()
                    .then(|| mode.current_name().to_owned().into()),
                quiet: None,
                caret,
                ..PillParts::default()
            },
            touch,
            tokens,
        )
        .on_click(
            cx.listener(|this, _event, window, cx| this.open_picker(Picker::Mode, window, cx)),
        );
        row = row.child(draft.features.tips.on(
            pill,
            &draft.features.places[2],
            shell.t.lang.options().hint_mode,
            touch,
        ));
    }
    row.children(super::features::controls(
        shell, controls, density, touch, tokens, cx,
    ))
}

/// The kit's pill's gap between its words.
const WORD_GAP: f32 = 4.0;

/// The model control's words: the model in use and, when the machine has
/// several agents, the agent's name (quieter).
fn model_words(shell: &Shell, shown: Option<&Shown>) -> (String, Option<String>) {
    let t = shell.t;
    let words = t.lang.options();
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let several = machine(shell, &online).is_some_and(|machine| machine.agents.len() > 1);
    match (&shell.areas.draft.agent, shown) {
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
    }
}

/// The model selector's pill: the model in use and, when the machine has
/// several agents, the agent's name quieter.
fn model_pill(
    shell: &Shell,
    shown: Option<&Shown>,
    caret: bool,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Stateful<Div> {
    let words = shell.t.lang.options();
    let draft = &shell.areas.draft;
    let (label, quiet) = model_words(shell, shown);
    let pill = pill_with(
        "pick-model",
        &draft.lists.model,
        PillParts {
            leading: Some(Glyph::Bot),
            label: Some(label.into()),
            quiet: quiet.map(Into::into),
            caret,
            ..PillParts::default()
        },
        touch,
        tokens,
    )
    .on_click(cx.listener(|this, _event, window, cx| this.open_picker(Picker::Model, window, cx)));
    let features = &draft.features;
    features
        .tips
        .on(pill, &features.places[0], words.hint_model, touch)
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
    // A project chosen (track F's `workspace::place`): the pill is the
    // project's (its mark and name), then Paseo's Isolation and Branch.
    let pills = shell.place_pills();
    let folder_parts = pills.as_ref().map_or_else(
        || PillParts {
            leading: Some(Glyph::Folder),
            label: Some(if draft.folder.trim().is_empty() {
                t.home_folder.into()
            } else {
                draft.folder.clone().into()
            }),
            caret: true,
            ..PillParts::default()
        },
        |pills| PillParts {
            initial: Some(pills.initial.clone()),
            label: Some(pills.name.clone()),
            caret: true,
            ..PillParts::default()
        },
    );
    let isolation = pills.as_ref().and_then(|pills| pills.isolation);
    let base = pills.and_then(|pills| pills.base);
    // Each says what it is on hover too (Paseo's tooltips on the
    // new-workspace fields), in the composer's tooltips.
    let labels = shell.words().labels();
    let tips = &draft.features.tips;
    let places = &shell.areas.workspace.place.tips;
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
                PillParts {
                    leading: Some(Glyph::Machine),
                    label: Some(machine_label.into()),
                    quiet: None,
                    caret: true,
                    ..PillParts::default()
                },
                touch,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| {
                this.open_picker(Picker::Machine, window, cx);
            })),
        )
        .child(
            tips.on(
                pill_with(
                    "pick-folder",
                    &draft.lists.folder,
                    folder_parts,
                    touch,
                    tokens,
                )
                .on_click(cx.listener(|this, _event, window, cx| {
                    this.open_picker(Picker::Folder, window, cx);
                })),
                &places[0],
                labels.tip_place,
                touch,
            ),
        )
        .children(project_pills(shell, isolation, base, touch, tokens, cx))
}

/// A chosen project's Isolation and Branch pills (track F's words), each
/// with its tooltip.
fn project_pills(
    shell: &Shell,
    isolation: Option<(&'static str, Glyph)>,
    base: Option<vessel_app_kit::ui::SharedString>,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl Iterator<Item = vessel_app_kit::ui::Stateful<Div>> {
    let labels = shell.words().labels();
    let tips = &shell.areas.draft.features.tips;
    let places = &shell.areas.workspace.place.tips;
    let isolation = isolation.map(|(label, leading)| {
        let pill = pill_with(
            "pick-isolation",
            &shell.areas.workspace.place.isolation,
            PillParts {
                leading: Some(leading),
                label: Some(label.into()),
                caret: true,
                ..PillParts::default()
            },
            touch,
            tokens,
        )
        .on_click(cx.listener(|this, _event, window, cx| {
            this.open_picker(Picker::Isolation, window, cx);
        }));
        tips.on(pill, &places[1], labels.tip_isolation, touch)
    });
    let base = base.map(|label| {
        let pill = pill_with(
            "pick-base",
            &shell.areas.workspace.place.base,
            PillParts {
                leading: Some(Glyph::Branch),
                // "Start from main": the branch alone could be the
                // one the chat runs on.
                prefix: Some(labels.start_from.into()),
                label: Some(label),
                caret: true,
                ..PillParts::default()
            },
            touch,
            tokens,
        )
        .on_click(cx.listener(|this, _event, window, cx| {
            this.open_picker(Picker::Base, window, cx);
        }));
        tips.on(pill, &places[2], labels.tip_base, touch)
    });
    isolation.into_iter().chain(base)
}

/// What a row of a list means.
#[derive(Debug, Clone)]
enum Meaning {
    /// Open an agent's models.
    Browse(HarnessRef),
    /// Start on this agent, with this choice (none: its default model).
    Agent(HarnessRef, Option<OptionChoice>),
    /// One of the draft agent's options; `close` the list after it.
    Choose(OptionChoice, bool),
    Machine(String),
    Folder(String),
    /// A project, its isolation or its starting branch (track F's).
    Place(PlacePick),
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
            // The controls in the card open above it, those under it below
            // (Paseo's `top-start` and `bottom-start`).
            // A feature's list from a row of the features sheet opens below
            // it (Paseo's default, `bottom-start`).
            side: match which {
                Picker::Machine | Picker::Folder | Picker::Isolation | Picker::Base => Side::Below,
                Picker::Features if shell.areas.draft.features.sheet => Side::Below,
                Picker::Model | Picker::Thinking | Picker::Mode | Picker::Features => Side::Above,
            },
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
        Picker::Isolation => place_rows(shell.isolation_rows(), &mut built),
        Picker::Base => {
            let (rows, empty) = shell.base_rows(&query);
            built.header.search = true;
            built.header.empty = empty.into();
            place_rows(rows, &mut built);
        }
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
                    // Paseo checks a model only under the agent in use: the
                    // other agents' own defaults are not a choice made here.
                    let in_use = shell.areas.draft.agent.as_ref() == Some(&agent);
                    for row in &mut built.rows {
                        row.checked = row.checked && in_use;
                    }
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
        Picker::Features
        | Picker::Model
        | Picker::Machine
        | Picker::Folder
        | Picker::Isolation
        | Picker::Base => {
            if let Some(id) = &shell.areas.draft.feature {
                feature_list(shell, &controls, id, query, built);
            }
        }
    }
}

/// One feature's list (Paseo's combobox of the feature's options): from
/// its control, a search row and no title; from the features sheet, its
/// name above and no search. "켜짐" and "꺼짐" for an on/off feature (only
/// the sheet opens one: its control switches it), the option names alone
/// for a choice, the one in effect checked; a choice closes the list.
fn feature_list(shell: &Shell, controls: &Controls, id: &OptionId, query: &str, built: &mut Built) {
    let words = shell.t.lang.options();
    let from_sheet = shell.areas.draft.features.sheet;
    built.header.search = !from_sheet;
    if let Some(toggle) = controls.toggles.iter().find(|toggle| &toggle.id == id) {
        if from_sheet {
            built.header.title = Some(feature_name(&toggle.name, &toggle.id).to_owned().into());
        }
        for (on, label) in [(true, words.setting_on), (false, words.setting_off)] {
            if !matches(label, query) {
                continue;
            }
            let mut row = ComboRow::new(label);
            row.checked = toggle.on == on;
            built.rows.push(row);
            built
                .meanings
                .push(Meaning::Choose(toggle.choose(on), true));
        }
    } else if let Some(select) = controls.selects.iter().find(|select| &select.id == id) {
        if from_sheet {
            built.header.title = Some(feature_name(&select.name, &select.id).to_owned().into());
        }
        for value in &select.values {
            if !matches(&value.name, query) {
                continue;
            }
            let mut row = ComboRow::new(value.name.clone());
            row.checked = value.id == select.current;
            built.rows.push(row);
            built
                .meanings
                .push(Meaning::Choose(select.choose(value.id.clone()), true));
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

/// Track F's rows, as they come.
fn place_rows(rows: PlaceRows, built: &mut Built) {
    for (row, pick) in rows {
        built.rows.push(row);
        built.meanings.push(Meaning::Place(pick));
    }
}

/// The machine's projects (track F's rows, Paseo's project picker), then
/// the folders under their own heading, then "Add project".
fn folder_rows(shell: &Shell, query: &str, built: &mut Built) {
    let t = shell.t;
    let words = t.lang.options();
    built.header.search = true;
    built.header.empty = words.no_folders.into();
    let projects = shell.project_rows(query);
    let first_folder = projects.len();
    let with_projects = !projects.is_empty();
    place_rows(projects, built);
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
    if with_projects && let Some(row) = built.rows.get_mut(first_folder) {
        row.heading = Some(shell.words().labels().folders.into());
    }
    if let Some((row, pick)) = shell.add_project_row() {
        built.rows.push(row);
        built.meanings.push(Meaning::Place(pick));
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
            }
        }
        Meaning::Machine(name) => {
            list_of(shell, which).close();
            shell.choose_machine(name, window, cx);
        }
        Meaning::Folder(folder) => {
            shell.choose_folder(folder);
            shell.forget_place();
            shell.close_picker(which, window, cx);
        }
        Meaning::Place(pick) => {
            shell.close_picker(which, window, cx);
            shell.choose_place(pick, window, cx);
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
            Picker::Isolation => |shell: &mut Shell| list_of(shell, Picker::Isolation),
            Picker::Base => |shell: &mut Shell| list_of(shell, Picker::Base),
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
            Picker::Thinking
            | Picker::Mode
            | Picker::Features
            | Picker::Machine
            | Picker::Folder
            | Picker::Isolation
            | Picker::Base => None,
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
