//! Who sizes a terminal (ADR 0116): Paseo's `terminal-size-ownership.ts`
//! for terminals and terminal-mode sessions alike.
//!
//! [Decision Log]
//! - Purpose: every client showing a terminal fits it to its own pane and
//!   sends the size; with two clients on one terminal, whichever sent last
//!   won, and the shell's size flipped each time either redrew.
//! - Choice: Paseo's rule. A resize is a claim (or has no intent, as every
//!   client below `SHARED_TERMINALS_PROTOCOL` sends) or an update. A claim
//!   applies and makes its connection the owner; an update applies only
//!   from the owner and is otherwise dropped, still answered as done, as
//!   Paseo's returns `false` without an error.
//! - Owner: the connection's identity (`connection_life::Owner`), compared
//!   by pointer and kept alive by the slot, so a later connection can never
//!   match a gone one; a gone owner's terminal takes the next claim.
//! - Cost: a call forwarded through a home node carries that home's
//!   connection, so the clients behind one home share its ownership (they
//!   fight as before among themselves, not with clients connected
//!   directly).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vessel_proto::{ResizeIntent, SessionId};

use crate::connection_life::Owner;

/// One terminal's size owner, shared by the clones of its registry entry
/// and gone with it.
pub(crate) type Slot = Arc<Mutex<Option<Owner>>>;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether a resize from `connection` with `intent` applies to the
/// terminal whose owner `slot` keeps; a claim takes the slot.
pub(crate) fn admit(slot: &Slot, connection: &Owner, intent: Option<ResizeIntent>) -> bool {
    decide(&mut lock(slot), connection, intent)
}

fn decide(owner: &mut Option<Owner>, connection: &Owner, intent: Option<ResizeIntent>) -> bool {
    match intent {
        None | Some(ResizeIntent::Claim) => {
            *owner = Some(connection.clone());
            true
        }
        Some(ResizeIntent::Update) => owner.as_ref() == Some(connection),
    }
}

/// The size owners of terminal-mode sessions, which have no registry entry
/// of the node's to keep one in: one per live session that was resized.
#[derive(Debug, Default)]
pub(crate) struct SessionSizes {
    owners: Mutex<HashMap<SessionId, Owner>>,
}

impl SessionSizes {
    /// As [`admit`], for `session`. A session seen for the first time drops
    /// the owners of sessions no longer `live`, so the map holds at most
    /// the live ones.
    pub(crate) fn admit(
        &self,
        session: SessionId,
        connection: &Owner,
        intent: Option<ResizeIntent>,
        live: impl Fn(SessionId) -> bool,
    ) -> bool {
        let mut owners = lock(&self.owners);
        if !owners.contains_key(&session) {
            owners.retain(|kept, _| live(*kept));
        }
        let mut owner = owners.remove(&session);
        let applies = decide(&mut owner, connection, intent);
        if let Some(owner) = owner {
            drop(owners.insert(session, owner));
        }
        applies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_life::ConnectionLife;

    #[test]
    fn a_claim_takes_the_size_and_only_the_owner_updates_it() {
        let slot = Slot::default();
        let app = ConnectionLife::default();
        let page = ConnectionLife::default();
        // Nobody claimed it: an update has no owner to match.
        assert!(!admit(&slot, &app.owner(), Some(ResizeIntent::Update)));
        assert!(admit(&slot, &app.owner(), Some(ResizeIntent::Claim)));
        assert!(admit(&slot, &app.owner(), Some(ResizeIntent::Update)));
        assert!(!admit(&slot, &page.owner(), Some(ResizeIntent::Update)));
        // No intent is a claim (every older client).
        assert!(admit(&slot, &page.owner(), None));
        assert!(!admit(&slot, &app.owner(), Some(ResizeIntent::Update)));
        assert!(admit(&slot, &page.owner(), Some(ResizeIntent::Update)));
    }

    #[test]
    fn a_gone_owner_never_matches_a_later_connection() {
        let slot = Slot::default();
        let gone = ConnectionLife::default();
        assert!(admit(&slot, &gone.owner(), Some(ResizeIntent::Claim)));
        drop(gone);
        let later = ConnectionLife::default();
        assert!(!admit(&slot, &later.owner(), Some(ResizeIntent::Update)));
        assert!(admit(&slot, &later.owner(), Some(ResizeIntent::Claim)));
    }

    #[test]
    fn sessions_keep_owners_only_while_live() {
        let sizes = SessionSizes::default();
        let ended = SessionId::random();
        let live = SessionId::random();
        let connection = ConnectionLife::default();
        assert!(sizes.admit(ended, &connection.owner(), None, |_| true));
        assert!(sizes.admit(live, &connection.owner(), None, |id| id == live));
        assert_eq!(lock(&sizes.owners).len(), 1);
        assert!(sizes.admit(live, &connection.owner(), Some(ResizeIntent::Update), |_| true));
    }
}
