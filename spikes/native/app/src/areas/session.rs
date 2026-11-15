//! A session: its conversation as it grows, and the composer (DESIGN.md
//! section 6.15: the prompt in the interface face, as the page draws it,
//! monospace being for code and the terminal only; the agent's reply as
//! Markdown body text; thoughts quieter, tool calls as rows with their
//! state; the composer a surface at the foot; over it, the conversation's slot:
//! a permission, a question, the queue).
//!
//! The conversation is a list that draws only what is in view (GPUI's
//! `list`, measured rows), so its cost does not grow with its length.
//!
//! A session started from the draft opens before the node has answered:
//! its first prompt is shown at once with "Starting <agent> on <machine>…"
//! (agent-orchestrator's local echo and its "gets ready" line), and the
//! conversation takes its place when the first record arrives.

use std::cell::Cell;

use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement, ListAlignment, ListState, ParentElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _};
use vessel_app_link::Command;
use vessel_app_view::prefs::ContentWidth;
use vessel_app_view::session::{SendFacts, send_delivery, shows_at_once};
use vessel_app_view::{Card, Conversation};
use vessel_proto::{Delivery, EventRecord, SessionId};

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
        // Track P: the session composer swaps the draft it holds.
        self.composer_switched(open.session, window, cx);
        // An ended session takes prompts too: sending starts it again.
        let takes_prompts = open.structured;
        self.areas.session.pending = None;
        self.areas.session.resuming = None;
        self.areas.session.resume_error = None;
        self.follow(open, cx);
        let placeholder = self
            .t
            .composer_placeholder(&super::settings::send_key_words(self));
        self.areas
            .session
            .composer
            .set_placeholder(&placeholder, window, cx);
        if takes_prompts {
            self.areas.session.composer.focus(window, cx);
        }
    }

    /// Show a session's conversation, following it.
    fn follow(&mut self, open: Open, cx: &mut Context<'_, Self>) {
        self.conversation_opened(&open.machine, open.session, cx);
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
    /// The machine `from` is called `to` now (Settings › Machines, track
    /// E2, after the book was written): the session open and a session
    /// being started follow the new name.
    #[expect(
        dead_code,
        reason = "track E2's Settings › Machines calls it after a book edit (remove on its merge)"
    )]
    pub(crate) fn session_machine_renamed(&mut self, from: &str, to: &str) {
        let view = &mut self.areas.session;
        if let Some(open) = &mut view.open
            && open.machine == from
        {
            to.clone_into(&mut open.machine);
        }
        if let Some(pending) = &mut view.pending
            && pending.machine == from
        {
            to.clone_into(&mut pending.machine);
        }
    }

    /// The machine `name` is forgotten (track E2, after the book was
    /// written): a session of it open or being started is let go, and the
    /// session screen, if it showed one, goes back to the new chat.
    #[expect(
        dead_code,
        reason = "track E2's Settings › Machines calls it after a book edit (remove on its merge)"
    )]
    pub(crate) fn session_machine_forgotten(&mut self, name: &str) {
        let view = &mut self.areas.session;
        if view
            .pending
            .as_ref()
            .is_some_and(|pending| pending.machine == name)
        {
            view.pending = None;
        }
        if view.open.as_ref().is_some_and(|open| open.machine == name) {
            view.open = None;
            view.resuming = None;
            view.resume_error = None;
            *view.restore.borrow_mut() = None;
            if self.route == Route::Session {
                self.route_to(Route::Draft);
            }
        }
    }

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
    pub(crate) fn records(
        &mut self,
        session: &SessionId,
        records: &[EventRecord],
        cx: &mut Context<'_, Self>,
    ) {
        self.conversation_records(session, records, cx);
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
        if !self.session_may_send(&text)
            || self.areas.session.resuming.is_some()
            || self.conversation_cannot_resume()
        {
            return;
        }
        if open.live {
            let delivery = self.delivery();
            // A send shows in the conversation at once (Paseo's pending
            // message; track C's tail), unless it waits in the queue,
            // whose rows show it.
            if shows_at_once(delivery, self.areas.session.conversation.turn_running()) {
                self.conversation_sending(&text, cx);
            }
            if self.session_sends_files(&text, delivery, window, cx) {
                return;
            }
            let Some(open) = &self.areas.session.open else {
                return;
            };
            self.command(Command::Prompt {
                machine: open.machine.clone(),
                session: open.session,
                text,
                delivery,
            });
        } else {
            // Ended or interrupted: start it again with its conversation,
            // then send (Paseo and GOAT continue a session the same way).
            // The message shows in the conversation at once, the agent's
            // working footer under it, while the session starts.
            self.conversation_sending(&text, cx);
            let Some(open) = &self.areas.session.open else {
                return;
            };
            self.command(Command::Resume {
                machine: open.machine.clone(),
                session: open.session,
                text: text.clone(),
            });
            self.areas.session.resuming = Some(text);
            self.areas.session.resume_error = None;
        }
        self.session_composer_sent(window, cx);
        cx.notify();
    }

    /// What a plain send does: Settings' default send by Paseo's rules
    /// (`view::session::send_delivery`), given whether a turn runs, whether
    /// the agent takes a message into it, and whether a permission waits.
    fn delivery(&self) -> Delivery {
        send_delivery(
            self.prefs.settings.composer.delivery,
            SendFacts {
                turn_running: self.areas.session.conversation.turn_running(),
                steers: self.steers(),
                permission_waits: self.areas.conversation.permission_waits(),
            },
        )
    }

    /// Whether the open session's agent takes a message into a running
    /// turn.
    pub(crate) fn steers(&self) -> bool {
        self.areas
            .conversation
            .timeline
            .facts()
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.steer)
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
        self.conversation_send_failed();
    }

    /// A prompt the link did not take: its text back in the composer of
    /// the session it was typed in (the composer emptied when it was
    /// sent).
    pub(crate) fn prompt_unsent(&mut self, session: SessionId, text: String) {
        let view = &mut self.areas.session;
        if view
            .open
            .as_ref()
            .is_some_and(|open| open.session == session)
        {
            *view.restore.borrow_mut() = Some(text);
            self.conversation_send_failed();
        }
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
        return starting(pending, shell, tokens, window, cx);
    }
    let Some(open) = &view.open else {
        return div();
    };
    let detail = format!("{} · {}", open.machine, open.agent);
    let body: AnyElement = if open.structured {
        crate::region::timeline(shell, tokens, window, cx)
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
    let screen = div()
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
        .children(resume_note(shell, open, tokens))
        .when(open.structured, |frame| {
            // The conversation's slot over the card (`conversation::dock`:
            // the queue), at the card's width.
            let dock = super::conversation::dock(shell, tokens, window, cx);
            // An ended session its agent cannot pick up again has no
            // composer: the banner in the dock says so (Paseo's archived
            // agent shows only its callout); a send would only try to
            // start it again.
            let ended_for_good = shell.conversation_cannot_resume();
            // The draft's pills on the session's own options (Paseo's
            // AgentControls on a running agent).
            let touch = crate::areas::draft::compact(window);
            let (controls, over) =
                crate::areas::draft::running_controls(shell, touch, tokens, window, cx).unzip();
            frame
                .children(dock.map(column))
                .when(!ended_for_good, |frame| {
                    frame.child(composer(shell, tokens, working, controls, window, cx))
                })
                .children(over.into_iter().flatten())
        })
        .children(super::conversation::overlays(shell, tokens, window, cx));
    // Track F's slot: the screen, with the code panel beside it when out.
    crate::areas::workspace::code_beside(shell, screen, tokens, window, cx)
}

/// Under an ended session's conversation: why it could not be started
/// again (Paseo's send error, its words back in the composer). While it
/// starts, the message is in the conversation with the agent's working
/// footer under it (track C's tail, slice M): nothing is said here.
fn resume_note(shell: &Shell, open: &Open, tokens: &Tokens) -> Option<Div> {
    if !open.structured || open.live {
        return None;
    }
    let why = shell.areas.session.resume_error.as_ref()?;
    Some(column(
        div().w_full().py(px(8.0)).child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.destructive)
                .child(shell.t.resume_failed(why)),
        ),
    ))
}

/// The first prompt, shown while the session starts; in a new worktree,
/// track F's card of its steps in place of the line that says so.
fn starting(
    pending: &Pending,
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let t = shell.t;
    let card = crate::areas::workspace::start_card(shell, tokens, window, cx);
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
                .child(column(card.map_or_else(
                    || {
                        div()
                            .py(px(8.0))
                            .type_(Type::Meta)
                            .text_color(tokens.st_building)
                            .child(t.starting_on(&pending.agent, &pending.machine))
                    },
                    |card| div().py(px(8.0)).child(card),
                ))),
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
    controls: Option<Div>,
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
    let typed = !field.text(cx).trim().is_empty() || shell.session_has_files();
    let stop = working && !typed;
    let button = vessel_app_kit::send_button(
        "session-send",
        if stop {
            vessel_app_kit::SendKind::Stop
        } else {
            vessel_app_kit::SendKind::Send
        },
        stop || shell.session_may_send(&field.text(cx)),
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
    // Track P: the card with its `/` menu, its files and its text.
    column(
        crate::areas::composer::session_card(shell, tokens, touch, window, cx).child(
            // The draft's row: the agent's controls on the left (Paseo's
            // button row bleeds 6 past the card's padding), the meter
            // and send/stop on the right.
            div()
                .mx(px(-6.0))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(crate::areas::composer::attach_button(
                    false, touch, tokens, cx,
                ))
                .child(div().flex_1().min_w_0().children(controls))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .children(super::conversation::meter(shell, tokens, window, cx))
                        .child(button),
                ),
        ),
    )
    .pb(px(12.0))
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

thread_local! {
    /// How wide the chat column grows (Settings › Appearance › Content
    /// width); `None`: the window's width.
    ///
    /// [Decision Log]
    /// - Purpose: the person's content width, for the conversation and
    ///   the composer under it together (the page's `--content-width` on
    ///   `.timeline-column` and `.session-dock-column`).
    /// - Choice: one value read by [`column`], which every row of the
    ///   conversation and the composer is drawn in, set by
    ///   [`set_content_width`]. Kept beside the column rather than passed
    ///   to it: the conversation's rows are drawn by its list where the
    ///   settings are not at hand, and one value keeps the composer and
    ///   the messages on the same edges.
    /// - Difference: "Comfortable" is Paseo's chat column, 820, where the
    ///   page takes agent-orchestrator's 768 (the owner's rule: Paseo's
    ///   app); "Wide" is GOAT's 1152 as the page's, "Full" the window's.
    static CONTENT_WIDTH: Cell<Option<f32>> = const { Cell::new(Some(COLUMN_WIDTH)) };
}

/// The chat column grows to `width` (the next frame draws it).
pub(crate) fn set_content_width(width: ContentWidth) {
    // The native default follows Paseo's 820-point column. Every explicit
    // width uses its validated points, including future custom widths.
    let points = if width == ContentWidth::Comfortable {
        Some(COLUMN_WIDTH)
    } else {
        width.points().map(f32::from)
    };
    CONTENT_WIDTH.set(points);
}

/// `content` in the chat column.
pub(crate) fn column(content: impl IntoElement) -> Div {
    let inner = div().w_full().child(content);
    let inner = match CONTENT_WIDTH.get() {
        Some(width) => inner.max_w(px(width)),
        None => inner,
    };
    div()
        .w_full()
        .flex()
        .flex_row()
        .justify_center()
        .px(px(COLUMN_INSET))
        .child(inner)
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
