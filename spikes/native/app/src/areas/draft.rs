//! A new conversation: type, send, it runs (agent-orchestrator's `/new`
//! and GOAT's draft thread, which the owner named; their flow, read from
//! their sources, is reproduced here).
//!
//! As theirs: the app opens on this screen with the composer focused under
//! a heading ("What do you want to work on?"), one of four suggestions as
//! the placeholder; Enter sends; the session is created on the first send,
//! not before; the prompt is shown at once and the conversation opens with
//! it, the reply streaming in; the session is titled after the prompt (the
//! node does that); a send that fails puts the text back.
//!
//! Laid out as the page's new session (`web/src/features/compose`), so
//! the two read the same (ADR 0036): in the box's foot the agent, model
//! and effort control and the permissions control, Send at the right;
//! under the box the machine and the folder. The agent's options are asked
//! of its machine (`harness.options`, seconds the first time), shown as
//! "Loading models…" meanwhile, and placed by the page's rules
//! (`vessel_app_view::controls_of`); a choice is sent only when it changes
//! something, and the choices a start was made with are what the next
//! start on that machine begins from, until the person chooses again.
//!
//! Different, for now, and each named: the controls' choices open in a
//! panel under the box, not in a menu over it (the kit's menu is track C's
//! and replaces the panel when it lands); no profile or terminal choice
//! (the page's "+" menu); one line of text (Shift+Enter for a new line
//! comes with the multi-line field); the folder is typed or picked from
//! the ones used before (projects and worktrees come with the projects
//! track); no worktree is prepared ahead (agent-orchestrator does, to start
//! faster). The choices are kept only once the agent's options are known:
//! a start sent while they load keeps the earlier ones (the page keeps
//! none then, losing them).

use std::collections::HashMap;

use vessel_app_kit::ui::{
    Context, Div, ElementId, FluentBuilder as _, Hsla, InteractiveElement as _, ParentElement as _,
    Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button, choice, dot, kicker};
use vessel_app_link::Command;
use vessel_app_view::{
    Controls, Machine, Picks, SelectControl, agent_name, choices_for, controls_of, with_picks,
};
use vessel_proto::{HarnessRef, OptionChoice, SessionOption};

use crate::routes::Route;
use crate::shell::Shell;

/// Which picker is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picker {
    Machine,
    /// The agent, its model, its effort and its other settings.
    Agent,
    Permissions,
    Folder,
}

/// What the app knows of one agent's options on one machine (the page's
/// `OptionsState`).
#[derive(Debug, Clone)]
pub(crate) enum Probe {
    Loading,
    Ready(Vec<SessionOption>),
    Failed(String),
}

/// What the agent control shows (the page's `ControlsState`; the app
/// starts no terminals yet, so no terminal case).
enum Shown {
    Loading,
    Failed(String),
    /// Boxed: a few hundred bytes beside a string.
    Ready(Box<Controls>),
}

/// The draft: the composer, the folder field, and what is chosen.
pub(crate) struct Draft {
    pub(crate) composer: TextField,
    pub(crate) folder: TextField,
    pub(crate) machine: Option<String>,
    pub(crate) agent: Option<HarnessRef>,
    pub(crate) picking: Option<Picker>,
    /// Which of the suggestions is the placeholder (one per launch).
    suggestion: usize,
    /// Why the last send did not go, shown beside the composer.
    pub(crate) error: Option<String>,
    /// The text of a send not answered yet, put back if it fails.
    sending: Option<String>,
    /// A failed send's text, put back in the composer on the next frame
    /// (setting a field's text needs the window, which an event from the
    /// link does not have).
    restore: std::cell::RefCell<Option<String>>,
    /// What each agent offers, by machine and agent, as last asked.
    probes: HashMap<(String, HarnessRef), Probe>,
    /// The person's choices for this machine's agent, once `chosen`; until
    /// then the last start's (the page's `startingPicks`).
    picks: Picks,
    chosen: bool,
    /// The machine and agent the choices are for: another starts over.
    settled: Option<(String, HarnessRef)>,
}

impl Draft {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        // One suggestion per launch, as agent-orchestrator picks one per
        // mount; the clock's low bits are random enough for a placeholder.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let suggestion = usize::try_from(nanos % 4).unwrap_or(0);
        Self {
            composer: TextField::new(
                "",
                |shell: &mut Shell, text, window, cx| shell.send_draft(text, window, cx),
                window,
                cx,
            ),
            folder: TextField::new(
                "",
                |shell: &mut Shell, _text, window, cx| {
                    shell.areas.draft.picking = None;
                    shell.areas.draft.composer.focus(window, cx);
                    cx.notify();
                },
                window,
                cx,
            ),
            machine: None,
            agent: None,
            picking: None,
            suggestion,
            error: None,
            sending: None,
            restore: std::cell::RefCell::new(None),
            probes: HashMap::new(),
            picks: Picks::default(),
            chosen: false,
            settled: None,
        }
    }
}

/// Focus the composer, fill in the choices, and ask the agent again what
/// it offers (the node answers from its recent probe; what is shown stays
/// until the answer).
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.settle_draft();
    shell.probe(true);
    let t = shell.t;
    let draft = &shell.areas.draft;
    let placeholder = t
        .suggestions
        .get(draft.suggestion)
        .copied()
        .unwrap_or(t.prompt_placeholder);
    draft.composer.set_placeholder(placeholder, window, cx);
    draft
        .folder
        .set_placeholder(t.folder_placeholder, window, cx);
    draft.composer.focus(window, cx);
}

impl Shell {
    /// Keep the draft's machine and agent among those that can start a
    /// conversation now: the one chosen, else the remembered one, else the
    /// first online machine and its first agent. Another machine or agent
    /// starts its choices over, and its options are asked for.
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
            .filter(|agent| agents.iter().any(|offered| &offered.harness == agent))
            .or_else(|| agents.first().map(|agent| agent.harness.clone()));
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

    /// Ask the draft's machine what its agent offers: when not asked yet,
    /// or (`again`) to refresh an answer or retry a failure.
    pub(crate) fn probe(&mut self, again: bool) {
        let Some(key) = self.areas.draft.settled.clone() else {
            return;
        };
        let ask = match self.areas.draft.probes.get(&key) {
            None => true,
            Some(Probe::Loading) => false,
            Some(Probe::Ready(_) | Probe::Failed(_)) => again,
        };
        if !ask {
            return;
        }
        let probes = &mut self.areas.draft.probes;
        if !matches!(probes.get(&key), Some(Probe::Ready(_))) {
            let _earlier = probes.insert(key.clone(), Probe::Loading);
        }
        let (machine, harness) = key;
        self.command(Command::Options { machine, harness });
    }

    /// An agent's options arrived. A failure to refresh keeps what was
    /// shown.
    pub(crate) fn options_arrived(
        &mut self,
        machine: String,
        harness: HarnessRef,
        outcome: Result<Vec<SessionOption>, String>,
    ) {
        let probes = &mut self.areas.draft.probes;
        let key = (machine, harness);
        match outcome {
            Ok(options) => {
                let _earlier = probes.insert(key, Probe::Ready(options));
            }
            Err(why) => {
                if !matches!(probes.get(&key), Some(Probe::Ready(_))) {
                    let _earlier = probes.insert(key, Probe::Failed(why));
                }
            }
        }
    }

    /// The choices the draft starts with: the person's, once they chose
    /// any; until then the machine's last start of this agent's.
    fn effective_picks(&self) -> Picks {
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

    fn probed(&self) -> Option<&Probe> {
        let key = self.areas.draft.settled.as_ref()?;
        self.areas.draft.probes.get(key)
    }

    fn shown(&self) -> Option<Shown> {
        let (_, agent) = self.areas.draft.settled.as_ref()?;
        Some(match self.probed() {
            None | Some(Probe::Loading) => Shown::Loading,
            Some(Probe::Failed(why)) => Shown::Failed(why.clone()),
            Some(Probe::Ready(options)) => Shown::Ready(Box::new(controls_of(
                &with_picks(options, &self.effective_picks()),
                &agent_name(agent),
                self.t.lang,
            ))),
        })
    }

    pub(crate) fn pick(&mut self, picker: Picker, cx: &mut Context<'_, Self>) {
        let draft = &mut self.areas.draft;
        draft.picking = if draft.picking == Some(picker) {
            None
        } else {
            Some(picker)
        };
        if draft.picking == Some(Picker::Agent) && matches!(self.probed(), Some(Probe::Failed(_))) {
            self.probe(true);
        }
        cx.notify();
    }

    fn choose_machine(&mut self, name: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        let folder = self.prefs.folders.get(&name).cloned().unwrap_or_default();
        self.areas.draft.folder.set_text(&folder, window, cx);
        self.areas.draft.machine = Some(name);
        self.areas.draft.agent = None;
        self.areas.draft.picking = None;
        self.settle_draft();
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }

    fn choose_agent(&mut self, agent: HarnessRef, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.draft.agent = Some(agent);
        self.settle_draft();
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }

    /// One of the agent's options chosen; `close` the panel (a value
    /// chosen; a switch flipped keeps it open, as a menu's switch does).
    fn choose_option(
        &mut self,
        choice: OptionChoice,
        close: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let mut picks = self.effective_picks();
        picks.pick(choice);
        let draft = &mut self.areas.draft;
        draft.picks = picks;
        draft.chosen = true;
        if close {
            draft.picking = None;
            draft.composer.focus(window, cx);
        }
        cx.notify();
    }

    fn choose_folder(&mut self, folder: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.draft.folder.set_text(folder, window, cx);
        self.areas.draft.picking = None;
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
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
        if text.trim().is_empty() || self.areas.draft.sending.is_some() {
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
        let options = match self.probed() {
            Some(Probe::Ready(offered)) => {
                let options = choices_for(offered, &self.effective_picks());
                self.prefs.keep_choices(&machine, &harness, &options);
                options
            }
            _ => Vec::new(),
        };
        let folder = self.areas.draft.folder.text(cx).trim().to_owned();
        self.prefs.machine = Some(machine.clone());
        self.prefs.agent = Some(harness.clone());
        if folder.is_empty() {
            let _forgotten = self.prefs.folders.remove(&machine);
        } else {
            let _earlier = self.prefs.folders.insert(machine.clone(), folder.clone());
        }
        self.command(Command::SavePrefs(self.prefs.clone()));
        self.command(Command::Start {
            machine: machine.clone(),
            harness: harness.clone(),
            folder: (!folder.is_empty()).then_some(folder),
            prompt: text.clone(),
            options,
        });
        let draft = &mut self.areas.draft;
        draft.error = None;
        draft.picking = None;
        draft.sending = Some(text.clone());
        // The next draft begins from what this one started with.
        draft.picks = Picks::default();
        draft.chosen = false;
        draft.composer.clear(window, cx);
        self.begin_pending(machine, agent_name(&harness), text, cx);
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

    let mut column = div()
        .w_full()
        .max_w(px(760.0))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(vessel_app_kit::mark(40.0, tokens))
        .child(
            div()
                .type_(Type::Title)
                .text_size(px(22.0))
                .line_height(px(28.0))
                .child(t.welcome),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(composer_box(shell, shown.as_ref(), tokens, cx))
                .child(tray(shell, &online, tokens, cx)),
        );
    if let Some(error) = &draft.error {
        column = column.child(note(error.clone(), tokens.danger));
    }
    if let (Some(Shown::Failed(why)), Some(agent)) = (&shown, &draft.agent) {
        let words = t.lang.options();
        let failed = words.failed(&format!("{} — {why}", agent_name(agent)));
        column = column.child(note(failed, tokens.text_tertiary));
    }
    if let Some(picker) = draft.picking {
        column = column.child(panel(shell, picker, &online, shown.as_ref(), tokens, cx));
    }
    if shell.loaded && online.is_empty() {
        column = column.child(note(
            if shell.fleet.machines.is_empty() {
                t.no_machine_yet
            } else {
                t.no_machine_connected
            }
            .to_owned(),
            tokens.text_tertiary,
        ));
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .child(column)
}

/// The box: the composer, and in its foot the agent's controls and Send.
fn composer_box(
    shell: &Shell,
    shown: Option<&Shown>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(12.0))
        .bg(tokens.bg_surface)
        .border_1()
        .border_color(tokens.border_subtle)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(shell.areas.draft.composer.element())
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(tools(shell, shown, tokens, cx))
                .child(
                    button("send-draft", shell.t.send, true, tokens).on_click(cx.listener(
                        |this, _event, window, cx| {
                            let text = this.areas.draft.composer.text(cx);
                            this.send_draft(text, window, cx);
                        },
                    )),
                ),
        )
}

/// The agent, model and effort control, and the permissions control when
/// the agent offers permissions modes (`agent-menu.tsx`,
/// `permissions-menu.tsx`).
fn tools(
    shell: &Shell,
    shown: Option<&Shown>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let words = t.lang.options();
    let draft = &shell.areas.draft;
    let mut row = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(6.0));
    let Some(agent) = &draft.agent else {
        return row.child(chip(
            "pick-agent",
            t.agent.to_owned(),
            None,
            None,
            false,
            tokens,
        ));
    };
    let name = agent_name(agent);
    let (main, effort) = match shown {
        Some(Shown::Loading) | None => (words.loading_models.to_owned(), None),
        Some(Shown::Failed(_)) => (name, None),
        Some(Shown::Ready(controls)) => {
            let (main, effort) = controls.picker_label(&name);
            (main.to_owned(), effort.map(str::to_owned))
        }
    };
    row = row.child(
        chip(
            "pick-agent",
            main,
            effort,
            None,
            draft.picking == Some(Picker::Agent),
            tokens,
        )
        .on_click(cx.listener(|this, _event, _window, cx| this.pick(Picker::Agent, cx))),
    );
    if let Some(permissions) = permissions_of(shown) {
        row = row.child(
            chip(
                "pick-permissions",
                permissions.current_name().to_owned(),
                None,
                None,
                draft.picking == Some(Picker::Permissions),
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.pick(Picker::Permissions, cx))),
        );
    }
    row
}

/// The permissions control, when the agent offers permissions modes.
fn permissions_of(shown: Option<&Shown>) -> Option<&SelectControl> {
    match shown {
        Some(Shown::Ready(controls)) => controls.permissions.as_ref(),
        _ => None,
    }
}

/// The tray under the box: where it runs, the machine and the folder
/// (`where-tray.tsx`).
fn tray(shell: &Shell, online: &[&Machine], tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    let draft = &shell.areas.draft;
    let machine = draft
        .machine
        .as_deref()
        .and_then(|name| online.iter().find(|machine| machine.name == name).copied());
    let folder = draft.folder.text(cx);
    let folder_label = if folder.trim().is_empty() {
        t.home_folder.to_owned()
    } else {
        folder
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(6.0))
        .px(px(4.0))
        .child(
            chip(
                "pick-machine",
                draft
                    .machine
                    .clone()
                    .unwrap_or_else(|| t.machine.to_owned()),
                None,
                Some(if machine.is_some() {
                    tokens.st_ready
                } else {
                    tokens.text_tertiary
                }),
                draft.picking == Some(Picker::Machine),
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.pick(Picker::Machine, cx))),
        )
        .child(
            chip(
                "pick-folder",
                folder_label,
                None,
                None,
                draft.picking == Some(Picker::Folder),
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.pick(Picker::Folder, cx))),
        )
}

/// The open picker's choices, under the box.
fn panel(
    shell: &Shell,
    picker: Picker,
    online: &[&Machine],
    shown: Option<&Shown>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let body = match picker {
        Picker::Machine => machines(shell, online, tokens, cx),
        Picker::Agent => agent_panel(shell, online, shown, tokens, cx),
        Picker::Permissions => permissions_of(shown).map_or_else(div, |permissions| {
            permissions_panel(permissions, tokens, cx)
        }),
        Picker::Folder => folders(shell, tokens, cx),
    };
    div()
        .w_full()
        .p(px(12.0))
        .bg(tokens.bg_surface)
        .border_1()
        .border_color(tokens.border_subtle)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(body)
}

fn machines(
    shell: &Shell,
    online: &[&Machine],
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let draft = &shell.areas.draft;
    let mut list = div().flex().flex_row().flex_wrap().gap(px(6.0));
    for (index, machine) in online.iter().enumerate() {
        let name = machine.name.clone();
        let chosen = draft.machine.as_deref() == Some(machine.name.as_str());
        list = list.child(
            choice(
                ElementId::Name(format!("machine-{index}").into()),
                machine.name.clone(),
                chosen,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.choose_machine(name.clone(), window, cx);
            })),
        );
    }
    list
}

/// The agent, then its model and effort, then its other settings under
/// its name (`agent-menu.tsx`); a note while it is asked or when it could
/// not say.
fn agent_panel(
    shell: &Shell,
    online: &[&Machine],
    shown: Option<&Shown>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let words = t.lang.options();
    let draft = &shell.areas.draft;
    let agents = draft
        .machine
        .as_deref()
        .and_then(|name| online.iter().find(|machine| machine.name == name))
        .map_or(&[][..], |machine| machine.agents.as_slice());
    let mut row = div().flex().flex_row().flex_wrap().gap(px(6.0));
    if agents.is_empty() {
        row = row.child(note(t.no_agent.to_owned(), tokens.text_tertiary));
    }
    for (index, agent) in agents.iter().enumerate() {
        let harness = agent.harness.clone();
        let chosen = draft.agent.as_ref() == Some(&agent.harness);
        row = row.child(
            choice(
                ElementId::Name(format!("agent-{index}").into()),
                agent.name.clone(),
                chosen,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.choose_agent(harness.clone(), window, cx);
            })),
        );
    }
    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(section(words.agent, row, tokens));
    match shown {
        Some(Shown::Ready(controls)) => {
            if let Some(model) = &controls.model {
                column = column.child(section(words.model, values(model, tokens, cx), tokens));
            }
            if let Some(effort) = &controls.effort {
                column = column.child(section(words.effort, values(effort, tokens, cx), tokens));
            }
            if !controls.toggles.is_empty() || !controls.selects.is_empty() {
                let name = draft.agent.as_ref().map(agent_name).unwrap_or_default();
                column = column.child(settings(
                    &words.settings(&name),
                    controls,
                    shell,
                    tokens,
                    cx,
                ));
            }
        }
        Some(Shown::Loading) | None => {
            column = column.child(note(words.asking.to_owned(), tokens.text_tertiary));
        }
        Some(Shown::Failed(_)) => {}
    }
    column
}

/// The agent's switches, then its other choices, each by its own name.
fn settings(
    heading: &str,
    controls: &Controls,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.options();
    let mut column = div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(kicker(heading.to_owned(), tokens));
    for toggle in &controls.toggles {
        let flip = toggle.choose(!toggle.on);
        column = column.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(toggle.name.clone()),
                )
                .child(
                    choice(
                        ElementId::Name(format!("toggle-{}", toggle.id).into()),
                        if toggle.on { words.on } else { words.off },
                        toggle.on,
                        tokens,
                    )
                    .on_click(cx.listener(move |this, _event, window, cx| {
                        this.choose_option(flip.clone(), false, window, cx);
                    })),
                ),
        );
    }
    for select in &controls.selects {
        column = column.child(section(&select.name, values(select, tokens, cx), tokens));
    }
    column
}

/// A select's values as pills, in the agent's order and words, each run
/// of a group under the group's name.
fn values(control: &SelectControl, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut column = div().flex().flex_col().gap(px(6.0));
    let mut row = div().flex().flex_row().flex_wrap().gap(px(6.0));
    let mut group: Option<&str> = None;
    let mut started = false;
    for value in &control.values {
        let this_group = value.group.as_deref();
        if started && this_group != group {
            column = column.child(row);
            row = div().flex().flex_row().flex_wrap().gap(px(6.0));
        }
        if (!started || this_group != group)
            && let Some(name) = this_group
        {
            column = column.child(
                div()
                    .type_(Type::Tiny)
                    .text_color(tokens.text_tertiary)
                    .child(name.to_owned()),
            );
        }
        group = this_group;
        started = true;
        let picked = control.choose(value.id.clone());
        row = row.child(
            choice(
                ElementId::Name(format!("option-{}-{}", control.id, value.id).into()),
                value.name.clone(),
                value.id == control.current,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.choose_option(picked.clone(), true, window, cx);
            })),
        );
    }
    column.child(row)
}

/// The permissions modes as rows: the mode's name, the agent's
/// description of it under, the one in effect marked (`permissions-menu.tsx`).
fn permissions_panel(control: &SelectControl, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut column = div().flex().flex_col().gap(px(2.0));
    for value in &control.values {
        let current = value.id == control.current;
        let picked = control.choose(value.id.clone());
        column = column.child(
            div()
                .id(ElementId::Name(format!("mode-{}", value.id).into()))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(vessel_app_kit::theme::radius::R2))
                .when(current, |row| row.bg(tokens.bg_active))
                .hover(|style| style.bg(tokens.bg_hover))
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(value.name.clone())
                        .when(current, |row| {
                            row.child(div().text_color(tokens.text_secondary).child("✓"))
                        }),
                )
                .children(value.description.clone().map(|description| {
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.text_tertiary)
                        .child(description)
                }))
                .on_click(cx.listener(move |this, _event, window, cx| {
                    this.choose_option(picked.clone(), true, window, cx);
                })),
        );
    }
    column
}

fn folders(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    let mut column = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(shell.areas.draft.folder.element());
    let used: Vec<String> = shell.prefs.folders.values().cloned().collect();
    let mut row = div().flex().flex_row().flex_wrap().gap(px(6.0)).child(
        choice("folder-home", t.home_folder, false, tokens).on_click(cx.listener(
            |this, _event, window, cx| {
                this.choose_folder("", window, cx);
            },
        )),
    );
    for (index, folder) in used.into_iter().enumerate() {
        let chosen_folder = folder.clone();
        row = row.child(
            choice(
                ElementId::Name(format!("folder-{index}").into()),
                folder,
                false,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.choose_folder(&chosen_folder, window, cx);
            })),
        );
    }
    column = column.child(row);
    column
}

/// A labelled group of choices in a panel.
fn section(label: &str, body: Div, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(kicker(label.to_owned(), tokens))
        .child(body)
}

/// A sentence under the box.
fn note(text: String, color: Hsla) -> Div {
    div().type_(Type::Meta).text_color(color).child(text)
}

/// A chip: a dot (a machine's state) or none, the value, a quieter second
/// value (the effort), a caret.
fn chip(
    id: &'static str,
    label: String,
    quiet: Option<String>,
    state: Option<Hsla>,
    open: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(vessel_app_kit::theme::radius::R2))
        .when(open, |chip| chip.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .type_(Type::Meta)
        .text_color(tokens.text_secondary)
        .children(state.map(dot))
        .child(label)
        .children(quiet.map(|quiet| div().text_color(tokens.text_tertiary).child(quiet)))
        .child(div().text_color(tokens.text_tertiary).child("▾"))
}
