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
//! the model. Differences from Paseo: no profiles (Vessel's
//! profiles are the node's, not in this build); no attachments button
//! yet. The choices are kept only once the agent's options are known: a
//! start sent while they load keeps the earlier ones.

mod book;
mod density;
mod effort_card;
mod features;
mod pickers;
mod probe;

use vessel_app_kit::ui::{
    Context, Div, Hsla, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{ComboboxState, SendKind, TextField, Tokens, Type, TypeExt as _, send_button};
use vessel_app_link::Command;
use vessel_app_view::offers::Offered;
use vessel_app_view::{
    Controls, Machine, Picks, agent_name, choices_for, controls_with_effort, with_picks,
};
use vessel_proto::{HarnessRef, OptionChoice};

use crate::areas::composer::{attach_button, draft_card};
use crate::routes::Route;
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

/// The agent's kind, when it is one of the four (an agent added on the
/// node has none the effort table could know).
pub(crate) const fn kind_of(agent: &HarnessRef) -> Option<vessel_proto::HarnessKind> {
    match agent {
        HarnessRef::Known(kind) => Some(*kind),
        HarnessRef::Added(_) => None,
    }
}

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

    /// Send the draft: create the session with its first prompt and the
    /// choices that change something, remember them, and open the
    /// conversation with the prompt shown.
    pub(crate) fn send_draft(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if (text.trim().is_empty() && !self.draft_has_files()) || self.areas.draft.sending.is_some()
        {
            return;
        }
        self.settle_draft();
        let (Some(machine), Some(harness)) = (
            self.areas.draft.machine.clone(),
            self.areas.draft.agent.clone(),
        ) else {
            self.areas.draft.error = Some(self.t.choose_first.to_owned());
            cx.notify();
            return;
        };
        // A project chosen in the tray that cannot be started in yet
        // (track F's `workspace::place`).
        if let Some(why) = self.place_blocked() {
            self.areas.draft.error = Some(why);
            cx.notify();
            return;
        }
        let options = self
            .offer_options(&machine, &harness)
            .map(|offered| choices_for(offered, &self.effective_picks()));
        if let Some(options) = &options {
            self.prefs.keep_choices(&machine, &harness, options);
        }
        let options = options.unwrap_or_default();
        let folder = self.areas.draft.folder.trim().to_owned();
        self.prefs.machine = Some(machine.clone());
        self.prefs.agent = Some(harness.clone());
        if folder.is_empty() {
            let _forgotten = self.prefs.folders.remove(&machine);
        } else {
            let _earlier = self.prefs.folders.insert(machine.clone(), folder.clone());
        }
        self.command(Command::SavePrefs(self.prefs.clone()));
        // In a project, track F's start (a new worktree made first, its
        // steps on the session's screen); else in the folder.
        if !self.start_in_place(&machine, &harness, &options, &text) {
            self.command(self.with_draft_files(Command::Start {
                machine: machine.clone(),
                harness: harness.clone(),
                folder: (!folder.is_empty()).then_some(folder),
                prompt: text.clone(),
                options,
            }));
        }
        let draft = &mut self.areas.draft;
        draft.error = None;
        draft.sending = Some(text.clone());
        // The next draft begins from what this one started with.
        draft.picks = Picks::default();
        draft.chosen = false;
        draft.composer.clear(window, cx);
        self.begin_pending(machine, agent_name(&harness), text, cx);
        self.brand.sent();
        self.route = Route::Session;
        cx.notify();
    }

    /// The session started: the draft is spent.
    pub(crate) fn draft_started(&mut self) {
        self.areas.draft.sending = None;
    }

    /// The start failed: back to the draft, its text restored.
    pub(crate) fn start_failed(&mut self) {
        if self.areas.draft.sending.is_some() {
            self.route_to(Route::Draft);
            let text = self.areas.draft.sending.take();
            *self.areas.draft.restore.borrow_mut() = text;
        }
    }
}

/// Whether the window is at Paseo's compact width (touch-sized controls,
/// pickers as sheets).
pub(crate) fn compact(window: &Window) -> bool {
    window.viewport_size().width < px(vessel_app_kit::picker::COMPACT)
}

/// The draft screen.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let restored = shell.areas.draft.restore.borrow_mut().take();
    if let Some(text) = restored {
        // A failed start's text, back in the composer (once).
        shell.areas.draft.composer.set_text(&text, window, cx);
    }
    let t = shell.t;
    let draft = &shell.areas.draft;
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let shown = shell.shown();
    let touch = compact(window);
    let neutral = tokens.neutral;
    let typed = !draft.composer.text(cx).trim().is_empty() || shell.draft_has_files();
    let can_send = typed && draft.agent.is_some();
    let attach = attach_button(true, touch, tokens, cx);
    let card = draft_card(shell, tokens, touch, window, cx).child(
        // Paseo's button row bleeds 6 past the card's padding.
        div()
            .mx(px(-6.0))
            .flex()
            .flex_row()
            .items_end()
            .justify_between()
            .gap(px(8.0))
            .child(attach)
            .child(pickers::controls(shell, shown.as_ref(), touch, tokens, window, cx).flex_1())
            .child(
                send_button("send-draft", SendKind::Send, can_send, touch, tokens).on_click(
                    cx.listener(|this, _event, window, cx| {
                        let text = this.areas.draft.composer.text(cx);
                        this.send_draft(text, window, cx);
                    }),
                ),
            ),
    );

    let mut column = div()
        .w_full()
        .max_w(px(820.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        // The brand's drawing measures the heading, the card and the
        // pickers (worker B's `brand.rs`); it is the mark, drawn large.
        .on_children_prepainted(shell.brand.new_session(0))
        .child(
            div()
                // As wide as its words, so the brand's drawing measures the
                // text and not the column (worker B's `brand.rs`).
                .self_start()
                .type_(Type::Title)
                .text_size(px(vessel_app_kit::fonts::ui(22.0)))
                .line_height(px(vessel_app_kit::fonts::ui(28.0)))
                .text_color(neutral.foreground)
                .child(t.welcome),
        )
        .child(card)
        .child(pickers::place(shell, &online, touch, tokens, cx).self_start())
        .child(super::import::entry(shell, tokens, cx));
    if let Some(error) = &draft.error {
        column = column.child(note(error.clone(), neutral.destructive));
    }
    if let (Some(Shown::Failed(why)), Some(agent)) = (&shown, &draft.agent) {
        let words = t.lang.options();
        let failed = words.failed(&format!("{} — {why}", agent_name(agent)));
        column = column.child(note(failed, neutral.foreground_muted));
    }
    if shell.loaded && online.is_empty() {
        column = column.child(note(
            if shell.fleet.machines.is_empty() {
                t.no_machine_yet
            } else {
                t.no_machine_connected
            }
            .to_owned(),
            neutral.foreground_muted,
        ));
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px(px(if touch { 12.0 } else { 32.0 }))
        .child(column)
        .children(overlays(
            shell,
            &online,
            shown.as_ref(),
            tokens,
            touch,
            window,
            cx,
        ))
}

/// A running session's controls: the draft's pills on its options (the
/// session target, `Shell::controls_target`) and what is drawn over them
/// (the effort card, the tooltip, the open list); `None` until the
/// session's agent reported options.
pub(crate) fn running_controls(
    shell: &Shell,
    touch: bool,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<(Div, Vec<vessel_app_kit::ui::AnyElement>)> {
    let shown = shell.shown()?;
    let row = pickers::controls(shell, Some(&shown), touch, tokens, window, cx);
    let online: Vec<&Machine> = shell.fleet.online().collect();
    let over = overlays(shell, &online, Some(&shown), tokens, touch, window, cx);
    Some((row, over))
}

/// What is drawn over the new chat: the features sheet, the composer's
/// tooltip, and the open list.
fn overlays(
    shell: &Shell,
    online: &[&Machine],
    shown: Option<&Shown>,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<vessel_app_kit::ui::AnyElement> {
    let ready = match shown {
        Some(Shown::Ready(controls)) => Some(controls.as_ref()),
        Some(Shown::Loading | Shown::Failed(_)) | None => None,
    };
    let mut over = Vec::new();
    over.extend(effort_card::card(shell, ready, touch, tokens, window, cx));
    over.extend(shell.areas.draft.features.tips.render(tokens));
    over.extend(pickers::open_list(shell, online, shown, tokens, window, cx));
    over
}

/// A sentence under the composer.
fn note(text: String, color: Hsla) -> Div {
    div().type_(Type::Meta).text_color(color).child(text)
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
