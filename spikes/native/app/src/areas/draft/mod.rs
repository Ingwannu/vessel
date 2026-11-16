//! A new conversation: type, send, it runs (agent-orchestrator's `/new`
//! and GOAT's draft thread, which the owner named; their flow, read from
//! their sources, is reproduced here).
//!
//! As theirs: the app opens on this screen with the composer focused under
//! a heading ("What do you want to work on?"), one of four suggestions as
//! the placeholder; Enter sends, Shift+Enter starts a line; the session is
//! created on the first send, not before; the prompt is shown at once and
//! the conversation opens with it, the reply streaming in; the session is
//! titled after the prompt (the node does that); a send that fails puts
//! the text back.
//!
//! The composer and its controls are Paseo's (the owner on 0.0.0.0.3: "paseo
//! 처럼 … GOAT랑"; `packages/app/src/composer`): the card is the input, with
//! no box or ring inside it (`kit::composer_card`, `TextField::composer`);
//! under the text, in the card, the model selector, the thinking control,
//! the mode control and the features control, in Paseo's order, as pills
//! with a glyph and a caret; the round send button at the right. Each opens
//! the kit's list (`kit::combobox`, on track C's popup surface): the model
//! selector lists every agent of the machine as Paseo lists its providers
//! (each with its count of models, "Loading" while its machine asks it, or
//! why it cannot start, disabled), and an agent opens to its models, with
//! a search; a model chosen under another agent switches to that agent.
//! Under the card, the machine and the folder, as the same pills (Paseo
//! chooses the host on its new-workspace form; Vessel's new chat has no
//! form, so they sit under the card).
//!
//! The agent's options are asked of its machine (`harness.options`,
//! seconds the first time) and placed by the page's rules
//! (`vessel_app_view::controls_of`); a choice is sent only when it changes
//! something, and the choices a start was made with are what the next
//! start on that machine begins from, until the person chooses again.
//!
//! Official provider artwork comes through `app::agent_mark` (ADR 0048);
//! unknown agents and marks whose brand minimum does not fit use Generic.
//! When a machine has more than one agent the pill names the agent after
//! the model. Choices are kept once the options are known; a start while
//! loading retains the preceding choices. Creation rules live in `create`.

mod book;
mod create;
mod density;
mod draw;
mod effort_card;
mod features;
mod pickers;
mod probe;
mod status;
mod submit;
mod workspace;
pub(crate) use draw::{compact, render, running_controls};
pub(crate) use workspace::render_panel as workspace_panel;

use vessel_app_kit::ui::{Context, Div, ParentElement as _, Styled as _, Window};
use vessel_app_kit::{ComboboxState, TextField, Tokens};
use vessel_app_view::offers::Offered;
use vessel_app_view::{Controls, Machine, Picks, agent_name, controls_with_effort, with_picks};
use vessel_proto::{HarnessRef, OptionChoice};

use crate::shell::Shell;

/// What the controls show (the page's `ControlsState`; the app starts no
/// terminals yet, so no terminal case).
pub(crate) enum Shown {
    Loading,
    Failed(String),
    /// Boxed: a few hundred bytes beside a string.
    Ready(Box<Controls>),
}

/// The pickers' lists, one each (each notes where its pill is drawn).
pub(crate) struct Lists {
    pub(crate) model: ComboboxState,
    pub(crate) thinking: ComboboxState,
    pub(crate) mode: ComboboxState,
    pub(crate) features: ComboboxState,
    pub(crate) machine: ComboboxState,
    pub(crate) folder: ComboboxState,
}

/// The draft: the composer, and what is chosen.
pub(crate) struct Draft {
    workspace_drafts: workspace::WorkspaceDrafts,
    /// Correlated progress retained while another route is shown.
    pub(crate) creation: create::CreationState,
    pub(crate) composer: TextField,
    pub(crate) machine: Option<String>,
    pub(crate) agent: Option<HarnessRef>,
    /// The folder on the machine; empty for its home.
    pub(crate) folder: String,
    pub(crate) lists: Lists,
    /// The agent whose models the model selector shows; `None` for the
    /// list of agents.
    pub(crate) browsing: Option<HarnessRef>,
    /// The feature whose list the features list shows.
    pub(crate) feature: Option<vessel_proto::OptionId>,
    /// The row's measures and tooltips.
    pub(crate) features: features::FeatureState,
    /// The model-and-effort control's card.
    pub(crate) effort: effort_card::CardState,
    /// Which of the suggestions is the placeholder (one per launch).
    suggestion: usize,
    /// Why the last send did not go, shown under the composer.
    pub(crate) error: Option<String>,
    /// The text of a send not answered yet, put back if it fails.
    sending: Option<String>,
    /// A failed send's text, put back in the composer on the next frame
    /// (setting a field's text needs the window, which an event from the
    /// link does not have).
    restore: std::cell::RefCell<Option<String>>,
    /// The person's choices for this machine's agent, once `chosen`; until
    /// then the last start's (the page's `startingPicks`).
    picks: Picks,
    chosen: bool,
    /// The machine and agent the choices are for: another starts over.
    settled: Option<(String, HarnessRef)>,
}

/// Whose options the controls show (track P's `areas::composer`).
pub(crate) use crate::areas::composer::ControlsTarget;

impl Draft {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        // One suggestion per launch, as agent-orchestrator picks one per
        // mount; the clock's low bits are random enough for a placeholder.
        // chrono's clock, not `SystemTime`: in a browser (wasm32) std has
        // no clock and panics, chrono reads the page's.
        let nanos = chrono::Utc::now().timestamp_subsec_nanos();
        let suggestion = usize::try_from(nanos % 4).unwrap_or(0);
        Self {
            workspace_drafts: workspace::WorkspaceDrafts::default(),
            creation: create::CreationState::default(),
            composer: TextField::composer(
                "",
                |shell: &mut Shell, text, window, cx| shell.send_draft(text, window, cx),
                window,
                cx,
            ),
            machine: None,
            agent: None,
            folder: String::new(),
            lists: Lists {
                model: ComboboxState::new(window, cx),
                thinking: ComboboxState::new(window, cx),
                mode: ComboboxState::new(window, cx),
                features: ComboboxState::new(window, cx),
                machine: ComboboxState::new(window, cx),
                folder: ComboboxState::new(window, cx),
            },
            browsing: None,
            feature: None,
            features: features::FeatureState::new(),
            effort: effort_card::CardState::new(cx),
            suggestion,
            error: None,
            sending: None,
            restore: std::cell::RefCell::new(None),
            picks: Picks::default(),
            chosen: false,
            settled: None,
        }
    }
}

pub(crate) use probe::kind_of;

/// Focus the composer, fill in the choices, and ask the agent again what
/// it offers (the node answers from its recent probe; what is shown stays
/// until the answer).
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.settle_draft();
    shell.probe(true);
    refresh_words(shell, window, cx);
    shell.areas.draft.composer.focus(window, cx);
}

/// Refresh only the language-dependent placeholder; preserve the draft.
pub(crate) fn refresh_words(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let t = shell.t;
    let draft = &shell.areas.draft;
    let placeholder = t.suggestions.get(draft.suggestion).map_or_else(
        || t.prompt_placeholder(&super::settings::send_key_words(shell)),
        |suggestion| (*suggestion).to_owned(),
    );
    draft.composer.set_placeholder(&placeholder, window, cx);
}

impl Shell {
    /// Keep the draft's machine and agent among those that can start a
    /// conversation now: the one chosen, else the remembered one, else the
    /// first online machine and its first agent that can start. Another
    /// machine or agent starts its choices over, and its options are asked
    /// for.
    pub(crate) fn settle_draft(&mut self) {
        let online: Vec<&Machine> = self.fleet.online().collect();
        let wanted = self
            .areas
            .draft
            .machine
            .clone()
            .or_else(|| self.prefs.machine.clone());
        let machine = wanted
            .and_then(|name| online.iter().find(|machine| machine.name == name).copied())
            .or_else(|| online.first().copied());
        let agents = machine.map_or(&[][..], |machine| machine.agents.as_slice());
        let wanted = self
            .areas
            .draft
            .agent
            .clone()
            .or_else(|| self.prefs.agent.clone());
        let agent = wanted
            .filter(|agent| {
                agents
                    .iter()
                    .any(|offered| &offered.harness == agent && offered.available())
            })
            .or_else(|| {
                agents
                    .iter()
                    .find(|agent| agent.available())
                    .map(|agent| agent.harness.clone())
            });
        let draft = &mut self.areas.draft;
        draft.machine = machine.map(|machine| machine.name.clone());
        draft.agent = agent;
        let now = draft.machine.clone().zip(draft.agent.clone());
        if draft.settled != now {
            draft.picks = Picks::default();
            draft.chosen = false;
            draft.settled = now;
        }
        self.probe(false);
    }

    /// The choices the draft starts with: the person's, once they chose
    /// any; until then the machine's last start of this agent's.
    pub(crate) fn effective_picks(&self) -> Picks {
        let draft = &self.areas.draft;
        if draft.chosen {
            return draft.picks.clone();
        }
        draft
            .settled
            .as_ref()
            .and_then(|(machine, agent)| self.prefs.choices(machine, agent))
            .map(|kept| Picks::from_choices(kept.to_vec()))
            .unwrap_or_default()
    }

    /// The running session's options as it last reported them, laid out
    /// as the draft's (`None` until the agent reported any).
    pub(crate) fn session_controls(&self, agent: &HarnessRef) -> Option<Controls> {
        let options = self.areas.conversation.timeline.options()?;
        Some(controls_with_effort(
            options,
            kind_of(agent),
            &agent_name(agent),
            self.t.lang,
        ))
    }

    /// What the controls show for the draft's agent, or for the running
    /// session on screen.
    pub(crate) fn shown(&self) -> Option<Shown> {
        if let ControlsTarget::Session { agent, .. } = self.controls_target() {
            return self
                .session_controls(&agent)
                .map(|controls| Shown::Ready(Box::new(controls)));
        }
        let (machine, agent) = self.areas.draft.settled.as_ref()?;
        Some(match self.offered(machine, agent) {
            Offered::Loading => Shown::Loading,
            Offered::Failed(why) => Shown::Failed(why.to_owned()),
            Offered::Ready { options, .. } => Shown::Ready(Box::new(controls_with_effort(
                &with_picks(options, &self.effective_picks()),
                kind_of(agent),
                &agent_name(agent),
                self.t.lang,
            ))),
        })
    }

    pub(crate) fn choose_machine(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.draft.folder = self.prefs.folders.get(&name).cloned().unwrap_or_default();
        self.areas.draft.machine = Some(name);
        self.areas.draft.agent = None;
        self.settle_draft();
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn choose_agent(&mut self, agent: HarnessRef) {
        self.areas.draft.agent = Some(agent);
        self.settle_draft();
    }

    /// One of the agent's options chosen: on a running session the
    /// session's own (track P's `change_running_option`), else a pick for
    /// the draft's start.
    pub(crate) fn choose_option(&mut self, choice: OptionChoice) {
        if let ControlsTarget::Session {
            machine,
            session,
            agent,
        } = self.controls_target()
        {
            self.change_running_option(machine, session, agent, choice);
            return;
        }
        let mut picks = self.effective_picks();
        picks.pick(choice);
        let draft = &mut self.areas.draft;
        draft.picks = picks;
        draft.chosen = true;
    }

    pub(crate) fn choose_folder(&mut self, folder: String) {
        self.areas.draft.folder = folder;
    }
}

/// A picker's pill with its list's anchor in it.
pub(crate) fn pill_with(
    id: &'static str,
    list: &ComboboxState,
    parts: vessel_app_kit::PillParts,
    touch: bool,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    vessel_app_kit::pill_parts(id, parts, list.is_open(), touch, tokens)
        .relative()
        .child(list.anchor())
}
