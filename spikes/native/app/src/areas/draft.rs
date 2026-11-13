//! A new conversation: type, send, it runs (agent-orchestrator's `/new`
//! and GOAT's draft thread, which the owner named; their flow, read from
//! their sources, is reproduced here).
//!
//! As theirs: the app opens on this screen with the composer focused under
//! a heading ("What do you want to work on?"), one of four suggestions as
//! the placeholder; the machine, agent and folder sit in a tray under the
//! composer, remembered between launches (agent-orchestrator keeps the
//! last agent per host and project; here per machine); Enter sends; the
//! session is created on the first send, not before; the prompt is shown
//! at once and the conversation opens with it, the reply streaming in;
//! the session is titled after the prompt (the node does that); a send
//! that fails puts the text back.
//!
//! Different, for now, and each named: no model, effort or mode picker
//! (the agent's own defaults; both references have them, next slice); one
//! line of text (Shift+Enter for a new line comes with the multi-line
//! field); the folder is typed or picked from the ones used before
//! (projects and worktrees come with the projects track); no worktree is
//! prepared ahead (agent-orchestrator does, to start faster).

use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button, dot};
use vessel_app_link::Command;
use vessel_app_view::{Machine, agent_name};
use vessel_proto::HarnessRef;

use crate::routes::Route;
use crate::shell::Shell;

/// Which tray picker is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picker {
    Machine,
    Agent,
    Folder,
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
        }
    }
}

/// Focus the composer and fill in the choices.
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.settle_draft();
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
    /// first online machine and its first agent.
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
        self.areas.draft.machine = machine.map(|machine| machine.name.clone());
        self.areas.draft.agent = agent;
    }

    pub(crate) fn pick(&mut self, picker: Picker, cx: &mut Context<'_, Self>) {
        let draft = &mut self.areas.draft;
        draft.picking = if draft.picking == Some(picker) {
            None
        } else {
            Some(picker)
        };
        cx.notify();
    }

    fn choose_machine(&mut self, name: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        let folder = self.prefs.folders.get(&name).cloned().unwrap_or_default();
        self.areas.draft.folder.set_text(&folder, window, cx);
        self.areas.draft.machine = Some(name);
        self.areas.draft.picking = None;
        self.settle_draft();
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }

    fn choose_agent(&mut self, agent: HarnessRef, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.draft.agent = Some(agent);
        self.areas.draft.picking = None;
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }

    fn choose_folder(&mut self, folder: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.draft.folder.set_text(folder, window, cx);
        self.areas.draft.picking = None;
        self.areas.draft.composer.focus(window, cx);
        cx.notify();
    }

    /// Send the draft: create the session with its first prompt, remember
    /// the choices, and open the conversation with the prompt shown.
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
        });
        let draft = &mut self.areas.draft;
        draft.error = None;
        draft.picking = None;
        draft.sending = Some(text.clone());
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
    let tray = tray(shell, &online, tokens, cx);

    let composer = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(12.0))
        .bg(tokens.bg_surface)
        .border_1()
        .border_color(tokens.border_subtle)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(draft.composer.element())
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(tray)
                .child(
                    button("send-draft", t.send, true, tokens).on_click(cx.listener(
                        |this, _event, window, cx| {
                            let text = this.areas.draft.composer.text(cx);
                            this.send_draft(text, window, cx);
                        },
                    )),
                ),
        );

    let mut column = div()
        .w_full()
        .max_w(px(760.0))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(
            div()
                .type_(Type::Title)
                .text_size(px(22.0))
                .line_height(px(28.0))
                .child(t.welcome),
        )
        .child(composer);
    if let Some(error) = &draft.error {
        column = column.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.danger)
                .child(error.clone()),
        );
    }
    if let Some(picker) = draft.picking {
        column = column.child(options(shell, picker, &online, tokens, cx));
    }
    if shell.loaded && online.is_empty() {
        column = column.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(if shell.fleet.machines.is_empty() {
                    t.no_machine_yet
                } else {
                    t.no_machine_connected
                }),
        );
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

/// The tray under the composer: the machine, the agent, the folder.
fn tray(shell: &Shell, online: &[&Machine], tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    let draft = &shell.areas.draft;
    let machine = draft
        .machine
        .as_deref()
        .and_then(|name| online.iter().find(|machine| machine.name == name).copied());
    let agent_label = draft
        .agent
        .as_ref()
        .map_or_else(|| t.agent.to_owned(), agent_name);
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
        .child(
            chip(
                "pick-machine",
                draft
                    .machine
                    .clone()
                    .unwrap_or_else(|| t.machine.to_owned()),
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
                "pick-agent",
                agent_label,
                None,
                draft.picking == Some(Picker::Agent),
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.pick(Picker::Agent, cx))),
        )
        .child(
            chip(
                "pick-folder",
                folder_label,
                None,
                draft.picking == Some(Picker::Folder),
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.pick(Picker::Folder, cx))),
        )
}

/// The open picker's choices, under the composer.
fn options(
    shell: &Shell,
    picker: Picker,
    online: &[&Machine],
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let draft = &shell.areas.draft;
    let mut list = div().flex().flex_row().flex_wrap().gap(px(6.0));
    match picker {
        Picker::Machine => {
            for (index, machine) in online.iter().enumerate() {
                let name = machine.name.clone();
                let chosen = draft.machine.as_deref() == Some(machine.name.as_str());
                list = list.child(
                    vessel_app_kit::choice(
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
        }
        Picker::Agent => {
            let agents = draft
                .machine
                .as_deref()
                .and_then(|name| online.iter().find(|machine| machine.name == name))
                .map_or(&[][..], |machine| machine.agents.as_slice());
            if agents.is_empty() {
                list = list.child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.text_tertiary)
                        .child(t.no_agent),
                );
            }
            for (index, agent) in agents.iter().enumerate() {
                let harness = agent.harness.clone();
                let chosen = draft.agent.as_ref() == Some(&agent.harness);
                list = list.child(
                    vessel_app_kit::choice(
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
        }
        Picker::Folder => {
            let mut column = div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(draft.folder.element());
            let used: Vec<String> = shell.prefs.folders.values().cloned().collect();
            let mut row = div().flex().flex_row().flex_wrap().gap(px(6.0)).child(
                vessel_app_kit::choice("folder-home", t.home_folder, false, tokens).on_click(
                    cx.listener(|this, _event, window, cx| {
                        this.choose_folder("", window, cx);
                    }),
                ),
            );
            for (index, folder) in used.into_iter().enumerate() {
                let chosen_folder = folder.clone();
                row = row.child(
                    vessel_app_kit::choice(
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
            return column;
        }
    }
    list
}

/// A tray chip: a dot (a machine's state) or none, the value, a caret.
fn chip(
    id: &'static str,
    label: String,
    state: Option<vessel_app_kit::ui::Hsla>,
    open: bool,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
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
        .child(div().text_color(tokens.text_tertiary).child("▾"))
}

use vessel_app_kit::ui::FluentBuilder as _;
