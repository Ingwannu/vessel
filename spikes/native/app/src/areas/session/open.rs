//! The visible session identity shared by navigation and session rendering.
use vessel_app_view::Card;
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
        }
    }
}
