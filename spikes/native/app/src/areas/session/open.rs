//! The visible session identity shared by navigation and session rendering.
use crate::routes::Route;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::{Card, Conversation};
use vessel_app_view::{machine_book::Name, sessions::SessionAddress};
use vessel_proto::SessionId;

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
    /// Fleet metadata hides the archived composer before session.get answers.
    pub(crate) archived_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Deep-link reads retain the authenticated node across alias replacement.
    pub(crate) binding: Option<vessel_app_link::sessions::Binding>,
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
            archived_at: card.detail.marks.archived_at,
            binding: None,
        }
    }
}

impl crate::shell::Shell {
    /// Read only the originally admitted owner of this exact legacy open target.
    /// Hidden/unbound targets return None; the current Fleet is never authority.
    #[cfg(test)]
    pub(crate) fn opened_session_address(
        &self,
        host: &Name,
        session: SessionId,
    ) -> Option<SessionAddress> {
        let open = self.areas.session.open.as_ref()?;
        if open.machine != host.as_str() || open.session != session {
            return None;
        }
        open.binding.as_ref().map(|binding| binding.address)
    }
    /// Show a session's conversation, following it.
    pub(super) fn follow(&mut self, open: Open, cx: &mut Context<'_, Self>) {
        self.creation_departing();
        self.conversation_opened(&open, cx);
        let view = &mut self.areas.session;
        view.conversation = Conversation::default();
        view.entries.reset(0);
        if !self.terminal_follow(&open, cx) {
            return;
        }
        self.command(open.binding.clone().map_or_else(
            || Command::Open {
                machine: open.machine.clone(),
                session: open.session,
            },
            |binding| Command::SessionAccess(vessel_app_link::sessions::Access::Follow(binding)),
        ));
        self.areas.session.open = Some(open);
        self.route = Route::Session;
        cx.notify();
    }
}

impl crate::shell::Shell {
    /// Open a session from a card or a row, the composer focused.
    pub(crate) fn open_session(
        &mut self,
        mut open: Open,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if open.binding.is_none()
            && let Some(node) = self
                .fleet
                .machines
                .iter()
                .find(|machine| machine.name == open.machine)
                .and_then(|machine| machine.node)
        {
            let Ok(host) = Name::parse(&open.machine) else {
                self.notice = Some(self.words.sessions().navigation_refused().to_owned());
                cx.notify();
                return;
            };
            open.binding = Some(vessel_app_link::sessions::Binding {
                host,
                address: SessionAddress {
                    node,
                    session: open.session,
                },
            });
        }
        self.cancel_session_navigation();
        // Track P: the session composer swaps the draft it holds.
        self.composer_switched(open.session, window, cx);
        // An ended session takes prompts too: sending starts it again.
        let structured = open.structured;
        let takes_prompts = structured && open.archived_at.is_none();
        self.areas.session.pending = None;
        self.areas.session.resuming = None;
        self.areas.session.resume_error = None;
        self.follow(open, cx);
        let placeholder = self
            .t
            .composer_placeholder(&crate::areas::settings::send_key_words(self));
        self.areas
            .session
            .composer
            .set_placeholder(&placeholder, window, cx);
        if takes_prompts {
            self.areas.session.composer.focus(window, cx);
        } else if !structured {
            self.focus_terminal(window, cx);
        }
    }
}
