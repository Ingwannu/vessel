//! A pane's exact selected admission is independent of whichever agent is active.
//!
//! [Decision Log]
//! - Existing assets: W12's `LayoutKey`/`TabId` and G4's opaque `ConnectionContext`.
//! - Choice: locate only the exact legacy Session key through session.get;
//!   real Workspace keys wait for W11's authoritative registry adapter.
//! - Why: guessing a root from cwd, selected session or a first matching node
//!   alias can cross workspace/credential boundaries. Late contexts cannot bind.
//! - Cost: an unhydrated/restored workspace is explicitly unavailable until
//!   its owner supplies the canonical registry projection; no second registry.
use super::{ReadFailure, RepositoryKey};
use crate::connection::ConnectionContext;
use crate::layout::{LayoutKey, TabId};
use crate::workspace::git::RequestKey;
use crate::workspace::root::session_root;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{SessionId, SessionInfo};

/// Presentation ownership includes logical workspace, never `TabId` alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PanelAddress {
    /// Authoritative workspace identity or the explicit legacy Session bridge.
    pub workspace: LayoutKey,
    /// Stable presentation slot from the one pane manager.
    pub tab: TabId,
}

/// Delivery precedes ticket settlement: an old reply cannot retire a new owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerAdmission {
    /// Another owner/admission issued the reply; change nothing here.
    Ignore,
    /// This owner's original admission retired; invalidate it and redraw.
    Retire,
    /// Same live admission; the exact operation ticket must still match.
    MatchTicket,
}

/// Compare the reply with its owner before checking current connection facts.
/// Reversing this order lets an old A reply retire a healthy replacement B.
#[must_use]
pub fn answer_admission(
    owner: Option<&ConnectionContext>,
    current: Option<&ConnectionContext>,
    reply: &ConnectionContext,
) -> AnswerAdmission {
    if owner != Some(reply) {
        AnswerAdmission::Ignore
    } else if current != Some(reply) {
        AnswerAdmission::Retire
    } else {
        AnswerAdmission::MatchTicket
    }
}

/// Root discovery retains the original connection, owner and monotonic request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationTicket {
    address: PanelAddress,
    context: ConnectionContext,
    session: SessionId,
    key: RequestKey,
}
impl LocationTicket {
    /// Route only to this originally selected authenticated admission.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        &self.context
    }
    /// Request the exact session named in the layout's compatibility key.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
    /// Answers are never installed by the currently selected tab alone.
    #[must_use]
    pub const fn address(&self) -> &PanelAddress {
        &self.address
    }
}

/// Refusals before any root discovery request; none starts a fake spinner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LocationRefusal {
    /// W11's registry adapter has not provided a workspace descriptor.
    #[error("authoritative workspace information is not available")]
    NeedsRegistry,
    /// This admission does not belong to the explicit key the pane shows.
    #[error("the selected connection does not match this panel")]
    DestinationChanged,
}

/// Only a located original session may establish this pane's repository.
#[derive(Debug, Default)]
pub enum PanelBinding {
    /// No authority was selected; not a read in progress.
    #[default]
    Idle,
    /// Exactly this discovery may install a root or failure.
    Locating(LocationTicket),
    /// Retained original session facts, rather than a guessed cwd/root.
    Ready {
        /// Selected immutable admission; retirement requires locating again.
        context: ConnectionContext,
        /// Whole original metadata, including genuinely absent workspace.
        session: Box<SessionInfo>,
    },
    /// A settled failure; no automatic endless refresh is implied.
    Failed(ReadFailure),
}
impl PanelBinding {
    /// Begin discovery only for the exact explicit compatibility destination.
    ///
    /// # Errors
    /// A real workspace needs registry admission; a different alias is refused.
    pub fn begin(
        &mut self,
        address: PanelAddress,
        context: ConnectionContext,
        key: RequestKey,
    ) -> Result<LocationTicket, LocationRefusal> {
        let session = match &address.workspace {
            LayoutKey::Workspace { .. } => return Err(LocationRefusal::NeedsRegistry),
            LayoutKey::Session { machine, session } => {
                if machine.as_str() != context.machine().as_str() {
                    return Err(LocationRefusal::DestinationChanged);
                }
                *session
            }
        };
        let ticket = LocationTicket {
            address,
            context,
            session,
            key,
        };
        *self = Self::Locating(ticket.clone());
        Ok(ticket)
    }

    /// Duplicates, retired epochs and same-root A→B→A answers cannot settle.
    #[must_use]
    pub fn finish(
        &mut self,
        ticket: &LocationTicket,
        answer: Result<Box<SessionInfo>, ReadFailure>,
    ) -> bool {
        if !matches!(self, Self::Locating(current) if current == ticket) {
            return false;
        }
        *self = match answer {
            Ok(session) if session.id == ticket.session => Self::Ready {
                context: ticket.context.clone(),
                session,
            },
            Ok(session) => Self::Failed(ReadFailure::UnexpectedSession {
                requested: ticket.session,
                received: session.id,
            }),
            Err(why) => Self::Failed(why),
        };
        true
    }

    /// A newer machine snapshot cannot leave an old discovery or root usable.
    /// Return whether the binding changed, so unchanged fleets cost no redraw.
    pub fn reconcile(&mut self, current: Option<&ConnectionContext>) -> bool {
        let original = match self {
            Self::Locating(ticket) => &ticket.context,
            Self::Ready { context, .. } => context,
            Self::Idle | Self::Failed(_) => return false,
        };
        if current == Some(original) {
            return false;
        }
        *self = Self::Failed(ReadFailure::DestinationChanged);
        true
    }

    /// A genuinely unregistered session returns None; callers never fall back.
    #[must_use]
    pub fn repository(&self) -> Option<RepositoryKey> {
        match self {
            Self::Ready { context, session } => {
                session_root(session).map(|root: WorkspaceRoot| RepositoryKey {
                    node: context.node(),
                    root,
                })
            }
            Self::Idle | Self::Locating(_) | Self::Failed(_) => None,
        }
    }
    /// Current selected admission is returned only with current hydrated facts.
    #[must_use]
    pub const fn context(&self) -> Option<&ConnectionContext> {
        match self {
            Self::Ready { context, .. } => Some(context),
            Self::Idle | Self::Locating(_) | Self::Failed(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
