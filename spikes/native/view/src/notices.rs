//! What is worth telling the person (the page's `data/notices.ts`): a
//! session that starts to need them, or one that was working and
//! finished, between two reads of the machines. The app raises the
//! system's notification for each (`app`'s notifier), when Settings ›
//! Notifications asks for it.
//!
//! [Decision Log] The first sighting is never a notice (the page's,
//! milestone 11)
//! - Purpose: opening the app, or a machine coming back, must not set off
//!   a notification for every session that was already waiting.
//! - Choice: a session absent from the earlier read says nothing; one
//!   that turns to "needs you" is told once, until it leaves that state;
//!   one that was working and now rests (idle, exited, ended) is
//!   "finished". An answered question that goes back to work says
//!   nothing.

use std::collections::HashMap;

use vessel_proto::SessionId;

use crate::board::{Card, Fleet, Phrase};

mod announce;
pub mod claims;
pub mod terminals;
mod tone;

pub use announce::{Announce, Seen, Unseen, announce};
pub use tone::samples;

/// Why the app tells the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoticeKind {
    /// The session waits for them.
    NeedsYou,
    /// The session was working and stopped.
    Finished,
}

/// One thing to tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Why.
    pub kind: NoticeKind,
    /// The session's machine.
    pub machine: String,
    /// The session.
    pub session: SessionId,
    /// Its name, for the notification's body.
    pub title: String,
}

/// What a session was doing, as far as notices care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Doing {
    NeedsYou,
    Working,
    Resting,
}

fn doing(card: &Card) -> Doing {
    if card.needs_you() {
        return Doing::NeedsYou;
    }
    if card.is_working() {
        return Doing::Working;
    }
    // An older node says no phrase: a live process counts as working.
    let from_status = !matches!(card.phrase, Phrase::Node(_));
    if from_status && card.live {
        Doing::Working
    } else {
        Doing::Resting
    }
}

/// The notices between two reads of the machines. Sessions not in
/// `before` (or whose machine was not) are first sightings and say
/// nothing.
#[must_use]
pub fn notices(before: &Fleet, after: &Fleet) -> Vec<Notice> {
    let was: HashMap<(&str, SessionId), Doing> = before
        .machines
        .iter()
        .flat_map(|machine| &machine.sessions)
        .map(|card| ((card.machine.as_str(), card.session), doing(card)))
        .collect();
    after
        .machines
        .iter()
        .flat_map(|machine| &machine.sessions)
        .filter_map(|card| {
            let then = *was.get(&(card.machine.as_str(), card.session))?;
            let kind = match (doing(card), then) {
                (Doing::NeedsYou, Doing::Working | Doing::Resting) => NoticeKind::NeedsYou,
                // A failure is not announced (Paseo never notifies an
                // error; ADR 0114): it stays on the card and in the inbox.
                (Doing::Resting, Doing::Working) if card.detail.process == Phrase::Failed => {
                    return None;
                }
                (Doing::Resting, Doing::Working) => NoticeKind::Finished,
                (Doing::NeedsYou, Doing::NeedsYou)
                | (Doing::Working, Doing::NeedsYou | Doing::Working | Doing::Resting)
                | (Doing::Resting, Doing::NeedsYou | Doing::Resting) => return None,
            };
            Some(Notice {
                kind,
                machine: card.machine.clone(),
                session: card.session,
                title: card.title.clone(),
            })
        })
        .collect()
}

/// The window's title with `count` notices not yet seen: "(2) Vessel".
#[must_use]
pub fn title_with(base: &str, count: usize) -> String {
    if count > 0 {
        format!("({count}) {base}")
    } else {
        base.to_owned()
    }
}
