//! A session: its conversation as it grows, and the composer (DESIGN.md
//! section 6.15: the prompt in the interface face, as the page draws it,
//! monospace being for code and the terminal only; the agent's reply as
//! Markdown body text; thoughts quieter, tool calls as rows with their
//! state; the composer a surface at the foot; a permission above it).
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
    Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button};
use vessel_app_link::Command;
use vessel_app_view::{Asked, Card, Conversation};
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
    /// An ended session being started again: the prompt that will be sent
    /// (shown under the conversation while it starts).
    pub(crate) resuming: Option<String>,
    /// Why starting it again failed, under the composer.
    pub(crate) resume_error: Option<String>,
    /// A failed resume's text, put back in the composer on the next frame.
    restore: std::cell::RefCell<Option<String>>,
    pub(crate) pending: Option<Pending>,
    pub(crate) conversation: Conversation,
    pub(crate) entries: ListState,
    pub(crate) composer: TextField,
}

impl SessionView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: None,
            resuming: None,
            resume_error: None,
            restore: std::cell::RefCell::new(None),
            pending: None,
            conversation: Conversation::default(),
            entries: ListState::new(0, ListAlignment::Bottom, px(800.0)),
            composer: TextField::composer(
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
        // An ended session takes prompts too: sending starts it again.
        let takes_prompts = open.structured;
        self.areas.session.pending = None;
        self.areas.session.resuming = None;
        self.areas.session.resume_error = None;
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
        self.conversation_opened(&open.machine, open.session);
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
        self.conversation_records(session, records);
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
        if text.trim().is_empty() || self.areas.session.resuming.is_some() {
            return;
        }
        if open.live {
            self.command(Command::Prompt {
                machine: open.machine.clone(),
                session: open.session,
                text,
            });
        } else {
            // Ended or interrupted: start it again with its conversation,
            // then send (Paseo and GOAT continue a session the same way).
            self.command(Command::Resume {
                machine: open.machine.clone(),
                session: open.session,
                text: text.clone(),
            });
            self.areas.session.resuming = Some(text);
            self.areas.session.resume_error = None;
        }
        self.areas.session.composer.clear(window, cx);
        cx.notify();
    }

    /// The ended session runs again: follow it afresh (its log goes on
    /// after its final status).
    pub(crate) fn resumed(&mut self, session: SessionId, cx: &mut Context<'_, Self>) {
        let Some(open) = self.areas.session.open.clone() else {
            return;
        };
        if open.session != session {
            return;
        }
        self.areas.session.resuming = None;
        self.follow(Open { live: true, ..open }, cx);
    }

    /// It could not be started again: say why, and put the text back.
    pub(crate) fn resume_failed(&mut self, session: SessionId, why: String) {
        let view = &mut self.areas.session;
        if view
            .open
            .as_ref()
            .is_none_or(|open| open.session != session)
        {
            return;
        }
        *view.restore.borrow_mut() = view.resuming.take();
        view.resume_error = Some(why);
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
    window: &mut Window,
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
        super::conversation::timeline(shell, tokens, window, cx)
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
        .child(super::conversation::bar(
            shell,
            t.title(&open.title).to_owned(),
            detail,
            tokens,
            window,
            cx,
        ))
        .child(body)
        .children(
            view.conversation
                .asked()
                .first()
                .map(|asked| permission(asked, shell, tokens, cx)),
        )
        .children(resume_note(shell, open, tokens))
        .when(open.structured, |frame| {
            frame.child(composer(shell, tokens, working, window, cx))
        })
        .children(super::conversation::overlays(shell, tokens, window, cx))
}

/// Under an ended session's conversation: that sending starts it again;
/// while it starts, the prompt and that it is starting; why it could not.
fn resume_note(shell: &Shell, open: &Open, tokens: &Tokens) -> Option<Div> {
    if !open.structured || open.live {
        return None;
    }
    let t = shell.t;
    let view = &shell.areas.session;
    let neutral = tokens.neutral;
    let (text, color) = if let Some(why) = &view.resume_error {
        (t.resume_failed(why), neutral.destructive)
    } else if view.resuming.is_some() {
        (t.resuming.to_owned(), tokens.st_building)
    } else {
        (t.ended.to_owned(), neutral.foreground_muted)
    };
    let mut note = div().w_full().flex().flex_col().gap(px(8.0)).py(px(8.0));
    if let Some(prompt) = &view.resuming {
        note = note.child(bubble(prompt, tokens));
    }
    Some(column(note.child(
        div().type_(Type::Meta).text_color(color).child(text),
    )))
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
                .child(column(
                    div()
                        .py(px(8.0))
                        .type_(Type::Meta)
                        .text_color(tokens.st_building)
                        .child(t.starting_on(&pending.agent, &pending.machine)),
                )),
        )
}

/// The composer, as Paseo's (`composer/input/input.tsx`): the card is the
/// input; the round button at the right sends (queued while a turn runs:
/// Paseo's queue is the same arrow), and while a turn runs with nothing
/// typed it is the red stop.
fn composer(
    shell: &Shell,
    tokens: &Tokens,
    working: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let touch = crate::areas::draft::compact(window);
    let field = &shell.areas.session.composer;
    let restored = shell.areas.session.restore.borrow_mut().take();
    if let Some(text) = restored {
        // A failed resume's text, back in the composer (once).
        field.set_text(&text, window, cx);
    }
    let typed = !field.text(cx).trim().is_empty();
    let stop = working && !typed;
    let button = vessel_app_kit::send_button(
        "session-send",
        if stop {
            vessel_app_kit::SendKind::Stop
        } else {
            vessel_app_kit::SendKind::Send
        },
        stop || typed,
        touch,
        tokens,
    )
    .on_click(cx.listener(move |this, _event, window, cx| {
        if stop {
            this.stop_turn(cx);
        } else {
            let text = this.areas.session.composer.text(cx);
            this.submit_prompt(text, window, cx);
        }
    }));
    column(
        vessel_app_kit::composer_card(tokens, touch)
            .child(field.element(tokens, touch, cx))
            .child(div().flex().flex_row().justify_end().child(button)),
    )
    .pb(px(12.0))
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

/// Paseo's chat column (`agent-stream/view.tsx`, `composer/dock`): the
/// conversation and the composer share one column, at most 820 wide in
/// the middle, 16 from the window's edges, so the composer's card and the
/// messages start at the same left edge (the owner, on 0.0.0.0.5: the card
/// was narrower than the conversation and did not line up with it).
const COLUMN_WIDTH: f32 = 820.0;

/// The column's distance from the window's edges.
const COLUMN_INSET: f32 = 16.0;

/// `content` in the chat column.
pub(crate) fn column(content: impl IntoElement) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .justify_center()
        .px(px(COLUMN_INSET))
        .child(div().w_full().max_w(px(COLUMN_WIDTH)).child(content))
}

/// A person's message, in the chat column.
fn person(text: &str, tokens: &Tokens) -> Div {
    column(bubble(text, tokens))
}

/// A person's message: its bubble, 6 above and under.
fn bubble(text: &str, tokens: &Tokens) -> Div {
    div().w_full().py(px(6.0)).child(
        div()
            .p(px(10.0))
            .bg(tokens.bg_surface)
            .border_1()
            .border_color(tokens.border_subtle)
            .rounded(px(vessel_app_kit::theme::radius::R2))
            .type_(Type::Body)
            .text_color(tokens.text_primary)
            .child(text.to_owned()),
    )
}

use vessel_app_kit::ui::FluentBuilder as _;
use vessel_app_kit::ui::StatefulInteractiveElement as _;
