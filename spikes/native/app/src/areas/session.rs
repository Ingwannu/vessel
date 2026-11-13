//! A session: its conversation as it grows, and the composer (DESIGN.md
//! section 6.15: the prompt in mono, the agent's reply as Markdown body
//! text, thoughts quieter, tool calls as rows with their state; the
//! composer a surface at the foot; a permission above it).
//!
//! The conversation is a list that draws only what is in view (GPUI's
//! `list`, measured rows), so its cost does not grow with its length.
//!
//! A session started from the draft opens before the node has answered:
//! its first prompt is shown at once with "Starting <agent> on <machine>…"
//! (agent-orchestrator's local echo and its "gets ready" line), and the
//! conversation takes its place when the first record arrives.

use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement, ListAlignment, ListState, ParentElement as _,
    Styled as _, Window, div, list, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button, markdown};
use vessel_app_link::Command;
use vessel_app_view::{Asked, Card, Conversation, Speaker, ToolState};
use vessel_proto::{EventRecord, PermissionId, PermissionOptionId, SessionId};

use super::common::title_bar;
use crate::routes::Route;
use crate::shell::Shell;

/// The session on screen.
#[derive(Debug, Clone)]
pub(crate) struct Open {
    pub(crate) machine: String,
    pub(crate) session: SessionId,
    pub(crate) title: String,
    pub(crate) agent: String,
    pub(crate) structured: bool,
    /// Whether it takes prompts (an ended session shows its conversation
    /// without a composer).
    pub(crate) live: bool,
}

impl Open {
    pub(crate) fn of(card: &Card) -> Self {
        Self {
            machine: card.machine.clone(),
            session: card.session,
            title: card.title.clone(),
            agent: card.agent.clone(),
            structured: card.structured,
            live: card.live,
        }
    }
}

/// A session asked for from the draft and not created yet.
#[derive(Debug, Clone)]
pub(crate) struct Pending {
    pub(crate) machine: String,
    pub(crate) agent: String,
    pub(crate) prompt: String,
}

/// The session area's state.
pub(crate) struct SessionView {
    pub(crate) open: Option<Open>,
    pub(crate) pending: Option<Pending>,
    pub(crate) conversation: Conversation,
    pub(crate) entries: ListState,
    pub(crate) composer: TextField,
}

impl SessionView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: None,
            pending: None,
            conversation: Conversation::default(),
            entries: ListState::new(0, ListAlignment::Bottom, px(800.0)),
            composer: TextField::new(
                "",
                |shell: &mut Shell, text, window, cx| shell.submit_prompt(text, window, cx),
                window,
                cx,
            ),
        }
    }
}

impl Shell {
    /// Open a session from a card or a row, the composer focused.
    pub(crate) fn open_session(
        &mut self,
        open: Open,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let takes_prompts = open.structured && open.live;
        self.areas.session.pending = None;
        self.follow(open, cx);
        let t = self.t;
        self.areas
            .session
            .composer
            .set_placeholder(t.composer_placeholder, window, cx);
        if takes_prompts {
            self.areas.session.composer.focus(window, cx);
        }
    }

    /// Show a session's conversation, following it.
    fn follow(&mut self, open: Open, cx: &mut Context<'_, Self>) {
        let view = &mut self.areas.session;
        view.conversation = Conversation::default();
        view.entries.reset(0);
        if open.structured {
            self.command(Command::Open {
                machine: open.machine.clone(),
                session: open.session,
            });
        }
        self.areas.session.open = Some(open);
        self.route = Route::Session;
        cx.notify();
    }

    /// The draft was sent: show its prompt while the session starts.
    pub(crate) fn begin_pending(
        &mut self,
        machine: String,
        agent: String,
        prompt: String,
        cx: &mut Context<'_, Self>,
    ) {
        let view = &mut self.areas.session;
        view.open = None;
        view.conversation = Conversation::default();
        view.entries.reset(0);
        view.pending = Some(Pending {
            machine,
            agent,
            prompt,
        });
        cx.notify();
    }

    /// The node created the session the draft asked for: follow it, the
    /// prompt still shown until its first record arrives.
    pub(crate) fn started(&mut self, card: &Card, cx: &mut Context<'_, Self>) {
        self.draft_started();
        let Some(pending) = self.areas.session.pending.clone() else {
            return;
        };
        if self.route == Route::Session {
            let mut open = Open::of(card);
            if open.title.is_empty() {
                // The node titles a session from its first prompt when the
                // turn starts (80 characters of its first line); until then
                // the same, here.
                open.title = first_line(&pending.prompt);
            }
            self.follow(open, cx);
        }
    }

    /// The fleet was read again: the open session's title, if the node
    /// has given it one since.
    pub(crate) fn retitle(&mut self) {
        let Some(open) = &mut self.areas.session.open else {
            return;
        };
        let card = self
            .fleet
            .machines
            .iter()
            .flat_map(|machine| machine.sessions.iter())
            .find(|card| card.session == open.session);
        if let Some(card) = card {
            if !card.title.is_empty() {
                open.title.clone_from(&card.title);
            }
            open.live = card.live;
        }
    }

    /// Records of the session followed.
    pub(crate) fn records(&mut self, session: &SessionId, records: &[EventRecord]) {
        let view = &mut self.areas.session;
        if view
            .open
            .as_ref()
            .is_none_or(|open| &open.session != session)
        {
            return;
        }
        let before = view.conversation.entries().len();
        for record in records {
            view.conversation.apply(record);
        }
        let after = view.conversation.entries().len();
        if after > 0 {
            view.pending = None;
        }
        // The last entry may have grown (a chunk), and new ones follow it:
        // measure it again with them.
        let from = before.saturating_sub(1);
        view.entries
            .splice(from..before, after.saturating_sub(from));
    }

    pub(crate) fn submit_prompt(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(open) = &self.areas.session.open else {
            return;
        };
        if text.trim().is_empty() {
            return;
        }
        self.command(Command::Prompt {
            machine: open.machine.clone(),
            session: open.session,
            text,
        });
        self.areas.session.composer.clear(window, cx);
    }

    /// Stop the open session's running turn.
    pub(crate) fn stop_turn(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(open) = &self.areas.session.open {
            self.command(Command::Stop {
                machine: open.machine.clone(),
                session: open.session,
            });
            cx.notify();
        }
    }

    pub(crate) fn answer(
        &mut self,
        id: PermissionId,
        option: PermissionOptionId,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(open) = &self.areas.session.open {
            self.command(Command::Answer {
                machine: open.machine.clone(),
                session: open.session,
                id,
                option,
            });
            cx.notify();
        }
    }
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let view = &shell.areas.session;
    if let Some(pending) = &view.pending
        && view.conversation.entries().is_empty()
    {
        return starting(pending, shell, tokens);
    }
    let Some(open) = &view.open else {
        return div();
    };
    let detail = format!("{} · {}", open.machine, open.agent);
    let body: AnyElement = if open.structured {
        let entries = list(
            view.entries.clone(),
            cx.processor(|this: &mut Shell, index: usize, _window, cx| {
                entry(this, index, &vessel_app_kit::tokens(cx))
            }),
        )
        .flex_1()
        .size_full();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(entries)
            .into_any_element()
    } else {
        div()
            .flex_1()
            .p(px(24.0))
            .type_(Type::Body)
            .text_color(tokens.text_secondary)
            .child(t.terminal_not_yet)
            .into_any_element()
    };
    let working = view.conversation.turn_running();
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(
            t.title(&open.title).to_owned(),
            Some(detail),
            tokens,
        ))
        .child(body)
        .children(
            view.conversation
                .asked()
                .first()
                .map(|asked| permission(asked, shell, tokens, cx)),
        )
        .when(open.structured && open.live, |frame| {
            frame.child(composer(shell, tokens, working, cx))
        })
        .when(open.structured && !open.live, |frame| {
            frame.child(
                div()
                    .px(px(24.0))
                    .py(px(12.0))
                    .border_t_1()
                    .border_color(tokens.border_faint)
                    .type_(Type::Meta)
                    .text_color(tokens.text_tertiary)
                    .child(t.ended),
            )
        })
}

/// The first prompt, shown while the session starts.
fn starting(pending: &Pending, shell: &Shell, tokens: &Tokens) -> Div {
    let t = shell.t;
    let title: String = pending
        .prompt
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(100)
        .collect();
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(
            title,
            Some(format!("{} · {}", pending.machine, pending.agent)),
            tokens,
        ))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .justify_end()
                .child(person(&pending.prompt, tokens))
                .child(
                    div()
                        .px(px(24.0))
                        .py(px(8.0))
                        .type_(Type::Meta)
                        .text_color(tokens.st_building)
                        .child(t.starting_on(&pending.agent, &pending.machine)),
                ),
        )
}

fn composer(shell: &Shell, tokens: &Tokens, working: bool, cx: &mut Context<'_, Shell>) -> Div {
    let t = shell.t;
    div()
        .p(px(12.0))
        .border_t_1()
        .border_color(tokens.border_faint)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().child(shell.areas.session.composer.element()))
                .when(working, |row| {
                    row.child(
                        button("stop", t.stop, false, tokens)
                            .on_click(cx.listener(|this, _event, _window, cx| this.stop_turn(cx))),
                    )
                })
                .child(
                    button("send", if working { t.queue } else { t.send }, true, tokens).on_click(
                        cx.listener(|this, _event, window, cx| {
                            let text = this.areas.session.composer.text(cx);
                            this.submit_prompt(text, window, cx);
                        }),
                    ),
                ),
        )
        .child(
            div()
                .pt(px(6.0))
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(if working {
                    t.working_queues
                } else {
                    t.enter_sends
                }),
        )
}

/// The oldest permission waiting: what it is for, and a button per answer
/// (the allowing ones primary).
fn permission(asked: &Asked, shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut answers = div().flex().flex_row().gap(px(8.0));
    for (index, (option, words, allows)) in asked.options.iter().enumerate() {
        let id = asked.id;
        let option = option.clone();
        answers = answers.child(
            button(
                ElementId::Name(format!("answer-{index}").into()),
                words.clone(),
                *allows,
                tokens,
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.answer(id, option.clone(), cx);
            })),
        );
    }
    div()
        .mx(px(12.0))
        .mb(px(8.0))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .bg(tokens.bg_surface)
        .border_1()
        .border_color(tokens.st_needs_you)
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.text_primary)
                .child(shell.t.asks_to_run(asked.what.as_deref())),
        )
        .child(answers)
}

/// The first line of a prompt, at most 80 characters (the node's rule
/// for a title, `SessionTitle::from_prompt`).
fn first_line(prompt: &str) -> String {
    prompt
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(80)
        .collect()
}

fn person(text: &str, tokens: &Tokens) -> Div {
    div().w_full().px(px(24.0)).py(px(6.0)).child(
        div()
            .p(px(10.0))
            .bg(tokens.bg_surface)
            .border_1()
            .border_color(tokens.border_subtle)
            .rounded(px(vessel_app_kit::theme::radius::R2))
            .type_(Type::Mono)
            .text_color(tokens.text_primary)
            .child(text.to_owned()),
    )
}

fn entry(shell: &Shell, index: usize, tokens: &Tokens) -> AnyElement {
    let t = shell.t;
    let Some(entry) = shell.areas.session.conversation.entries().get(index) else {
        return div().into_any_element();
    };
    let row = div().w_full().px(px(24.0)).py(px(6.0));
    match entry.speaker {
        Speaker::Person => person(&entry.text, tokens).into_any_element(),
        Speaker::Agent => row
            .type_(Type::Body)
            .text_color(tokens.text_primary)
            .child(markdown(
                ElementId::Name(format!("entry-{index}").into()),
                entry.text.clone(),
            ))
            .into_any_element(),
        Speaker::Thought => row
            .type_(Type::Meta)
            .text_color(tokens.text_tertiary)
            .child(entry.text.clone())
            .into_any_element(),
        Speaker::Tool => {
            let state = entry.tool.unwrap_or(ToolState::Pending);
            let color = match state {
                ToolState::Pending | ToolState::Running => tokens.st_building,
                ToolState::Done => tokens.st_ready,
                ToolState::Failed => tokens.danger,
            };
            row.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .items_center()
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(color)
                            .child(t.tool_state(state)),
                    )
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(tokens.text_secondary)
                            .child(entry.text.clone()),
                    ),
            )
            .into_any_element()
        }
        Speaker::Notice => row
            .type_(Type::Meta)
            .text_color(tokens.text_tertiary)
            .children(entry.notice.as_ref().map(|notice| t.notice(notice)))
            .into_any_element(),
    }
}

use vessel_app_kit::ui::FluentBuilder as _;
use vessel_app_kit::ui::StatefulInteractiveElement as _;
