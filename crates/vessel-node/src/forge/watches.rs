//! Putting a session on a watch and taking it off: one watch per
//! repository and branch, at most [`MAX_WATCHES`], a person's session
//! before an agent's (ADR 0029).

use std::collections::BTreeSet;

use tokio::time::Instant;
use vessel_proto::SessionId;
use vessel_proto::forge::{ForgeAccess, ForgeLogin};

use super::folder::Spot;
use super::watcher::{MAX_WATCHES, Watch, Watcher};

impl Watcher {
    /// Put `session` on the watch for `spot`, within the bound.
    pub(super) fn attach(&mut self, session: SessionId, spot: &Spot, login: Option<ForgeLogin>) {
        let key = spot.key();
        let Some(held) = self.sessions.get_mut(&session) else {
            return;
        };
        held.head = Some(spot.head.clone());
        let person = held.person;
        if held.key.as_ref() == Some(&key) {
            if let Some(watch) = self.watches.get_mut(&key) {
                watch.login = login;
            }
            return;
        }
        self.detach(session);
        if !self.watches.contains_key(&key) && !self.room_for(person) {
            if let Some(held) = self.sessions.get_mut(&session) {
                held.access = ForgeAccess::Limit;
            }
            return;
        }
        let now = Instant::now();
        let watch = self.watches.entry(key.clone()).or_insert_with(|| Watch {
            sessions: BTreeSet::new(),
            person: false,
            login: None,
            error: None,
            listed: Vec::new(),
            known: None,
            read: None,
            seen: false,
            due: now,
            busy: false,
            urgent: false,
            last_find: None,
            last_full: None,
            changed_at: now,
            failures: 0,
            replies: Vec::new(),
        });
        let _new = watch.sessions.insert(session);
        watch.person |= person;
        watch.login = login;
        if let Some(held) = self.sessions.get_mut(&session) {
            held.key = Some(key);
        }
    }

    /// Whether a new watch fits: under the bound, or a person's that takes
    /// the place of a watch only agents' sessions are on.
    fn room_for(&mut self, person: bool) -> bool {
        if self.watches.len() < MAX_WATCHES {
            return true;
        }
        if !person {
            return false;
        }
        let Some(key) = self
            .watches
            .iter()
            .find(|(_, watch)| !watch.person)
            .map(|(key, _)| key.clone())
        else {
            return false;
        };
        let Some(evicted) = self.watches.remove(&key) else {
            return false;
        };
        for session in evicted.sessions {
            if let Some(held) = self.sessions.get_mut(&session) {
                held.key = None;
                held.access = ForgeAccess::Limit;
            }
            self.publish(session);
        }
        true
    }

    /// Take `session` off its watch, dropping a watch left with none.
    pub(super) fn detach(&mut self, session: SessionId) {
        let Some(key) = self
            .sessions
            .get_mut(&session)
            .and_then(|held| held.key.take())
        else {
            return;
        };
        let Some(watch) = self.watches.get_mut(&key) else {
            return;
        };
        let _was = watch.sessions.remove(&session);
        if watch.sessions.is_empty()
            && let Some(watch) = self.watches.remove(&key)
        {
            for (_, reply) in watch.replies {
                drop(reply.send(None));
            }
        }
    }
}
