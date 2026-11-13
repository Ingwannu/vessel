//! The window's one view: what is shown (the board, a session, the pairing
//! form), the link's events folded into it, and the frame the screens are
//! drawn in (the sidebar on the shell, the page framed beside it, as the
//! page's layout has it: DESIGN.md section 6).

use tokio::sync::mpsc;
use vessel_app_kit::ui::{
    AsyncApp, Context, IntoElement, ListAlignment, ListState, ParentElement as _, Render,
    Styled as _, WeakEntity, Window, div, px,
};
use vessel_app_kit::{TextField, tokens};
use vessel_app_link::{Command, Event, Link, Offer};
use vessel_app_view::{Conversation, Fleet};
use vessel_proto::SessionId;

use crate::screens;

/// Which screen fills the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Screen {
    /// The board's lanes.
    Board,
    /// One session's conversation.
    Session,
    /// The pairing form.
    Pair,
}

/// The session on screen.
#[derive(Debug, Clone)]
pub(crate) struct Open {
    pub(crate) machine: String,
    pub(crate) session: SessionId,
    pub(crate) title: String,
    pub(crate) agent: String,
    pub(crate) structured: bool,
}

/// The window's state.
pub(crate) struct Shell {
    link: Option<Link>,
    pub(crate) fleet: Fleet,
    pub(crate) screen: Screen,
    pub(crate) open: Option<Open>,
    pub(crate) conversation: Conversation,
    pub(crate) entries: ListState,
    pub(crate) composer: TextField,
    pub(crate) pair_link: TextField,
    pub(crate) pair_name: TextField,
    pub(crate) offer: Option<Offer>,
    /// The last thing that failed or happened, shown in the frame's foot.
    pub(crate) notice: Option<String>,
    /// Whether the book has been read once (an empty fleet before then is
    /// not "no machines").
    pub(crate) loaded: bool,
}

impl Shell {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let composer = TextField::new(
            "Ask the agent… (Enter sends)",
            |this: &mut Self, text, window, cx| this.submit_prompt(text, window, cx),
            window,
            cx,
        );
        let pair_link = TextField::new(
            "Paste the link from `vessel node pair`",
            |this: &mut Self, text, window, cx| this.read_offer(&text, window, cx),
            window,
            cx,
        );
        let pair_name = TextField::new(
            "A name for this machine, e.g. laptop",
            |this: &mut Self, text, _window, cx| this.pair(text, cx),
            window,
            cx,
        );
        let (link, notice) = match Link::start(crate::LABEL) {
            Ok((link, events)) => {
                cx.spawn(async move |this, cx| receive(this, events, cx).await)
                    .detach();
                (Some(link), None)
            }
            Err(error) => (
                None,
                Some(format!("The link to nodes could not start: {error}")),
            ),
        };
        Self {
            link,
            fleet: Fleet::default(),
            screen: Screen::Board,
            open: None,
            conversation: Conversation::default(),
            entries: ListState::new(0, ListAlignment::Bottom, px(800.0)),
            composer,
            pair_link,
            pair_name,
            offer: None,
            notice,
            loaded: false,
        }
    }

    fn command(&self, command: Command) {
        if let Some(link) = &self.link {
            link.send(command);
        }
    }

    fn on_event(&mut self, event: Event, cx: &mut Context<'_, Self>) {
        match event {
            Event::Fleet(fleet) => {
                self.fleet = fleet;
                if !self.loaded {
                    self.loaded = true;
                    if self.fleet.machines.is_empty() {
                        self.screen = Screen::Pair;
                    }
                }
            }
            Event::Records { session, records } => {
                if self
                    .open
                    .as_ref()
                    .is_none_or(|open| open.session != session)
                {
                    return;
                }
                let before = self.conversation.entries().len();
                for record in &records {
                    self.conversation.apply(record);
                }
                let after = self.conversation.entries().len();
                // The last entry may have grown (a chunk), and new ones
                // follow it: measure it again with them.
                let from = before.saturating_sub(1);
                self.entries
                    .splice(from..before, after.saturating_sub(from));
            }
            Event::Sent { .. } => self.notice = None,
            Event::Paired(done) => {
                self.notice = Some(done);
                self.offer = None;
                self.screen = Screen::Board;
            }
            Event::Failed(why) => self.notice = Some(why),
        }
        cx.notify();
    }

    /// Show a session's conversation, following it.
    pub(crate) fn open_session(
        &mut self,
        open: Open,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.conversation = Conversation::default();
        self.entries.reset(0);
        if open.structured {
            self.command(Command::Open {
                machine: open.machine.clone(),
                session: open.session.clone(),
            });
            self.composer.focus(window, cx);
        }
        self.open = Some(open);
        self.screen = Screen::Session;
        cx.notify();
    }

    pub(crate) fn show(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<'_, Self>) {
        if screen == Screen::Pair {
            self.pair_link.focus(window, cx);
        }
        self.screen = screen;
        cx.notify();
    }

    pub(crate) fn submit_prompt(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(open) = &self.open else {
            return;
        };
        if text.trim().is_empty() {
            return;
        }
        self.command(Command::Prompt {
            machine: open.machine.clone(),
            session: open.session.clone(),
            text,
        });
        self.composer.clear(window, cx);
    }

    fn read_offer(&mut self, text: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        match vessel_app_link::read_link(text) {
            Ok(offer) => {
                self.offer = Some(offer);
                self.notice = None;
                // The link is a single-use secret: once decoded it is held
                // only as the offer, not left on screen.
                self.pair_link.clear(window, cx);
                self.pair_name.focus(window, cx);
            }
            Err(error) => self.notice = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(crate) fn pair(&mut self, name: String, cx: &mut Context<'_, Self>) {
        let Some(offer) = self.offer.clone() else {
            self.notice = Some("Paste a pairing link first".to_owned());
            cx.notify();
            return;
        };
        if name.trim().is_empty() {
            self.notice = Some("Give the machine a name".to_owned());
            cx.notify();
            return;
        }
        self.notice = Some("Pairing…".to_owned());
        self.command(Command::Pair { offer, name });
        cx.notify();
    }

    pub(crate) fn pair_from_field(&mut self, cx: &mut Context<'_, Self>) {
        let name = self.pair_name.text(cx);
        self.pair(name, cx);
    }

    pub(crate) fn answer(
        &mut self,
        id: vessel_proto::PermissionId,
        option: vessel_proto::PermissionOptionId,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(open) = &self.open else {
            return;
        };
        self.command(Command::Answer {
            machine: open.machine.clone(),
            session: open.session.clone(),
            id,
            option,
        });
        cx.notify();
    }

    pub(crate) fn reload(&self) {
        self.command(Command::Reload);
    }
}

/// The link's events, into the view, until either side closes.
async fn receive(this: WeakEntity<Shell>, mut events: mpsc::Receiver<Event>, cx: &mut AsyncApp) {
    while let Some(event) = events.recv().await {
        if this
            .update(cx, |shell, cx| shell.on_event(event, cx))
            .is_err()
        {
            return;
        }
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = tokens(cx);
        let frame = match self.screen {
            Screen::Board => screens::board::render(self, &tokens, cx).into_any_element(),
            Screen::Session => {
                screens::session::render(self, &tokens, window, cx).into_any_element()
            }
            Screen::Pair => screens::pair::render(self, &tokens, cx).into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(tokens.bg_shell)
            .text_color(tokens.text_primary)
            .child(screens::sidebar::render(self, &tokens, cx))
            .child(
                div().flex_1().h_full().py(px(8.0)).pr(px(8.0)).child(
                    div()
                        .size_full()
                        .flex()
                        .flex_col()
                        .bg(tokens.bg_base)
                        .border_1()
                        .border_color(tokens.border_faint)
                        .rounded(px(vessel_app_kit::theme::radius::R5))
                        .overflow_hidden()
                        .child(frame)
                        .children(
                            self.notice
                                .clone()
                                .map(|notice| screens::foot(notice, &tokens)),
                        ),
                ),
            )
    }
}
