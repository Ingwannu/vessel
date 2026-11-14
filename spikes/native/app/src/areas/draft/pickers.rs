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
    AnyElement, Context, Div, FluentBuilder as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Leading, PillParts, Side, Tokens,
};
use vessel_app_view::{
    Agent, Controls, Machine, Reach, SelectControl, agent_name, controls_with_effort,
};
use vessel_proto::{HarnessRef, OptionChoice, OptionId};

use super::density::Density;
use super::features::feature_name;
use super::{ControlsTarget, Shown, pill_with};
use crate::areas::workspace::{PlacePick, PlaceRows};
use crate::shell::Shell;
use vessel_app_view::extras::{Extra, extra_of};
use vessel_app_view::offers::{Freshness, Offered};

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
    let name = shell.controls_machine()?;
    online.iter().find(|machine| machine.name == name).copied()
}

/// What a machine's agent offers, laid out, with the draft's choices on
/// it when it is the draft's agent.
fn controls_for(shell: &Shell, machine: &str, agent: &HarnessRef) -> Option<Controls> {
    // A running session's controls are its own reported options, and no
    // other agent is a choice there (the session's agent is fixed).
    if let ControlsTarget::Session {
        machine: on,
        agent: of,
        ..
    } = shell.controls_target()
    {
        return (on == machine && of == *agent)
            .then(|| shell.session_controls(agent))
            .flatten();
    }
    let options = shell.offer_options(machine, agent)?;
    let current = shell.areas.draft.agent.as_ref() == Some(agent);
    let options = if current {
        vessel_app_view::with_picks(options, &shell.effective_picks())
    } else {
        options.to_vec()
    };
    Some(controls_with_effort(
        &options,
        super::kind_of(agent),
        &agent_name(agent),
        shell.t.lang,
    ))
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
        let session = match self.controls_target() {
            ControlsTarget::Session { agent, .. } => Some(agent),
            ControlsTarget::Draft => None,
        };
        // Opening one of the draft agent's lists refreshes its list behind
        // what is shown (owner word 28; the store bounds the requests).
        if session.is_none() && matches!(which, Picker::Thinking | Picker::Mode | Picker::Features)
        {
            self.probe(true);
        }
        let search = match which {
            // A running session's list is its agent's models: nothing to
            // probe, no other agent to browse.
            Picker::Model if session.is_some() => Some(words.search_models),
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
                // Every agent's list behind the one shown (Paseo refetches
                // its whole snapshot when the selector opens).
                if let Some(name) = name {
                    for agent in agents {
                        self.probe_agent((name.clone(), agent), true);
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
        let leading = (which == Picker::Model)
            .then(|| {
                session
                    .as_ref()
                    .or(self.areas.draft.browsing.as_ref())
                    .map(crate::agent_mark::leading)
            })
            .flatten();
        list_of(self, which).set_header_leading(leading);
        list_of(self, which).open(search, window, cx);
        cx.notify();
    }

    fn close_picker(&mut self, which: Picker, window: &mut Window, cx: &mut Context<'_, Self>) {
        list_of(self, which).close();
        self.focus_controls_composer(window, cx);
        cx.notify();
    }
}

/// The controls in the card: the model-and-effort control (the owner's,
/// `effort_card.rs`), then the mode control, at the step of Paseo's ladder
/// the card's width allows (`density.rs`). Every other option an agent
/// sends is at the top of the model's list (`model_rows`).
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
    let (model, agent) = model_words(shell, shown);
    let words = super::effort_card::pill_words(ready, model, agent, tokens);
    let density = ready.map_or(Density::Full, |controls| {
        draft.features.density(
            controls,
            &super::features::ModelWords {
                model: &words.model,
                agent: words.agent.as_deref(),
                effort: words.effort.as_ref().map(|(word, _)| word.as_str()),
            },
            touch,
            window,
        )
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
    let mut parts = super::effort_card::pill_parts(
        &words,
        caret,
        density.agent_label(),
        density.effort_label(),
    );
    if !words.fast {
        parts.agent_mark = shell
            .controls_agent()
            .as_ref()
            .and_then(crate::agent_mark::of);
    }
    let open = lists.model.is_open() || draft.effort.open;
    let pill = vessel_app_kit::pill_parts("pick-model", parts, open, touch, tokens)
        .relative()
        .child(lists.model.anchor())
        // Tooltips are absent on touch; the effort card still needs the
        // trigger's measured bounds there to open from the model pill.
        .when(touch, |pill| {
            pill.child(vessel_app_kit::picker::anchor_probe(&draft.features.places[0]))
        })
        .on_click(cx.listener(|this, _event, window, cx| this.press_model_control(window, cx)));
    let hint = shell.t.lang.options().hint_model;
    row = row.child(
        draft
            .features
            .tips
            .on(pill, &draft.features.places[0], hint, touch),
    );
    let Some(controls) = ready else {
        return row;
    };
    if let Some(mode) = &controls.permissions {
        let pill = pill_with(
            "pick-mode",
            &lists.mode,
            PillParts {
                leading: Some(crate::areas::composer::modes::pill_glyph(
                    shell.controls_agent().as_ref().and_then(super::kind_of),
                    &mode.current,
                )),
                label: density
                    .mode_label()
                    .then(|| mode.current_name().to_owned().into()),
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
            &draft.features.places[1],
            shell.t.lang.options().hint_mode,
            touch,
        ));
    }
    row
}

/// The model control's words: the model in use and, when the machine has
/// several agents, the agent's name (quieter).
fn model_words(shell: &Shell, shown: Option<&Shown>) -> (String, Option<String>) {
    let t = shell.t;
    let words = t.lang.options();
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let several = machine(shell, &online).is_some_and(|machine| machine.agents.len() > 1);
    match (&shell.controls_agent(), shown) {
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
    /// Open one of the draft agent's options' own list (a choice at the
    /// top of the model's list).
    Feature(OptionId),
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
            note: None,
            // The controls in the card open above it, those under it below
            // (Paseo's `top-start` and `bottom-start`).
            // An option's list opens from the model control, above it.
            side: match which {
                Picker::Machine | Picker::Folder | Picker::Isolation | Picker::Base => Side::Below,
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

/// The agent in use, the agent whose models the list shows, and whether
/// the controls are a running session's: there the list is its agent's
/// models (it cannot change agent); on the draft, the agent browsed.
fn browsed(shell: &Shell) -> (Option<HarnessRef>, Option<HarnessRef>, bool) {
    let in_use = shell.controls_agent();
    let session = matches!(shell.controls_target(), ControlsTarget::Session { .. });
    let browsing = if session {
        in_use.clone()
    } else {
        shell.areas.draft.browsing.clone()
    };
    (in_use, browsing, session)
}

/// What the browsed agent's model list shows: on a running session its
/// own report (never the store's), else the store's (track P's
/// `areas::composer::offers`).
fn browsed_offer<'a>(
    shell: &'a Shell,
    machine: &str,
    agent: &HarnessRef,
    session: bool,
) -> Offered<'a> {
    if session {
        Offered::Ready {
            options: &[],
            freshness: Freshness::Fresh,
        }
    } else {
        shell.offered(machine, agent)
    }
}

/// The line above a list kept from before whose refresh failed: said once,
/// quietly (owner word 28).
fn stale_note(shell: &Shell, freshness: Freshness) -> Option<vessel_app_kit::ui::SharedString> {
    (freshness == Freshness::Stale).then(|| shell.words.composer().offers_stale().to_owned().into())
}

fn model_rows(shell: &Shell, online: &[&Machine], query: &str, built: &mut Built) {
    let words = shell.t.lang.options();
    built.width = MODEL_WIDTH;
    built.header.search = true;
    let Some(here) = machine(shell, online) else {
        built.header.empty = words.no_hosts.into();
        return;
    };
    // The draft agent's other options first, while not searching and not
    // browsing another agent's models.
    let (in_use, browsing, session) = browsed(shell);
    let browsing_other = browsing
        .as_ref()
        .is_some_and(|browsing| in_use.as_ref() != Some(browsing));
    let on_top = if query.is_empty() && !browsing_other {
        option_rows_on_top(shell, built)
    } else {
        0
    };
    let first_model = built.rows.len();
    let single = here.agents.len() == 1;
    if let Some(agent) = browsing {
        // One agent's models (Paseo's provider view).
        built.header.title = Some(agent_name(&agent).into());
        built.header.back = !single && !session;
        match browsed_offer(shell, &here.name, &agent, session) {
            Offered::Loading => built.header.empty = words.loading.into(),
            Offered::Failed(why) => {
                built.header.empty = format!("{} — {why}", words.error).into();
            }
            Offered::Ready { freshness, .. } => {
                built.header.note = stale_note(shell, freshness);
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
                    let in_use = in_use.as_ref() == Some(&agent);
                    for row in &mut built.rows {
                        row.checked = row.checked && in_use;
                    }
                } else {
                    let mut row = ComboRow::new(words.default_model);
                    row.checked = in_use.as_ref() == Some(&agent);
                    built.rows.push(row);
                    built.meanings.push(Meaning::Agent(agent.clone(), None));
                }
            }
        }
        mark_model_rows(&mut built.rows, first_model, &agent);
        heading_after_options(on_top, first_model, words.model, built);
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
                    row.leading = Some(crate::agent_mark::leading(&agent.harness));
                    row.checked = row.checked && in_use.as_ref() == Some(&agent.harness);
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
    heading_after_options(on_top, first_model, words.model, built);
}

fn mark_model_rows(rows: &mut [ComboRow], first: usize, agent: &HarnessRef) {
    for row in rows.iter_mut().skip(first) {
        row.leading = Some(crate::agent_mark::leading(agent));
    }
}

/// Under the options at the top, the models' heading.
fn heading_after_options(on_top: usize, first: usize, heading: &str, built: &mut Built) {
    if on_top > 0
        && let Some(row) = built.rows.get_mut(first)
    {
        row.heading = Some(heading.to_owned().into());
    }
}

/// The draft agent's options the effort control does not drive, as rows
/// at the top of the model's list (where the owner's reference has its
/// one switch), in the app's words (`view::extras`): Claude Code's
/// answering agent, a choice opening its own list; Codex's plan mode, a
/// switch. Each says in a line what it does; an option the app has no
/// words for gets no row. How many rows were added.
fn option_rows_on_top(shell: &Shell, built: &mut Built) -> usize {
    let Some(Shown::Ready(controls)) = shell.shown() else {
        return 0;
    };
    let kind = shell.controls_agent().as_ref().and_then(super::kind_of);
    let words = shell.t.lang.options();
    let before = built.rows.len();
    for select in &controls.selects {
        if controls
            .slider
            .as_ref()
            .is_some_and(|effort| effort.drives(&select.id))
        {
            continue;
        }
        let Some(extra) = extra_of(kind, &select.id) else {
            continue;
        };
        match extra {
            Extra::AnsweringAgent => {
                let mut row = ComboRow::new(words.answering_agent);
                row.note = Some(words.answering_agent_note.into());
                row.trailing = Some(
                    value_words(
                        select.current.as_str(),
                        select.current_name(),
                        words.default_model,
                    )
                    .into(),
                );
                row.opens = true;
                built.rows.push(row);
                built.meanings.push(Meaning::Feature(select.id.clone()));
            }
            Extra::PlanMode => {
                let Some((on_value, off_value)) = extra.switch_values() else {
                    continue;
                };
                let on = select.current.as_str() == on_value;
                let mut row = ComboRow::new(words.plan_mode);
                row.note = Some(words.plan_mode_note.into());
                row.switch = Some(on);
                built.rows.push(row);
                let next = if on { off_value } else { on_value };
                built.meanings.push(Meaning::Choose(
                    select.choose(vessel_proto::OptionValueId::new(next)),
                    false,
                ));
            }
        }
    }
    built.rows.len().saturating_sub(before)
}

/// A value as a list says it: the agent's "default" in the app's words,
/// any other by the agent's name for it.
fn value_words<'a>(id: &str, name: &'a str, default: &'a str) -> &'a str {
    if id == vessel_app_view::extras::DEFAULT_VALUE {
        default
    } else {
        name
    }
}

/// An agent's row in the model selector: its count of models, "Loading",
/// "Error", or why it cannot start (disabled).
fn agent_row(shell: &Shell, machine: &str, agent: &Agent) -> ComboRow {
    let words = shell.t.lang.options();
    let mut row = ComboRow::new(agent.name.clone());
    row.leading = Some(crate::agent_mark::leading(&agent.harness));
    row.checked = shell.controls_agent().as_ref() == Some(&agent.harness);
    if agent.unavailable.is_some() {
        // The app's own short words, not the node's sentence (which a
        // narrow list cuts, in English in a Korean list).
        row.disabled = true;
        row.trailing = Some(
            if agent.not_installed {
                words.not_installed
            } else {
                words.unavailable
            }
            .into(),
        );
        return row;
    }
    row.opens = true;
    row.trailing = Some(
        match shell.offered(machine, &agent.harness) {
            Offered::Loading => words.loading_short.to_owned(),
            Offered::Failed(_) => words.error.to_owned(),
            Offered::Ready { .. } => controls_for(shell, machine, &agent.harness)
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
                let first = built.rows.len();
                value_rows(
                    mode,
                    query,
                    None,
                    &|choice| Meaning::Choose(choice, true),
                    built,
                );
                // Paseo's `ModeComboboxOption`: the mode's mark and its
                // name, no description (track P, after the coordinator's
                // review of the session's menu).
                let kind = shell.controls_agent().as_ref().and_then(super::kind_of);
                let rows = built.rows.iter_mut().skip(first);
                for (row, meaning) in rows.zip(built.meanings.iter().skip(first)) {
                    row.detail = None;
                    if let Meaning::Choose(choice, _) = meaning
                        && let vessel_proto::ChoiceValue::Select { value, .. } = &choice.value
                    {
                        row.leading = crate::areas::composer::modes::row_glyph(kind, value)
                            .map(Leading::Mark);
                    }
                }
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

/// One option's list (Paseo's combobox of the option's values, with its
/// `header`), opened from its row at the top of the model's list: its
/// name above and no search. "켜짐" and "꺼짐" for an on/off option (its
/// row's switch flips it; the list is there for completeness), the
/// values' names for a choice, the one in effect checked; a choice closes
/// the list.
fn feature_list(shell: &Shell, controls: &Controls, id: &OptionId, query: &str, built: &mut Built) {
    let words = shell.t.lang.options();
    built.header.search = false;
    if let Some(toggle) = controls.toggles.iter().find(|toggle| &toggle.id == id) {
        built.header.title = Some(feature_name(&toggle.name, &toggle.id).to_owned().into());
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
        let kind = shell.controls_agent().as_ref().and_then(super::kind_of);
        let title = match extra_of(kind, &select.id) {
            Some(Extra::AnsweringAgent) => words.answering_agent.to_owned(),
            Some(Extra::PlanMode) => words.plan_mode.to_owned(),
            None => feature_name(&select.name, &select.id).to_owned(),
        };
        built.header.title = Some(title.into());
        for value in &select.values {
            let shown = value_words(value.id.as_str(), &value.name, words.default_model);
            if !matches(shown, query) {
                continue;
            }
            let mut row = ComboRow::new(shown.to_owned());
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
        // The app's word for the state, not the transport's text (the
        // sidebar's line says what to do and its tooltip keeps the
        // transport's); the state, as the line would not fit the row.
        if matches!(machine.reach, Reach::Unreachable(_)) {
            row.disabled = true;
            row.detail = Some(shell.t.unreachable.into());
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
            list_of(shell, which).set_header_leading(Some(crate::agent_mark::leading(&agent)));
            shell.areas.draft.browsing = Some(agent);
            let words = shell.t.lang.options();
            list_of(shell, which).reset_query(words.search_models, window, cx);
        }
        Meaning::Agent(agent, choice) => {
            // A running session keeps its agent; only its option changes.
            let draft = matches!(shell.controls_target(), ControlsTarget::Draft);
            if draft && shell.areas.draft.agent.as_ref() != Some(&agent) {
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
        Meaning::Feature(id) => shell.open_feature(id, window, cx),
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
                            list_of(shell, Picker::Model).set_header_leading(None);
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
