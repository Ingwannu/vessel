//! The visible session identity shared by navigation and session rendering.
use crate::routes::Route;
use vessel_app_kit::ui::Context;
use vessel_app_link::Command;
use vessel_app_view::{Card, Conversation};
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
    /// Show a session's conversation, following it.
    pub(super) fn follow(&mut self, open: Open, cx: &mut Context<'_, Self>) {
        self.conversation_opened(&open.machine, open.session, open.binding.as_ref(), cx);
        let view = &mut self.areas.session;
        view.conversation = Conversation::default();
        view.entries.reset(0);
        if open.structured {
            self.command(open.binding.clone().map_or_else(
                || Command::Open {
                    machine: open.machine.clone(),
                    session: open.session,
                },
                |binding| {
                    Command::SessionAccess(vessel_app_link::sessions::Access::Follow(binding))
                },
            ));
        }
        self.areas.session.open = Some(open);
        self.route = Route::Session;
        cx.notify();
    }
}
