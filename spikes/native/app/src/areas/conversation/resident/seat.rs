//! Staged successor of the `Copy` fn-pointer Seat. A concrete context travels
//! unchanged into a row closure; resolution never reacquires a selected session.
use super::{ConversationArea, SessionContext, Shell};

/// Derived local views share their parent's admission, not a fabricated session.
/// Product rendering adopts this in W12's storage/follow cutover, not alongside
/// the old main-session `Seat::SESSION` path.
#[derive(Debug, Clone)]
pub(crate) enum ResidentSeat {
    Main(SessionContext),
    Origin(SessionContext),
}

impl ResidentSeat {
    pub(crate) fn main(context: SessionContext) -> Self {
        Self::Main(context)
    }

    pub(crate) fn origin(context: SessionContext) -> Self {
        Self::Origin(context)
    }

    /// Missing/stale parent or absent derived view is a refusal, never a fallback.
    pub(crate) fn get<'a>(&self, shell: &'a Shell) -> Option<&'a ConversationArea> {
        let (Self::Main(context) | Self::Origin(context)) = self;
        let area = shell.conversation_scoped(context).ok()?;
        match self {
            Self::Main(_) => Some(area),
            Self::Origin(_) => area.origin.area(),
        }
    }

    /// Row input must still belong to the exact visible/focused presentation.
    /// Retained field callbacks use the separate owner local-claim refresh seam.
    pub(crate) fn get_mut<'a>(&self, shell: &'a mut Shell) -> Option<&'a mut ConversationArea> {
        let (Self::Main(context) | Self::Origin(context)) = self;
        let area = shell.conversation_input_scoped(context).ok()?;
        match self {
            Self::Main(_) => Some(area),
            Self::Origin(_) => area.origin.area_mut(),
        }
    }
}
