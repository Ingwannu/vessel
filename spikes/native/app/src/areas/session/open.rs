//! The typed address and metadata for opening a session. Card conversion
//! stays beside the value, independent of composer and conversation drawing.
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
