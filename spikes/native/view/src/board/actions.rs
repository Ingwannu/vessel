//! A card's own actions (the page's `lane.tsx`, `archive.tsx` and
//! `card-actions.tsx`): in a lane, an ended session on a machine that
//! keeps marks offers to be archived and a live one to be killed; in the
//! archive, an archived session offers to be restored and deleted.

use vessel_proto::{ProtocolVersion, SessionId, SessionInfo};

use super::{Card, Fleet};
use crate::session::CONVERSATION_PROTOCOL;

/// What a card's action slot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardActions {
    /// Nothing (an ended session on an older machine, or one in the
    /// archive that was never archived).
    None,
    /// Kill, behind a confirm.
    Kill,
    /// Archive, behind a confirm.
    Archive,
    /// Restore and Delete, the delete behind a confirm.
    Restore {
        /// Restoring starts it again (its agent can pick up its
        /// conversation); else it is only unarchived, back to its lane.
        again: bool,
    },
}

impl Card {
    /// Its actions on a machine speaking `protocol` (`None`: not
    /// connected), in a lane or, `archive`, in the archive.
    #[must_use]
    pub fn actions(&self, protocol: Option<ProtocolVersion>, archive: bool) -> CardActions {
        if archive {
            return if self.detail.marks.archived_at.is_some() {
                CardActions::Restore {
                    again: self.detail.can_start_again,
                }
            } else {
                CardActions::None
            };
        }
        if self.live {
            CardActions::Kill
        } else if protocol.is_some_and(|protocol| protocol >= CONVERSATION_PROTOCOL) {
            CardActions::Archive
        } else {
            CardActions::None
        }
    }
}

impl Fleet {
    /// The protocol `machine` speaks, while it is connected.
    #[must_use]
    pub fn protocol_of(&self, machine: &str) -> Option<ProtocolVersion> {
        self.machines
            .iter()
            .find(|candidate| candidate.name == machine)
            .and_then(|machine| machine.protocol)
    }

    /// Put an act's answer in place of its card on `machine` (the page's
    /// `putSession`): an archived card leaves its lane at once, a restored
    /// one comes back. A card not listed is left for the next read.
    pub fn put_session(&mut self, machine: &str, info: &SessionInfo) {
        let card = Card::from_session(machine, info);
        if let Some(slot) = self
            .machines
            .iter_mut()
            .filter(|candidate| candidate.name == machine)
            .flat_map(|machine| machine.sessions.iter_mut())
            .find(|slot| slot.session == card.session)
        {
            *slot = card;
        }
    }

    /// The title of `session`, on whichever machine lists it (a forked
    /// session's first line names its source); `None` when it is not
    /// listed or has no title yet.
    #[must_use]
    pub fn title_of(&self, session: SessionId) -> Option<&str> {
        self.machines
            .iter()
            .flat_map(|machine| machine.sessions.iter())
            .find(|card| card.session == session)
            .map(|card| card.title.as_str())
            .filter(|title| !title.trim().is_empty())
    }

    /// A session just made from another on `machine` (a fork's answer):
    /// its card in place when listed, else added to the machine's, so the
    /// list shows it before the next read. Nothing on a machine not
    /// listed.
    pub fn add_session(&mut self, machine: &str, info: &SessionInfo) {
        let card = Card::from_session(machine, info);
        let Some(listed) = self
            .machines
            .iter_mut()
            .find(|candidate| candidate.name == machine)
        else {
            return;
        };
        if let Some(slot) = listed
            .sessions
            .iter_mut()
            .find(|slot| slot.session == card.session)
        {
            *slot = card;
        } else {
            listed.sessions.push(card);
        }
    }

    /// Take a deleted session's card off `machine` (the page's
    /// `dropSession`).
    pub fn drop_session(&mut self, machine: &str, session: SessionId) {
        for listed in self
            .machines
            .iter_mut()
            .filter(|candidate| candidate.name == machine)
        {
            listed.sessions.retain(|card| card.session != session);
        }
    }
}
